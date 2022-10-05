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

use std::io::{Result, Write};

#[cfg(target_os = "linux")]
use std::fs::OpenOptions;

pub struct Subtitle {
    number: u16,
    timestamp: String,
    caption: String
}

impl Subtitle {
    pub fn from(sub_count :u16, past_ts :u128, now :u128, sub_body :String)
        -> Self {
        
        let (first_hour, first_minute, first_second, first_ms) = get_timestamp(past_ts);
        let (second_hour, second_minute, second_second, second_ms) = get_timestamp(now);

        Subtitle {
            number: sub_count,
            timestamp: format!("{:02}:{:02}:{:02},{:03} --> {:02}:{:02}:{:02},{:03}",
                        first_hour, first_minute, first_second, first_ms,
                        second_hour, second_minute, second_second, second_ms),
            caption: sub_body
        }
    }

    pub fn from_lines(lines :Vec<String>) -> Vec<Self> {
        let mut subs = Vec::new();

        let count = 1;
        let mut ms = 0;

        for line in lines {
            let (first_hour, first_minute, first_second, first_ms) = get_timestamp(ms);
            ms += 5000;
            let (second_hour, second_minute, second_second, second_ms) = get_timestamp(ms);

            let sub = Subtitle {
                number: count,
                timestamp: format!("{:02}:{:02}:{:02},{:03} --> {:02}:{:02}:{:02},{:03}",
                            first_hour, first_minute, first_second, first_ms,
                            second_hour, second_minute, second_second, second_ms),
                caption: line
            };
            subs.push(sub)
        }

        subs
    }

    pub fn flush_all(subtitles :Vec<Self>) -> Result<()> {
        let mut file = OpenOptions::new().append(true).create(true).open("recording.srt").unwrap();
    
        for subtitle in subtitles {
            writeln!(file, "{}", subtitle.number)?;
            writeln!(file, "{}", subtitle.timestamp)?;
            writeln!(file, "{}\n", subtitle.caption)?;
        }
    
        Ok(())
    }
    
    pub fn flush_one(self) -> Result<()> {
        let mut file = OpenOptions::new().append(true).create(true).open("recording.srt").unwrap();
    
        writeln!(file, "{}", self.number)?;
        writeln!(file, "{}", self.timestamp)?;
        writeln!(file, "{}\n", self.caption)?;
    
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn get_timestamp(timestamp :u128) -> (u128, u128, u128, u128) {
    let mut ms = timestamp;

    let mut seconds = 0;
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
