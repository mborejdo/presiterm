//! presiterm: Markdown in, a WezTerm presentation out.
//!
//! A thin Markdown -> WezTerm renderer and a tiny pager: [`deck`] parses the dialect, [`render`]
//! lays a slide out as styled cells ([`canvas`]), [`pager`] shows it with the keys, reload and
//! control socket, [`diagram`] draws mermaid, Typst and pictures, [`window`] is the WezTerm glue,
//! [`lint`] checks a deck, [`handbook`] is the manual.

// Cell and pixel arithmetic converts between usize, u32, f32 and f64; a slide is a few hundred
// cells, so no value gets near a truncating or precision-losing range. Allowed for this crate
// only: the library crates do no layout math.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    reason = "layout math on a few hundred cells and pixels, far inside every range"
)]

pub mod canvas;
pub mod config;
pub mod deck;
pub mod diagram;
pub mod handbook;
pub mod highlight;
pub mod keys;
pub mod lint;
pub mod new_deck;
pub mod pager;
pub mod render;
pub mod term;
pub mod window;
