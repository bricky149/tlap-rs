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

extern crate coqui_stt;
extern crate cpal;
extern crate hound;

use std::env::args;

mod enums;
mod speech;
mod subtitle;

use enums::TranscriptionType;
use speech::*;
use subtitle::Subtitle;

const USAGE :&str = "
tlap
Transliterate Language for an Accessibility Purpose

USAGE

tlap REALTIME
tlap POSTRECORD FILE

ARGUMENTS

REALTIME/RT
Tells the program to transliterate live audio.

POSTRECORD/PR
Tells the program to transliterate recorded audio.

FILE
Audio file to transliterate from. Used when 'postrecord' or 'pr' is passed.
";

fn main() {
	let mut args = args();

	// Second arg is speech-to-text type
	// nth() is used to skip first arg (executable name)
	let stt_type = match args.nth(1) {
		Some(stt) => {
			// Accept upper or lower case
			match stt.as_str() {
				"postrecord" | "POSTRECORD" => TranscriptionType::PostRecord,
				"pr" | "PR" => TranscriptionType::PostRecord,
				"realtime" | "REALTIME" => TranscriptionType::RealTime,
				"rt" | "RT" => TranscriptionType::RealTime,
				_ => TranscriptionType::Invalid
			}
		}
		None => TranscriptionType::Invalid
	};

	// Use hard-coded path to save user from adding an extra arg
	let model = match get_model("models/") {
		Ok(m) => m,
		Err(e) => {
			eprintln!("{:?}", e);
			return
		}
	};

	match stt_type {
		TranscriptionType::PostRecord => {
			// Third arg is audio file to read from
			let audio_path = match args.next() {
				Some(p) => p,
				None => {
					eprintln!("No file given.");
					println!("{}", USAGE);
					return
				}
			};
			let subs_path = audio_path.replace(".wav", ".srt");

			let audio_buffer = match get_all_samples(audio_path) {
				Ok(b) => b,
				Err(e) => {
					eprintln!("{:?}", e);
					return
				}
			};

			let audio_lines = match split_audio_lines(audio_buffer) {
				Ok(l) => l,
				Err(e) => {
					eprintln!("{:?}", e);
					return
				}
			};

			match transcribe(model, audio_lines, subs_path) {
				Ok(()) => println!("done."),
				Err(e) => eprintln!("{:?}", e)
			}
		}
		TranscriptionType::RealTime => {
			let stream = match get_input_stream() {
				Ok(s) => s,
				Err(e) => {
					eprintln!("{:?}", e);
					return
				}
			};

			record_input(model, stream)
		}
		TranscriptionType::Invalid => {
			// Either a non-existent type or nothing was given
			eprintln!("No valid speech-to-text type given.");
			println!("{}", USAGE)
		}
	}
}
