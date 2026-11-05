//! comrak's arena AST to the owned tree.

use comrak::nodes::{AstNode, ListDelimType, ListType, NodeValue, TableAlignment};
use comrak::{Arena, Options, parse_document};

use crate::source::{Prepared, prepare};
use crate::tree::{Align, Block, Directive, Document, Inline, Item, Kind, List, Table};

/// The comrak options colonmark parses with: GFM tables and strikethrough, `__underline__`,
/// `==highlight==`, colon-fence block directives, no setext headings.
fn options() -> Options<'static> {
    let mut options = Options::default();
    let extension = &mut options.extension;
    extension.table = true;
    extension.strikethrough = true;
    extension.underline = true;
    extension.highlight = true;
    extension.block_directive = true;
    options.parse.ignore_setext = true;
    options
}

/// Parse a Markdown document with colon-fence directives into the owned tree.
///
/// # Examples
///
/// ```
/// use colonmark::Kind;
///
/// let doc = colonmark::parse("# Title\n\n:::{note} Careful\n:class: big\nBody\n:::\n");
/// assert!(matches!(&doc.blocks[0].kind, Kind::Heading { level: 1, .. }));
/// let Kind::Directive(note) = &doc.blocks[1].kind else { unreachable!() };
/// assert_eq!((note.name.as_str(), note.argument.as_str()), ("note", "Careful"));
/// assert_eq!(note.option("class"), Some("big"));
/// assert_eq!(note.raw, "Body\n");
/// assert_eq!(doc.blocks[1].line, 3);
/// ```
#[must_use]
pub fn parse(source: &str) -> Document {
    let prepared = prepare(source);
    let arena = Arena::new();
    let root = parse_document(&arena, &prepared.text, &options());
    let lines: Vec<&str> = source.lines().collect();
    let context = Context {
        prepared: &prepared,
        lines: &lines,
    };
    Document {
        blocks: context.blocks(root),
        frontmatter: prepared.frontmatter.clone(),
    }
}

struct Context<'a> {
    prepared: &'a Prepared,
    lines: &'a [&'a str],
}

impl Context<'_> {
    fn blocks<'a>(&self, parent: &'a AstNode<'a>) -> Vec<Block> {
        parent
            .children()
            .filter_map(|node| self.block(node))
            .collect()
    }

    fn block<'a>(&self, node: &'a AstNode<'a>) -> Option<Block> {
        let data = node.data();
        let line = data.sourcepos.start.line;
        let kind = match &data.value {
            NodeValue::Heading(heading) => Kind::Heading {
                level: heading.level,
                content: inlines(node),
            },
            NodeValue::Paragraph => Kind::Paragraph(inlines(node)),
            NodeValue::List(list) => Kind::List(List {
                start: (list.list_type == ListType::Ordered).then_some(list.start),
                marker: match (list.list_type, list.delimiter) {
                    (ListType::Ordered, ListDelimType::Period) => '.',
                    (ListType::Ordered, ListDelimType::Paren) => ')',
                    (ListType::Bullet, _) => char::from(list.bullet_char),
                },
                tight: list.tight,
                items: node
                    .children()
                    .map(|item| Item {
                        line: item.data().sourcepos.start.line,
                        blocks: self.blocks(item),
                    })
                    .collect(),
            }),
            NodeValue::CodeBlock(code) => Kind::Code {
                info: code.info.trim().to_string(),
                text: code.literal.clone(),
            },
            NodeValue::Table(table) => Kind::Table(table_of(node, &table.alignments)),
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
                Kind::Quote(self.blocks(node))
            },
            NodeValue::ThematicBreak => Kind::Rule,
            NodeValue::HtmlBlock(html) => Kind::Html(html.literal.clone()),
            NodeValue::BlockDirective(directive) => {
                let end = data.sourcepos.end.line;
                Kind::Directive(self.directive(node, &directive.info, line, end))
            },
            _ => return None,
        };
        Some(Block { line, kind })
    }

    fn directive<'a>(
        &self,
        node: &'a AstNode<'a>,
        info: &str,
        line: usize,
        end: usize,
    ) -> Directive {
        let (name, argument) = split_info(info);
        let options = self
            .prepared
            .options
            .get(&line)
            .cloned()
            .unwrap_or_default();
        // The body runs from after the option lines to before the closing fence (if there is one).
        let first = line + options.len();
        let closed = self
            .lines
            .get(end.saturating_sub(1))
            .is_some_and(|l| l.trim().chars().all(|c| c == ':') && end > line);
        let last = if closed { end - 1 } else { end };
        let raw = self
            .lines
            .get(first.min(last)..last.min(self.lines.len()))
            .unwrap_or_default()
            .iter()
            .fold(String::new(), |mut raw, l| {
                raw.push_str(l);
                raw.push('\n');
                raw
            });
        Directive {
            name,
            argument,
            options,
            blocks: self.blocks(node),
            raw,
        }
    }
}

/// `{name} argument` or `name argument` as (name, argument).
fn split_info(info: &str) -> (String, String) {
    let info = info.trim();
    if let Some(rest) = info.strip_prefix('{')
        && let Some((name, argument)) = rest.split_once('}')
    {
        return (name.trim().to_string(), argument.trim().to_string());
    }
    let (name, argument) = info.split_once(char::is_whitespace).unwrap_or((info, ""));
    (name.to_string(), argument.trim().to_string())
}

fn table_of<'a>(node: &'a AstNode<'a>, alignments: &[TableAlignment]) -> Table {
    let align = alignments
        .iter()
        .map(|a| match a {
            TableAlignment::Left => Align::Left,
            TableAlignment::Center => Align::Center,
            TableAlignment::Right => Align::Right,
            TableAlignment::None => Align::None,
        })
        .collect();
    let mut head = Vec::new();
    let mut rows = Vec::new();
    for row in node.children() {
        let cells: Vec<Vec<Inline>> = row.children().map(inlines).collect();
        if matches!(row.data().value, NodeValue::TableRow(true)) {
            head = cells;
        } else {
            rows.push(cells);
        }
    }
    Table { align, head, rows }
}

/// The inline children of `node`, adjacent text runs merged.
fn inlines<'a>(node: &'a AstNode<'a>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for child in node.children() {
        let Some(inline) = inline(child) else {
            continue;
        };
        if let (Some(Inline::Text(last)), Inline::Text(text)) = (out.last_mut(), &inline) {
            last.push_str(text);
        } else {
            out.push(inline);
        }
    }
    out
}

fn inline<'a>(node: &'a AstNode<'a>) -> Option<Inline> {
    let data = node.data();
    Some(match &data.value {
        NodeValue::Text(text) => Inline::Text(text.to_string()),
        NodeValue::Code(code) => Inline::Code(code.literal.clone()),
        NodeValue::Emph => Inline::Emph(inlines(node)),
        NodeValue::Strong => Inline::Strong(inlines(node)),
        NodeValue::Underline => Inline::Underline(inlines(node)),
        NodeValue::Highlight => Inline::Highlight(inlines(node)),
        NodeValue::Strikethrough => Inline::Strike(inlines(node)),
        NodeValue::Link(link) => Inline::Link {
            url: link.url.clone(),
            content: inlines(node),
        },
        NodeValue::Image(link) => Inline::Image {
            url: link.url.clone(),
            title: link.title.clone(),
            alt: crate::plain_text(&inlines(node)),
        },
        NodeValue::HtmlInline(html) => Inline::Html(html.clone()),
        NodeValue::SoftBreak => Inline::SoftBreak,
        NodeValue::LineBreak => Inline::LineBreak,
        // Escapes, entities and anything else that carries text: its children.
        _ => {
            let content = inlines(node);
            return (!content.is_empty()).then(|| Inline::Text(crate::plain_text(&content)));
        },
    })
}

#[cfg(test)]
mod tests;
