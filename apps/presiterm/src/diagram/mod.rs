//! Diagrams and pictures: mermaid -> SVG -> PNG in-process, files read once, everything cached.
//!
//! Pure Rust. Mermaid is drawn by mermaid-rs-renderer in the deck's design tokens, Typst by
//! typstkit. A diagram is drawn at the pixel size of its cell rectangle, so a new font size draws
//! it again, sharp.
//!
//! This file is the spine: [`Assets`] loads, caches and hands out frames. The colours are `colours`,
//! SVG to PNG is `raster`, Typst diagrams are `typst`, stepped diagrams are `steps`.

mod colours;
mod raster;
mod steps;
#[cfg(test)]
mod tests;
mod typst;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use swatchbook::{Fonts, Palette, Sizes};

use crate::deck::{Deck, Diagram, Picture};

pub use colours::options;
pub use raster::image_size;
pub use steps::step_nodes;

use raster::{svg_size, svg_to_png};
use steps::{number_nodes, step_map, typst_steps};
use typst::{TYPST_PPI, TYPST_ROW_PT, typst_source, typst_within_budget, within_budget};

/// What a picture or diagram frame needs from the layout.
#[derive(Clone, Debug)]
pub struct Loaded {
    /// Encoded image (PNG, GIF or JPEG).
    pub bytes: Arc<Vec<u8>>,
    /// Pixel width / pixel height.
    pub aspect: f64,
}

/// What a deck's pictures are drawn in: its palette, faces and sizes, and the corner logo.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    /// The palette of the deck's ground.
    pub palette: Palette,
    /// The font families.
    pub fonts: Fonts,
    /// The diagram text sizes.
    pub sizes: Sizes,
    /// A picture for the top left corner of every slide.
    pub logo: Option<PathBuf>,
}

impl Look {
    /// Take the look of `deck`.
    #[must_use]
    pub fn of(deck: &Deck) -> Self {
        Self {
            palette: *deck.palette(),
            fonts: deck.tokens.fonts.clone(),
            sizes: deck.tokens.sizes,
            logo: deck.logo.clone(),
        }
    }
}

/// Loads and caches everything that is not text.
#[derive(Debug)]
pub struct Assets {
    base: PathBuf,
    look: Look,
    cache: Option<PathBuf>,
    svgs: HashMap<u64, Arc<String>>,
    pngs: HashMap<(u64, u32), Arc<Vec<u8>>>,
    files: HashMap<PathBuf, Arc<Vec<u8>>>,
}

/// The picture cache: `$XDG_CACHE_HOME/presiterm`, else `~/.cache/presiterm` (path resolution, not config).
fn default_cache() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .map(|cache| cache.join("presiterm"))
}

impl Assets {
    /// Create the assets of `deck` in `base`, cached under the user's cache dir.
    #[must_use]
    pub fn new(base: &Path, deck: &Deck) -> Self {
        Self::with_cache(base, deck, default_cache())
    }

    /// Create the assets of `deck` in `base`, with the picture cache in `cache` (`None` keeps
    /// everything in memory).
    #[must_use]
    pub fn with_cache(base: &Path, deck: &Deck, cache: Option<PathBuf>) -> Self {
        Self {
            base: base.to_path_buf(),
            look: Look::of(deck),
            cache,
            svgs: HashMap::new(),
            pngs: HashMap::new(),
            files: HashMap::new(),
        }
    }

    /// Draw the frames of a diagram: one, or with `steps` one more than there are nodes.
    pub fn frames(&mut self, diagram: &Diagram) -> Result<Vec<Arc<String>>> {
        let count = if diagram.steps {
            step_nodes(diagram).len() + 1
        } else {
            1
        };
        (0..count).map(|k| self.svg(diagram, k)).collect()
    }

    /// Count the key presses a diagram takes (0 without steps).
    #[must_use]
    pub fn steps(diagram: &Diagram) -> usize {
        if diagram.steps && diagram.typst {
            typst_steps(diagram)
        } else if diagram.steps {
            step_nodes(diagram).len()
        } else {
            0
        }
    }

    fn svg(&mut self, diagram: &Diagram, lit: usize) -> Result<Arc<String>> {
        let source = if lit == 0 {
            diagram.source.clone()
        } else {
            let node = &step_nodes(diagram)[lit - 1];
            let p = &self.look.palette;
            format!(
                "{}\n    style {node} fill:{},stroke:{},color:{}\n",
                diagram.source.trim_end(),
                p.accent,
                p.ink,
                p.ground
            )
        };
        let key = fnv(&format!("{:?}|{source}", self.look));
        if let Some(svg) = self.svgs.get(&key) {
            return Ok(svg.clone());
        }
        let options = options(&self.look.palette, &self.look.fonts);
        let svg = mermaid_rs_renderer::render_with_options(&source, options)
            .map_err(|e| anyhow::anyhow!("{}: mermaid: {e}", diagram.origin))?;
        let svg = Arc::new(svg);
        self.svgs.insert(key, svg.clone());
        Ok(svg)
    }

    /// Measure width / height of the first frame.
    pub fn aspect(&mut self, diagram: &Diagram) -> Result<f64> {
        if diagram.typst {
            let png = self.typst_png(diagram, 0)?;
            let (w, h) = image_size(&png).context("typst: the PNG has no size")?;
            return Ok(f64::from(w) / f64::from(h));
        }
        let svg = self.svg(diagram, 0)?;
        let (w, h) = svg_size(&svg, &self.look.fonts.sans)?;
        Ok(f64::from(w / h))
    }

    /// Work out the rows a Typst diagram needs for its text to match a line of slide text: its natural
    /// height in points over `TYPST_ROW_PT`.
    pub fn natural_rows(&mut self, diagram: &Diagram) -> Result<usize> {
        let png = self.typst_png(diagram, 0)?;
        let (_, h) = image_size(&png).context("typst: the PNG has no size")?;
        let pt = f64::from(h) * 72.0 / f64::from(TYPST_PPI);
        Ok(((pt / TYPST_ROW_PT).ceil() as usize).max(1))
    }

    /// Draw the configured logo `px_height` high, with its width / height; `None` without one.
    pub fn logo(&mut self, px_height: u32) -> Option<Result<Loaded>> {
        let path = self.look.logo.clone()?;
        let picture = Picture {
            alt: "logo".into(),
            path: path.display().to_string(),
            height: None,
            width: None,
        };
        Some(self.picture(&picture, Some(px_height)))
    }

    /// Return the PNG of frame `lit`, `px_height` pixels high.
    pub fn diagram_png(
        &mut self,
        diagram: &Diagram,
        lit: usize,
        px_height: u32,
    ) -> Result<Arc<Vec<u8>>> {
        if diagram.typst {
            return self.typst_png(diagram, lit);
        }
        let svg = self.svg(diagram, lit)?;
        self.rasterise(&svg, px_height)
    }

    /// A Typst (fletcher) diagram as a tight transparent PNG, text and lines in the theme's ink.
    /// Drawn once at a fixed density; the terminal scales it to the cell rectangle.
    fn typst_png(&mut self, diagram: &Diagram, step: usize) -> Result<Arc<Vec<u8>>> {
        let body = if diagram.steps {
            number_nodes(&diagram.source, &step_map(diagram)).0
        } else {
            diagram.source.clone()
        };
        let source = typst_source(&body, &self.look, step);
        let key = (fnv(&format!("{TYPST_PPI}|{source}")), 0);
        if let Some(png) = self.pngs.get(&key) {
            return Ok(png.clone());
        }
        let on_disk = self
            .cache
            .as_ref()
            .map(|c| c.join(format!("{:016x}-typst.png", key.0)));
        if let Some(png) = on_disk
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .filter(|png| within_budget(png))
        {
            let png = Arc::new(png);
            self.pngs.insert(key, png.clone());
            return Ok(png);
        }
        let png = Arc::new(
            typst_within_budget(&source).with_context(|| format!("{}: typst", diagram.origin))?,
        );
        if let Some(path) = on_disk {
            let _ = std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")));
            let _ = std::fs::write(path, png.as_slice());
        }
        self.pngs.insert(key, png.clone());
        Ok(png)
    }

    fn rasterise(&mut self, svg: &Arc<String>, px_height: u32) -> Result<Arc<Vec<u8>>> {
        let key = (fnv(&format!("{:?}|{svg}", self.look)), px_height);
        if let Some(png) = self.pngs.get(&key) {
            return Ok(png.clone());
        }
        let on_disk = self
            .cache
            .as_ref()
            .map(|c| c.join(format!("{:016x}-{px_height}.png", key.0)));
        if let Some(png) = on_disk.as_ref().and_then(|p| std::fs::read(p).ok()) {
            let png = Arc::new(png);
            self.pngs.insert(key, png.clone());
            return Ok(png);
        }
        let png = Arc::new(svg_to_png(svg, &self.look.fonts.sans, px_height)?);
        if let Some(path) = on_disk {
            let _ = std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")));
            let _ = std::fs::write(path, png.as_slice());
        }
        self.pngs.insert(key, png.clone());
        Ok(png)
    }

    /// Load a picture file: PNG, GIF and JPEG pass straight through; SVG is drawn.
    pub fn picture(&mut self, picture: &Picture, px_height: Option<u32>) -> Result<Loaded> {
        let path = self.base.join(&picture.path);
        let bytes = if let Some(b) = self.files.get(&path) {
            b.clone()
        } else {
            let b = Arc::new(
                std::fs::read(&path).with_context(|| format!("{}: missing image", picture.path))?,
            );
            self.files.insert(path.clone(), b.clone());
            b
        };
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
        {
            let svg = Arc::new(String::from_utf8_lossy(&bytes).into_owned());
            let (w, h) = svg_size(&svg, &self.look.fonts.sans)?;
            let png = self.rasterise(&svg, px_height.unwrap_or_else(|| h.ceil() as u32))?;
            return Ok(Loaded {
                bytes: png,
                aspect: f64::from(w / h),
            });
        }
        let (w, h) = image_size(&bytes)
            .with_context(|| format!("{}: not a PNG, GIF or JPEG", picture.path))?;
        ensure!(w > 0 && h > 0, "{}: empty image", picture.path);
        Ok(Loaded {
            bytes,
            aspect: f64::from(w) / f64::from(h),
        })
    }

    /// List the files the deck points at, to notice an edit.
    #[must_use]
    pub fn watched(&self) -> Vec<PathBuf> {
        self.files.keys().cloned().collect()
    }
}

fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}
