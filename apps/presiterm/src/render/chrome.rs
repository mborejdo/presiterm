//! The parts of a page that are not the slide's body: heading, speaker notes and footer.

use super::text::{clip, put_cells, spans_to_cells, wrap};
use super::{Geometry, Palette, View, text_style};
use crate::canvas::{Canvas, Style, width};
use crate::deck::{Deck, Slide, Span};

pub(super) fn heading(
    canvas: &mut Canvas,
    text: &[Span],
    row: usize,
    pal: &Palette,
    geo: Geometry,
) {
    let base = text_style(pal).fg(pal.accent).bold();
    let mut cells = spans_to_cells(text, pal, base);
    let cell_w = |c: char| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
    if cells.iter().map(|(c, _)| cell_w(*c)).sum::<usize>() > geo.width {
        let mut used = 0;
        cells.retain(|(c, _)| {
            used += cell_w(*c);
            used < geo.width
        });
        cells.push(('…', base));
    }
    let w: usize = cells.iter().map(|(c, _)| cell_w(*c)).sum();
    let col = geo.left + (geo.width - w) / 2;
    put_cells(canvas, col, row, &cells);
    canvas.text(
        col,
        row + 1,
        &"─".repeat(w),
        text_style(pal).fg(pal.accent).dim(),
    );
}

pub(super) fn notes(canvas: &mut Canvas, slide: &Slide, pal: &Palette, geo: Geometry) {
    let title = format!("Notes: {}", slide.title().as_deref().unwrap_or("-"));
    heading(canvas, &[Span::from(title.as_str())], 2, pal, geo);
    let base = text_style(pal);
    let text = if slide.notes.is_empty() {
        "(no notes on this slide)".to_string()
    } else {
        slide.notes.clone()
    };
    let style = if slide.notes.is_empty() {
        base.fg(pal.muted).italic()
    } else {
        base
    };
    let mut row = geo.body_top;
    for source in text.lines() {
        let cells: Vec<(char, Style)> = source.chars().map(|c| (c, style)).collect();
        for line in wrap(&cells, geo.width) {
            if row >= geo.body_bottom {
                return;
            }
            put_cells(canvas, geo.left, row, &line);
            row += 1;
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the footer shows title, counter, status and overflow of one slide"
)]
pub(super) fn footer(
    canvas: &mut Canvas,
    deck: &Deck,
    index: usize,
    view: View,
    pal: &Palette,
    geo: Geometry,
    status: &str,
    overflow: usize,
) {
    let row = view.rows - 1;
    let muted = text_style(pal).fg(pal.muted);
    let counter = format!("[{}/{}]", index + 1, deck.slides.len());
    let right = (geo.left + geo.width).saturating_sub(width(&counter));
    canvas.text(right, row, &counter, text_style(pal).fg(pal.accent).bold());
    // The counter always shows; a note takes up to half of the rest; the title gets what is left.
    let room = right.saturating_sub(geo.left + 1);
    let note = if overflow > 0 && status.is_empty() {
        format!("{overflow} rows too many")
    } else {
        status.to_string()
    };
    let note_room = if note.is_empty() {
        0
    } else {
        width(&note).min(room / 2)
    };
    let title_room = if note_room == 0 {
        room
    } else {
        room.saturating_sub(note_room + 2)
    };
    let title = clip(&deck.title, title_room.max(1));
    canvas.text(geo.left, row, &title, muted);
    if note_room > 0 && title_room > 0 {
        let x = geo.left + width(&title) + 2;
        let style = if overflow > 0 {
            text_style(pal).fg(pal.warn)
        } else {
            text_style(pal).fg(pal.accent)
        };
        canvas.text(x, row, &clip(&note, note_room), style);
    }
}
