//! How a diagram or a picture takes its rows and columns on the slide, and the logo in the corner.

use std::sync::Arc;

use super::text::{Line, wrap};
use super::{Geometry, Page, Palette, Placement, Slot, View, text_style};
use crate::canvas::Style;
use crate::deck::{Diagram, Picture};
use crate::diagram::Assets;

pub(super) struct Media {
    /// Free rows of the share it was given: the picture sits in the middle of them.
    pub(super) slack: usize,
    pub(super) rows: usize,
    pub(super) cols: usize,
    pub(super) frames: Vec<Arc<Vec<u8>>>,
    pub(super) base: usize,
}

/// The configured logo, one row high, in the top left corner (row 0 holds no text). A logo that
/// cannot be drawn is a problem of the page, like any other picture.
pub(super) fn logo(page: &mut Page, view: View, geo: Geometry, assets: &mut Assets) {
    let px = view.cell.1.round() as u32;
    match assets.logo(px) {
        None => {},
        Some(Err(e)) => page.problems.push(format!("logo: {e:#}")),
        Some(Ok(loaded)) => {
            let cols = ((loaded.aspect * view.cell.1) / view.cell.0).ceil() as usize;
            page.images.push(Placement {
                row: 0,
                col: geo.left,
                rows: 1,
                cols: cols.min(geo.width),
                bytes: loaded.bytes,
            });
        },
    }
}

/// A picture in the middle of the rows it was given.
pub(super) fn push_media(slots: &mut Vec<Slot>, media: Media) {
    let top = media.slack / 2;
    let bottom = media.slack - top;
    slots.extend((0..top).map(|_| Slot::Line(Line::new(Vec::new()))));
    slots.push(Slot::Media(media));
    slots.extend((0..bottom).map(|_| Slot::Line(Line::new(Vec::new()))));
}

pub(super) fn error_slots(
    slots: &mut Vec<Slot>,
    page: &mut Page,
    message: &str,
    pal: &Palette,
    geo: Geometry,
) {
    page.problems.push(message.to_string());
    let style = text_style(pal).fg(pal.stop).bold();
    let cells: Vec<(char, Style)> = format!("⚠ {message}").chars().map(|c| (c, style)).collect();
    slots.extend(
        wrap(&cells, geo.width)
            .into_iter()
            .map(|l| Slot::Line(Line::new(l))),
    );
}

pub(super) fn fit(
    aspect: f64,
    wanted: usize,
    max_cols: Option<usize>,
    view: View,
    geo: Geometry,
) -> (usize, usize) {
    let (cw, ch) = view.cell;
    let width = max_cols.map_or(geo.width, |w| w.min(geo.width));
    let by_width = ((width as f64 * cw) / (aspect * ch)).floor() as usize;
    let rows = wanted.min(by_width).max(1);
    let cols = ((rows as f64 * ch * aspect) / cw).ceil() as usize;
    (rows, cols.clamp(1, width))
}

pub(super) fn media_diagram(
    d: &Diagram,
    base: usize,
    auto_rows: usize,
    view: View,
    geo: Geometry,
    assets: &mut Assets,
) -> anyhow::Result<Media> {
    let aspect = assets.aspect(d)?;
    // A Typst diagram is never drawn bigger than its text is at the slide's own size: `h=` and `w=` only
    // shrink it, so a small diagram does not grow to a size where its text outweighs the slide's.
    let mut wanted = d.height.unwrap_or(auto_rows);
    if d.typst {
        wanted = wanted.min(assets.natural_rows(d)?);
    }
    let (rows, cols) = fit(aspect, wanted, d.width, view, geo);
    let px = (rows as f64 * view.cell.1).round() as u32;
    let steps = Assets::steps(d);
    let frames = (0..=steps)
        .map(|k| assets.diagram_png(d, k, px))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Media {
        slack: if d.height.is_none() {
            auto_rows.saturating_sub(rows)
        } else {
            0
        },
        rows,
        cols,
        frames,
        base,
    })
}

pub(super) fn media_picture(
    p: &Picture,
    auto_rows: usize,
    view: View,
    geo: Geometry,
    assets: &mut Assets,
) -> anyhow::Result<Media> {
    let first = assets.picture(p, None)?;
    let (rows, cols) = fit(
        first.aspect,
        p.height.unwrap_or(auto_rows),
        p.width,
        view,
        geo,
    );
    let loaded = if p.path.to_lowercase().ends_with(".svg") {
        assets.picture(p, Some((rows as f64 * view.cell.1).round() as u32))?
    } else {
        first
    };
    Ok(Media {
        slack: if p.height.is_none() {
            auto_rows.saturating_sub(rows)
        } else {
            0
        },
        rows,
        cols,
        frames: vec![loaded.bytes],
        base: 0,
    })
}
