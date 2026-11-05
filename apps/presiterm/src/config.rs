//! What presiterm falls back on, and the config files that change it.
//!
//! Precedence, strongest first: a command-line flag, the deck's frontmatter, the closest
//! `presiterm.toml`, the user file, the built-in default. The two file levels are read by
//! [`Config::load`], the one place that touches config files:
//!
//! 1. `presiterm.toml`, found by walking up from the deck's folder to the filesystem root; the closest
//!    one wins and farther ones are ignored.
//! 2. The user file `$XDG_CONFIG_HOME/presiterm/config.toml`, else `~/.config/presiterm/config.toml`.
//! 3. The built-in defaults below.
//!
//! Files are not merged as wholes: a key the closest file lacks falls through to the next level.
//! A missing file is skipped; an unknown key or a malformed value is an error that names the file
//! and the key. Settings never come from environment variables; the environment reads that remain
//! are path resolution (`XDG_CONFIG_HOME`/`HOME` here,
//! `XDG_CACHE_HOME`/`HOME` for the picture cache in `diagram`) and terminal detection
//! (`TERM_PROGRAM`/`WEZTERM_*` in `term`).

use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::deck::Deck;

/// The file name looked up in the deck's folder and every folder above it.
pub const PROJECT_FILE: &str = "presiterm.toml";

/// Font size in points when neither a flag, the deck nor the window names one.
pub const DEFAULT_FONT_SIZE: f32 = 42.0;

/// Screen size in points a window must fit when `--screen` is not given.
pub const DEFAULT_SCREEN: (f64, f64) = (1728.0, 1117.0);

/// Window grid in cells (columns, rows) when `--cols` and `--rows` are not given.
pub const DEFAULT_GRID: (usize, usize) = (58, 20);

/// Where a value came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Source {
    /// The built-in default.
    #[default]
    Default,
    /// This config file.
    File(PathBuf),
}

impl fmt::Display for Source {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => write!(out, "default"),
            Self::File(path) => write!(out, "{}", path.display()),
        }
    }
}

/// Where each setting of a [`Config`] came from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sources {
    /// Source of the font size.
    pub font_size: Source,
    /// Source of the screen size.
    pub screen: Source,
    /// Source of the window grid.
    pub grid: Source,
    /// Source of the design tokens.
    pub theme: Source,
    /// Source of the logo.
    pub logo: Source,
}

/// What happened to a config file that was looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileState {
    /// Read; its keys count.
    Used,
    /// A `presiterm.toml` farther up than the closest one; not read.
    Shadowed,
    /// The user file does not exist; skipped.
    Missing,
}

impl fmt::Display for FileState {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Used => "used",
            Self::Shadowed => "ignored (a closer presiterm.toml wins)",
            Self::Missing => "not found",
        })
    }
}

/// A config file presiterm looked at, and what it did with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigFile {
    /// Where the file is (or would be).
    pub path: PathBuf,
    /// Whether it counts.
    pub state: FileState,
}

/// What presiterm uses when the flags and the deck say nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Font size in points.
    pub font_size: f32,
    /// Screen size in points.
    pub screen: (f64, f64),
    /// Window grid in cells (columns, rows).
    pub grid: (usize, usize),
    /// The design tokens decks are drawn in: the `[theme]` table over the defaults.
    pub theme: swatchbook::Theme,
    /// A picture for the top left corner of every slide, resolved against its config file.
    pub logo: Option<PathBuf>,
    /// Where each of the values above came from.
    pub sources: Sources,
    /// The config files looked at, closest first.
    pub files: Vec<ConfigFile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_size: DEFAULT_FONT_SIZE,
            screen: DEFAULT_SCREEN,
            grid: DEFAULT_GRID,
            theme: swatchbook::Theme::default(),
            logo: None,
            sources: Sources::default(),
            files: Vec::new(),
        }
    }
}

/// The keys of a config file as written, before they are checked; an unknown key fails here.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLayer {
    font_size: Option<toml::Value>,
    screen: Option<toml::Value>,
    grid: Option<toml::Value>,
    theme: Option<toml::Value>,
    logo: Option<toml::Value>,
}

/// The keys of one config file, checked.
#[derive(Debug, Default, PartialEq)]
struct Layer {
    font_size: Option<f32>,
    screen: Option<(f64, f64)>,
    grid: Option<(usize, usize)>,
    theme: Option<swatchbook::Theme>,
    logo: Option<PathBuf>,
}

/// How a TOML value reads in an error message.
fn got(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => format!("\"{text}\""),
        other => other.type_str().to_string(),
    }
}

/// Two positive numbers from a TOML string like `"1728x1117"`; `key` and `file` name it in the error.
fn size_pair(file: &Path, key: &str, value: &toml::Value, example: &str) -> Result<(f64, f64)> {
    let file = file.display();
    let toml::Value::String(text) = value else {
        anyhow::bail!(
            "{file}: {key}: expected a string like \"{example}\", got {}",
            got(value)
        )
    };
    let (a, b) = pair::<f64>(text, &format!("{file}: {key}"))?;
    anyhow::ensure!(
        a.is_finite() && b.is_finite() && a > 0.0 && b > 0.0,
        "{file}: {key}: expected two positive numbers like {example}, got \"{text}\""
    );
    Ok((a, b))
}

impl RawLayer {
    /// Check each key's value; `path` names the file in the error.
    fn check(self, path: &Path) -> Result<Layer> {
        let font_size = self
            .font_size
            .as_ref()
            .map(|value| {
                let points = match value {
                    toml::Value::Integer(n) => Some(*n as f32),
                    toml::Value::Float(x) => Some(*x as f32),
                    _ => None,
                };
                points
                    .filter(|p| p.is_finite() && *p > 0.0)
                    .with_context(|| {
                        format!(
                            "{}: font_size: expected a positive number of points like 42, got {}",
                            path.display(),
                            got(value)
                        )
                    })
            })
            .transpose()?;
        let screen = self
            .screen
            .as_ref()
            .map(|value| size_pair(path, "screen", value, "1728x1117"))
            .transpose()?;
        let grid = self
            .grid
            .as_ref()
            .map(|value| {
                let (cols, rows) = size_pair(path, "grid", value, "58x20")?;
                anyhow::ensure!(
                    cols.fract() == 0.0 && rows.fract() == 0.0,
                    "{}: grid: expected two whole numbers like 58x20, got {}",
                    path.display(),
                    got(value)
                );
                Ok((cols as usize, rows as usize))
            })
            .transpose()?;
        let theme = self
            .theme
            .map(|value| {
                value
                    .try_into::<swatchbook::Theme>()
                    .with_context(|| format!("{}: theme", path.display()))
            })
            .transpose()?;
        let logo = self
            .logo
            .as_ref()
            .map(|value| {
                let toml::Value::String(file) = value else {
                    anyhow::bail!(
                        "{}: logo: expected a path like \"logo.svg\", got {}",
                        path.display(),
                        got(value)
                    )
                };
                Ok(path.parent().unwrap_or_else(|| Path::new(".")).join(file))
            })
            .transpose()?;
        Ok(Layer {
            font_size,
            screen,
            grid,
            theme,
            logo,
        })
    }
}

/// Read and check one config file.
fn read_layer(path: &Path) -> Result<Layer> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let raw: RawLayer =
        toml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    raw.check(path)
}

/// Every `presiterm.toml` from `dir` up to the filesystem root, closest first.
fn project_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let start = std::path::absolute(dir)
        .with_context(|| format!("resolve the deck folder {}", dir.display()))?;
    Ok(start
        .ancestors()
        .map(|folder| folder.join(PROJECT_FILE))
        .filter(|file| file.is_file())
        .collect())
}

/// The user config file: `$XDG_CONFIG_HOME/presiterm/config.toml`, else `~/.config/presiterm/config.toml`.
///
/// `None` when neither variable is set. This is path resolution, not a setting.
#[must_use]
pub fn user_config_path() -> Option<PathBuf> {
    user_config_path_from(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    )
}

/// [`user_config_path`] with the two variables passed in.
fn user_config_path_from(xdg: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let base = xdg
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })?;
    Some(base.join("presiterm").join("config.toml"))
}

/// The folder config is looked up from for a deck path: the folder itself, else the file's folder.
#[must_use]
pub fn deck_dir(deck: &Path) -> PathBuf {
    if deck.is_dir() {
        return deck.to_path_buf();
    }
    deck.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Parse two numbers joined by `x`, as in `1728x1117`; `what` names the flag or key in the error.
///
/// # Examples
///
/// ```
/// use presiterm::config::pair;
///
/// assert_eq!(pair::<usize>("80x24", "--grid").expect("pair"), (80, 24));
/// assert!(pair::<usize>("80", "--grid").is_err());
/// ```
pub fn pair<T>(text: &str, what: &str) -> Result<(T, T)>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let (a, b) = text
        .split_once(['x', 'X', ':'])
        .with_context(|| format!("{what}: expected two numbers like 80x24, got \"{text}\""))?;
    let number = |part: &str| {
        part.trim()
            .parse()
            .with_context(|| format!("{what}: expected two numbers like 80x24, got \"{text}\""))
    };
    Ok((number(a)?, number(b)?))
}

/// The first of `layers` (strongest first) that sets the key `get` reads, with its file.
fn first<T: Clone>(
    layers: &[(PathBuf, Layer)],
    get: impl Fn(&Layer) -> Option<&T>,
) -> Option<(T, Source)> {
    layers.iter().find_map(|(path, layer)| {
        get(layer).map(|value| (value.clone(), Source::File(path.clone())))
    })
}

impl Config {
    /// Load the config for a deck in `deck_dir`: the closest `presiterm.toml`, then the user file,
    /// then the defaults, key by key.
    ///
    /// # Examples
    ///
    /// ```
    /// use presiterm::config::Config;
    ///
    /// let folder = tempfile::tempdir().expect("tempdir");
    /// std::fs::write(folder.path().join("presiterm.toml"), "font_size = 54\n").expect("write");
    /// let config = Config::load(folder.path()).expect("load");
    /// assert_eq!(config.font_size, 54.0);
    /// ```
    pub fn load(deck_dir: &Path) -> Result<Self> {
        Self::load_from(deck_dir, user_config_path().as_deref())
    }

    /// Load like [`Config::load`] with the user file's path given (`None`: no user file), so a
    /// test never reads the real home directory.
    pub fn load_from(deck_dir: &Path, user_file: Option<&Path>) -> Result<Self> {
        let mut files = Vec::new();
        let mut layers = Vec::new();
        for (index, path) in project_files(deck_dir)?.into_iter().enumerate() {
            let state = if index == 0 {
                layers.push((path.clone(), read_layer(&path)?));
                FileState::Used
            } else {
                FileState::Shadowed
            };
            files.push(ConfigFile { path, state });
        }
        if let Some(path) = user_file {
            let state = if path.is_file() {
                layers.push((path.to_path_buf(), read_layer(path)?));
                FileState::Used
            } else {
                FileState::Missing
            };
            files.push(ConfigFile {
                path: path.to_path_buf(),
                state,
            });
        }
        let mut config = Self {
            files,
            ..Self::default()
        };
        if let Some((value, source)) = first(&layers, |layer| layer.font_size.as_ref()) {
            config.font_size = value;
            config.sources.font_size = source;
        }
        if let Some((value, source)) = first(&layers, |layer| layer.screen.as_ref()) {
            config.screen = value;
            config.sources.screen = source;
        }
        if let Some((value, source)) = first(&layers, |layer| layer.grid.as_ref()) {
            config.grid = value;
            config.sources.grid = source;
        }
        if let Some((value, source)) = first(&layers, |layer| layer.theme.as_ref()) {
            config.theme = value;
            config.sources.theme = source;
        }
        if let Some((value, source)) = first(&layers, |layer| layer.logo.as_ref()) {
            config.logo = Some(value);
            config.sources.logo = source;
        }
        Ok(config)
    }

    /// Pick the font size: the flag, then the deck's frontmatter, then the window's, then the
    /// config (a file, else the default).
    ///
    /// # Examples
    ///
    /// ```
    /// use presiterm::config::Config;
    ///
    /// let config = Config::default();
    /// assert_eq!(config.resolve_font_size(Some(54.0), Some(36.0), None), 54.0);
    /// assert_eq!(config.resolve_font_size(None, Some(36.0), Some(30.0)), 36.0);
    /// assert_eq!(config.resolve_font_size(None, None, Some(30.0)), 30.0);
    /// assert_eq!(config.resolve_font_size(None, None, None), 42.0);
    /// ```
    #[must_use]
    pub fn resolve_font_size(
        &self,
        flag: Option<f32>,
        deck: Option<f32>,
        window: Option<f32>,
    ) -> f32 {
        flag.or(deck).or(window).unwrap_or(self.font_size)
    }

    /// The font size a config file asks for, `None` when only the default applies.
    #[must_use]
    pub fn font_override(&self) -> Option<f32> {
        matches!(self.sources.font_size, Source::File(_)).then_some(self.font_size)
    }

    /// Give `deck` the configured design tokens and logo.
    pub fn dress(&self, deck: &mut Deck) {
        deck.tokens = self.theme.clone();
        deck.logo.clone_from(&self.logo);
    }

    /// Pick the screen size: the flag, else the config.
    #[must_use]
    pub fn resolve_screen(&self, flag: Option<(f64, f64)>) -> (f64, f64) {
        flag.unwrap_or(self.screen)
    }

    /// Pick the window grid (columns, rows): each flag, else the config.
    #[must_use]
    pub fn resolve_grid(&self, cols: Option<usize>, rows: Option<usize>) -> (usize, usize) {
        (cols.unwrap_or(self.grid.0), rows.unwrap_or(self.grid.1))
    }

    /// The grid `presiterm lint` checks on: the flag, else the one a config file sets, else `None`
    /// (the grid the screen gives).
    #[must_use]
    pub fn lint_grid(&self, flag: Option<(usize, usize)>) -> Option<(usize, usize)> {
        flag.or_else(|| matches!(self.sources.grid, Source::File(_)).then_some(self.grid))
    }
}

impl fmt::Display for Config {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(out, "config files, closest first:")?;
        if self.files.is_empty() {
            writeln!(out, "  (none)")?;
        }
        for file in &self.files {
            writeln!(out, "  {}  {}", file.path.display(), file.state)?;
        }
        writeln!(
            out,
            "effective values (a command-line flag beats the deck's frontmatter, which beats these):"
        )?;
        writeln!(
            out,
            "  font_size = {}  ({})",
            self.font_size, self.sources.font_size
        )?;
        writeln!(
            out,
            "  screen    = {}x{}  ({})",
            self.screen.0, self.screen.1, self.sources.screen
        )?;
        writeln!(
            out,
            "  grid      = {}x{}  ({})",
            self.grid.0, self.grid.1, self.sources.grid
        )?;
        writeln!(
            out,
            "  theme     = {}  ({})",
            if self.sources.theme == Source::Default {
                "built-in"
            } else {
                "[theme] table"
            },
            self.sources.theme
        )?;
        write!(
            out,
            "  logo      = {}  ({})",
            self.logo
                .as_deref()
                .map_or_else(|| "none".to_string(), |p| p.display().to_string()),
            self.sources.logo
        )
    }
}

#[cfg(test)]
mod tests;
