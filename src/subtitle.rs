/*
	Copyright 2021-2023 Bricky <thatbrickster@tutanota.com>

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
    https://github.com/emk/subtitles-rs
*/

use std::fs::OpenOptions;
use std::io::{Result, Write};

#[cfg(test)]
use std::fmt;

pub struct Subtitle {
    index :usize,
    period :Period,
    caption :String
}

impl Subtitle {
    pub fn new(index :usize, begin :u128, end :u128, caption :String) -> Self {
        let period = Period::new(begin, end);

        Self {
            index, period, caption
        }
    }

    pub fn write_to(self, path :String) -> Result<()> {
        let mut file = if self.index != 1 {
            OpenOptions::new()
            .append(true)
            .open(path)?
        } else {
            OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?
        };

        writeln!(file, "{}\n{} --> {}\n{}\n",
            self.index,
            format_time(self.period.begin),
            format_time(self.period.end),
            self.caption
        )?;

        Ok(())
    }
}

#[cfg(test)]
trait Display {
    fn to_string(&self) -> String;
}

#[cfg(test)]
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
    fn new(begin :u128, end :u128) -> Self {
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
        // Arrange
        let index = 1;
        let begin = 119123;
        let end = 123123;
        let caption = "something something capitalism bad".into();

        // Act
        let sub = Subtitle::new(index, begin, end, caption);

        // Assert
        assert_eq!(
            "1\n00:01:59,123 --> 00:02:03,123\nsomething something capitalism bad\n",
            sub.to_string()
        )
    }
}
