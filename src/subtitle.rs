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

use std::fs::OpenOptions;
use std::io::{Result, Write};

pub struct Subtitle {
    number: u16,
    timestamp: String,
    caption: String
}

impl Subtitle {
    pub fn from(sub_count :u16, past_ts :u128, now :u128, sub_body :String) -> Self {
        let (first_hour, first_minute, first_second, first_ms) = get_timestamp(past_ts);
        let (second_hour, second_minute, second_second, second_ms) = get_timestamp(now);

        Self {
            number: sub_count,
            timestamp: format!("{:02}:{:02}:{:02},{:03} --> {:02}:{:02}:{:02},{:03}",
                        first_hour, first_minute, first_second, first_ms,
                        second_hour, second_minute, second_second, second_ms),
            caption: sub_body
        }
    }

    pub fn from_line(count :u16, timestamp :u128, line :String) -> Self {
        let mut ms = timestamp;
        let (first_hour, first_minute, first_second, first_ms) = get_timestamp(ms);
        ms += 4000;
        let (second_hour, second_minute, second_second, second_ms) = get_timestamp(ms);

        Self {
            number: count,
            timestamp: format!("{:02}:{:02}:{:02},{:03} --> {:02}:{:02}:{:02},{:03}",
                        first_hour, first_minute, first_second, first_ms,
                        second_hour, second_minute, second_second, second_ms),
            caption: line
        }
    }
    
    pub fn write(self, subs_path :String) -> Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(subs_path)?;

        writeln!(file, "{}", self.number)?;
        writeln!(file, "{}", self.timestamp)?;
        writeln!(file, "{}\n", self.caption)?;
    
        Ok(())
    }
}

fn get_timestamp(timestamp :u128) -> (u128, u128, u128, u128) {
    let mut ms = timestamp;

    let mut seconds =  if ms > 999 {
        ms / 1000
    } else {
        0
    };
    ms -= 1000 * seconds;

    let mut minutes = if seconds > 59 {
        seconds / 60
    } else {
        0
    };
    seconds -= 60 * minutes;

    let hours = if minutes > 59 {
        minutes / 60
    } else {
        0
    };
    minutes -= 60 * hours;

    (hours, minutes, seconds, ms)
}
