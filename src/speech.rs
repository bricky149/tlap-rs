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
use crate::enums::*;

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

    if let Ok(rd) = dir_path.read_dir() {
        for entry in rd {
            if let Ok(entry) = entry {
                let file_path = entry.path();

                if let Some(ext) = file_path.extension() {
                    if ext == "tflite" {
                        model_path = file_path.display().to_string()
                    } else if ext == "scorer" {
                        scorer_path = file_path.display().to_string()
                    }
                }
            } else {
                continue
            }
        }
    } else {
        return None
    }

    if let Ok(mut m) = Model::new(model_path) {
        if let Err(e) = m.enable_external_scorer(scorer_path) {
            eprintln!("Continuing without scorer due to: {}", e)
        }
        Some(m)
    } else {
        None
    }
}

/*
    This was inspired by code from Tyler Anton (https://github.com/tylerdotdev)
    https://stackoverflow.com/questions/67105792/getting-blank-results-from-deepspeech-with-portaudio-in-rust
*/
pub fn record_input(model :Model) -> Result<(), TlapError> {
    // Capture input device
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => return Err(TlapError::NoInputDevice)
    };

    let config = get_input_config();

    // Prepare audio recording file for writing
    let spec = get_spec();
    let mut writer = match hound::WavWriter::create(LIVE_RECORDING_PATH, spec) {
        Ok(w) => w,
        Err(_e) => return Err(TlapError::CreateRecordingFailed)
    };
   
    let err_fn = |err| eprintln!("{}", err);

    let stream = device.build_input_stream(
        &config,
        move |data :&[i16], _: &_| {
            // err_fn called if unwrap fails
            for &sample in data.iter() {
                writer.write_sample(sample).unwrap_or_default()
            }
            // Write samples to file to avoid reading
            // nothing back when streaming
            writer.flush().unwrap_or_default()
        },
        err_fn,
    );
    let stream = match stream {
        Ok(s) => s,
        Err(_e) => return Err(TlapError::NoInputStream)
    };

    // Subtitle properties
    let start = Instant::now();
    let mut sub_num = 1;

    // Allows model to be used across threads
    let model = Arc::new(Mutex::new(model));

    while let Ok(()) = stream.play() {
        // Wait for enough input to process
        thread::sleep(Duration::from_secs(4));

        let model_arc = model.clone();
        let now_ts = start.elapsed().as_millis();
        
        thread::spawn(move ||
            transcribe_live(model_arc, sub_num, now_ts)
        );

        // Use eprint!() as a progress indicator as per Rust docs
        eprint!("\r Written {} subtitles so far...", sub_num);

        // Prepare for next iteration
        sub_num += 1
    }

    Err(TlapError::WriteRecordingFailed)
}

fn transcribe_live(model_arc :Arc<Mutex<Model>>, sub_num :u16, ts :u128)
    -> Result<(), TlapError> {

    if let Some(s) = get_audio_samples(LIVE_RECORDING_PATH.into()) {
        let samples_length = s.len();

        let new_samples = if samples_length > FOUR_SECONDS {
            let cursor = samples_length - FOUR_SECONDS;
            s.split_at(cursor).1
        } else {
            // First loop falls short of 64K samples
            s.split_at(0).1
        };

        if let Ok(mut m) = model_arc.try_lock() {
            if let Ok(t) = m.speech_to_text(new_samples) {
                let sub = Subtitle::from(sub_num, ts, t);

                if let Err(_e) = sub.write_to(LIVE_SUBTITLES_PATH.into()) {
                    return Err(TlapError::WriteSubtitlesFailed)
                }
            } else {
                return Err(TlapError::TranscriptionFailed)
            }
        } else {
            return Err(TlapError::ModelLockFailed)
        }
    } else {
        return Err(TlapError::ReadFileFailed)
    }

    Ok(())
}

pub fn transcribe(mut model :Model, sample_lines :Vec<[i16;FOUR_SECONDS]>,
    subs_path :String) -> Result<(), TlapError> {

    let sub_total = sample_lines.len();
    let mut sub_count = 1;
    let mut timestamp = 4000;

    for line in sample_lines {
        if let Ok(t) = model.speech_to_text(&line) {
            let sub = Subtitle::from(sub_count, timestamp, t);

            // Use eprint!() as a progress indicator as per Rust docs
            if let Ok(()) = sub.write_to(subs_path.clone()) {
                eprint!("\r Processed subtitle {} of {}...",
                    sub_count, sub_total);

                // Prepare for next iteration
                sub_count += 1;
                timestamp += 4000
            } else {
                return Err(TlapError::WriteSubtitlesFailed)
            }
        } else {
            return Err(TlapError::TranscriptionFailed)
        }
    }

    Ok(())
}

/*
	This code was inspired by the RustAudio example client
	https://github.com/RustAudio/deepspeech-rs/blob/master/examples/client_simple.rs
*/
pub fn get_audio_samples(audio_path :String) -> Option<Vec<i16>> {
    if let Ok(mut r) = hound::WavReader::open(audio_path) {
        // unwrap_or_default or unwrap_or(0) will quietly
        // replace malformed samples with silence
        let samples :Vec<i16> = r.samples()
            .map(|s| s.unwrap_or_default())
            .collect();
        Some(samples)
    } else {
        None
    }
}

pub fn split_audio_lines(audio_buffer :Vec<i16>)
    -> Result<Vec<[i16;FOUR_SECONDS]>, TlapError> {

    let mut audio_lines :Vec<[i16;FOUR_SECONDS]> = Vec::new();

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

            if let Ok(l) = current_samples.try_into() {
                audio_lines.push(l)
            } else {
                return Err(TlapError::AudioSplitFailed)
            }
        } else {
            // Put remaining chunks in a vector and fill the empty
            // space with zeroes so it is the right size
            current_samples = sample_buffer;
            let len = FOUR_SECONDS - current_samples.len();
            let mut last_samples = vec![0i16;len];

            for s in current_samples {
                last_samples.push(*s)
            }

            if let Ok(l) = last_samples.try_into() {
                audio_lines.push(l)
            } else {
                return Err(TlapError::AudioSplitFailed)
            }
        }
    }

    Ok(audio_lines)
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
