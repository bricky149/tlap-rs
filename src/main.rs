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

mod speech;
use speech::*;

mod subtitle;
use subtitle::*;

const USAGE :&str = "tlap
Transliterate Language for an Accessibility Purpose

USAGE
tlap {rt|realtime}
tlap {pr|postrecord} [audiofilepath] [subtitlefilepath]

ARGUMENTS
pr/postrecord, rt/realtime
Determines whether to transliterate live or pre-recorded audio.

audiofilepath
Audio file to transliterate from. Used when 'pr/postrecord' is passed.
If nothing is given, audio will be saved as 'recording.wav' in the current directory.

subfilepath
Subtitle file to write to. Used when 'pr/postrecord' is passed.
If nothing is given, appends '.srt' to the passed audio file path.";

enum TranscriptionType {
	Invalid,
	PostRecord,
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
				_ => TranscriptionType::Invalid
			}
		}
		None => TranscriptionType::Invalid
	};
	let audio_path = match args().nth(2) {
		Some(path) => path,
		None => "recording.wav".into()
	};
	let subs_path = match args().nth(3) {
		Some(subs) => subs,
		None => audio_path.clone() + ".srt"
	};

	match stt_type {
		TranscriptionType::PostRecord => {
			let audio_buffer = get_audio_samples(audio_path);
			let audio_lines = split_audio_lines(audio_buffer);
			
			let model = get_model();
            get_transcript(model, audio_lines, subs_path)
		}
		TranscriptionType::RealTime => {
			let model = get_model();
			record_input(model)
		}
		TranscriptionType::Invalid => {
			eprintln!("Invalid speech-to-text type given.\n");
			println!("{}", USAGE)
		}
	}
}
