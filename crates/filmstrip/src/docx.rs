//! The Word file: A4 landscape, the GIF as the only picture, a line of links under it.
//! docx-rs writes the picture as a PNG part; a second pass over the zip swaps the bytes for the
//! animated GIF (every frame inside the file) and fixes the relationship and the content types.
//! Word for Mac and current Word for Windows play it in place.

use std::io::{Cursor, Read, Write};

use anyhow::{Context, Result, bail};
use docx_rs::{
    AlignmentType, Docx, Hyperlink, HyperlinkType, LineSpacing, PageMargin, PageOrientationType,
    Paragraph, Pic, Run, RunFonts,
};
use zip::write::SimpleFileOptions;

use crate::Frame;

const TWIP_PER_CM: f64 = 566.93;
const EMU_PER_TWIP: u32 = 635;
const A4_W: f64 = 29.7; // landscape: width x height in cm
const A4_H: f64 = 21.0;
const MARGIN_CM: f64 = 1.2;
/// The GIF's part name inside the package.
const GIF_PART: &str = "word/media/film.gif";

/// A clickable entry of the line under the picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// What the reader sees.
    pub label: String,
    /// Where it goes (`https://...`, `mailto:...`).
    pub url: String,
}

/// How the link line looks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Style {
    /// Font family of the link line.
    pub font: String,
    /// Link colour as six hex digits (`5B4FB3`).
    pub colour: String,
    /// Separator colour as six hex digits.
    pub separator: String,
    /// Text size in half-points (22 = 11 pt).
    pub size: usize,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            font: "Arial".into(),
            colour: "1D6FB8".into(),
            separator: "9CA3AF".into(),
            size: 22,
        }
    }
}

/// Build the document: `poster` sizes and previews the picture, `gif` is what plays, `links`
/// go in one line under it.
///
/// # Examples
///
/// ```
/// use filmstrip::Frame;
/// use filmstrip::docx::{Link, Style};
///
/// let poster = Frame::solid(32, 18, [0; 3]);
/// let gif = filmstrip::gif::encode(&[poster.clone()], &[], 10, 4).expect("gif");
/// let links = [Link { label: "example.org".into(), url: "https://example.org".into() }];
/// let docx = filmstrip::docx::wrap(&poster, &gif, &links, &Style::default()).expect("docx");
/// assert_eq!(&docx[..2], b"PK");
/// ```
pub fn wrap(poster: &Frame, gif: &[u8], links: &[Link], style: &Style) -> Result<Vec<u8>> {
    if !gif.starts_with(b"GIF8") {
        bail!("not a GIF");
    }
    let poster_png = poster.png()?;

    let page_w = (A4_W * TWIP_PER_CM).round() as u32;
    let page_h = (A4_H * TWIP_PER_CM).round() as u32;
    let margin = (MARGIN_CM * TWIP_PER_CM).round() as u32;
    let text_w = page_w - 2 * margin;
    let text_h = page_h - 2 * margin;

    // Fit the picture into the text area, keep the ratio.
    let ratio = f64::from(poster.height) / f64::from(poster.width);
    let mut pic_w = text_w;
    let mut pic_h = (f64::from(pic_w) * ratio).round() as u32;
    if pic_h > text_h {
        pic_h = text_h;
        pic_w = (f64::from(pic_h) / ratio).round() as u32;
    }
    let pic = Pic::new_with_dimensions(poster_png.clone(), poster.width, poster.height)
        .size(pic_w * EMU_PER_TWIP, pic_h * EMU_PER_TWIP);

    // Vertical centring: top space as "before" spacing; the link line below takes ~ 1.1 cm.
    let link_line = if links.is_empty() {
        0
    } else {
        (1.1 * TWIP_PER_CM) as u32
    };
    let lead = text_h.saturating_sub(pic_h + link_line) / 2;
    let margin = i32::try_from(margin)?;

    let fonts = RunFonts::new()
        .ascii(&style.font)
        .hi_ansi(&style.font)
        .cs(&style.font)
        .east_asia(&style.font);
    let mut docx = Docx::new()
        .default_fonts(fonts)
        .page_size(page_w, page_h)
        .page_orient(PageOrientationType::Landscape)
        .page_margin(
            PageMargin::new()
                .top(margin)
                .bottom(margin)
                .left(margin)
                .right(margin)
                .header(0)
                .footer(0)
                .gutter(0),
        )
        .add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .line_spacing(LineSpacing::new().before(lead))
                .add_run(Run::new().add_image(pic)),
        );

    if !links.is_empty() {
        let mut p = Paragraph::new()
            .align(AlignmentType::Center)
            .line_spacing(LineSpacing::new().before(240));
        for (i, link) in links.iter().enumerate() {
            if i > 0 {
                p = p.add_run(
                    Run::new()
                        .add_text("    |    ")
                        .size(style.size)
                        .color(&style.separator),
                );
            }
            let h = Hyperlink::new(&link.url, HyperlinkType::External).add_run(
                Run::new()
                    .add_text(&link.label)
                    .size(style.size)
                    .color(&style.colour)
                    .underline("single"),
            );
            p = p.add_hyperlink(h);
        }
        docx = docx.add_paragraph(p);
    }

    let mut buf = Vec::new();
    docx.build()
        .pack(Cursor::new(&mut buf))
        .context("pack the docx")?;
    embed_gif(&buf, &poster_png, gif)
}

/// Replace the poster PNG part with the GIF and fix the relationship target and content types.
fn embed_gif(docx: &[u8], poster_png: &[u8], gif: &[u8]) -> Result<Vec<u8>> {
    let mut zin = zip::ZipArchive::new(Cursor::new(docx))?;
    let mut parts: Vec<(String, Vec<u8>)> = Vec::new();
    let mut poster_name: Option<String> = None;
    for i in 0..zin.len() {
        let mut f = zin.by_index(i)?;
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        if f.name().starts_with("word/media/") && data == poster_png {
            poster_name = Some(f.name().to_owned());
        }
        parts.push((f.name().to_owned(), data));
    }
    let Some(poster_name) = poster_name else {
        bail!("the poster image part is not in the packed document");
    };
    let old_rel = poster_name.trim_start_matches("word/").to_owned();
    let new_rel = GIF_PART.trim_start_matches("word/").to_owned();

    let mut out = Vec::new();
    {
        let mut zout = zip::ZipWriter::new(Cursor::new(&mut out));
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in parts {
            let (name, data) = if name == poster_name {
                (GIF_PART.to_owned(), gif.to_vec())
            } else if name == "[Content_Types].xml" {
                let xml = String::from_utf8(data)?;
                let xml = if xml.contains("Extension=\"gif\"") {
                    xml
                } else {
                    xml.replacen(
                        "<Default ",
                        "<Default Extension=\"gif\" ContentType=\"image/gif\"/><Default ",
                        1,
                    )
                };
                (name, xml.into_bytes())
            } else if name == "word/_rels/document.xml.rels" {
                let xml = String::from_utf8(data)?;
                if !xml.contains(&old_rel) {
                    bail!("the relationship to {old_rel} is missing");
                }
                (name, xml.replace(&old_rel, &new_rel).into_bytes())
            } else {
                (name, data)
            };
            zout.start_file(name, opts)?;
            zout.write_all(&data)?;
        }
        zout.finish()?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(docx: &[u8], name: &str) -> Option<Vec<u8>> {
        let mut zin = zip::ZipArchive::new(Cursor::new(docx)).unwrap();
        let mut f = zin.by_name(name).ok()?;
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        Some(data)
    }

    #[test]
    fn the_gif_is_the_picture_and_the_links_are_hyperlinks() {
        let poster = Frame::solid(32, 18, [20, 30, 40]);
        let gif = crate::gif::encode(std::slice::from_ref(&poster), &[], 10, 4).unwrap();
        let links = [
            Link {
                label: "a.example".into(),
                url: "https://a.example".into(),
            },
            Link {
                label: "mail".into(),
                url: "mailto:x@a.example".into(),
            },
        ];
        let docx = wrap(&poster, &gif, &links, &Style::default()).unwrap();
        assert_eq!(part(&docx, GIF_PART).unwrap(), gif);
        assert!(part(&docx, "word/media/image1.png").is_none());
        let rels = String::from_utf8(part(&docx, "word/_rels/document.xml.rels").unwrap()).unwrap();
        assert!(rels.contains("media/film.gif"));
        assert!(rels.contains("https://a.example"));
        assert!(rels.contains("mailto:x@a.example"));
        let types = String::from_utf8(part(&docx, "[Content_Types].xml").unwrap()).unwrap();
        assert!(types.contains("Extension=\"gif\""));
        let body = String::from_utf8(part(&docx, "word/document.xml").unwrap()).unwrap();
        assert!(body.contains("a.example"));
        assert!(body.contains("w:orient=\"landscape\""));
    }

    #[test]
    fn a_non_gif_is_refused() {
        let poster = Frame::solid(2, 2, [0; 3]);
        assert!(wrap(&poster, b"not a gif", &[], &Style::default()).is_err());
    }
}
