/*
	Copyright 2021 Bricky <bricky149@teknik.io>

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
extern crate dasp_interpolate;
extern crate dasp_signal;
extern crate deepspeech;

use audrey::read::Reader;
use dasp_interpolate::linear::Linear;
use dasp_signal::{from_iter, Signal, interpolate::Converter};
use deepspeech::Model;

#[cfg(target_os = "linux")]
extern crate hound;
#[cfg(target_os = "linux")]
use hound::{SampleFormat, WavSpec, WavWriter};

#[cfg(target_os = "linux")]
extern crate portaudio;
#[cfg(target_os = "linux")]
use portaudio as pa;

use std::env::args;
use std::fs::File;
use std::io::{Result, Write};
use std::path::Path;

#[cfg(target_os = "linux")]
use std::fs::OpenOptions;
#[cfg(target_os = "linux")]
use std::time::Instant;

const USAGE :&str = "tlap
Transliterate Language for an Accessibility Purpose

USAGE
tlap {rt|realtime} [subfilepath]
tlap {pr|postrecord} audiofilepath [subfilepath]

ARGUMENTS
pr/postrecord, rt/realtime
Determines whether to transliterate live or pre-recorded audio.

audiofilepath (mandatory if pr/postrecord argument given)
Audio file to transliterate from.

subfilepath (optional)
Subtitle file to write to.
If no argument given it will create `subs.srt` in the current working directory.";
// Model has been trained on this specific sample rate
const SAMPLE_RATE :u32 = 16000;

#[cfg(target_os = "linux")]
fn get_timestamp(timestamp :u128) -> (u128, u128, u128, u128) {
	let mut seconds = 0;
	let mut ms = timestamp;
	if ms > 999 {
		seconds = timestamp / 1000;
		ms -= 1000 * seconds
	}
	let mut minutes = 0;
	if seconds > 59 {
		minutes = seconds / 60;
		seconds -= 60 * minutes
	}
	let mut hours = 0;
	if minutes > 59 {
		hours = minutes / 60;
		minutes -= 60 * hours
	}
	(hours, minutes, seconds, ms)
}

#[cfg(target_os = "linux")]
fn write_subs_realtime(subs_path :&str, sub_count :u16, past_ts :u128, now :u128,
    sub_line :&str) -> Result<()> {
	// Open and append to subtitles file
	let mut file = OpenOptions::new().append(true).create(true).open(subs_path).unwrap();
	// Write subtitle to file
	let (first_hour, first_minute, first_second, first_ms) = get_timestamp(past_ts);
	let (second_hour, second_minute, second_second, second_ms) = get_timestamp(now);
	writeln!(file, "{}", format_args!("{}", sub_count))?;
	writeln!(file, "{}", format_args!("{:02}:{:02}:{:02},{:03} --> {:02}:{:02}:{:02},{:03}",
        first_hour, first_minute, first_second, first_ms,
        second_hour, second_minute, second_second, second_ms))?;
	writeln!(file, "{}\n", sub_line)?;
	// Written without error
	Ok(())
}

/*
	This code was originally authored by Tyler Anton (https://github.com/tylerdotdev)
	https://stackoverflow.com/questions/67105792/getting-blank-results-from-deepspeech-with-portaudio-in-rust
*/
#[cfg(target_os = "linux")]
fn stt_intermediate(mut model :Model, subs_path :String) {
    let pa = pa::PortAudio::new().unwrap();
    let input_settings = pa.default_input_stream_settings(1, 16000.0, 1024).unwrap();
	let mut stream = model.create_stream().unwrap();
	// Subtitle properties
	let start = Instant::now();
	let mut prev_text_len = 0;
	let mut prev_words = String::from(" ");
	let mut sub_written = false;
	let mut sub_count = 1;
	let mut past_ts = 0;
	// Prepare audio recording file for writing
	let rec_path = subs_path.clone() + ".wav";
	let spec = WavSpec {
        bits_per_sample: 16,
        channels: 1,
        sample_format: SampleFormat::Int,
        sample_rate: 16000,
    };
	let mut writer = WavWriter::create(rec_path, spec).unwrap();
	// Main audio loop
    let process_audio = move |pa::InputStreamCallbackArgs { buffer, .. }| {
        stream.feed_audio(buffer);
        match stream.intermediate_decode() {
            Ok(mut text) => {
                let current_text_len = text.chars().count();
                // Reduce noise by acting only when new words are added to buffer
                if current_text_len > prev_text_len {
                    // Uncomment for real-time subtitles
                    // let mut stdout = std::io::stdout();
                    // let mut lock = stdout.lock();
                    // writeln!(lock, "IRL subtitles: {}", text);
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
                    match write_subs_realtime(&subs_path, sub_count, past_ts, now, &sub_line) {
                        Ok(()) => println!("Running for {} seconds...",
                                    format_args!("{}", now / 1000)),
                        Err(e) => eprintln!("Error writing subtitles: {:?}", e)
                    }
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
fn stt_intermediate(mut _model :Model, _subs_path :String) {
	eprintln!("Real-time subtitling is only available on Linux.")
}

fn write_subs_postrecord(sub_lines :Vec<String>, subs_path :String) -> Result<()> {
	let mut file = File::create(subs_path).unwrap();
	// Subtitle properties
	let mut sub_count = 1;
	let mut sub_hour = 0;
	let mut sub_min = 0;
	let mut sub_sec = 0;
	// Each line will consist of five seconds' worth of speech
    for line in sub_lines {
		writeln!(file, "{}", format_args!("{}", sub_count))?;
		sub_count += 1;
		// Write subtitle pending timestamp rollover check
		sub_sec += 5;
		if sub_sec > 59 {
			sub_min += 1;
			if sub_min > 59 {
				sub_hour += 1;
				writeln!(file, "{}",
                    format_args!("{:02}:59:55,000 --> {:02}:00:00,000",
					sub_hour-1, sub_hour))?;
				sub_min = 0
			} else {
				writeln!(file, "{}",
                    format_args!("{:02}:{:02}:55,000 --> {:02}:{:02}:00,000",
					sub_hour, sub_min-1, sub_hour, sub_min))?
			}
			sub_sec = 0
		} else {
			writeln!(file, "{}",
                format_args!("{:02}:{:02}:{:02},000 --> {:02}:{:02}:{:02},000",
                sub_hour, sub_min, sub_sec-5, sub_hour, sub_min, sub_sec))?
		}
		writeln!(file, "{}\n", line)?
    }
	// Written without error
    Ok(())
}

fn stt_postrecord(mut model :Model, audio_buf :Vec<i16>) -> Vec<String> {
	// Instead of splitting the buffer and getting incorrect results
	// feed the buffer directly into the model stream and count every
	// 80000 samples (16000 samples * 5 seconds) as a subtitle line
	let mut stream = model.create_stream().unwrap();
	// Store ten minutes' worth of lines before realloc
	let mut sub_lines = Vec::with_capacity(120);
	let mut sample_vec = Vec::with_capacity(1);
	let mut sample_count = 0;
	let mut prev_words = String::from(" ");
	let mut progress = 0;
	for sample in audio_buf {
        sample_vec.push(sample);
        stream.feed_audio(&sample_vec);
        sample_vec.pop();
        sample_count += 1;
        if sample_count % 80000 != 0 { continue }
        match stream.intermediate_decode() {
            Ok(mut text) => {
                // We are only interested in new speech
                let mut last_word = prev_words.rfind(' ').unwrap_or(0);
                prev_words = text.clone();
                let mut sub_line = text.split_off(last_word);
                // Remove last word as it may change in next iteration
                last_word = sub_line.rfind(' ').unwrap_or(0);
                sub_line.truncate(last_word);
                sub_lines.push(sub_line);
                progress += 5;
                println!("{} seconds processed...", format_args!("{}", progress))
            },
            Err(e) => eprintln!("DeepSpeech error: {:?}", e)
        }
	}
	// This will get the text from the samples not processed above
	// as a result of there being less than five seconds of audio left
    match stream.intermediate_decode() {
        Ok(mut text) => {
            // Do not panic if nothing was interpreted
            let last_word = prev_words.rfind(' ').unwrap_or(0);
            let sub_line = text.split_off(last_word);
            sub_lines.push(sub_line);
            println!("Finished processing audio.")
        },
        Err(e) => eprintln!("DeepSpeech error: {:?}", e)
    }
	sub_lines
}

//fn transcribe(mut model :Model, audio_buf :Vec<i16>) -> Vec<String> {
//	// Split the buffer into five-second chunks and
//	// process each chunk as a subtitle line
//	let mut lines = Vec::with_capacity(120);
//	let mut progress = 0;
//	let audio_chunks :Vec<&[i16]> = audio_buf.chunks(80000).collect();
//	for chunk in audio_chunks {
//		let result = model.speech_to_text(chunk).unwrap();
//		if result.is_empty() {
//			lines.push(String::from("*silence*"))
//		} else {
//			lines.push(result)
//		}
//		progress += 5;
//		println!("{} seconds processed...", format_args!("{}", progress))
//	}
//	lines
//}

fn get_audio_samples(audio_path :String) -> Vec<i16> {
	let audio_file = File::open(audio_path).unwrap();
	let mut reader = Reader::new(audio_file).unwrap();
	let desc = reader.description();
	assert_eq!(1, desc.channel_count(), "Only monoaural audio is accepted");
	// Obtain buffer of samples
	let audio_buf :Vec<i16> = if desc.sample_rate() == SAMPLE_RATE {
		reader.samples().map(|s| s.unwrap()).collect()
	} else {
		// We need to interpolate to the target sample rate
		let interpolator = Linear::new([0i16], [0]);
		let conv = Converter::from_hz_to_hz(
			from_iter(reader.samples::<i16>().map(|s| [s.unwrap()])),
			interpolator,
			desc.sample_rate() as f64,
			SAMPLE_RATE as f64);
		conv.until_exhausted().map(|v| v[0]).collect()
	};
	//let audio_len = audio_buf.len() as i32 / SAMPLE_RATE as i32;
	audio_buf
}

fn get_model() -> Model {
	let dir_path = Path::new("models/");
	let mut graph_name = dir_path.join("").into_boxed_path();
	let mut scorer_name = dir_path.join("").into_boxed_path();
	// Search for model and scorer
	for file in dir_path.read_dir().unwrap().flatten() {
        let file_path = file.path();
        if file_path.is_file() {
            if let Some(ext) = file_path.extension() {
                if ext == "pbmm" {
                    graph_name = file_path.into_boxed_path()
                } else if ext == "scorer" {
                    scorer_name = file_path.into_boxed_path()
                }
            }
        }
	}
	// Return loaded model and scorer
	let mut m = Model::load_from_files(&graph_name).unwrap();
	m.enable_external_scorer(&scorer_name).unwrap();
	m
}

enum TranscriptionType {
	PostRecord = 1,
	RealTime
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
			let subs_path = match args().nth(3) {
				Some(subs) => subs,
				None => String::from("subs.srt")
			};
			let audio_buf = get_audio_samples(audio_path);
            let lines = stt_postrecord(model, audio_buf);
			match write_subs_postrecord(lines, subs_path) {
				Ok(()) => println!("Subtitles written successfully."),
				Err(e) => eprintln!("Error writing subtitles: {:?}", e)
			};
		},
		TranscriptionType::RealTime => {
			let subs_path = match args().nth(2) {
				Some(subs) => subs,
				None => String::from("subs.srt")
			};
			stt_intermediate(model, subs_path)
		}
	}
}
