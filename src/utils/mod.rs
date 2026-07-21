use crate::Error;
use num_traits::Float;
use std::{fs::File, io::Write};
use bytes::Bytes;
use num_format::{Locale, ToFormattedString};

pub mod discord_utils;
pub mod osu_utils;
pub mod osu_pp;
pub mod database;

pub fn save_file(bytes: Bytes, path: &str) -> Result<(), Error> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    Ok(())
}
pub trait CommaFormat {
    fn format(&self) -> String;
}

pub trait CommaFormatFloat {
    fn format(&self) -> String;
    fn two_decimal(&self) -> f32;
}

impl<T> CommaFormat for T
where
    T: ToFormattedString,
{
    fn format(&self) -> String {
        self.to_formatted_string(&Locale::en)
    }
}

impl<T> CommaFormatFloat for T
where
    T: ToString + Float,
{
    /// Formats a float into a string with comma-separated integer part and up to two decimals.
    fn format(&self) -> String {
        let integer_part = self
            .floor()
            .to_i32()
            .unwrap_or(0)
            .to_formatted_string(&Locale::en);

        let decimal_part = (self.fract().to_f32().unwrap_or(0.0) * 100.0).round() / 100.0;

        if decimal_part == 0.0 {
            return integer_part;
        }

        let mut formatted_decimals: String = decimal_part.to_string().chars().skip(1).collect();
        if formatted_decimals.len() == 2 {
            formatted_decimals.push('0');
        }

        format!("{}{}", integer_part, formatted_decimals)
    }

    fn two_decimal(&self) -> f32 {
        let num = self.to_f32().unwrap_or(0.0);
        (num * 100.0).round() / 100.0
    }
}
