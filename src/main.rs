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
extern crate dasp_interpolate;
extern crate dasp_signal;

#[cfg(target_os = "linux")]
extern crate hound;
#[cfg(target_os = "linux")]
extern crate portaudio;

use std::env::args;
//use std::io::Write;

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

	let model = TlapCoqui::new();

	match stt_type {
		TranscriptionType::PostRecord => {
			let audio_path = match args().nth(2) {
				Some(path) => path,
				None => {
					eprintln!("Please specify an audio file.");
					println!("{}", USAGE);
					std::process::exit(2)
				}
			};

			let audio_buffer = get_audio(audio_path);
            let lines = TlapCoqui::get_recorded_lines(model, audio_buffer);
			let subs = Subtitle::from_lines(lines);

			match Subtitle::flush_all(subs) {
				Ok(()) => println!("Subtitles written successfully."),
				Err(e) => eprintln!("Error writing subtitles: {:?}", e)
			};
		},
		TranscriptionType::RealTime => {
			TlapCoqui::get_live_lines(model)
		}
	}
}
