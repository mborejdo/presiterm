//! What a parsed deck is made of: spans, blocks, slides and the theme.

/// Which ground a deck stands on: `theme: dark` (default) or `theme: light` in the frontmatter.
pub use swatchbook::Mode as Theme;

/// A run of inline text with its emphasis.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools, reason = "independent inline switches")]
pub struct Span {
    /// The text.
    pub text: String,
    /// `**bold**`.
    pub bold: bool,
    /// `*italic*` or `_italic_`.
    pub italic: bool,
    /// `` `code` ``.
    pub code: bool,
    /// `==marked==` or `__marked__`: underlined in the warn colour.
    pub underline: bool,
}

/// A bullet or numbered item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Nesting depth, 0 for the top level.
    pub level: usize,
    /// `Some(n)` for a numbered item.
    pub number: Option<usize>,
    /// The text.
    pub spans: Vec<Span>,
    /// Shown only after a key press (`+` bullets, or every bullet under `reveal`).
    pub reveal: bool,
}

/// A mermaid or Typst diagram with its options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagram {
    /// The mermaid source, or Typst (fletcher) source when `typst` is set.
    pub source: String,
    /// Typst instead of mermaid: drawn in-process, `steps=N` lights one `stepnode` per key press.
    pub typst: bool,
    /// Height in cells; `None` takes the rest of the slide.
    pub height: Option<usize>,
    /// Largest width in cells (`w=`); `None` is the text width.
    pub width: Option<usize>,
    /// Light one node per key press.
    pub steps: bool,
    /// The nodes to light, in order; empty means every node in order of appearance.
    pub nodes: Vec<String>,
    /// Where it came from, for error messages.
    pub origin: String,
}

/// A picture file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    /// Alt text.
    pub alt: String,
    /// Path as written, relative to the deck.
    pub path: String,
    /// Height in cells; `None` takes the rest of the slide.
    pub height: Option<usize>,
    /// Largest width in cells (`w=`); `None` is the text width.
    pub width: Option<usize>,
}

/// One block of a slide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    /// Lines of prose, one entry per source line.
    Paragraph(Vec<Vec<Span>>),
    /// Bullets and numbers.
    List(Vec<Item>),
    /// A fenced code block.
    Code {
        /// Language tag (used for colouring only, never shown).
        lang: String,
        /// The lines.
        lines: Vec<String>,
    },
    /// A table; cells are plain text.
    Table {
        /// Header cells.
        head: Vec<String>,
        /// Body rows.
        rows: Vec<Vec<String>>,
        /// Alignment per column: `'l'`, `'c'` or `'r'`.
        align: Vec<char>,
    },
    /// A block quote.
    Quote(Vec<Vec<Span>>),
    /// A `###` heading inside a slide.
    SubHeading(String),
    /// A picture file.
    Picture(Picture),
    /// A mermaid or Typst diagram (a table is one too, when built with Typst).
    Diagram(Diagram),
    /// Something presiterm cannot show; displayed in the slide and reported by lint.
    Error(String),
}

/// One slide.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Slide {
    /// The heading, with its inline markup.
    pub heading: Option<Vec<Span>>,
    /// Blocks in source order.
    pub blocks: Vec<Block>,
    /// Speaker notes; never on the slide.
    pub notes: String,
    /// 1-based line of the slide's first line in the file.
    pub line: usize,
}

impl Slide {
    /// Return the heading as plain text.
    #[must_use]
    pub fn title(&self) -> Option<String> {
        self.heading.as_deref().map(super::plain)
    }

    /// Centred title layout: a heading alone, or a heading with prose only.
    #[must_use]
    pub const fn is_title(&self, first: bool) -> bool {
        first || self.blocks.is_empty() && self.heading.is_some()
    }
}

/// A parsed deck.
#[derive(Clone, Debug, PartialEq)]
pub struct Deck {
    /// `title:` or the file name.
    pub title: String,
    /// `author:`.
    pub author: String,
    /// `theme:`.
    pub theme: Theme,
    /// The design tokens the deck is drawn in (the config file's `[theme]`, else the defaults).
    pub tokens: swatchbook::Theme,
    /// A picture for the top left corner of every slide (the config file's `logo`).
    pub logo: Option<std::path::PathBuf>,
    /// `font-size:` in points.
    pub font_size: Option<f32>,
    /// `reveal: true`: every bullet appears on a key press.
    pub reveal: bool,
    /// The slides.
    pub slides: Vec<Slide>,
}

impl Deck {
    /// Return the palette of the deck's ground.
    #[must_use]
    pub const fn palette(&self) -> &swatchbook::Palette {
        self.tokens.palette(self.theme)
    }
}
