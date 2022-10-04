use audrey::read::Reader;
use coqui_stt::Model;
use dasp_interpolate::linear::Linear;
use dasp_signal::{from_iter, Signal, interpolate::Converter};
use std::fs::File;

// 16000 samples * 5 seconds
const FIVE_SECONDS :usize = 80000;
// Model has been trained on this specific sample rate
const SAMPLE_RATE :u32 = 16000;

/*
	This code was originally authored by Jeremy Andrews (https://github.com/jeremyandrews)
	https://github.com/jeremyandrews/kakaia/blob/master/src/speech.rs
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

pub fn get_sample_lines(sample_buffer :Vec<i16>) -> Vec<[i16;80000]> {
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

pub fn stt_postrecord(mut model :Model, audio_buffer :Vec<i16>) -> Vec<String> {
	// Store an hours' worth of lines before realloc
	let mut sub_lines = Vec::with_capacity(720);

	let sample_lines = get_sample_lines(audio_buffer);
    let mut prev_words = String::from(" ");

	for line in sample_lines {
        match model.speech_to_text(&line) {
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
