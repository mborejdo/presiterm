//! The bytes that put a page on the terminal, sending only what changed since the last frame.

use std::fmt::Write as _;

use crate::render::{Page, Placement};
use crate::term;

/// What is on the terminal right now, to send only what changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    images: Vec<(usize, usize, usize, usize, u64)>,
}

pub(super) fn key_of(image: &Placement) -> (usize, usize, usize, usize, u64) {
    let hash = image.bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    (image.row, image.col, image.rows, image.cols, hash)
}

/// Build the bytes that put `page` on the terminal.
///
/// `full` clears the screen and draws everything (slide change, resize, reload). Otherwise every
/// text row is written again - padded to the full width, so nothing stale survives - except the rows
/// of an image that is already there, and only new or changed images are sent.
#[must_use]
pub fn frame_bytes(page: &Page, previous: &Drawn, full: bool) -> (String, Drawn) {
    let now = Drawn {
        images: page.images.iter().map(key_of).collect(),
    };
    let kept = |image: &(usize, usize, usize, usize, u64)| !full && previous.images.contains(image);
    let mut out = String::new();
    if full {
        out.push_str(term::CLEAR);
    }
    for (row, line) in page.canvas.ansi_rows().iter().enumerate() {
        let under_kept_image = now
            .images
            .iter()
            .any(|i| kept(i) && row >= i.0 && row < i.0 + i.2);
        if !under_kept_image {
            let _ = write!(out, "\x1b[{};1H{line}", row + 1);
        }
    }
    for (image, key) in page.images.iter().zip(&now.images) {
        if !kept(key) {
            let _ = write!(
                out,
                "\x1b[{};{}H{}",
                image.row + 1,
                image.col + 1,
                term::inline_image(&image.bytes, image.rows)
            );
        }
    }
    (out, now)
}
