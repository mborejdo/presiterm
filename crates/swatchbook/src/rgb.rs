//! A truecolor value: parsed from `#rrggbb`, printed back the same way.

use std::fmt;
use std::str::FromStr;

use serde::Deserialize;

/// A truecolor value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct Rgb(pub u8, pub u8, pub u8);

/// A colour that is not `#rrggbb` (or `rrggbb`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseRgbError(String);

impl fmt::Display for ParseRgbError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "expected a colour like \"#1d6fb8\", got \"{}\"",
            self.0
        )
    }
}

impl std::error::Error for ParseRgbError {}

impl FromStr for Rgb {
    type Err = ParseRgbError;

    /// Parse `#rrggbb` or `rrggbb`.
    ///
    /// # Examples
    ///
    /// ```
    /// use swatchbook::Rgb;
    ///
    /// assert_eq!("#C16974".parse(), Ok(Rgb(193, 105, 116)));
    /// assert!("#fff".parse::<Rgb>().is_err());
    /// ```
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let hex = text.trim().trim_start_matches('#');
        let error = || ParseRgbError(text.to_string());
        if hex.len() != 6 || !hex.is_ascii() {
            return Err(error());
        }
        let part = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).map_err(|_| error());
        Ok(Self(part(0)?, part(2)?, part(4)?))
    }
}

impl TryFrom<String> for Rgb {
    type Error = ParseRgbError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl fmt::Display for Rgb {
    /// Print as `#rrggbb`.
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl Rgb {
    /// Mix towards `other` by `part` (0.0 is `self`, 1.0 is `other`).
    ///
    /// # Examples
    ///
    /// ```
    /// use swatchbook::Rgb;
    ///
    /// assert_eq!(Rgb(0, 0, 0).mix(Rgb(100, 200, 50), 0.5), Rgb(50, 100, 25));
    /// ```
    #[must_use]
    pub fn mix(self, other: Self, part: f32) -> Self {
        let channel = |a: u8, b: u8| {
            let value = (f32::from(b) - f32::from(a)).mul_add(part.clamp(0.0, 1.0), f32::from(a));
            // Clamped to 0..=255 above, so the cast cannot truncate.
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a mix of two channels stays in 0..=255"
            )]
            let byte = value.round() as u8;
            byte
        };
        Self(
            channel(self.0, other.0),
            channel(self.1, other.1),
            channel(self.2, other.2),
        )
    }

    /// The WCAG relative luminance, 0.0 (black) to 1.0 (white).
    #[must_use]
    pub fn luminance(self) -> f64 {
        let linear = |channel: u8| {
            let c = f64::from(channel) / 255.0;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.0722f64.mul_add(
            linear(self.2),
            0.2126f64.mul_add(linear(self.0), 0.7152 * linear(self.1)),
        )
    }

    /// The WCAG contrast ratio against `other`, 1.0 to 21.0.
    #[must_use]
    pub fn contrast(self, other: Self) -> f64 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_round_trips_through_parse() {
        let colour = Rgb(0x1d, 0x6f, 0xb8);
        assert_eq!(colour.to_string(), "#1d6fb8");
        assert_eq!(colour.to_string().parse(), Ok(colour));
    }

    #[test]
    fn contrast_of_black_on_white_is_21() {
        assert!((Rgb(0, 0, 0).contrast(Rgb(255, 255, 255)) - 21.0).abs() < 1e-9);
    }

    #[test]
    fn a_multibyte_string_of_six_bytes_is_an_error_not_a_panic() {
        assert!("ä1234".parse::<Rgb>().is_err());
    }
}
