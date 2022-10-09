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
use std::path::Path;

mod speech;
use speech::*;

mod subtitle;
use subtitle::*;

const USAGE :&str = "
tlap
Transliterate Language for an Accessibility Purpose

USAGE
tlap REALTIME
tlap POSTRECORD FILE

REALTIME/RT
Tells the program to transliterate live audio.

POSTRECORD/PR
Tells the program to transliterate recorded audio.

FILE
Audio file to transliterate from. Used when 'postrecord' or 'pr' is passed.
";

enum TranscriptionType {
	Invalid,
	RealTime,
	PostRecord
}

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
		Some(m) => m,
		None => {
			eprintln!("No Coqui model found in models directory.");
			return
		}
	};

	match stt_type {
		TranscriptionType::PostRecord => {
			// Third arg is audio file to read from
			let file_name = match args.next() {
				Some(a) => a,
				None => {
					eprintln!("No file given.");
					println!("{}", USAGE);
					return
				}
			};
			let file_path = Path::new(&file_name);

			let audio_path = if file_path.is_file() {
				file_path.display().to_string()
			} else {
				eprintln!("Invalid file given.");
				println!("{}", USAGE);
				return
			};
			let subs_path = audio_path.clone() + ".srt";

			let audio_buffer = get_audio_samples(audio_path);
			let audio_lines = split_audio_lines(audio_buffer);

            get_transcript(model, audio_lines, subs_path)
		}
		TranscriptionType::RealTime => record_input(model),
		TranscriptionType::Invalid => {
			// Either a non-existent type or nothing was given
			eprintln!("No valid speech-to-text type given.");
			println!("{}", USAGE)
		}
	}
}
