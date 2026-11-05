//! Inline markup as styled spans: colonmark's inline tree flattened to runs of text.

use colonmark::{Inline, Kind};

use super::Span;

/// The emphasis that applies at one point of the tree.
#[derive(Clone, Copy, Default)]
struct Marks {
    bold: bool,
    italic: bool,
    underline: bool,
}

/// Flatten `inlines` into lines of spans: every soft or hard break starts a new line.
pub(super) fn lines(inlines: &[Inline]) -> Vec<Vec<Span>> {
    let mut out = vec![Vec::new()];
    push(inlines, Marks::default(), &mut out);
    out.into_iter().map(merge).collect()
}

/// Flatten `inlines` into one line of spans; breaks become spaces.
pub(super) fn line(inlines: &[Inline]) -> Vec<Span> {
    let mut joined: Vec<Span> = Vec::new();
    for (n, mut part) in lines(inlines).into_iter().enumerate() {
        if n > 0 {
            joined.push(text(" ", Marks::default()));
        }
        joined.append(&mut part);
    }
    merge(joined)
}

fn text(text: &str, marks: Marks) -> Span {
    Span {
        text: text.to_string(),
        bold: marks.bold,
        italic: marks.italic,
        code: false,
        underline: marks.underline,
    }
}

fn push(inlines: &[Inline], marks: Marks, out: &mut Vec<Vec<Span>>) {
    for inline in inlines {
        let current = out.last_mut();
        match (inline, current) {
            (Inline::SoftBreak | Inline::LineBreak, _) => out.push(Vec::new()),
            (Inline::Text(t) | Inline::Html(t), Some(line)) => line.push(text(t, marks)),
            (Inline::Image { alt, .. }, Some(line)) => line.push(text(alt, marks)),
            (Inline::Code(code), Some(line)) => line.push(Span {
                code: true,
                ..text(code, marks)
            }),
            (Inline::Strong(inner), _) => push(
                inner,
                Marks {
                    bold: true,
                    ..marks
                },
                out,
            ),
            (Inline::Emph(inner), _) => push(
                inner,
                Marks {
                    italic: true,
                    ..marks
                },
                out,
            ),
            (Inline::Underline(inner) | Inline::Highlight(inner), _) => push(
                inner,
                Marks {
                    underline: true,
                    ..marks
                },
                out,
            ),
            (Inline::Strike(inner) | Inline::Link { content: inner, .. }, _) => {
                push(inner, marks, out);
            },
            (_, None) => {},
        }
    }
}

/// Join neighbouring spans of the same look.
fn merge(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for span in spans {
        match out.last_mut() {
            Some(last)
                if (last.bold, last.italic, last.code, last.underline)
                    == (span.bold, span.italic, span.code, span.underline) =>
            {
                last.text.push_str(&span.text);
            },
            _ => out.push(span),
        }
    }
    out
}

/// Inline markup of one line of text as spans: `**bold**`, `*italic*`, `` `code` ``,
/// `==marked==` and `__marked__` (underlined), `[text](url)` (the text).
///
/// # Examples
///
/// ```
/// let spans = presiterm::deck::inline("a **bold** word");
/// assert_eq!(spans.len(), 3);
/// assert!(spans[1].bold);
/// assert_eq!(presiterm::deck::plain(&spans), "a bold word");
/// ```
#[must_use]
pub fn inline(text: &str) -> Vec<Span> {
    colonmark::parse(text)
        .blocks
        .into_iter()
        .find_map(|block| match block.kind {
            Kind::Paragraph(content) | Kind::Heading { content, .. } => Some(line(&content)),
            _ => None,
        })
        .unwrap_or_default()
}

/// Join the text of `spans`, without any emphasis.
#[must_use]
pub fn plain(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}
