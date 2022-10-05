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

use crate::Subtitle;

use audrey::read::Reader;
use coqui_stt::{Model, Stream};
use dasp_interpolate::linear::Linear;
use dasp_signal::{from_iter, Signal, interpolate::Converter};
use std::fs::File;
use std::path::Path;

#[cfg(target_os = "linux")]
use hound::{SampleFormat, WavSpec, WavWriter};
#[cfg(target_os = "linux")]
use portaudio as pa;
#[cfg(target_os = "linux")]
use std::time::Instant;

// 16000 samples * 5 seconds
const FIVE_SECONDS :usize = 80000;
// Model has been trained on this specific sample rate
const SAMPLE_RATE :u32 = 16000;

pub struct TlapCoqui {
    model: Model
}

impl TlapCoqui {
    pub fn new() -> Self {
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
            m.enable_external_scorer(scorer_name).unwrap();
        }
        TlapCoqui {
            model: m
        }
    }

    pub fn get_recorded_lines(mut self, audio_buffer :Vec<i16>) -> Vec<String> {
        // Store an hours' worth of lines before realloc
        let mut sub_lines = Vec::with_capacity(720);
    
        let sample_lines = get_sample_lines(audio_buffer);
        let mut prev_words = String::from(" ");
    
        for line in sample_lines {
            match self.model.speech_to_text(&line) {
                Ok(mut text) => {
                    // We are only interested in new speech
                    let mut last_word = prev_words.rfind(' ').unwrap_or(0);
                    prev_words = text.clone();
                    let mut sub_line = text.split_off(last_word);
    
                    // Remove last word as it may change in next iteration
                    last_word = sub_line.rfind(' ').unwrap_or(0);
                    sub_line.truncate(last_word);
    
                    sub_lines.push(sub_line);
                },
                Err(e) => eprintln!("Coqui error: {:?}", e)
            }
        }
    
        sub_lines
    }

    /*
        This code was originally authored by Tyler Anton (https://github.com/tylerdotdev)
        https://stackoverflow.com/questions/67105792/getting-blank-results-from-deepspeech-with-portaudio-in-rust
    */
    #[cfg(target_os = "linux")]
    pub fn get_live_lines(mut self) {
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
            let mut model_stream = Stream::from_model(&mut self.model).unwrap();
            model_stream.feed_audio(buffer);

            match model_stream.intermediate_decode() {
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

                    // Write subs every five or so seconds we are looping
                    if time_diff > 5000 && !sub_written {
                        // We are only interested in new speech
                        let mut last_word = prev_words.rfind(' ').unwrap_or(0);
                        prev_words = text.clone();
                        let mut sub_line = text.split_off(last_word);

                        // Remove last word as it may change in next iteration
                        last_word = sub_line.rfind(' ').unwrap_or(0);
                        sub_line.truncate(last_word);

                        // Write subtitle
                        let sub = Subtitle::from(sub_count, past_ts, now, sub_line);
                        match Subtitle::flush_one(sub) {
                            Ok(()) => println!("Running for {} seconds...",
                                                format_args!("{}", now / 1000)),
                            Err(e) => eprintln!("Error writing subtitles: {:?}", e)
                        };
                        sub_written = true;

                        // Prepare for next iteration
                        sub_count += 1;
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
        let mut pa_stream = pa.open_non_blocking_stream(input_settings, process_audio).unwrap();
        pa_stream.start().unwrap();

        // Keep audio loop alive
        while pa_stream.is_active().unwrap() {}
    }

    #[cfg(not(target_os = "linux"))]
    fn stt_intermediate(mut _model :Model) {
        eprintln!("Real-time subtitling is only available on Linux.")
    }
}

/*
	This code was taken from the RustAudio example client.
	https://github.com/RustAudio/deepspeech-rs/blob/master/examples/client_simple.rs
*/
pub fn get_audio(audio_path :String) -> Vec<i16> {
	let audio_file = File::open(audio_path).unwrap();
	let mut reader = Reader::new(audio_file).unwrap();

	let desc = reader.description();
	assert_eq!(1, desc.channel_count(), "Only monoaural audio is accepted");

	// Obtain buffer of samples
	let audio_buffer :Vec<i16> = if desc.sample_rate() == SAMPLE_RATE {
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

	audio_buffer
}

fn get_sample_lines(sample_buffer :Vec<i16>) -> Vec<[i16;80000]> {
    let mut sample_lines :Vec<[i16;80000]> = Vec::new();

    let sample_length = sample_buffer.len();
    let total_lines = sample_length / FIVE_SECONDS;
    let mut prev_cursor = 0;
    
    for lines_left in total_lines..=0 {
        let cursor = if lines_left > 0 {
            prev_cursor + 80000
        } else {
            sample_length - prev_cursor
        };

        // TODO: We leak memory by not assigning the second value from split_at()
        let current_samples = sample_buffer.split_at(cursor).1;
        match current_samples.try_into() {
            Ok(l) => sample_lines.push(l),
            Err(e) => eprintln!("{:?}", e)
        };

        prev_cursor = cursor;
    }

    sample_lines
}
