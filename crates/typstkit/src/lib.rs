//! In-process Typst: a source string or a file in, PNG, SVG or PDF out.
//!
//! An [`Engine`] holds the fonts and the file resolvers and compiles any number of documents.
//! [`Main::Source`] compiles a string (its relative imports resolve against the root, if one is
//! set); [`Main::File`] compiles a file under the root. `sys.inputs` takes string pairs.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use anyhow::{Result, anyhow, bail};
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Dict, IntoValue, Str};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst_as_lib::TypstEngine;
use typst_as_lib::file_resolver::FileResolver;
use typst_layout::PagedDocument;

/// What to compile.
#[derive(Clone, Copy, Debug)]
pub enum Main<'a> {
    /// Typst source text.
    Source(&'a str),
    /// A file, as a path relative to the root (`templates/cv.typ`).
    File(&'a str),
}

/// Builds an [`Engine`].
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
        let scratch = Arc::new(Mutex::new(None));
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
            turn: Mutex::new(()),
        }
    }
}

/// The id of a project file at `path`.
fn project_file(path: &str) -> Result<FileId> {
    let vpath = VirtualPath::new(path).map_err(|e| anyhow!("{path}: {e:?}"))?;
    Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
}

/// The path [`Main::Source`] text is compiled under.
const SCRATCH: &str = "/__typstkit_main.typ";

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().get_with_slash().into())
}

/// Serves the text of the [`Main::Source`] being compiled.
struct Scratch(Arc<Mutex<Option<Source>>>);

impl FileResolver for Scratch {
    fn resolve_binary(&self, id: FileId) -> FileResult<Cow<'_, Bytes>> {
        Err(not_found(id))
    }

    fn resolve_source(&self, id: FileId) -> FileResult<Cow<'_, Source>> {
        let source = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match source.as_ref() {
            Some(source) if id == source.id() => Ok(Cow::Owned(source.clone())),
            _ => Err(not_found(id)),
        }
    }
}

/// Compiles Typst documents; fonts are parsed once, packages fetched once.
pub struct Engine {
    inner: TypstEngine,
    scratch: Arc<Mutex<Option<Source>>>,
    turn: Mutex<()>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Engine {
    /// Start building an engine.
    #[must_use]
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Compile `main` with `inputs` as `sys.inputs` into a laid-out document.
    pub fn document(&self, main: Main<'_>, inputs: &[(&str, &str)]) -> Result<PagedDocument> {
        let mut dict = Dict::new();
        for (key, value) in inputs {
            dict.insert((*key).into(), Str::from(*value).into_value());
        }
        // One compile at a time: the scratch source is shared. The resolver locks `scratch` itself
        // during the compile, so that lock is only held to swap the source.
        let _turn = self.turn.lock().unwrap_or_else(PoisonError::into_inner);
        let set = |source: Option<Source>| {
            *self.scratch.lock().unwrap_or_else(PoisonError::into_inner) = source;
        };
        let id = match main {
            Main::Source(text) => {
                let id = project_file(SCRATCH)?;
                set(Some(Source::new(id, text.to_string())));
                id
            },
            Main::File(path) => project_file(path)?,
        };
        let warned = self
            .inner
            .compile_with_input::<_, _, PagedDocument>(id, dict);
        set(None);
        warned.output.map_err(|e| match e {
            typst_as_lib::TypstAsLibError::TypstSource(diagnostics) => {
                anyhow!("{}", describe(&diagnostics))
            },
            other => anyhow!("{other}"),
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
        let Some(page) = document.pages().first() else {
            bail!("the document has no page");
        };
        let options = typst_render::RenderOptions {
            pixel_per_pt: typst::utils::Scalar::new(f64::from(ppi) / 72.0),
            render_bleed: false,
        };
        typst_render::render(page, &options)
            .encode_png()
            .map_err(|e| anyhow!("encode the PNG: {e}"))
    }

    /// Compile `main` and draw its first page as SVG.
    pub fn svg(&self, main: Main<'_>) -> Result<String> {
        let document = self.document(main, &[])?;
        let Some(page) = document.pages().first() else {
            bail!("the document has no page");
        };
        Ok(typst_svg::svg(page, &typst_svg::SvgOptions::default()))
    }

    /// Compile `main` with `inputs` to PDF bytes.
    #[cfg(feature = "pdf")]
    pub fn pdf(
        &self,
        main: Main<'_>,
        inputs: &[(&str, &str)],
        options: &typst_pdf::PdfOptions,
    ) -> Result<Vec<u8>> {
        let document = self.document(main, inputs)?;
        typst_pdf::pdf(&document, options)
            .map_err(|diagnostics| anyhow!("{}", describe(&diagnostics)))
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
