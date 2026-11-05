//! How the Markdown blocks of one slide become presiterm's blocks: lists, quotes and notes,
//! fences, pictures and tables.

use std::fmt::Write as _;
use std::path::Path;

use colonmark::{Align, Inline, Kind};

use super::spans::{line, lines};
use super::{Block, Diagram, Item, Picture, Slide, Span, plain};

/// Turn the Markdown `blocks` of a slide into its blocks and notes.
pub(super) fn fill(slide: &mut Slide, blocks: Vec<colonmark::Block>, reveal: bool, base: &Path) {
    let mut previous_was_list = false;
    for block in blocks {
        let is_list = matches!(block.kind, Kind::List(_));
        match block.kind {
            Kind::Heading { content, .. } => {
                slide.blocks.push(Block::SubHeading(plain(&line(&content))));
            },
            Kind::Paragraph(content) => paragraph(slide, &content, base),
            Kind::List(list) => {
                let mut items = Vec::new();
                let mut after = Vec::new();
                flatten(&list, 0, reveal, &mut items, &mut after);
                // `- a` then `+ b` are two lists to CommonMark (another marker), one to the slide.
                match slide.blocks.last_mut() {
                    Some(Block::List(open)) if previous_was_list => open.append(&mut items),
                    _ => slide.blocks.push(Block::List(items)),
                }
                fill(slide, after, reveal, base);
            },
            Kind::Code { info, text } => {
                let body = text.lines().map(str::to_string).collect();
                slide.blocks.push(fence_block(&info, body, block.line));
            },
            Kind::Table(table) => slide
                .blocks
                .push(table_block(&table, format!("line {}", block.line))),
            Kind::Quote(inner) => slide.blocks.push(quote(inner)),
            Kind::Html(html) => slide.blocks.push(Block::Paragraph(
                html.lines().map(|l| vec![Span::from(l)]).collect(),
            )),
            Kind::Directive(directive) if directive.name.eq_ignore_ascii_case("notes") => {
                push_note(&mut slide.notes, directive.raw.trim());
            },
            Kind::Directive(directive) => fill(slide, directive.blocks, reveal, base),
            Kind::Rule => {},
        }
        previous_was_list = is_list;
    }
}

impl From<&str> for Span {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_string(),
            ..Self::default()
        }
    }
}

/// A paragraph: speaker notes after `Note:`, pictures on lines of their own, else lines of text.
fn paragraph(slide: &mut Slide, content: &[Inline], base: &Path) {
    let text_lines = lines(content);
    if let Some(first) = text_lines.first().and_then(|l| note_start(&plain(l))) {
        push_note(&mut slide.notes, &first);
        for rest in &text_lines[1..] {
            push_note(&mut slide.notes, plain(rest).trim());
        }
        return;
    }
    let mut text: Vec<Vec<Span>> = Vec::new();
    let sources = content.split(|i| matches!(i, Inline::SoftBreak | Inline::LineBreak));
    for (source, spans) in sources.zip(text_lines) {
        if let Some(picture) = picture_line(source, base) {
            if !text.is_empty() {
                slide
                    .blocks
                    .push(Block::Paragraph(std::mem::take(&mut text)));
            }
            slide.blocks.push(picture);
        } else {
            text.push(spans);
        }
    }
    if !text.is_empty() {
        slide.blocks.push(Block::Paragraph(text));
    }
}

/// The items of `list` at nesting `level`, nested lists flattened after their parent item.
/// Blocks inside an item that are neither text nor a list go to `after`.
fn flatten(
    list: &colonmark::List,
    level: usize,
    reveal: bool,
    items: &mut Vec<Item>,
    after: &mut Vec<colonmark::Block>,
) {
    for (index, item) in list.items.iter().enumerate() {
        let mut spans: Vec<Span> = Vec::new();
        let mut nested = Vec::new();
        for block in &item.blocks {
            match &block.kind {
                Kind::Paragraph(content) => {
                    if !spans.is_empty() {
                        spans.push(Span::from(" "));
                    }
                    spans.extend(line(content));
                },
                Kind::List(inner) => nested.push(inner),
                _ => after.push(block.clone()),
            }
        }
        let number = list.start.map(|start| start + index);
        items.push(Item {
            level,
            number,
            spans,
            reveal: list.marker == '+' || reveal && number.is_none(),
        });
        for inner in nested {
            flatten(inner, level + 1, reveal, items, after);
        }
    }
}

/// A block quote as quoted lines: its paragraphs line by line; anything else is named, not drawn.
fn quote(inner: Vec<colonmark::Block>) -> Block {
    let mut quoted = Vec::new();
    for block in inner {
        if let Kind::Paragraph(content) = block.kind {
            quoted.extend(lines(&content));
        } else {
            return Block::Error(format!("line {}: a quote holds only text here", block.line));
        }
    }
    Block::Quote(quoted)
}

fn push_note(notes: &mut String, line: &str) {
    if !notes.is_empty() {
        notes.push('\n');
    }
    notes.push_str(line);
}

/// The text after a `Note:` or `Notes:` opener, any case.
fn note_start(line: &str) -> Option<String> {
    let t = line.trim_start();
    ["notes:", "note:"]
        .iter()
        .find(|p| {
            t.get(..p.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(p))
        })
        .map(|p| t[p.len()..].trim().to_string())
}

/// The size and step options of a fence or a picture: `h=9`, `w=30`, `steps`, `steps=a,b`.
#[derive(Debug, Default)]
struct Options {
    height: Option<usize>,
    width: Option<usize>,
    steps: bool,
    nodes: Vec<String>,
}

impl Options {
    /// Read one option word; `false` when it is not one.
    fn read(&mut self, word: &str) -> bool {
        if word == "steps" {
            self.steps = true;
        } else if let Some(n) = word.strip_prefix("h=") {
            self.height = n.parse().ok();
        } else if let Some(n) = word.strip_prefix("w=") {
            self.width = n.parse().ok();
        } else if let Some(n) = word
            .strip_prefix("steps=")
            .or_else(|| word.strip_prefix("nodes="))
        {
            self.steps = true;
            self.nodes = n
                .split(',')
                .filter(|n| !n.is_empty())
                .map(str::to_string)
                .collect();
        } else {
            return false;
        }
        true
    }

    fn diagram(self, source: String, typst: bool, origin: String) -> Diagram {
        Diagram {
            source,
            typst,
            height: self.height,
            width: self.width,
            steps: self.steps,
            nodes: self.nodes,
            origin,
        }
    }
}

/// A fenced code block: a diagram for `mermaid`, `steps` and `typst`, else code.
fn fence_block(info: &str, body: Vec<String>, line: usize) -> Block {
    let mut words = info
        .split(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .filter(|w| !w.is_empty());
    let kind = words.next().unwrap_or_default();
    let origin = format!("line {line}");
    match kind {
        "mermaid" | "steps" | "typst" => {
            let mut options = Options {
                steps: kind == "steps",
                ..Options::default()
            };
            for word in words {
                options.read(word);
            }
            Block::Diagram(options.diagram(body.join("\n"), kind == "typst", origin))
        },
        _ => Block::Code {
            lang: kind.to_string(),
            lines: body,
        },
    }
}

/// Whether `path` ends in one of `extensions`, ignoring case.
///
/// # Examples
///
/// ```
/// assert!(presiterm::deck::extension_is("flow.MMD", &["mmd", "mermaid"]));
/// assert!(!presiterm::deck::extension_is("flow.png", &["mmd", "mermaid"]));
/// ```
#[must_use]
pub fn extension_is(path: &str, extensions: &[&str]) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// A line that is only `![alt](path)` and an optional `{h=7,w=20}`: a picture, or a diagram file.
fn picture_line(source: &[Inline], base: &Path) -> Option<Block> {
    let (path, alt, attrs) = match source {
        [Inline::Image { url, alt, .. }] => (url, alt, ""),
        [Inline::Image { url, alt, .. }, Inline::Text(rest)] => (url, alt, rest.trim()),
        _ => return None,
    };
    let braced = attrs.is_empty() || attrs.starts_with('{') && attrs.ends_with('}');
    if path.is_empty() || !braced {
        return None;
    }
    let mut options = Options::default();
    let mut words = attrs
        .trim_matches(['{', '}'])
        .split([' ', ';', ','])
        .filter(|w| !w.is_empty() && *w != "c");
    while let Some(word) = words.next() {
        // `{steps=a,b,c}` was cut at the commas: the words after the first node are more nodes.
        if !options.read(word) && options.steps && !options.nodes.is_empty() && !word.contains('=')
        {
            options.nodes.push(word.to_string());
            options.nodes.extend(
                words
                    .by_ref()
                    .take_while(|w| !w.contains('='))
                    .map(str::to_string),
            );
        }
    }
    if extension_is(path, &["mmd", "mermaid"]) {
        return Some(match std::fs::read_to_string(base.join(path)) {
            Ok(source) => Block::Diagram(options.diagram(source, false, path.clone())),
            Err(e) => Block::Error(format!("{path}: {e}")),
        });
    }
    Some(Block::Picture(Picture {
        alt: alt.clone(),
        path: path.clone(),
        height: options.height,
        width: options.width,
    }))
}

/// A table: a Typst picture when built with Typst (cells are never cut at a column edge), else
/// aligned text.
fn table_block(table: &colonmark::Table, origin: String) -> Block {
    let cell = |cell: &Vec<Inline>| plain(&line(cell));
    let head: Vec<String> = table.head.iter().map(cell).collect();
    let rows: Vec<Vec<String>> = table
        .rows
        .iter()
        .map(|row| {
            (0..head.len())
                .map(|i| row.get(i).map(cell).unwrap_or_default())
                .collect()
        })
        .collect();
    let align: Vec<char> = table
        .align
        .iter()
        .map(|a| match a {
            Align::Center => 'c',
            Align::Right => 'r',
            Align::Left | Align::None => 'l',
        })
        .collect();
    if cfg!(feature = "typst") {
        Block::Diagram(table_picture(&head, &rows, &align, origin))
    } else {
        Block::Table { head, rows, align }
    }
}

/// A table as Typst source: the theme's accent header and a tinted band on every other row.
fn table_picture(head: &[String], rows: &[Vec<String>], align: &[char], origin: String) -> Diagram {
    let cell = |text: &str| {
        let mut out = String::from("[");
        for c in text.chars() {
            if "\\#$@*_`<>[]~=+-/".contains(c) {
                out.push('\\');
            }
            out.push(c);
        }
        out.push(']');
        out
    };
    let row = |cells: &[String], wrap: &str| {
        cells
            .iter()
            .map(|c| format!("{wrap}{}", cell(c)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let columns = vec!["auto"; head.len()].join(", ");
    let aligns: Vec<&str> = align
        .iter()
        .map(|a| match a {
            'c' => "center",
            'r' => "right",
            _ => "left",
        })
        .collect();
    let mut source = format!(
        "#let h(t) = text(fill: ground, weight: \"bold\", t)\n#table(\n  columns: ({columns},),\n  \
         align: ({},),\n  stroke: none,\n  inset: (x: 12pt, y: 8pt),\n  \
         fill: (x, y) => if y == 0 {{ accent }} else if calc.odd(y) {{ accent.transparentize(85%) }} else {{ none }},\n",
        aligns.join(", ")
    );
    for (cells, wrap) in std::iter::once((head, "h")).chain(rows.iter().map(|r| (r.as_slice(), "")))
    {
        let _ = writeln!(source, "  {},", row(cells, wrap));
    }
    source.push(')');
    Options::default().diagram(source, true, origin)
}
