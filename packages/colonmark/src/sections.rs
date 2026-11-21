//! Cutting a document into sections at headings and rules: slides, chapters.

use crate::tree::{Block, Inline, Kind};

/// A run of blocks that starts at a heading, a rule or the start of the document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Section {
    /// The heading that opened the section, `None` after a rule or at the start.
    pub heading: Option<Vec<Inline>>,
    /// The 1-based line the section starts on: its heading, or the line after its rule.
    pub line: usize,
    /// The blocks after the heading, up to the next cut.
    pub blocks: Vec<Block>,
}

/// Cut `blocks` before every heading of level `max_level` or lower and at every rule.
///
/// Only top-level blocks cut; a heading inside a directive or a quote stays where it is. The rule
/// itself is dropped. A section with neither heading nor blocks is left out.
///
/// # Examples
///
/// ```
/// let doc = colonmark::parse("# One\n\ntext\n\n---\n\n## Two\n\n### Three\n");
/// let sections = colonmark::sections(doc.blocks, 2, 1);
/// assert_eq!(sections.len(), 2);
/// assert_eq!(sections[1].line, 7);
/// assert_eq!(sections[1].blocks.len(), 1);
/// ```
#[must_use]
pub fn sections(blocks: Vec<Block>, max_level: u8, first_line: usize) -> Vec<Section> {
    let mut out = Vec::new();
    let mut current = Section {
        line: first_line,
        ..Section::default()
    };
    let mut flush = |section: Section| {
        if section.heading.is_some() || !section.blocks.is_empty() {
            out.push(section);
        }
    };
    for block in blocks {
        match block.kind {
            Kind::Heading { level, content } if level <= max_level => {
                flush(std::mem::replace(
                    &mut current,
                    Section {
                        heading: Some(content),
                        line: block.line,
                        blocks: Vec::new(),
                    },
                ));
            },
            Kind::Rule => {
                flush(std::mem::replace(
                    &mut current,
                    Section {
                        line: block.line + 1,
                        ..Section::default()
                    },
                ));
            },
            kind => current.blocks.push(Block { kind, ..block }),
        }
    }
    flush(current);
    out
}
