//! SVG to PNG with the theme's own face, and the pixel size of an encoded picture.

use anyhow::{Context, Result, ensure};
use resvg::{tiny_skia, usvg};

/// The largest edge presiterm draws, in pixels.
pub(super) const MAX_EDGE: f32 = 4096.0;

/// The SVG options: the vendored faces, `sans` as the sans-serif family. A family that is not
/// vendored is looked up among the system's fonts.
pub(super) fn svg_options(sans: &str) -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    let fonts = options.fontdb_mut();
    for face in swatchbook::fonts::FACES {
        fonts.load_font_data(face.bytes.to_vec());
    }
    if swatchbook::fonts::family(sans).next().is_none() {
        fonts.load_system_fonts();
    }
    fonts.set_sans_serif_family(sans);
    options
}

fn tree(svg: &str, sans: &str) -> Result<usvg::Tree> {
    let tree = usvg::Tree::from_str(svg, &svg_options(sans)).context("not an SVG")?;
    let size = tree.size();
    ensure!(
        size.width() > 0.0 && size.height() > 0.0,
        "the SVG has no size"
    );
    Ok(tree)
}

pub(super) fn svg_size(svg: &str, sans: &str) -> Result<(f32, f32)> {
    let size = tree(svg, sans)?.size();
    Ok((size.width(), size.height()))
}

pub(super) fn svg_to_png(svg: &str, sans: &str, px_height: u32) -> Result<Vec<u8>> {
    let tree = tree(svg, sans)?;
    let (w, h) = (tree.size().width(), tree.size().height());
    let scale = (px_height as f32 / h).min(MAX_EDGE / w).min(MAX_EDGE / h);
    let (pw, ph) = (
        ((w * scale).round() as u32).max(1),
        ((h * scale).round() as u32).max(1),
    );
    let mut pixmap = tiny_skia::Pixmap::new(pw, ph).context("allocate the bitmap")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().context("encode the PNG")
}

/// Read the pixel size of a PNG, GIF or JPEG from its header.
#[must_use]
pub fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.starts_with(b"GIF8") && bytes.len() >= 10 {
        let w = u16::from_le_bytes([bytes[6], bytes[7]]);
        let h = u16::from_le_bytes([bytes[8], bytes[9]]);
        return Some((u32::from(w), u32::from(h)));
    }
    if bytes.starts_with(&[0xff, 0xd8]) {
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xff {
                return None;
            }
            let marker = bytes[i + 1];
            if (0xc0..=0xcf).contains(&marker) && !matches!(marker, 0xc4 | 0xc8 | 0xcc) {
                let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]);
                let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]);
                return Some((u32::from(w), u32::from(h)));
            }
            i += 2 + usize::from(u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]));
        }
        return None;
    }
    if bytes.len() >= 24 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
        let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
        return Some((w, h));
    }
    None
}
