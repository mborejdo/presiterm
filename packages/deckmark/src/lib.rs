//! deckmark: the presiterm deck dialect.
//!
//! [`deck`] parses a deck file into slides of blocks, [`diagram`] draws its mermaid and Typst
//! diagrams and pictures in the deck's tokens, [`faces`] is the Typst engine with the deck's font
//! files. presiterm presents a deck; a deck rendered as a film reads the same model.

// Pixel arithmetic converts between u32, f32 and f64; a picture is a few thousand pixels, far
// inside every range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    reason = "pixel math on a few thousand pixels, far inside every range"
)]

use std::path::{Path, PathBuf};

pub mod deck;
pub mod diagram;
#[cfg(feature = "typst")]
pub mod faces;

use deck::{Block, Slide};
use diagram::Assets;

/// Count the key presses that reveal everything on `slide`: its `+` bullets and the lit nodes
/// of its stepped diagrams.
#[must_use]
pub fn fragments(slide: &Slide) -> usize {
    slide
        .blocks
        .iter()
        .map(|b| match b {
            Block::List(items) => items.iter().filter(|i| i.reveal).count(),
            Block::Diagram(d) => Assets::steps(d),
            _ => 0,
        })
        .sum()
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
