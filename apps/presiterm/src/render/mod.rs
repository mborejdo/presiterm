//! Slide layout: a [`Slide`] at a step and a grid size becomes a [`Canvas`] of styled cells plus
//! the image rectangles. Every size is derived from the grid (`cols` x `rows`) and the cell size in
//! pixels, so any font size reflows.
//!
//! This file is the spine: the types a caller sees, [`page`] and the pass that stacks a slide's
//! pieces. How text becomes lines is `text`, how a picture takes its space is `media`, the
//! heading, notes and footer are `chrome`.

mod chrome;
mod media;
#[cfg(test)]
mod tests;
mod text;

use std::sync::Arc;

use swatchbook::Palette;

use crate::canvas::{Canvas, Style};
use crate::deck::{Block, Deck, Diagram, Picture, Slide};
use crate::diagram::Assets;

use chrome::{footer, heading, notes};
use media::{Media, error_slots, logo, media_diagram, media_picture, push_media};
use text::{Line, block_lines, paint};

/// The grid presiterm lays out on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Columns.
    pub cols: usize,
    /// Rows.
    pub rows: usize,
    /// One cell in pixels (width, height); sizes images.
    pub cell: (f64, f64),
}

/// The plain text style of a palette: ink on the ground.
const fn text_style(pal: &Palette) -> Style {
    Style::new(pal.ink, pal.ground)
}

/// An image placed on the screen.
#[derive(Clone, Debug)]
pub struct Placement {
    /// Top row.
    pub row: usize,
    /// Left column.
    pub col: usize,
    /// Height in cells (what the terminal is told).
    pub rows: usize,
    /// Width in cells (what it works out).
    pub cols: usize,
    /// The encoded picture.
    pub bytes: Arc<Vec<u8>>,
}

/// One laid-out screen.
#[derive(Debug)]
pub struct Page {
    /// The text layer.
    pub canvas: Canvas,
    /// The image layer.
    pub images: Vec<Placement>,
    /// Key presses this slide takes to reveal everything.
    pub fragments: usize,
    /// Body rows the slide wanted beyond what fits (0 when it fits).
    pub overflow: usize,
    /// Rows of body the slide uses.
    pub body_rows: usize,
    /// Anything wrong: missing image, broken diagram, clipped code.
    pub problems: Vec<String>,
}

/// What to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The slide.
    Slide,
    /// The speaker notes of the slide.
    Notes,
}

enum Piece<'a> {
    Lines(Vec<Line>),
    Diagram { diagram: &'a Diagram, base: usize },
    Picture(&'a Picture),
}

enum Slot {
    Line(Line),
    Media(Media),
}

/// Geometry derived from the grid.
#[derive(Clone, Copy, Debug)]
struct Geometry {
    left: usize,
    width: usize,
    body_top: usize,
    body_bottom: usize,
}

fn geometry(view: View) -> Geometry {
    let margin = if view.cols >= 30 { 2 } else { 0 };
    let width = (view.cols.saturating_sub(2 * margin)).clamp(1, 80);
    Geometry {
        left: (view.cols - width) / 2,
        width,
        body_top: 5.min(view.rows.saturating_sub(3)),
        body_bottom: view.rows.saturating_sub(2),
    }
}

/// Return the columns the slide text may use at this grid.
#[must_use]
pub fn content_width(view: View) -> usize {
    geometry(view).width
}

/// Return the body rows `presiterm lint` allows before it calls a slide crowded: air below the text.
#[must_use]
pub const fn comfortable_rows(view: View) -> usize {
    view.rows.saturating_sub(10)
}

/// Count the key presses that reveal everything on `slide`.
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

/// Lay out slide `index` of `deck` at `step`.
pub fn page(
    deck: &Deck,
    index: usize,
    step: usize,
    view: View,
    assets: &mut Assets,
    mode: Mode,
    status: &str,
) -> Page {
    let pal = *deck.palette();
    let geo = geometry(view);
    let mut canvas = Canvas::new(view.cols, view.rows, text_style(&pal));
    let slide = &deck.slides[index];
    let mut page = Page {
        canvas: Canvas::new(0, 0, text_style(&pal)),
        images: Vec::new(),
        fragments: fragments(slide),
        overflow: 0,
        body_rows: 0,
        problems: Vec::new(),
    };

    if mode == Mode::Notes {
        notes(&mut canvas, slide, &pal, geo);
    } else {
        slide_body(
            &mut canvas,
            &mut page,
            deck,
            index,
            step,
            view,
            assets,
            &pal,
            geo,
        );
    }
    footer(
        &mut canvas,
        deck,
        index,
        view,
        &pal,
        geo,
        status,
        page.overflow,
    );
    logo(&mut page, view, geo, assets);
    page.canvas = canvas;
    page
}

#[allow(
    clippy::too_many_arguments,
    reason = "one layout pass needs all of its context"
)]
#[allow(
    clippy::too_many_lines,
    reason = "one layout pass: pieces, sizes, slots, painting"
)]
fn slide_body(
    canvas: &mut Canvas,
    page: &mut Page,
    deck: &Deck,
    index: usize,
    step: usize,
    view: View,
    assets: &mut Assets,
    pal: &Palette,
    geo: Geometry,
) {
    let slide = &deck.slides[index];
    let title = slide.is_title(index == 0);
    let avail = geo.body_bottom.saturating_sub(geo.body_top);

    // Pieces, with the reveal counter running across the slide.
    let mut counter = 0usize;
    let mut pieces: Vec<Piece<'_>> = Vec::new();
    for block in &slide.blocks {
        pieces.push(match block {
            Block::Diagram(d) => {
                let base = counter;
                counter += Assets::steps(d);
                Piece::Diagram { diagram: d, base }
            },
            Block::Picture(p) => Piece::Picture(p),
            other => Piece::Lines(block_lines(
                other,
                pal,
                geo,
                title,
                &mut counter,
                &mut page.problems,
            )),
        });
    }

    // Media sizes: fixed heights first, the rest shared.
    let text_rows: usize = pieces
        .iter()
        .map(|p| if let Piece::Lines(l) = p { l.len() } else { 0 })
        .sum();
    let gaps = pieces.len().saturating_sub(1);
    let fixed_media: usize = pieces
        .iter()
        .map(|p| match p {
            Piece::Diagram { diagram, .. } => diagram.height.unwrap_or(0),
            Piece::Picture(pic) => pic.height.unwrap_or(0),
            Piece::Lines(_) => 0,
        })
        .sum();
    let auto_count = pieces
        .iter()
        .filter(|p| match p {
            Piece::Diagram { diagram, .. } => diagram.height.is_none(),
            Piece::Picture(pic) => pic.height.is_none(),
            Piece::Lines(_) => false,
        })
        .count();
    let auto_rows = avail
        .saturating_sub(text_rows + gaps + fixed_media)
        .checked_div(auto_count)
        .map_or(0, |share| share.max(3));

    let mut slots: Vec<Slot> = Vec::new();
    for (n, piece) in pieces.into_iter().enumerate() {
        if n > 0 {
            slots.push(Slot::Line(Line::new(Vec::new())));
        }
        match piece {
            Piece::Lines(lines) => slots.extend(lines.into_iter().map(Slot::Line)),
            Piece::Diagram { diagram, base } => {
                match media_diagram(diagram, base, auto_rows, view, geo, assets) {
                    Ok(media) => push_media(&mut slots, media),
                    Err(e) => error_slots(&mut slots, page, &format!("{e:#}"), pal, geo),
                }
            },
            Piece::Picture(picture) => match media_picture(picture, auto_rows, view, geo, assets) {
                Ok(media) => push_media(&mut slots, media),
                Err(e) => error_slots(&mut slots, page, &format!("{e:#}"), pal, geo),
            },
        }
    }

    let total: usize = slots
        .iter()
        .map(|s| match s {
            Slot::Line(_) => 1,
            Slot::Media(m) => m.rows,
        })
        .sum();
    let heading_rows = if slide.heading.is_some() { 3 } else { 0 };
    page.body_rows = total;
    page.overflow = total.saturating_sub(avail);

    let mut row;
    if title {
        let block = heading_rows + total;
        row = (view.rows.saturating_sub(2).saturating_sub(block)) / 2;
        if let Some(h) = &slide.heading {
            heading(canvas, h, row, pal, geo);
        }
        row += heading_rows;
    } else {
        if let Some(h) = &slide.heading {
            heading(canvas, h, 2, pal, geo);
        }
        row = geo.body_top;
    }
    for slot in &slots {
        match slot {
            Slot::Line(line) => {
                if row < geo.body_bottom && line.frag.is_none_or(|f| f <= step) {
                    paint(canvas, line, row, geo);
                }
                row += 1;
            },
            Slot::Media(media) => {
                let lit = step.saturating_sub(media.base).min(media.frames.len() - 1);
                if row + media.rows <= geo.body_bottom {
                    page.images.push(Placement {
                        row,
                        col: geo.left + (geo.width - media.cols) / 2,
                        rows: media.rows,
                        cols: media.cols,
                        bytes: media.frames[lit].clone(),
                    });
                }
                row += media.rows;
            },
        }
    }
}
