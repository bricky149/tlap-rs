/*
	Copyright 2021-2022 Bricky <bricky149@teknik.io>

    This file is part of tlap.

    tlap is free software: you can redistribute it and/or modify
    it under the terms of the GNU Lesser General Public License as
    published by the Free Software Foundation, either version 3 of
    the License, or (at your option) any later version.

    tlap is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
    GNU Lesser General Public License for more details.

    You should have received a copy of the GNU Lesser General Public
    License along with tlap. If not, see <https://www.gnu.org/licenses/>.
*/

use coqui_stt::Model;
use cpal::{BufferSize, SampleRate, StreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::Subtitle;

#[cfg(target_os = "linux")]
use hound::{SampleFormat, WavSpec, WavWriter};

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
use std::time::Instant;

// 16000 samples * 4 seconds
const FOUR_SECONDS :usize = 64000;
// Use current working directory
const RECORDING_PATH: &str = "recording.wav";
// Model has been trained on this specific sample rate
const SAMPLE_RATE :u32 = 16000;

pub fn get_model() -> Model {
    let dir_path = Path::new("models/");
    let mut graph_name = String::new();
    let mut scorer_name = String::new();

    // Search for model and scorer
    for file in dir_path.read_dir().unwrap().flatten() {
        let file_path = file.path();
        
        if file_path.is_file() {
            if let Some(ext) = file_path.extension() {
                if ext == "tflite" {
                    graph_name = file_path.display().to_string()
                } else if ext == "scorer" {
                    scorer_name = file_path.display().to_string()
                }
            }
        }
    }

    // Return loaded model and optional scorer
    let mut m = Model::new(graph_name).unwrap();
    if !scorer_name.is_empty() {
        m.enable_external_scorer(scorer_name).unwrap()
    }
    m
}

/*
    This was inspired by code from Tyler Anton (https://github.com/tylerdotdev)
    https://stackoverflow.com/questions/67105792/getting-blank-results-from-deepspeech-with-portaudio-in-rust
*/
pub fn record_input(mut model :Model) {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No input device available");
    let config = get_input_config();

    // Prepare audio recording file for writing
    let spec = get_spec();

    let writer = hound::WavWriter::create(RECORDING_PATH, spec).unwrap();
    let writer = Arc::new(Mutex::new(Some(writer)));

    let err_fn = |err| eprintln!("An error occurred on the input audio stream: {}", err);

    let stream = device.build_input_stream(
        &config,
        move |data, _: &_| {
            write_input_data(data, &writer)
        },
        err_fn,
    ).unwrap();
    stream.play().unwrap();

    // Subtitle properties
    let start = Instant::now();
    let mut sub_count = 1;
    let mut past_ts = 0;

    loop {
        // Wait for enough input to process
        std::thread::sleep(std::time::Duration::from_millis(4000));

        let samples = get_audio_samples(RECORDING_PATH.into());
        let samples_length = samples.len();

        let new_samples = if samples_length > FOUR_SECONDS {
            let cursor = samples_length - FOUR_SECONDS;
            samples.split_at(cursor).1
        } else {
            // First loop falls short of 64K samples
            samples.split_at(0).1
        };

        match model.speech_to_text(new_samples) {
            Ok(text) => {
                let now = start.elapsed().as_millis();
                let sub = Subtitle::from(sub_count, past_ts, now, text);

                match Subtitle::flush_one(sub) {
                    Ok(()) => println!("Running for {} seconds...",
                                        format_args!("{}", now / 1000)),
                    Err(e) => eprintln!("Error writing subtitles: {:?}", e)
                };

                // Prepare for next iteration
                sub_count += 1;
                past_ts = now
            }
            Err(e) => eprintln!("Coqui error: {:?}", e)
        }
    }

    // Unreachable for now
    //drop(stream)
}

pub fn get_transcript(mut model :Model, sample_lines :Vec<[i16;64000]>) -> Vec<String> {
    // Store an hours' worth of lines before realloc
    let mut sub_lines = Vec::with_capacity(720);

    for line in sample_lines {
        match model.speech_to_text(&line) {
            Ok(text) => {
                println!("{}", text);
                sub_lines.push(text)
            },
            Err(e) => eprintln!("Coqui error: {:?}", e)
        }
    }

    sub_lines
}

/*
	This code was inspired by the RustAudio example client.
	https://github.com/RustAudio/deepspeech-rs/blob/master/examples/client_simple.rs
*/
pub fn get_audio_samples(audio_path :String) -> Vec<i16> {
    let mut reader = hound::WavReader::open(audio_path).unwrap();
    let samples :Vec<i16> = reader.samples()
                                .map(|s| s.unwrap())
                                .collect();
    samples
}

pub fn split_audio_lines(audio_buffer :Vec<i16>) -> Vec<[i16;64000]> {
    let mut audio_lines :Vec<[i16;64000]> = Vec::new();

    let audio_length = audio_buffer.len();
    let total_lines = audio_length / FOUR_SECONDS;

    let mut current_samples;
    let mut samples_to_process = audio_buffer.split_at(0).1;
    let mut sample_buffer;

    for line_num in 0..=total_lines {
        if line_num == 0 {
            current_samples = audio_buffer.split_at(audio_length).0;
            sample_buffer = samples_to_process;
            samples_to_process = sample_buffer
        } else if line_num < total_lines {
            current_samples = samples_to_process.split_at(FOUR_SECONDS).0;
            sample_buffer = samples_to_process.split_at(FOUR_SECONDS).1;
            samples_to_process = sample_buffer
        } else {
            current_samples = samples_to_process
        };

        match current_samples.try_into() {
            Ok(l) => audio_lines.push(l),
            Err(e) => eprintln!("{:?}", e)
        };
    }

    audio_lines
}

fn get_input_config() -> StreamConfig {
    StreamConfig {
        channels: 1,
        sample_rate: SampleRate(16000),
        buffer_size: BufferSize::Fixed(1024)
    }
}

fn get_spec() -> WavSpec {
    WavSpec {
        bits_per_sample: 16,
        channels: 1,
        sample_format: SampleFormat::Int,
        sample_rate: SAMPLE_RATE
    }
}

type WavWriterHandle = Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>;

fn write_input_data(input :&[i16], writer :&WavWriterHandle) {
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                writer.write_sample(sample).ok();
            }
            writer.flush().ok();
        }
    }
}
