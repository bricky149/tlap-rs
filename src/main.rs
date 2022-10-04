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

extern crate audrey;

extern crate coqui_stt;
use coqui_stt::{Model, Stream};

extern crate dasp_interpolate;
extern crate dasp_signal;

#[cfg(target_os = "linux")]
extern crate hound;
#[cfg(target_os = "linux")]
extern crate portaudio;

#[cfg(target_os = "linux")]
use hound::{SampleFormat, WavSpec, WavWriter};
#[cfg(target_os = "linux")]
use portaudio as pa;

//use std::io::Write;
use std::env::args;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::time::Instant;

mod speech;
use speech::*;

mod subtitle;
use subtitle::*;

const USAGE :&str = "tlap
Transliterate Language for an Accessibility Purpose

USAGE
tlap {rt|realtime} [subfilepath]
tlap {pr|postrecord} audiofilepath [subfilepath]

ARGUMENTS
pr/postrecord, rt/realtime
Determines whether to transliterate live or pre-recorded audio.

audiofilepath (mandatory if pr/postrecord argument given)
Audio file to transliterate from.";

enum TranscriptionType {
	PostRecord = 1,
	RealTime
}

/*
	This code was originally authored by Tyler Anton (https://github.com/tylerdotdev)
	https://stackoverflow.com/questions/67105792/getting-blank-results-from-deepspeech-with-portaudio-in-rust
*/
#[cfg(target_os = "linux")]
pub fn stt_intermediate(mut model :Model) {
    let pa = pa::PortAudio::new().unwrap();
    let input_settings = pa.default_input_stream_settings(1, 16000.0, 1024).unwrap();

	// Subtitle properties
	let start = Instant::now();
	let mut prev_text_len = 0;
	let mut prev_words = String::from(" ");
	let mut sub_written = false;
	let mut sub_count = 1;
	let mut past_ts = 0;

	// Prepare audio recording file for writing
	let rec_path = "recording.wav";
	let spec = WavSpec {
        bits_per_sample: 16,
        channels: 1,
        sample_format: SampleFormat::Int,
        sample_rate: 16000,
    };
	let mut writer = WavWriter::create(rec_path, spec).unwrap();

	// Main audio loop
    let process_audio = move |pa::InputStreamCallbackArgs { buffer, .. }| {
		let mut stream = Stream::from_model(&mut model).unwrap();
        stream.feed_audio(buffer);

        match stream.intermediate_decode() {
            Ok(mut text) => {
                let current_text_len = text.chars().count();
                // Reduce noise by acting only when new words are added to buffer
                if current_text_len > prev_text_len {
                    // Uncomment for real-time subtitles
                    //let stdout = std::io::stdout();
                    //let mut lock = stdout.lock();
                    //writeln!(lock, "IRL subtitles: {}", text).unwrap();

                    sub_written = false;
                    prev_text_len = current_text_len
                }

				let now = start.elapsed().as_millis();
				let time_diff = now - past_ts;

				// Write subs every 4.8-5.2 seconds we are looping
				// We allow for some latency (~400ms in dev)
				if time_diff > 4800 && time_diff < 5200 && !sub_written {
                    // We are only interested in new speech
                    let mut last_word = prev_words.rfind(' ').unwrap_or(0);
                    prev_words = text.clone();
                    let mut sub_line = text.split_off(last_word);

                    // Remove last word as it may change in next iteration
                    last_word = sub_line.rfind(' ').unwrap_or(0);
                    sub_line.truncate(last_word);

					// Write subtitle
                    let sub = Subtitle::from(sub_count, past_ts, now, sub_line);
					match flush_one(sub) {
						Ok(()) => println!("Running for {} seconds...",
											format_args!("{}", now / 1000)),
						Err(e) => eprintln!("Error writing subtitles: {:?}", e)
					};
                    sub_written = true;

                    // Prepare for next iteration
                    sub_count += 1;
                    past_ts = now
				} else if time_diff > 5200 {
					// Either we were too late to write subtitles or no text was decoded
					// Pretend we made it to avoid permanently falling behind
					past_ts = now
				}

				// Save recorded audio to Wave file
				for slice in buffer {
					writer.write_sample(*slice).unwrap()
				}

				pa::Continue
            }
            Err(e) => {
                eprintln!("DeepSpeech error: {:?}", e);
                pa::Complete
            }
        }
    };
	// Open audio stream
    let mut stream = pa.open_non_blocking_stream(input_settings, process_audio).unwrap();
    stream.start().unwrap();

	// Keep audio loop alive
    while stream.is_active().unwrap() {}
}

#[cfg(not(target_os = "linux"))]
fn stt_intermediate(mut _model :Model) {
	eprintln!("Real-time subtitling is only available on Linux.")
}

fn get_model() -> Model {
	let dir_path = Path::new("models/");
	let mut graph_name = String::from("models/model.tflite");
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
		m.enable_external_scorer(scorer_name).unwrap();
	}
	m
}

fn main() {
	let stt_type = match args().nth(1) {
		Some(stt) => {
			match stt.as_str() {
				"postrecord" => TranscriptionType::PostRecord,
				"pr" => TranscriptionType::PostRecord,
				"realtime" => TranscriptionType::RealTime,
				"rt" => TranscriptionType::RealTime,
				_ => {
					eprintln!("Invalid speech-to-text type given.");
					println!("{}", USAGE);
					std::process::exit(1)
				}
			}
		}
		None => {
			println!("{}", USAGE);
			std::process::exit(1)
		}
	};

	let model = get_model();

	match stt_type {
		TranscriptionType::PostRecord => {
			let audio_path = match args().nth(2) {
				Some(audio) => audio,
				None => {
					eprintln!("Please specify an audio file.");
					println!("{}", USAGE);
					std::process::exit(2)
				}
			};

			let audio_buffer = get_audio(audio_path);
            let lines = stt_postrecord(model, audio_buffer);
			let subs = Subtitle::from_lines(lines);

			match flush_all(subs) {
				Ok(()) => println!("Subtitles written successfully."),
				Err(e) => eprintln!("Error writing subtitles: {:?}", e)
			};
		},
		TranscriptionType::RealTime => {
			stt_intermediate(model)
		}
	}
}
