//! WezTerm glue: the generated config, the window launcher, the font handler and `presiterm doctor`.
//!
//! The only program presiterm starts is `wezterm` itself.

use std::fmt;
use std::io::Write as _;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use swatchbook::Palette;

use crate::config::Config;
use crate::deck;
use crate::term;

/// Window grid of a presiterm window.
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    /// Columns.
    pub cols: usize,
    /// Rows.
    pub rows: usize,
    /// Font size in points.
    pub font_size: f32,
}

/// The Lua that makes WezTerm follow `presiterm_font`: add it to your own `wezterm.lua` to use the
/// font keys outside a `presiterm --window`.
pub const HANDLER: &str = "wezterm.on('user-var-changed', function(window, pane, name, value)
  if name == 'presiterm_font' then
    local overrides = window:get_config_overrides() or {}
    overrides.font_size = tonumber(value)
    window:set_config_overrides(overrides)
  end
end)";

/// Work out the cell size in points of `JetBrains Mono` at `font_size` (advance 0.6 em, line height 1.32 em).
#[must_use]
pub fn cell_points(font_size: f32) -> (f64, f64) {
    (f64::from(font_size) * 0.6, f64::from(font_size) * 1.32)
}

/// Work out the grid a window of `screen` points holds at `font_size`, less the window padding.
#[must_use]
pub fn grid_for(font_size: f32, screen: (f64, f64)) -> (usize, usize, (f64, f64)) {
    let (cw, ch) = cell_points(font_size);
    let cols = ((screen.0 - 80.0) / cw).floor().max(1.0) as usize;
    let rows = ((screen.1 - 40.0) / ch).floor().max(1.0) as usize;
    (cols, rows, (cw, ch))
}

/// Build the config a presiterm window starts with: no tab bar, the theme's colours, the font, the handler.
#[must_use]
pub fn config_lua(geometry: Geometry, palette: &Palette) -> String {
    format!(
        "local wezterm = require 'wezterm'
local config = wezterm.config_builder()
config.font = wezterm.font('JetBrains Mono', {{ weight = 'Medium' }})
config.font_size = {size}
config.line_height = 1.0
config.cell_width = 1.0
config.enable_tab_bar = false
config.window_padding = {{ left = 40, right = 40, top = 20, bottom = 20 }}
config.initial_cols = {cols}
config.initial_rows = {rows}
config.adjust_window_size_when_changing_font_size = false
config.window_close_confirmation = 'NeverPrompt'
config.audible_bell = 'Disabled'
-- no ligatures: `|=` and `->` stay what was typed
config.harfbuzz_features = {{ 'calt=0', 'clig=0', 'liga=0' }}
config.colors = {{ foreground = '{fg}', background = '{bg}' }}
{HANDLER}
return config
",
        size = geometry.font_size,
        cols = geometry.cols,
        rows = geometry.rows,
        fg = palette.ink,
        bg = palette.ground,
    )
}

/// Open a presiterm window on `deck`: WezTerm with the generated config, running this binary.
/// Returns only on error (it replaces the process).
pub fn launch(
    deck: &Path,
    geometry: Geometry,
    palette: &Palette,
    inner_args: &[String],
) -> Result<()> {
    let deck = deck
        .canonicalize()
        .with_context(|| format!("resolve {}", deck.display()))?;
    let cwd = deck.parent().map_or_else(|| ".".into(), Path::to_path_buf);
    let config = std::env::temp_dir().join(format!("presiterm-{}.lua", std::process::id()));
    std::fs::write(&config, config_lua(geometry, palette))
        .with_context(|| format!("write {}", config.display()))?;
    let me = std::env::current_exe().context("find the presiterm binary")?;
    // `--config-file` (not `--config`) and before `start`: initial_cols/rows are read from it.
    let error = Command::new("wezterm")
        .arg("--config-file")
        .arg(&config)
        .arg("start")
        .arg("--always-new-process")
        .arg("--cwd")
        .arg(&cwd)
        .arg("--")
        .arg(me)
        .arg(&deck)
        .arg("--window-font")
        .arg(geometry.font_size.to_string())
        .args(inner_args)
        .exec();
    Err(anyhow::Error::new(error).context("start wezterm (is it installed and on PATH?)"))
}

/// What `presiterm --window` forwards to the window it opens, as plain values.
#[derive(Clone, Debug)]
pub struct OpenOptions {
    /// The deck to present.
    pub file: PathBuf,
    /// Window width in cells from `--cols`; else the config's grid.
    pub cols: Option<usize>,
    /// Window height in cells from `--rows`; else the config's grid.
    pub rows: Option<usize>,
    /// What the flags leave open: the config files and the defaults.
    pub config: Config,
    /// Font size in points from the flag; beats the deck's frontmatter.
    pub font_size: Option<f32>,
    /// Slide to start at, 1-based.
    pub slide: Option<usize>,
    /// Every bullet is a reveal.
    pub reveal: bool,
    /// Do not reload on file changes.
    pub no_watch: bool,
    /// Control socket path.
    pub control: Option<PathBuf>,
    /// Cell size in pixels, as the text of `--cell-px`.
    pub cell_px: Option<String>,
}

/// Start a dedicated WezTerm window that runs a deck, passing the presenting flags on.
/// Returns only on error (it replaces the process).
pub fn open(options: &OpenOptions) -> Result<()> {
    let file = options.file.as_path();
    let deck = deck::read(file, false, &options.config)?.deck;
    let font = options
        .config
        .resolve_font_size(options.font_size, deck.font_size, None);
    let (cols, rows) = options.config.resolve_grid(options.cols, options.rows);
    let mut inner = Vec::new();
    if let Some(slide) = options.slide {
        inner.extend(["--slide".to_string(), slide.to_string()]);
    }
    if options.reveal {
        inner.push("--reveal".into());
    }
    if options.no_watch {
        inner.push("--no-watch".into());
    }
    if let Some(control) = &options.control {
        inner.extend(["--control".to_string(), control.display().to_string()]);
    }
    if let Some(cell_px) = &options.cell_px {
        inner.extend(["--cell-px".to_string(), cell_px.clone()]);
    }
    launch(
        file,
        Geometry {
            cols,
            rows,
            font_size: font,
        },
        deck.palette(),
        &inner,
    )
}

/// What `presiterm doctor` found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Diagnosis {
    /// The running WezTerm follows `presiterm_font`.
    Follows,
    /// It does not; the handler has to go into `wezterm.lua`.
    NeedsHandler,
}

impl fmt::Display for Diagnosis {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Follows => write!(
                out,
                "ok: this WezTerm follows presiterm_font (cell size changed, then restored)"
            ),
            Self::NeedsHandler => write!(
                out,
                "The font keys need this handler in your wezterm.lua (before `return config`):\n\n{HANDLER}\n\n`presiterm --window` has it built in."
            ),
        }
    }
}

/// Check that the running WezTerm follows `presiterm_font` by changing the font size and back.
/// The caller prints the [`Diagnosis`].
pub fn doctor() -> Result<Diagnosis> {
    term::require_wezterm()?;
    let wait_for_change = |from: term::Size| -> bool {
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(2500) {
            std::thread::sleep(Duration::from_millis(100));
            if term::size().is_ok_and(|now| now != from) {
                std::thread::sleep(Duration::from_millis(300));
                return true;
            }
        }
        false
    };
    let set = |points: &str| {
        // stderr: the terminal even when stdout is redirected
        let _ =
            std::io::stderr().write_all(term::set_user_var("presiterm_font", points).as_bytes());
    };
    let before = term::size()?;
    set("30");
    let moved = wait_for_change(before);
    let mid = term::size()?;
    if moved {
        set("20");
        wait_for_change(mid);
    }
    set("reset");
    std::thread::sleep(Duration::from_millis(600));
    Ok(if moved {
        Diagnosis::Follows
    } else {
        Diagnosis::NeedsHandler
    })
}
