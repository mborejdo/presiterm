//! The Markdown dialect: a deck file in, slides of blocks out.
//!
//! colonmark parses the file (CommonMark, tables, colon-fence directives); this module cuts it
//! into slides at `#` and `##` headings and at `---`, and turns each slide's Markdown into the
//! blocks presiterm lays out. Each source line of a paragraph is its own line on the slide.
//! Nothing is dropped silently: what presiterm cannot show becomes a [`Block::Error`] that the
//! slide displays and `presiterm lint` reports.
//!
//! This file is the spine: [`parse`] reads the frontmatter, applies `{only}` and cuts the slides.
//! The model is `model`, the blocks of a slide are `blocks`, inline markup is `spans`.

mod blocks;
mod model;
mod spans;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::config::Config;

pub use blocks::extension_is;
pub use model::{Block, Deck, Diagram, Item, Picture, Slide, Span, Theme};
pub use spans::{inline, plain};

/// The comma-separated values of a frontmatter key, trimmed, empty ones left out.
fn list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parse `source`; `name` is the title when the frontmatter has none, `base` resolves the files
/// a slide points at (`.mmd` diagrams), `force_reveal` makes every bullet a key press.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// let deck = presiterm::deck::parse("# One\n\ntext\n\n---\n\n# Two\n", "talk", Path::new("."), false);
/// assert_eq!(deck.slides.len(), 2);
/// assert_eq!(deck.slides[0].title().as_deref(), Some("One"));
/// ```
#[must_use]
pub fn parse(source: &str, name: &str, base: &Path, force_reveal: bool) -> Deck {
    let document = colonmark::parse(source);
    let front = &document.frontmatter;
    let reveal = force_reveal || front.get("reveal") == Some("true");
    let mut deck = Deck {
        title: front.get("title").unwrap_or(name).to_string(),
        author: front.get("author").unwrap_or_default().to_string(),
        theme: if front.get("theme") == Some("light") {
            Theme::Light
        } else {
            Theme::Dark
        },
        tokens: swatchbook::Theme::default(),
        logo: None,
        font_size: front
            .get("font-size")
            .and_then(|v| v.trim_end_matches("pt").trim().parse().ok()),
        reveal,
        slides: Vec::new(),
    };
    let tags = list(front.get("tags"));
    let (content, problem) = match colonmark::only::apply(document.blocks.clone(), &tags) {
        Ok(kept) => (kept, None),
        Err(e) => (document.blocks, Some(format!("only: {e}"))),
    };
    for section in colonmark::sections(content, 2, front.body_line()) {
        let mut slide = Slide {
            heading: section.heading.as_deref().map(spans::line),
            line: section.line,
            ..Slide::default()
        };
        blocks::fill(&mut slide, section.blocks, reveal, base);
        deck.slides.push(slide);
    }
    if let (Some(message), Some(first)) = (problem, deck.slides.first_mut()) {
        first.blocks.insert(0, Block::Error(message));
    }
    deck
}

/// A deck read from its file, with the folder its paths are relative to and the text it came from.
#[derive(Clone, Debug)]
pub struct Loaded {
    /// The parsed deck, dressed in the config's tokens and logo.
    pub deck: Deck,
    /// The deck's folder: pictures and diagram files are relative to it.
    pub base: PathBuf,
    /// The file's text, to find where an edit happened.
    pub source: String,
}

/// Read and parse the deck in `file`, in the look `config` gives it; a deck without slides is an
/// error.
pub fn read(file: &Path, reveal: bool, config: &Config) -> Result<Loaded> {
    let source =
        std::fs::read_to_string(file).with_context(|| format!("read {}", file.display()))?;
    let base = file
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let name = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Slides");
    let mut deck = parse(&source, name, &base, reveal);
    config.dress(&mut deck);
    ensure!(
        !deck.slides.is_empty(),
        "{}: no slides found",
        file.display()
    );
    Ok(Loaded { deck, base, source })
}
