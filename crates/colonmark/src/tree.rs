//! The owned tree a document parses into: blocks with their line, and inline runs.

/// A parsed document: its frontmatter and its blocks in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    /// The `key: value` pairs between the leading `---` lines.
    pub frontmatter: crate::Frontmatter,
    /// The blocks after the frontmatter.
    pub blocks: Vec<Block>,
}

/// One block and where it starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The 1-based line of the block's first line in the source.
    pub line: usize,
    /// What the block is.
    pub kind: Kind,
}

impl Block {
    /// Call `visit` with every list of child blocks: a quote's, a directive's, each list item's.
    pub fn for_each_children(&self, mut visit: impl FnMut(&[Self])) {
        match &self.kind {
            Kind::Quote(blocks) => visit(blocks),
            Kind::Directive(directive) => visit(&directive.blocks),
            Kind::List(list) => list.items.iter().for_each(|item| visit(&item.blocks)),
            _ => {},
        }
    }

    /// Replace every list of child blocks by what `map` makes of it; the first error stops.
    pub fn map_children<E>(
        &mut self,
        mut map: impl FnMut(Vec<Self>) -> Result<Vec<Self>, E>,
    ) -> Result<(), E> {
        let mut replace = |blocks: &mut Vec<Self>| -> Result<(), E> {
            *blocks = map(std::mem::take(blocks))?;
            Ok(())
        };
        match &mut self.kind {
            Kind::Quote(blocks) => replace(blocks),
            Kind::Directive(directive) => replace(&mut directive.blocks),
            Kind::List(list) => list
                .items
                .iter_mut()
                .try_for_each(|item| replace(&mut item.blocks)),
            _ => Ok(()),
        }
    }
}

/// What a block is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// An ATX heading (`#` to `######`). Setext headings are not recognised: a `---` line is a
    /// [`Kind::Rule`].
    Heading {
        /// 1 to 6.
        level: u8,
        /// The heading text.
        content: Vec<Inline>,
    },
    /// A paragraph; its source line breaks are [`Inline::SoftBreak`]s.
    Paragraph(Vec<Inline>),
    /// A bullet or ordered list.
    List(List),
    /// A fenced or indented code block.
    Code {
        /// The info string after the opening fence (`rust`, `mermaid steps h=9`), empty if none.
        info: String,
        /// The code, with a trailing newline per line.
        text: String,
    },
    /// A GitHub-style table.
    Table(Table),
    /// A block quote.
    Quote(Vec<Block>),
    /// A thematic break (`---`, `***`, `___`).
    Rule,
    /// A raw HTML block.
    Html(String),
    /// A colon-fence directive.
    Directive(Directive),
}

/// A list and its items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct List {
    /// The number of the first item for an ordered list, `None` for bullets.
    pub start: Option<usize>,
    /// The marker of a bullet list (`-`, `*` or `+`); `.` or `)` for an ordered list.
    pub marker: char,
    /// No blank lines between the items.
    pub tight: bool,
    /// The items.
    pub items: Vec<Item>,
}

/// One list item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The 1-based line of the item's marker.
    pub line: usize,
    /// The item's content: usually a paragraph, maybe followed by a nested list.
    pub blocks: Vec<Block>,
}

/// A table: header cells, body rows and the alignment of each column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    /// The alignment of each column.
    pub align: Vec<Align>,
    /// The header row.
    pub head: Vec<Vec<Inline>>,
    /// The body rows.
    pub rows: Vec<Vec<Vec<Inline>>>,
}

/// The alignment of a table column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// No colon in the rule: left.
    #[default]
    None,
    /// `:--`.
    Left,
    /// `:-:`.
    Center,
    /// `--:`.
    Right,
}

/// A colon-fence directive: `:::{name} argument`, option lines, a body, `:::`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Directive {
    /// The name between the braces (or the first word after `::: `).
    pub name: String,
    /// The rest of the opening line, trimmed.
    pub argument: String,
    /// The `:key: value` lines right after the opener, in order.
    pub options: Vec<(String, String)>,
    /// The body as blocks.
    pub blocks: Vec<Block>,
    /// The body as written: the lines between the option lines and the closing fence.
    pub raw: String,
}

impl Directive {
    /// Return the value of option `key`, if the directive has it.
    #[must_use]
    pub fn option(&self, key: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// A run of inline content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Inline {
    /// Plain text.
    Text(String),
    /// `` `code` ``.
    Code(String),
    /// `*emphasis*` or `_emphasis_`.
    Emph(Vec<Self>),
    /// `**strong**`.
    Strong(Vec<Self>),
    /// `__underline__`.
    Underline(Vec<Self>),
    /// `==highlight==`.
    Highlight(Vec<Self>),
    /// `~~strikethrough~~`.
    Strike(Vec<Self>),
    /// `[text](url)`.
    Link {
        /// The target.
        url: String,
        /// The link text.
        content: Vec<Self>,
    },
    /// `![alt](url "title")`.
    Image {
        /// The picture's path or URL.
        url: String,
        /// The title in quotes after the URL, empty if none.
        title: String,
        /// The alt text, flattened to plain text.
        alt: String,
    },
    /// Raw inline HTML.
    Html(String),
    /// A line break in the source inside a paragraph.
    SoftBreak,
    /// A hard line break (two trailing spaces or a backslash).
    LineBreak,
}

/// Join the text of `inlines` without any markup; breaks become spaces, images their alt text.
///
/// # Examples
///
/// ```
/// let doc = colonmark::parse("a **b** [c](u)");
/// let colonmark::Kind::Paragraph(content) = &doc.blocks[0].kind else { unreachable!() };
/// assert_eq!(colonmark::plain_text(content), "a b c");
/// ```
#[must_use]
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    push_plain(inlines, &mut out);
    out
}

fn push_plain(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::Code(text) | Inline::Html(text) => out.push_str(text),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::Emph(content)
            | Inline::Strong(content)
            | Inline::Underline(content)
            | Inline::Highlight(content)
            | Inline::Strike(content)
            | Inline::Link { content, .. } => push_plain(content, out),
            Inline::SoftBreak | Inline::LineBreak => out.push(' '),
        }
    }
}
