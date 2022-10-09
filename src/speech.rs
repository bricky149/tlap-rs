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
use hound::{SampleFormat, WavSpec};

use crate::Subtitle;

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// 16000 samples * 4 seconds
const FOUR_SECONDS :usize = 64000;
// Use current directory for live transcriptions
const LIVE_RECORDING_PATH :&str = "recording.wav";
const LIVE_SUBTITLES_PATH :&str = "recording.srt";
// Model has been trained on this specific sample rate
const SAMPLE_RATE :u32 = 16000;

pub fn get_model(dir :&str) -> Option<Model> {
    let dir_path = Path::new(dir);
    let mut model_path = String::new();
    let mut scorer_path = String::new();

    for entry in dir_path.read_dir().expect("Unable to read models dir") {
        if let Ok(entry) = entry {
            let file_path = entry.path();
        
            if file_path.is_file() {
                if let Some(ext) = file_path.extension() {
                    if ext == "tflite" {
                        model_path = file_path.display().to_string()
                    } else if ext == "scorer" {
                        scorer_path = file_path.display().to_string()
                    }
                }
            }
        } else {
            return None
        }
    }

    let mut m = Model::new(model_path).expect("Unable to open model file");
    if !scorer_path.is_empty() {
        m.enable_external_scorer(scorer_path).expect("Unable to open scorer file")
    }
    Some(m)
}

/*
    This was inspired by code from Tyler Anton (https://github.com/tylerdotdev)
    https://stackoverflow.com/questions/67105792/getting-blank-results-from-deepspeech-with-portaudio-in-rust
*/
pub fn record_input(model :Model) {
    // Capture input device
    let host = cpal::default_host();
    let device = host.default_input_device().expect("No input device available");
    let config = get_input_config();

    // Prepare audio recording file for writing
    let spec = get_spec();
    let mut writer = hound::WavWriter::create(LIVE_RECORDING_PATH, spec)
                        .expect("Unable to create audio recording");

    let err_fn = |err|
        eprintln!("Input audio stream error: {}", err);

    let stream = device.build_input_stream(
        &config,
        move |data :&[i16], _: &_| {
            for &sample in data.iter() {
                writer.write_sample(sample).expect("Unable to write sample")
            }
            // Write samples to file to avoid reading
            // nothing back when streaming
            writer.flush().expect("Unable to flush samples to file")
        },
        err_fn,
    ).expect("Unable to build input stream");
    stream.play().expect("Unable to play input stream");

    // Subtitle properties
    let start = Instant::now();
    let mut sub_count = 1;
    let mut past_ts = 0;

    // Allows model to be used across threads
    let model_mutex = Mutex::new(model);
    let model_arc = Arc::new(model_mutex);

    loop {
        // Wait for enough input to process
        thread::sleep(Duration::from_secs(4));

        let now_ts = start.elapsed().as_millis();
        let thread_arc = model_arc.clone();

        thread::spawn(move || {
            if let Ok(mut model) = thread_arc.try_lock() {
                let samples = get_audio_samples(LIVE_RECORDING_PATH.into());
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
                        let sub = Subtitle::from(sub_count, past_ts, now_ts, text.clone());

                        match sub.write_to(LIVE_SUBTITLES_PATH.into()) {
                            Ok(()) => println!("{}", text),
                            Err(e) => eprintln!("Error writing subtitles: {}", e)
                        };
                    }
                    Err(e) => eprintln!("Error running Coqui: {}", e)
                }
            } else {
                eprintln!("Unable to use speech model, trying again in four seconds.")
            }
        });

        // Prepare for next iteration
        sub_count += 1;
        past_ts = now_ts
    }

    // Unreachable for now
    //drop(stream)
}

pub fn get_transcript(mut model :Model, sample_lines :Vec<[i16;64000]>,
    subs_path :String) {

    let mut sub_count = 1;
    let mut timestamp = 0;

    for line in sample_lines {
        match model.speech_to_text(&line) {
            Ok(text) => {
                let sub = Subtitle::from_line(sub_count, timestamp, text.clone());

                match sub.write_to(subs_path.clone()) {
                    Ok(()) => println!("{}", text),
                    Err(e) => eprintln!("Error writing subtitles: {}", e)
                };
            },
            Err(e) => eprintln!("Error running Coqui: {}", e)
        }

        // Prepare for next iteration
        sub_count += 1;
        timestamp += 4000;
    }
}

/*
	This code was inspired by the RustAudio example client
	https://github.com/RustAudio/deepspeech-rs/blob/master/examples/client_simple.rs
*/
pub fn get_audio_samples(audio_path :String) -> Vec<i16> {
    let mut reader = hound::WavReader::open(audio_path)
                                .expect("Invalid Wave file");
    let samples :Vec<i16> = reader.samples()
                                .map(|s| s.expect("Unable to read sample"))
                                .collect();
    samples
}

pub fn split_audio_lines(audio_buffer :Vec<i16>) -> Vec<[i16;64000]> {
    let mut audio_lines :Vec<[i16;64000]> = Vec::new();

    let audio_length = audio_buffer.len();
    let total_lines = audio_length / FOUR_SECONDS;

    let mut current_samples;
    let mut samples_to_process;
    let mut sample_buffer = audio_buffer.split_at(0).1;

    for _ in 0..=total_lines {
        let remaining_length = sample_buffer.len();

        if remaining_length >= FOUR_SECONDS {
            // We are interested in the last four seconds' worth of samples
            current_samples = sample_buffer.split_at(FOUR_SECONDS).0;
            samples_to_process = sample_buffer.split_at(FOUR_SECONDS).1;
            sample_buffer = samples_to_process;

            match current_samples.try_into() {
                Ok(l) => audio_lines.push(l),
                Err(e) => eprintln!("{}", e)
            };
        } else {
            // Put remaining chunks in a vector and fill the empty
            // space with zeroes so it is the right size
            current_samples = sample_buffer;
            let len = FOUR_SECONDS - current_samples.len();
            let mut last_samples = vec![0i16;len];

            for s in current_samples {
                last_samples.push(*s)
            }

            match last_samples.try_into() {
                Ok(l) => audio_lines.push(l),
                Err(e) => eprintln!("{:?}", e)
            };
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
