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
use cpal::{BufferSize, SampleRate, Stream, StreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

use crate::Subtitle;
use crate::TlapError;

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

pub fn get_model(dir :&str) -> Result<Model, TlapError> {
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
        return Err(TlapError::NoSpeechModel)
    }

    if let Ok(mut m) = Model::new(model_path) {
        if let Err(e) = m.enable_external_scorer(scorer_path) {
            eprintln!("Continuing without scorer due to: {}", e)
        }

        Ok(m)
    } else {
        Err(TlapError::InvalidSpeechModel)
    }
}

/*
    This was adapted from code written by Tyler Anton (https://github.com/tylerdotdev)
    https://stackoverflow.com/questions/67105792/getting-blank-results-from-deepspeech-with-portaudio-in-rust
*/
pub fn get_input_stream() -> Result<Stream, TlapError> {
    // Capture input device
    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => return Err(TlapError::NoInputDevice)
    };

    let config = get_input_config();

    // Prepare audio recording file for writing
    let spec = get_spec();
    let mut writer = match WavWriter::create(LIVE_RECORDING_PATH, spec) {
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

    if let Ok(s) = stream {
        Ok(s)
    } else {
        Err(TlapError::NoInputStream)
    }
}

pub fn record_input(model :Model, stream :Stream) {
    // Allows model to be used across threads
    let model = Arc::new(Mutex::new(model));

    // Subtitle properties
    let start = Instant::now();
    let mut sub_num = 1;
    let mut prev_ts = 0;

    while let Ok(()) = stream.play() {
        // Wait for enough input to process
        thread::sleep(Duration::from_secs(4));

        let model = model.clone();
        let now_ts = start.elapsed().as_millis();

        // Process audio on a separate (detached) thread
        thread::spawn(move ||
            if let Err(e) = transcribe_live(model, sub_num, prev_ts, now_ts) {
                eprintln!("{:?}", e)
            } else {
                // Use eprint!() as a progress indicator as per Rust docs
                eprint!("\r Written {} subtitles so far...", sub_num)
            }
        );
        // Prepare for next iteration
        sub_num += 1;
        prev_ts = now_ts
    }
}

fn transcribe_live(model :Arc<Mutex<Model>>, sub_num :usize, begin_ms :u128, end_ms :u128)
    -> Result<(), TlapError> {

    match get_new_samples(LIVE_RECORDING_PATH.into()) {
        Ok(s) => {
            if let Ok(mut m) = model.try_lock() {
                if let Ok(t) = m.speech_to_text(&s) {
                    let sub = Subtitle::new(sub_num, begin_ms, end_ms, t);
    
                    if let Err(_e) = sub.write_to(LIVE_SUBTITLES_PATH.into()) {
                        return Err(TlapError::WriteSubtitlesFailed)
                    }
                } else {
                    return Err(TlapError::TranscriptionFailed)
                }
            } else {
                return Err(TlapError::ModelLockFailed)
            }
        }
        Err(e) => return Err(e)
    }

    Ok(())
}

pub fn transcribe(mut model :Model, sample_lines :Vec<Vec<i16>>,
    subs_path :String) -> Result<(), TlapError> {

    let mut sub_total = sample_lines.len();
    let mut sub_count = 1;
    let mut prev_ts = 0;
    let mut now_ts;

    for line in sample_lines {
        if let Ok(t) = model.speech_to_text(&line) {
            if t.len() == 0 {
                sub_total -= 1;
                continue
            }

            // Dividing sample length by 15 gives a good timestamp estimation
            // 15 causes slight cumulative lag, 16 causes cumulative haste
            let sample_length :u128 = line.len().try_into().unwrap();
            now_ts = prev_ts + (sample_length / 15);

            let sub = Subtitle::new(sub_count, prev_ts, now_ts, t);

            // Use eprint!() as a progress indicator as per Rust docs
            if sub.write_to(subs_path.clone()).is_ok() {
                eprint!("\r Processed subtitle {} of {}...",
                    sub_count, sub_total);

                // Prepare for next iteration
                sub_count += 1;
                prev_ts = now_ts
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
	This was adapted from the RustAudio example client
	https://github.com/RustAudio/deepspeech-rs
*/
pub fn get_all_samples(audio_path :String) -> Result<Vec<i16>, TlapError> {
    if let Ok(mut r) = WavReader::open(audio_path) {
        // unwrap_or_default or unwrap_or(0) will quietly
        // replace malformed samples with silence
        let samples :Vec<i16> = r.samples()
            .map(|s| s.unwrap_or_default())
            .collect();

        Ok(samples)
    } else {
        Err(TlapError::ReadFileFailed)
    }
}

pub fn get_new_samples(audio_path :String) -> Result<Vec<i16>, TlapError> {
    if let Ok(mut r) = WavReader::open(audio_path) {
        // unwrap_or_default or unwrap_or(0) will quietly
        // replace malformed samples with silence
        let cursor = if r.duration() >= FOUR_SECONDS as u32 {
            r.duration() - FOUR_SECONDS as u32
        } else {
            0
        };

        if let Ok(()) = r.seek(cursor) {
            let samples :Vec<i16> = r.samples()
                .map(|s| s.unwrap_or_default())
                .collect();

            Ok(samples)
        } else {
            Err(TlapError::ReadFileFailed)
        }
    } else {
        Err(TlapError::ReadFileFailed)
    }
}

pub fn split_audio_lines(audio_buffer :Vec<i16>)
    -> Result<Vec<Vec<i16>>, TlapError> {

    let mut audio_lines :Vec<Vec<i16>> = Vec::with_capacity(2);

    let mut silence_periods = Vec::with_capacity(2);
    let mut silent_samples = 0;

    for (i, s) in audio_buffer.iter().enumerate() {
        // Check if sample has no amplitude
        if *s == 0 {
            // Add index of where we think there is silence
            // Lower values shorten lines but increase false positives
            // 1600 is 1/10th of model sample rate (100ms)
            if silent_samples >= 1600 {
                silence_periods.push(i);
                silent_samples = 0;
                continue
            }
            silent_samples += 1
        } else {
            // Silence broken
            silent_samples = 0
        }
    }

    let mut sample_buffer = audio_buffer.split_at(0).1;
    let mut current_samples;
    let mut cursor = 0;

    for i in silence_periods {
        // Work out where to split based on what we have
        // already processed
        let idx = i - cursor;

        current_samples = sample_buffer.split_at(idx).0;
        sample_buffer = sample_buffer.split_at(idx).1;

        if let Ok(l) = current_samples.try_into() {
            audio_lines.push(l)
        } else {
            return Err(TlapError::AudioSplitFailed)
        }
        // Store index so we know where to continue from
        cursor = i
    }

    Ok(audio_lines)
}

fn get_input_config() -> StreamConfig {
    StreamConfig {
        channels :1,
        sample_rate :SampleRate(SAMPLE_RATE),
        buffer_size :BufferSize::Default
    }
}

fn get_spec() -> WavSpec {
    WavSpec {
        channels :1,
        sample_rate :SAMPLE_RATE,
        bits_per_sample :16,
        sample_format :SampleFormat::Int
    }
}
