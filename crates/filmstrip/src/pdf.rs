//! The PDF: a Screen annotation over a rectangle of an existing page, with a Rendition action
//! that plays an embedded MP4. The page's open event fires that action, so Acrobat starts the
//! film by itself and loops it; other viewers show the page as it is (put the poster there).

use anyhow::{Context, Result, bail};
use lopdf::{Document, Object, Stream, dictionary};

/// Where the film plays, in PDF points from the lower left corner of the page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Bottom edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// Add the film to page `page_number` (1-based) of `pdf`; `name` is the embedded file's name
/// (`film.mp4`). The PDF/A claim in the metadata, if any, is dropped: multimedia is outside PDF/A.
///
/// # Examples
///
/// ```
/// use filmstrip::Frame;
/// use filmstrip::pdf::Rect;
///
/// let mp4 = filmstrip::video::encode(&[Frame::solid(32, 18, [0; 3])], 10).expect("mp4");
/// let page = filmstrip::pdf::blank_page(200.0, 100.0);
/// let rect = Rect { x: 10.0, y: 10.0, width: 180.0, height: 80.0 };
/// let pdf = filmstrip::pdf::embed(&page, 1, &mp4, rect, "film.mp4").expect("pdf");
/// assert!(pdf.starts_with(b"%PDF"));
/// ```
pub fn embed(pdf: &[u8], page_number: u32, mp4: &[u8], rect: Rect, name: &str) -> Result<Vec<u8>> {
    let mut doc = Document::load_mem(pdf).context("read the pdf")?;
    let Some(&page_id) = doc.get_pages().get(&page_number) else {
        bail!("the pdf has no page {page_number}");
    };
    let (x0, y0, x1, y1) = (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
    let rect_array = vec![
        Object::Real(x0 as f32),
        Object::Real(y0 as f32),
        Object::Real(x1 as f32),
        Object::Real(y1 as f32),
    ];

    // The film as an embedded file behind a file specification.
    let file = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "EmbeddedFile",
            "Subtype" => Object::Name(b"video/mp4".to_vec()),
            "Params" => dictionary! { "Size" => Object::Integer(i64::try_from(mp4.len())?) },
        },
        mp4.to_vec(),
    ));
    let filespec = doc.add_object(dictionary! {
        "Type" => "Filespec",
        "F" => Object::string_literal(name),
        "UF" => Object::string_literal(name),
        "EF" => dictionary! { "F" => file },
    });

    // Rendition: a media clip (the file, its MIME type, allowed to be written to a temp file
    // for playback) with play parameters: auto-play, loop forever, no controller, fit.
    let rendition = doc.add_object(dictionary! {
        "Type" => "Rendition",
        "S" => "MR",
        "N" => Object::string_literal(name),
        "C" => dictionary! {
            "Type" => "MediaClip",
            "S" => "MCD",
            "N" => Object::string_literal(name),
            "CT" => Object::string_literal("video/mp4"),
            "D" => filespec,
            "P" => dictionary! { "TF" => Object::string_literal("TEMPACCESS") },
        },
        "P" => dictionary! {
            "BE" => dictionary! { "A" => true, "RC" => 0, "C" => false, "F" => 3, "V" => 0 },
        },
    });

    // An empty appearance: the page itself draws what is under the annotation.
    let appearance = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(rect.width as f32),
                Object::Real(rect.height as f32),
            ],
        },
        Vec::new(),
    ));

    // Annotation and action reference each other, so reserve the ids first.
    let annot_id = doc.new_object_id();
    let action_id = doc.new_object_id();
    doc.objects.insert(
        action_id,
        Object::Dictionary(dictionary! {
            "Type" => "Action",
            "S" => "Rendition",
            "R" => rendition,
            "OP" => 0,
            "AN" => annot_id,
        }),
    );
    doc.objects.insert(
        annot_id,
        Object::Dictionary(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Screen",
            "Rect" => rect_array,
            "F" => 4,
            "P" => page_id,
            "T" => Object::string_literal(name),
            "Contents" => Object::string_literal(name),
            "A" => action_id,
            "AA" => dictionary! { "PO" => action_id },
            "AP" => dictionary! { "N" => appearance },
        }),
    );

    let page = doc
        .get_object_mut(page_id)
        .and_then(Object::as_dict_mut)
        .context("the page dictionary")?;
    let mut annots: Vec<Object> = match page.get(b"Annots") {
        Ok(Object::Array(a)) => a.clone(),
        _ => Vec::new(),
    };
    annots.push(Object::Reference(annot_id));
    page.set("Annots", annots);
    // Page open: play.
    page.set("AA", dictionary! { "O" => action_id });

    strip_pdfa_claim(&mut doc);

    let mut out = Vec::new();
    doc.save_to(&mut out).context("write the pdf")?;
    Ok(out)
}

/// A one-page PDF of `width` x `height` points with nothing on it, for tests and for callers
/// that draw the poster some other way.
#[must_use]
pub fn blank_page(width: f64, height: f64) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let tree_id = doc.new_object_id();
    let content = doc.add_object(Stream::new(dictionary! {}, Vec::new()));
    let leaf_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => tree_id,
        "MediaBox" => vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(width as f32),
            Object::Real(height as f32),
        ],
        "Contents" => content,
    });
    doc.objects.insert(
        tree_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(leaf_id)],
            "Count" => 1,
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => tree_id });
    doc.trailer.set("Root", catalog);
    let mut out = Vec::new();
    // A fresh document with one page always serialises.
    let _ = doc.save_to(&mut out);
    out
}

fn strip_pdfa_claim(doc: &mut Document) {
    let Ok(catalog) = doc.catalog() else { return };
    let Ok(meta_id) = catalog.get(b"Metadata").and_then(Object::as_reference) else {
        return;
    };
    if let Ok(stream) = doc.get_object_mut(meta_id).and_then(Object::as_stream_mut)
        && let Ok(xml) = std::str::from_utf8(&stream.content)
    {
        let cleaned: String = xml
            .lines()
            .filter(|l| !l.contains("pdfaid:"))
            .collect::<Vec<_>>()
            .join("\n");
        stream.set_plain_content(cleaned.into_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Frame;

    #[test]
    fn the_page_gets_a_screen_annotation_that_plays_on_open() {
        let mp4 = crate::video::encode(&[Frame::solid(32, 18, [0; 3])], 10).unwrap();
        let page = blank_page(200.0, 100.0);
        let rect = Rect {
            x: 10.0,
            y: 10.0,
            width: 180.0,
            height: 80.0,
        };
        let pdf = embed(&page, 1, &mp4, rect, "film.mp4").unwrap();
        let doc = Document::load_mem(&pdf).unwrap();
        let page_id = doc.get_pages()[&1];
        let page = doc.get_object(page_id).unwrap().as_dict().unwrap();
        let annots = page.get(b"Annots").unwrap().as_array().unwrap();
        assert_eq!(annots.len(), 1);
        let annot = doc.get_object(annots[0].as_reference().unwrap()).unwrap();
        let annot = annot.as_dict().unwrap();
        assert_eq!(annot.get(b"Subtype").unwrap().as_name().unwrap(), b"Screen");
        let action_id = annot.get(b"A").unwrap().as_reference().unwrap();
        let open = page.get(b"AA").unwrap().as_dict().unwrap();
        assert_eq!(open.get(b"O").unwrap().as_reference().unwrap(), action_id);
        let action = doc.get_object(action_id).unwrap().as_dict().unwrap();
        assert_eq!(action.get(b"S").unwrap().as_name().unwrap(), b"Rendition");
        // the film travels inside the file
        let rendition = doc
            .get_object(action.get(b"R").unwrap().as_reference().unwrap())
            .unwrap();
        let clip = rendition
            .as_dict()
            .unwrap()
            .get(b"C")
            .unwrap()
            .as_dict()
            .unwrap();
        let spec = doc
            .get_object(clip.get(b"D").unwrap().as_reference().unwrap())
            .unwrap();
        let ef = spec
            .as_dict()
            .unwrap()
            .get(b"EF")
            .unwrap()
            .as_dict()
            .unwrap();
        let file = doc
            .get_object(ef.get(b"F").unwrap().as_reference().unwrap())
            .unwrap();
        assert_eq!(file.as_stream().unwrap().content, mp4);
    }

    #[test]
    fn a_missing_page_is_an_error() {
        let mp4 = crate::video::encode(&[Frame::solid(32, 18, [0; 3])], 10).unwrap();
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        };
        assert!(embed(&blank_page(10.0, 10.0), 2, &mp4, rect, "f.mp4").is_err());
    }
}
