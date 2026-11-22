//! In-process Typst: a source string or a file in, PNG, SVG or PDF out.
//!
//! An [`Engine`] holds the fonts and the file resolvers and compiles any number of documents.
//! [`Main::Source`] compiles a string (its relative imports resolve against the root, if one is
//! set); [`Main::File`] compiles a file under the root. `sys.inputs` takes string pairs.
//! Compiles on one engine run side by side; Typst's cache stays until [`Engine::evict`].
//! [`Engine::query`] finds labelled `metadata` and where it sits on the page.

use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Context, Result, anyhow};
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Dict, IntoValue, Label, Selector, Str, Value};
use typst::introspection::MetadataElem;
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::utils::PicoStr;
use typst_as_lib::TypstEngine;
use typst_as_lib::file_resolver::FileResolver;
use typst_layout::{Page, PagedDocument};
#[cfg(feature = "pdf")]
pub use typst_pdf::PdfOptions;

/// What to compile.
#[derive(Clone, Copy, Debug)]
pub enum Main<'a> {
    /// Typst source text.
    Source(&'a str),
    /// A file, as a path relative to the root (`templates/report.typ`).
    File(&'a str),
}

/// One `sys.inputs` value.
#[derive(Clone, Copy, Debug)]
pub enum Input<'a> {
    /// A string (`sys.inputs.name`).
    Text(&'a str),
    /// Bytes, for `image(sys.inputs.picture)` and `json(sys.inputs.data)`.
    Bytes(&'a [u8]),
}

/// The pixels of a compiled page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGB, 3 bytes per pixel, drawn on white where the page is transparent.
    pub rgb: Vec<u8>,
    /// How many pages the document has (more than one means the content overflowed).
    pub pages: usize,
}

/// A labelled `metadata` of a compiled document, and where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// The value (only string values are found).
    pub value: String,
    /// The page, from 1.
    pub page: usize,
    /// Right from the page's left edge, in points.
    pub x: f64,
    /// Down from the page's top edge, in points.
    pub y: f64,
}

/// The builder of an [`Engine`].
#[derive(Debug, Default)]
pub struct Builder {
    fonts: Vec<Vec<u8>>,
    root: Option<PathBuf>,
    no_default_fonts: bool,
}

impl Builder {
    /// Add a font file's bytes (OpenType, TrueType or a collection).
    #[must_use]
    pub fn font(mut self, bytes: Vec<u8>) -> Self {
        self.fonts.push(bytes);
        self
    }

    /// Leave out the vendored Inter and Fira Code faces.
    #[must_use]
    pub const fn without_default_fonts(mut self) -> Self {
        self.no_default_fonts = true;
        self
    }

    /// Resolve files (and [`Main::File`]) under `root`.
    #[must_use]
    pub fn root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    /// Build the engine.
    #[must_use]
    pub fn build(self) -> Engine {
        let scratch = Arc::new(Mutex::default());
        let mut fonts: Vec<Vec<u8>> = if self.no_default_fonts {
            Vec::new()
        } else {
            swatchbook::fonts::FACES
                .iter()
                .map(|face| face.bytes.to_vec())
                .collect()
        };
        fonts.extend(self.fonts);
        let mut builder = TypstEngine::builder()
            .fonts(fonts)
            .comemo_evict_max_age(None)
            .add_file_resolver(Scratch(Arc::clone(&scratch)));
        if let Some(root) = self.root {
            builder = builder.with_file_system_resolver(root);
        }
        #[cfg(feature = "packages")]
        {
            builder = builder.with_package_file_resolver();
        }
        Engine {
            inner: builder.build(),
            scratch,
        }
    }
}

/// The id of a project file at `path`.
fn project_file(path: &str) -> Result<FileId> {
    let vpath = VirtualPath::new(path).map_err(|e| anyhow!("{path}: {e:?}"))?;
    Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().get_with_slash().into())
}

/// The [`Main::Source`] texts being compiled, by an id from their hash, with how many compiles use
/// each: the same text compiles under the same id, so Typst's cache carries over.
type Sources = Arc<Mutex<HashMap<FileId, (Source, usize)>>>;

/// The [`Main::Source`] texts being compiled, served to Typst.
struct Scratch(Sources);

impl FileResolver for Scratch {
    fn resolve_binary(&self, id: FileId) -> FileResult<Cow<'_, Bytes>> {
        Err(not_found(id))
    }

    fn resolve_source(&self, id: FileId) -> FileResult<Cow<'_, Source>> {
        let sources = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        sources
            .get(&id)
            .map(|(source, _)| Cow::Owned(source.clone()))
            .ok_or_else(|| not_found(id))
    }
}

/// The Typst compiler: fonts are parsed once, packages fetched once.
pub struct Engine {
    inner: TypstEngine,
    scratch: Sources,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Engine {
    /// Start building an engine.
    #[must_use]
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Compile `main` with string `inputs` as `sys.inputs` into a laid-out document.
    pub fn document(&self, main: Main<'_>, inputs: &[(&str, &str)]) -> Result<PagedDocument> {
        let inputs: Vec<(&str, Input<'_>)> =
            inputs.iter().map(|(k, v)| (*k, Input::Text(v))).collect();
        self.document_with(main, &inputs)
    }

    /// Compile `main` with `inputs` (strings or bytes) as `sys.inputs` into a laid-out document.
    pub fn document_with(
        &self,
        main: Main<'_>,
        inputs: &[(&str, Input<'_>)],
    ) -> Result<PagedDocument> {
        let mut dict = Dict::new();
        for (key, value) in inputs {
            let value = match value {
                Input::Text(text) => Str::from(*text).into_value(),
                Input::Bytes(bytes) => Bytes::new(bytes.to_vec()).into_value(),
            };
            dict.insert((*key).into(), value);
        }
        let sources = || self.scratch.lock().unwrap_or_else(PoisonError::into_inner);
        let (id, scratch) = match main {
            Main::Source(text) => {
                let mut hash = DefaultHasher::new();
                text.hash(&mut hash);
                let id = project_file(&format!("/__typstkit_{:016x}.typ", hash.finish()))?;
                sources()
                    .entry(id)
                    .or_insert_with(|| (Source::new(id, text.to_string()), 0))
                    .1 += 1;
                (id, true)
            },
            Main::File(path) => (project_file(path)?, false),
        };
        let warned = self
            .inner
            .compile_with_input::<_, _, PagedDocument>(id, dict);
        if let (true, Entry::Occupied(mut entry)) = (scratch, sources().entry(id)) {
            entry.get_mut().1 -= 1;
            if entry.get().1 == 0 {
                entry.remove();
            }
        }
        warned.output.map_err(|e| match e {
            typst_as_lib::TypstAsLibError::TypstSource(diagnostics) => {
                anyhow!("{}", describe(&diagnostics))
            },
            other => anyhow!("{other}"),
        })
    }

    /// Compile `main` with `inputs` and find every `metadata("..") <label>` with a string, in
    /// document order.
    ///
    /// # Examples
    ///
    /// ```
    /// use typstkit::{Input, Main};
    ///
    /// let engine = typstkit::Engine::builder().build();
    /// let source = "#set page(margin: 1in)\n#metadata(sys.inputs.word) <here>";
    /// let found = engine
    ///     .query(Main::Source(source), &[("word", Input::Text("hi"))], "here")
    ///     .expect("compiles");
    /// assert_eq!((found[0].value.as_str(), found[0].page, found[0].x), ("hi", 1, 72.0));
    /// ```
    pub fn query(
        &self,
        main: Main<'_>,
        inputs: &[(&str, Input<'_>)],
        label: &str,
    ) -> Result<Vec<Found>> {
        let document = self.document_with(main, inputs)?;
        let label = Label::new(PicoStr::intern(label)).context("an empty label")?;
        let introspector = document.introspector();
        let found = introspector.elements().query(&Selector::Label(label));
        Ok(found
            .iter()
            .filter_map(|content| {
                let Value::Str(value) = &content.to_packed::<MetadataElem>()?.value else {
                    return None;
                };
                let at = introspector.position(content.location()?)?;
                Some(Found {
                    value: value.to_string(),
                    page: at.page.get(),
                    x: at.point.x.to_pt(),
                    y: at.point.y.to_pt(),
                })
            })
            .collect())
    }

    /// Drop Typst's cached work that the last `max_age` evictions didn't see used (0: all of it).
    /// The cache is shared by every engine in the process; the typst CLI evicts at 10.
    pub fn evict(max_age: usize) {
        typst::comemo::evict(max_age);
    }

    /// Compile `main` with `inputs` and draw its first page at `ppi` pixels per inch as RGB.
    ///
    /// # Examples
    ///
    /// ```
    /// use typstkit::{Input, Main};
    ///
    /// let engine = typstkit::Engine::builder().build();
    /// let source = "#set page(width: 1in, height: 1in, margin: 0pt, fill: black)\n#sys.inputs.word";
    /// let raster = engine
    ///     .raster(Main::Source(source), &[("word", Input::Text("hi"))], 10.0)
    ///     .expect("compiles");
    /// assert_eq!((raster.width, raster.height, raster.pages), (10, 10, 1));
    /// assert_eq!(&raster.rgb[..3], &[0, 0, 0]);
    /// ```
    pub fn raster(&self, main: Main<'_>, inputs: &[(&str, Input<'_>)], ppi: f32) -> Result<Raster> {
        let document = self.document_with(main, inputs)?;
        let pixmap = typst_render::render(first_page(&document)?, &render_options(ppi));
        let mut rgb = Vec::with_capacity(pixmap.pixels().len() * 3);
        for px in pixmap.pixels() {
            let c = px.demultiply();
            // over white: a transparent page reads as paper, not as black
            let a = u16::from(c.alpha());
            let over =
                |v: u8| u8::try_from((u16::from(v) * a + 255 * (255 - a)) / 255).unwrap_or(255);
            rgb.extend_from_slice(&[over(c.red()), over(c.green()), over(c.blue())]);
        }
        Ok(Raster {
            width: pixmap.width(),
            height: pixmap.height(),
            rgb,
            pages: document.pages().len(),
        })
    }

    /// Compile `main` and draw its first page as a PNG at `ppi` pixels per inch.
    ///
    /// The page's fill is kept; set `#set page(fill: none)` for a transparent picture.
    ///
    /// # Examples
    ///
    /// ```
    /// let engine = typstkit::Engine::builder().build();
    /// let png = engine
    ///     .png(typstkit::Main::Source("#set page(width: 2cm, height: 1cm)\nHi"), 72.0)
    ///     .expect("compiles");
    /// assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    /// assert!(engine.png(typstkit::Main::Source("#nope("), 72.0).is_err());
    /// ```
    pub fn png(&self, main: Main<'_>, ppi: f32) -> Result<Vec<u8>> {
        let document = self.document(main, &[])?;
        typst_render::render(first_page(&document)?, &render_options(ppi))
            .encode_png()
            .map_err(|e| anyhow!("encode the PNG: {e}"))
    }

    /// Compile `main` and draw its first page as SVG.
    pub fn svg(&self, main: Main<'_>) -> Result<String> {
        let document = self.document(main, &[])?;
        Ok(typst_svg::svg(
            first_page(&document)?,
            &typst_svg::SvgOptions::default(),
        ))
    }

    /// Compile `main` with `inputs` (strings or bytes) to PDF bytes.
    #[cfg(feature = "pdf")]
    pub fn pdf(
        &self,
        main: Main<'_>,
        inputs: &[(&str, Input<'_>)],
        options: &PdfOptions,
    ) -> Result<Vec<u8>> {
        let document = self.document_with(main, inputs)?;
        typst_pdf::pdf(&document, options)
            .map_err(|diagnostics| anyhow!("{}", describe(&diagnostics)))
    }
}

/// The first page of `document`: the one every picture is drawn from.
fn first_page(document: &PagedDocument) -> Result<&Page> {
    document.pages().first().context("the document has no page")
}

/// How a page is drawn at `ppi` pixels per inch.
fn render_options(ppi: f32) -> typst_render::RenderOptions {
    typst_render::RenderOptions {
        pixel_per_pt: typst::utils::Scalar::new(f64::from(ppi) / 72.0),
        render_bleed: false,
    }
}

/// The messages of `diagnostics` with their hints, one per line.
fn describe(diagnostics: &[SourceDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(|d| {
            std::iter::once(d.message.to_string())
                .chain(d.hints.iter().map(|hint| format!("(hint: {})", hint.v)))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_engine_compiles_many_sources_and_reports_the_error_message() {
        let engine = Engine::builder().build();
        let a = engine.png(Main::Source("#set page(width: 1cm, height: 1cm)\na"), 72.0);
        let b = engine.png(Main::Source("#set page(width: 3cm, height: 1cm)\nb"), 72.0);
        assert_ne!(a.expect("a"), b.expect("b"));
        let error = engine
            .png(Main::Source("#let x = (\n"), 72.0)
            .expect_err("broken");
        assert!(error.to_string().contains("unclosed"), "{error}");
    }

    #[test]
    fn one_engine_compiles_side_by_side_and_evicts() {
        let engine = Engine::builder().build();
        let page = |n: u32| format!("#set page(width: {n}pt, height: 1cm)\n{n}");
        let pngs: Vec<Vec<u8>> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..8)
                .map(|n| {
                    let (engine, source) = (&engine, page(10 + n % 2));
                    scope.spawn(move || engine.png(Main::Source(&source), 72.0).expect("compiles"))
                })
                .collect();
            threads
                .into_iter()
                .map(|t| t.join().expect("joins"))
                .collect()
        });
        assert!(pngs.iter().step_by(2).all(|png| png == &pngs[0]));
        assert_ne!(pngs[0], pngs[1]);
        Engine::evict(0);
        assert!(engine.scratch.lock().expect("unlocked").is_empty());
        assert_eq!(
            engine.png(Main::Source(&page(10)), 72.0).expect("again"),
            pngs[0]
        );
    }

    #[test]
    fn inputs_reach_sys_inputs() {
        let engine = Engine::builder().build();
        let document = engine
            .document(Main::Source("#sys.inputs.name"), &[("name", "World")])
            .expect("compiles");
        assert_eq!(document.pages().len(), 1);
    }

    #[test]
    fn the_vendored_fonts_are_found_by_family() {
        let engine = Engine::builder().build();
        let svg = engine
            .svg(Main::Source(
                "#set page(width: auto, height: auto)\n#set text(font: \"Fira Code\", fallback: false)\nx",
            ))
            .expect("Fira Code is available");
        assert!(svg.starts_with("<svg"));
    }
}
