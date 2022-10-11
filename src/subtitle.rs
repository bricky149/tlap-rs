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

/*
    This was partly copied from subtitles-rs, written by Eric Kidd
    https://github.com/emk/subtitles-rs/
*/

use std::fmt;
use std::fs::OpenOptions;
use std::io::{Result, Write};

pub struct Subtitle {
    index :usize,
    period :Period,
    caption :String
}

impl Subtitle {
    pub fn from(index :usize, ts :u128, caption :String) -> Self {
        let period = Period::from(ts);

        Self {
            index, period, caption
        }
    }

    pub fn write_to(self, path :String) -> Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)?;

        writeln!(file, "{}\n{} --> {}\n{}\n",
            self.index,
            format_time(self.period.begin),
            format_time(self.period.end),
            self.caption
        )?;

        Ok(())
    }
}

trait Display {
    fn to_string(&self) -> String;
}

impl fmt::Display for Subtitle {
    fn fmt(&self, fmtr :&mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmtr,
            "{}\n{} --> {}\n{}\n",
            self.index,
            format_time(self.period.begin),
            format_time(self.period.end),
            self.caption
        )
    }
}

struct Period {
    begin :u128,
    end :u128
}

impl Period {
    fn from(end :u128) -> Self {
        let mut ms = end;
        ms -= 4000;
        let begin = ms;

        Self {
            begin, end
        }
    }
}

fn format_time(time: u128) -> String {
    let (h, rem) = (time / 3600000, time % 3600000);
    let (m, rem) = (rem / 60000, rem % 60000);
    let (s, ms) = (rem / 1000, rem % 1000);

    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

#[cfg(test)]
mod test {
    use crate::Subtitle;

    #[test]
    fn subtitle_to_string() {
        let text = "something something capitalism bad".into();

        let sub = Subtitle::from(1, 123123, text);

        assert_eq!(
            "1\n00:01:59,123 --> 00:02:03,123\nsomething something capitalism bad\n",
            sub.to_string()
        )
    }
}
