//! Design tokens: two palettes, the font families and the diagram text sizes.
//!
//! [`Theme::default`] is a complete, open theme. A [`Theme`] deserializes from a table in which
//! every key is optional: what the table leaves out keeps its default, so one line can change one
//! colour.
//!
//! ```
//! use swatchbook::{Mode, Rgb, Theme};
//!
//! let theme: Theme = toml::from_str("[dark]\naccent = \"#ff8800\"\n").expect("valid");
//! assert_eq!(theme.palette(Mode::Dark).accent, Rgb(0xff, 0x88, 0x00));
//! assert_eq!(theme.palette(Mode::Light), Theme::default().palette(Mode::Light));
//! ```

#[cfg(feature = "fonts")]
pub mod fonts;
mod rgb;

pub use rgb::{ParseRgbError, Rgb};
use serde::Deserialize;

/// Which of the two palettes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Light text on a dark ground.
    #[default]
    Dark,
    /// Dark text on a light ground.
    Light,
}

/// The colours of one ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Palette {
    /// The background.
    pub ground: Rgb,
    /// Body text.
    pub ink: Rgb,
    /// Secondary text, rules, lines.
    pub muted: Rgb,
    /// Headings, markers, highlights.
    pub accent: Rgb,
    /// Panels on the ground (code, boxes).
    pub surface: Rgb,
    /// Edges of panels.
    pub border: Rgb,
    /// Good news, strings.
    pub ok: Rgb,
    /// Attention, numbers, marked text.
    pub warn: Rgb,
    /// Errors.
    pub stop: Rgb,
}

impl Palette {
    /// The default dark palette.
    pub const DARK: Self = Self {
        ground: Rgb(0x17, 0x19, 0x1f),
        ink: Rgb(0xe8, 0xe6, 0xe3),
        muted: Rgb(0x8b, 0x93, 0xa7),
        accent: Rgb(0x7c, 0xc4, 0xfa),
        surface: Rgb(0x23, 0x27, 0x33),
        border: Rgb(0x36, 0x3c, 0x4d),
        ok: Rgb(0x8f, 0xd6, 0x94),
        warn: Rgb(0xf2, 0xc4, 0x6d),
        stop: Rgb(0xf2, 0x8b, 0x82),
    };

    /// The default light palette.
    pub const LIGHT: Self = Self {
        ground: Rgb(0xfb, 0xfa, 0xf7),
        ink: Rgb(0x1d, 0x21, 0x29),
        muted: Rgb(0x66, 0x70, 0x85),
        accent: Rgb(0x1d, 0x6f, 0xb8),
        surface: Rgb(0xee, 0xf0, 0xf4),
        border: Rgb(0xd4, 0xd8, 0xe0),
        ok: Rgb(0x2f, 0x7d, 0x3a),
        warn: Rgb(0xa1, 0x5c, 0x00),
        stop: Rgb(0xc0, 0x39, 0x2b),
    };

    /// Every token with its name, in declaration order (for templates and previews).
    #[must_use]
    pub const fn named(&self) -> [(&'static str, Rgb); 9] {
        [
            ("ground", self.ground),
            ("ink", self.ink),
            ("muted", self.muted),
            ("accent", self.accent),
            ("surface", self.surface),
            ("border", self.border),
            ("ok", self.ok),
            ("warn", self.warn),
            ("stop", self.stop),
        ]
    }
}

/// The font families a theme sets text in.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Fonts {
    /// Proportional text (diagram labels).
    pub sans: String,
    /// Monospaced text (code, diagrams drawn in a terminal's spirit).
    pub mono: String,
}

impl Default for Fonts {
    fn default() -> Self {
        Self {
            sans: "Inter".into(),
            mono: "Fira Code".into(),
        }
    }
}

/// Text sizes in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sizes {
    /// Text in a diagram.
    pub text: f32,
    /// Labels on a diagram's edges.
    pub label: f32,
}

impl Default for Sizes {
    fn default() -> Self {
        Self {
            text: 13.0,
            label: 11.0,
        }
    }
}

/// A complete set of tokens.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(from = "Overlay")]
pub struct Theme {
    /// The dark palette.
    pub dark: Palette,
    /// The light palette.
    pub light: Palette,
    /// The font families.
    pub fonts: Fonts,
    /// The text sizes.
    pub sizes: Sizes,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            dark: Palette::DARK,
            light: Palette::LIGHT,
            fonts: Fonts::default(),
            sizes: Sizes::default(),
        }
    }
}

impl Theme {
    /// Return the palette of `mode`.
    #[must_use]
    pub const fn palette(&self, mode: Mode) -> &Palette {
        match mode {
            Mode::Dark => &self.dark,
            Mode::Light => &self.light,
        }
    }
}

/// A theme as written: every key optional, unknown keys refused.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Overlay {
    #[serde(default)]
    dark: PaletteOverlay,
    #[serde(default)]
    light: PaletteOverlay,
    #[serde(default)]
    fonts: FontsOverlay,
    #[serde(default)]
    sizes: SizesOverlay,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaletteOverlay {
    ground: Option<Rgb>,
    ink: Option<Rgb>,
    muted: Option<Rgb>,
    accent: Option<Rgb>,
    surface: Option<Rgb>,
    border: Option<Rgb>,
    ok: Option<Rgb>,
    warn: Option<Rgb>,
    stop: Option<Rgb>,
}

impl PaletteOverlay {
    fn over(self, base: Palette) -> Palette {
        Palette {
            ground: self.ground.unwrap_or(base.ground),
            ink: self.ink.unwrap_or(base.ink),
            muted: self.muted.unwrap_or(base.muted),
            accent: self.accent.unwrap_or(base.accent),
            surface: self.surface.unwrap_or(base.surface),
            border: self.border.unwrap_or(base.border),
            ok: self.ok.unwrap_or(base.ok),
            warn: self.warn.unwrap_or(base.warn),
            stop: self.stop.unwrap_or(base.stop),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FontsOverlay {
    sans: Option<String>,
    mono: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SizesOverlay {
    text: Option<f32>,
    label: Option<f32>,
}

impl From<Overlay> for Theme {
    fn from(overlay: Overlay) -> Self {
        let base = Self::default();
        Self {
            dark: overlay.dark.over(base.dark),
            light: overlay.light.over(base.light),
            fonts: Fonts {
                sans: overlay.fonts.sans.unwrap_or(base.fonts.sans),
                mono: overlay.fonts.mono.unwrap_or(base.fonts.mono),
            },
            sizes: Sizes {
                text: overlay.sizes.text.unwrap_or(base.sizes.text),
                label: overlay.sizes.label.unwrap_or(base.sizes.label),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_table_is_the_default_theme() {
        let theme: Theme = toml::from_str("").expect("valid");
        assert_eq!(theme, Theme::default());
    }

    #[test]
    fn one_key_changes_one_token() {
        let theme: Theme = toml::from_str(
            "[light]\nink = \"000000\"\n[fonts]\nmono = \"Hack\"\n[sizes]\ntext = 15\n",
        )
        .expect("valid");
        assert_eq!(theme.light.ink, Rgb(0, 0, 0));
        assert_eq!(theme.light.accent, Palette::LIGHT.accent);
        assert_eq!(theme.fonts.mono, "Hack");
        assert_eq!(theme.fonts.sans, "Inter");
        assert!((theme.sizes.text - 15.0).abs() < f32::EPSILON);
    }

    #[test]
    fn unknown_keys_and_bad_colours_are_refused() {
        assert!(toml::from_str::<Theme>("[dark]\nacent = \"#000000\"\n").is_err());
        assert!(toml::from_str::<Theme>("[dark]\naccent = \"red\"\n").is_err());
        assert!(toml::from_str::<Theme>("colour = 1\n").is_err());
    }

    #[test]
    fn the_default_palettes_keep_text_readable_on_their_ground() {
        for palette in [Palette::DARK, Palette::LIGHT] {
            for (name, colour) in [("ink", palette.ink), ("accent", palette.accent)] {
                let ratio = colour.contrast(palette.ground);
                assert!(ratio >= 4.5, "{name}: contrast {ratio:.2}");
            }
        }
    }
}
