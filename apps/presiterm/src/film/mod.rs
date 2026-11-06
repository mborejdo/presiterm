//! `presiterm film`: the deck as a film. Every slide is a 16:9 card drawn by Typst in the deck's
//! tokens; every key press of the talk (a `+` bullet, a lit `steps` node) is a build-in step of
//! its own, with `enter` seconds of animation (bullets appear, marked text underlines itself,
//! metrics count up, a title lands word by word) and `hold` seconds of rest. The frames go to
//! `filmstrip`: a GIF, an MP4, a Word file that plays the GIF, a PDF that plays the MP4.
//!
//! This file is the spine: [`Options`], [`render`] and the containers. The card layout is
//! `film.typ`, the PDF page `page.typ`, both embedded.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use filmstrip::Frame;
use serde_json::{Value, json};
use typstkit::{Engine, Input, Main};

use crate::config::{Config, FilmSettings};
use crate::deck::{Block, Deck, Loaded, Slide};
use crate::diagram::Assets;
use crate::render::fragments;

/// The card layout.
const CARD: &str = include_str!("film.typ");
/// The PDF page under the film.
const PAGE: &str = include_str!("page.typ");
/// The PDF page (A4 landscape) and where the poster sits on it, in points; page.typ reads them
/// as inputs so the Screen annotation lands exactly on the poster.
const PDF_PAGE: (f64, f64) = (841.89, 595.28);
const PDF_MARGIN: f64 = 34.0;
const PDF_TOP: f64 = 58.0;
/// Pixels per logical point: the card is drawn at twice its size for crisp text.
const SCALE: f32 = 2.0;
/// Frames each build-in frame stays: 2 at 10 fps is the stop-motion look.
const TICKS: usize = 2;

/// How the film is timed and sized.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Frames per second.
    pub fps: u16,
    /// Width in pixels; the height follows 16:9. Even numbers only (the MP4 needs them).
    pub width: u32,
    /// Seconds every card (every step of it) stays, build-in included.
    pub hold: f32,
    /// Seconds of build-in at the start of every step.
    pub enter: f32,
    /// Seconds of crossfade between cards.
    pub fade: f32,
    /// Colours of the GIF palette (2 to 255).
    pub colours: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fps: 10,
            width: 640,
            hold: 7.0,
            enter: 1.6,
            fade: 0.4,
            colours: 48,
        }
    }
}

impl Options {
    /// Resolve the options: `flags` beat the deck's frontmatter (`source`), which beats the
    /// config's `[film]` table, which beats the defaults.
    pub fn resolve(flags: FilmSettings, source: &str, config: &Config) -> Result<Self> {
        let front = colonmark::Frontmatter::read(source);
        let deck = FilmSettings::from_frontmatter(|key| front.get(key).map(str::to_string))?;
        let set = flags.over(deck).over(config.film);
        let base = Self::default();
        Ok(Self {
            fps: set.fps.unwrap_or(base.fps),
            width: set.width.unwrap_or(base.width),
            hold: set.hold.unwrap_or(base.hold),
            enter: set.enter.unwrap_or(base.enter),
            fade: set.fade.unwrap_or(base.fade),
            colours: set.colours.unwrap_or(base.colours),
        })
    }
}

/// The frames of a rendered deck.
#[derive(Clone, Debug)]
pub struct Rendered {
    /// Every scene (a slide at a step) as its build-in frames, last frame fully built.
    pub scenes: Vec<Vec<Frame>>,
    /// Which slide each scene shows.
    pub slide_of: Vec<usize>,
    /// The whole film, in order.
    pub sequence: Vec<Frame>,
    /// The options it was rendered with.
    pub options: Options,
}

impl Rendered {
    /// The fully built last frame of every scene.
    #[must_use]
    pub fn stills(&self) -> Vec<Frame> {
        self.scenes
            .iter()
            .filter_map(|s| s.last().cloned())
            .collect()
    }

    /// The still of the first slide's last step: what a viewer sees before the film plays.
    #[must_use]
    pub fn poster(&self) -> Option<Frame> {
        let last_of_first = self
            .slide_of
            .iter()
            .rposition(|&slide| slide == 0)
            .unwrap_or(0);
        self.scenes
            .get(last_of_first)
            .and_then(|s| s.last().cloned())
    }

    /// The animated GIF.
    pub fn gif(&self) -> Result<Vec<u8>> {
        filmstrip::gif::encode(
            &self.sequence,
            &self.stills(),
            self.options.fps,
            self.options.colours,
        )
    }

    /// The MP4.
    pub fn mp4(&self) -> Result<Vec<u8>> {
        filmstrip::video::encode(&self.sequence, self.options.fps)
    }

    /// The Word file: the GIF and the deck's links under it.
    pub fn docx(&self, deck: &Deck, gif: &[u8]) -> Result<Vec<u8>> {
        let poster = self.poster().context("no frames")?;
        let links: Vec<filmstrip::docx::Link> = deck
            .links
            .iter()
            .map(|url| filmstrip::docx::Link {
                label: strip_scheme(url).to_string(),
                url: url.clone(),
            })
            .collect();
        let style = filmstrip::docx::Style {
            font: deck.tokens.fonts.sans.clone(),
            colour: hex(deck.palette().accent),
            separator: hex(deck.palette().muted),
            size: 22,
        };
        filmstrip::docx::wrap(&poster, gif, &links, &style)
    }

    /// The PDF: the poster on an A4 landscape page, the MP4 playing over it on page open.
    pub fn pdf(&self, deck: &Deck, mp4: &[u8], note: &str) -> Result<Vec<u8>> {
        let poster = self.poster().context("no frames")?.png()?;
        let theme = theme_json(deck);
        let links = deck.links.join("\n");
        let w = 2.0_f64.mul_add(-PDF_MARGIN, PDF_PAGE.0);
        let h = w * 9.0 / 16.0;
        let geometry = format!(
            "{} {} {} {} {} {}",
            PDF_PAGE.0, PDF_PAGE.1, PDF_MARGIN, PDF_TOP, w, h
        );
        let inputs = [
            ("poster", Input::Bytes(&poster)),
            ("theme", Input::Text(&theme)),
            ("title", Input::Text(&deck.title)),
            ("author", Input::Text(&deck.author)),
            ("links", Input::Text(&links)),
            ("note", Input::Text(note)),
            ("geometry", Input::Text(&geometry)),
        ];
        let page = crate::faces::engine(&deck.faces)?
            .pdf(
                Main::Source(PAGE),
                &inputs,
                &typstkit::PdfOptions::default(),
            )
            .context("the PDF page")?;
        let rect = filmstrip::pdf::Rect {
            x: PDF_MARGIN,
            y: PDF_PAGE.1 - PDF_TOP - h,
            width: w,
            height: h,
        };
        filmstrip::pdf::embed(&page, 1, mp4, rect, "film.mp4")
    }
}

/// A card that does not fit: slide index, its title, pages it would take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overflow {
    /// 0-based slide index.
    pub slide: usize,
    /// The slide's heading, or empty.
    pub title: String,
    /// Pages the card spilled onto (at least 2).
    pub pages: usize,
}

impl std::fmt::Display for Overflow {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "slide {} ({}): the card overflows onto {} pages; trim it",
            self.slide + 1,
            self.title,
            self.pages
        )
    }
}

/// Render the whole film. A card that overflows is an error naming the slide: nothing is cropped.
pub fn render(loaded: &Loaded, options: &Options) -> Result<Rendered> {
    if !options.width.is_multiple_of(2) || options.width < 64 {
        bail!("film width must be an even number of at least 64 pixels");
    }
    if options.fps == 0 {
        bail!("film fps must be at least 1");
    }
    let engine = crate::faces::engine(&loaded.deck.faces)?;
    let mut assets = Assets::new(&loaded.base, &loaded.deck);
    let n = ((options.enter * f32::from(options.fps)).round() as usize).max(1);
    let mut scenes = Vec::new();
    let mut slide_of = Vec::new();
    let mut overflows = Vec::new();
    for (index, slide) in loaded.deck.slides.iter().enumerate() {
        let steps = fragments(slide);
        for step in 0..=steps {
            let mut frames = Vec::with_capacity(n);
            for frame in 0..n {
                let (raster, pages) = card(
                    &engine,
                    &mut assets,
                    &loaded.deck,
                    index,
                    Shot {
                        step,
                        frame,
                        frames: n,
                    },
                    options,
                )?;
                if pages > 1 && step == steps && frame + 1 == n {
                    overflows.push(Overflow {
                        slide: index,
                        title: slide.title().unwrap_or_default(),
                        pages,
                    });
                }
                frames.push(raster);
            }
            scenes.push(frames);
            slide_of.push(index);
        }
    }
    if !overflows.is_empty() {
        let mut message = String::new();
        for o in &overflows {
            let _ = writeln!(message, "{o}");
        }
        bail!("{}", message.trim_end());
    }
    let timing = filmstrip::Timing {
        fps: options.fps,
        hold: options.hold,
        fade: options.fade,
        ticks: TICKS,
    };
    let sequence = filmstrip::sequence(&scenes, timing)?;
    Ok(Rendered {
        scenes,
        slide_of,
        sequence,
        options: options.clone(),
    })
}

/// Draw only the fully built card of every slide and report the ones that overflow; what
/// `presiterm lint --film` runs.
pub fn check(loaded: &Loaded, options: &Options) -> Result<Vec<Overflow>> {
    let engine = crate::faces::engine(&loaded.deck.faces)?;
    let mut assets = Assets::new(&loaded.base, &loaded.deck);
    let mut overflows = Vec::new();
    for (index, slide) in loaded.deck.slides.iter().enumerate() {
        let steps = fragments(slide);
        let (_, pages) = card(
            &engine,
            &mut assets,
            &loaded.deck,
            index,
            Shot {
                step: steps,
                frame: 0,
                frames: 1,
            },
            options,
        )?;
        if pages > 1 {
            overflows.push(Overflow {
                slide: index,
                title: slide.title().unwrap_or_default(),
                pages,
            });
        }
    }
    Ok(overflows)
}

/// One instant of a slide: which key press, which frame of its build-in.
#[derive(Clone, Copy, Debug)]
struct Shot {
    step: usize,
    frame: usize,
    frames: usize,
}

/// Draw one card; returns the pixels and how many pages the content took (1 = it fits).
fn card(
    engine: &Engine,
    assets: &mut Assets,
    deck: &Deck,
    index: usize,
    shot: Shot,
    options: &Options,
) -> Result<(Frame, usize)> {
    let slide = &deck.slides[index];
    let (card, media) = card_json(slide, index == 0, shot.step, assets, options)?;
    let theme = theme_json(deck);
    let logical = (options.width as f32 / SCALE).round();
    let logo = match deck.logo.as_ref() {
        Some(path) => Some(
            std::fs::read(path)
                .with_context(|| format!("logo {}: missing image", path.display()))?,
        ),
        None => None,
    };
    let strings = [
        ("card", card.to_string()),
        ("theme", theme),
        ("title", deck.title.clone()),
        ("index", index.to_string()),
        ("total", deck.slides.len().to_string()),
        ("step", shot.step.to_string()),
        ("frame", shot.frame.to_string()),
        ("frames", shot.frames.to_string()),
        ("width", logical.to_string()),
    ];
    let mut inputs: Vec<(&str, Input<'_>)> = strings
        .iter()
        .map(|(k, v)| (*k, Input::Text(v.as_str())))
        .collect();
    for (key, bytes) in &media {
        inputs.push((key.as_str(), Input::Bytes(bytes)));
    }
    if let Some(logo) = &logo {
        inputs.push(("logo", Input::Bytes(logo)));
    }
    let raster = engine
        .raster(Main::Source(CARD), &inputs, 72.0 * SCALE)
        .with_context(|| {
            format!(
                "slide {} ({})",
                index + 1,
                slide.title().unwrap_or_default()
            )
        })?;
    Ok((
        Frame {
            width: raster.width,
            height: raster.height,
            rgb: raster.rgb,
        },
        raster.pages,
    ))
}

/// The pictures and diagrams of a card as `sys.inputs` entries: key and encoded bytes.
type Media = Vec<(String, Vec<u8>)>;

/// The slide as the JSON the layout reads, with pictures and diagrams drawn for `step` as bytes.
fn card_json(
    slide: &Slide,
    first: bool,
    step: usize,
    assets: &mut Assets,
    options: &Options,
) -> Result<(Value, Media)> {
    let mut blocks = Vec::new();
    let mut media = Vec::new();
    let mut base = 0usize;
    // Diagrams are drawn about half a card high; the card is 9/16 of the width.
    let px_height = (options.width as f32 * 9.0 / 16.0 * 0.5).round() as u32;
    for block in &slide.blocks {
        let value = match block {
            Block::Paragraph(lines) => json!({ "kind": "paragraph", "lines": lines }),
            Block::List(items) => {
                base += items.iter().filter(|i| i.reveal).count();
                json!({ "kind": "list", "items": items })
            },
            Block::Code { lang, lines } => json!({ "kind": "code", "lang": lang, "lines": lines }),
            Block::Table { head, rows, .. } => {
                // without the typst feature a table is text; the film draws it as lines
                let mut lines = vec![head.join("  ")];
                lines.extend(rows.iter().map(|r| r.join("  ")));
                json!({ "kind": "code", "lang": "", "lines": lines })
            },
            Block::Quote(lines) => json!({ "kind": "quote", "lines": lines }),
            Block::SubHeading(text) => json!({ "kind": "subheading", "text": text }),
            Block::Metrics(items) => json!({ "kind": "metrics", "items": items }),
            Block::Picture(picture) => {
                let key = format!("img{}", media.len());
                let loaded = assets.picture(picture, Some(px_height))?;
                media.push((key.clone(), loaded.bytes.as_ref().clone()));
                json!({ "kind": "image", "key": key, "height": picture.height, "steps": 0 })
            },
            Block::Diagram(diagram) => {
                let key = format!("img{}", media.len());
                let steps = Assets::steps(diagram);
                let lit = step.saturating_sub(base).min(steps);
                base += steps;
                let png = assets.diagram_png(diagram, lit, px_height)?;
                media.push((key.clone(), png.as_ref().clone()));
                json!({ "kind": "image", "key": key, "height": diagram.height, "steps": steps })
            },
            Block::Error(text) => json!({ "kind": "error", "text": text }),
        };
        blocks.push(value);
    }
    let card = json!({
        "title": slide.heading,
        "first": slide.is_title(first),
        "blocks": blocks,
    });
    Ok((card, media))
}

fn hex(colour: swatchbook::Rgb) -> String {
    colour.to_string().trim_start_matches('#').to_uppercase()
}

fn theme_json(deck: &Deck) -> String {
    let pal = deck.palette();
    let mut map = serde_json::Map::new();
    for (name, colour) in pal.named() {
        map.insert(name.to_string(), Value::String(colour.to_string()));
    }
    map.insert("sans".into(), Value::String(deck.tokens.fonts.sans.clone()));
    map.insert("mono".into(), Value::String(deck.tokens.fonts.mono.clone()));
    map.insert(
        "display".into(),
        Value::String(deck.tokens.fonts.display.clone()),
    );
    Value::Object(map).to_string()
}

/// `https://example.org/x` as `example.org/x`, `mailto:a@b` as `a@b`.
#[must_use]
pub fn strip_scheme(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("mailto:"))
        .unwrap_or(url)
}

/// Where the film's files go: `<out or the deck's folder>/<deck stem>.<ext>`.
#[must_use]
pub fn output_path(deck_file: &Path, out: Option<&Path>, ext: &str) -> std::path::PathBuf {
    let stem = deck_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("film");
    let dir = out.map_or_else(
        || crate::config::deck_dir(deck_file),
        std::path::Path::to_path_buf,
    );
    dir.join(format!("{stem}.{ext}"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::deck;

    fn load(source: &str) -> Loaded {
        Loaded {
            deck: deck::parse(source, "test", Path::new("."), false),
            base: Path::new(".").to_path_buf(),
            source: source.to_string(),
        }
    }

    fn quick() -> Options {
        Options {
            fps: 4,
            width: 128,
            hold: 0.5,
            enter: 0.5,
            fade: 0.25,
            colours: 16,
        }
    }

    #[test]
    fn every_key_press_is_a_scene_and_the_film_loops_through_them() {
        let loaded = load(
            "---\ntitle: T\n---\n# One\n\n**bold** and ==marked==\n\n---\n# Two\n\n- a\n+ b\n+ c\n\n:::{metrics}\n1 200 | things\n60 % | share\n:::\n",
        );
        let rendered = render(&loaded, &quick()).unwrap();
        // slide 1: one scene; slide 2: steps 0, 1, 2
        assert_eq!(rendered.scenes.len(), 4);
        assert_eq!(rendered.slide_of, vec![0, 1, 1, 1]);
        assert!(rendered.scenes.iter().all(|s| s.len() == 2)); // enter 0.5 s at 4 fps
        let first = &rendered.sequence[0];
        assert_eq!((first.width, first.height), (128, 72));
        assert!(rendered.sequence.len() > 4 * 2);
        let gif = rendered.gif().unwrap();
        assert!(gif.starts_with(b"GIF89a"));
    }

    #[test]
    fn a_card_that_overflows_is_an_error_that_names_the_slide() {
        let mut source = String::from("# Too much\n\n");
        for i in 0..40 {
            let _ = writeln!(source, "- bullet number {i} with some words on it");
        }
        let loaded = load(&source);
        let error = render(&loaded, &quick()).unwrap_err().to_string();
        assert!(error.contains("slide 1 (Too much)"), "{error}");
        let found = check(&loaded, &quick()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].slide, 0);
    }

    #[test]
    fn the_poster_is_the_first_slide_fully_built() {
        let loaded = load("# A\n\n+ one\n+ two\n\n---\n# B\n");
        let rendered = render(&loaded, &quick()).unwrap();
        assert_eq!(rendered.slide_of, vec![0, 0, 0, 1]);
        let poster = rendered.poster().unwrap();
        assert_eq!(poster, *rendered.scenes[2].last().unwrap());
    }

    #[test]
    fn output_paths_sit_next_to_the_deck_or_in_out() {
        assert_eq!(
            output_path(Path::new("talk/slides.md"), None, "gif"),
            Path::new("talk/slides.gif")
        );
        assert_eq!(
            output_path(Path::new("talk/slides.md"), Some(Path::new("dist")), "mp4"),
            Path::new("dist/slides.mp4")
        );
        assert_eq!(strip_scheme("https://a.example/x"), "a.example/x");
        assert_eq!(strip_scheme("mailto:a@b"), "a@b");
    }
}
