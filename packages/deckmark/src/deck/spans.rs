//! Inline markup as styled spans: colonmark's inline tree flattened to runs of text.

use colonmark::{Inline, Kind};

use super::Span;

/// Flatten `inlines` into lines of spans: every soft or hard break starts a new line.
pub(super) fn lines(inlines: &[Inline]) -> Vec<Vec<Span>> {
    let mut out = vec![Vec::new()];
    push(inlines, &Span::default(), &mut out);
    out.into_iter().map(merge).collect()
}

/// Flatten `inlines` into one line of spans; breaks become spaces.
pub(super) fn line(inlines: &[Inline]) -> Vec<Span> {
    let mut joined: Vec<Span> = Vec::new();
    for (n, mut part) in lines(inlines).into_iter().enumerate() {
        if n > 0 {
            joined.push(Span::from(" "));
        }
        joined.append(&mut part);
    }
    merge(joined)
}

/// `text` with the emphasis of `marks`, a span whose text is empty.
fn text(text: &str, marks: &Span) -> Span {
    Span {
        text: text.to_string(),
        ..marks.clone()
    }
}

/// `text` cut at every `//` with whitespace (or the text's end) on both sides: the parts between,
/// the mark and the space after it gone. Every part but the first follows a pause.
fn pauses(text: &str) -> Vec<&str> {
    let (mut parts, mut from) = (Vec::new(), 0);
    for (at, _) in text.match_indices("//") {
        let apart = |c: Option<char>| c.is_none_or(char::is_whitespace);
        let after = text[at + 2..].chars().next();
        if apart(text[..at].chars().next_back()) && apart(after) {
            parts.push(&text[from..at]);
            from = at + 2 + after.map_or(0, char::len_utf8);
        }
    }
    parts.push(&text[from..]);
    parts
}

/// Push `inlines` onto the lines of `out`; `marks` is the emphasis around them.
fn push(inlines: &[Inline], marks: &Span, out: &mut Vec<Vec<Span>>) {
    for inline in inlines {
        let current = out.last_mut();
        match (inline, current) {
            (Inline::SoftBreak | Inline::LineBreak, _) => out.push(Vec::new()),
            (Inline::Text(t) | Inline::Html(t), Some(line)) => {
                for (n, part) in pauses(t).into_iter().enumerate() {
                    if n > 0 || !part.is_empty() {
                        line.push(Span {
                            pause: n > 0,
                            ..text(part, marks)
                        });
                    }
                }
            },
            (Inline::Image { alt, .. }, Some(line)) => line.push(text(alt, marks)),
            (Inline::Code(code), Some(line)) => line.push(Span {
                code: true,
                ..text(code, marks)
            }),
            (Inline::Strong(inner), _) => push(
                inner,
                &Span {
                    bold: true,
                    ..marks.clone()
                },
                out,
            ),
            (Inline::Emph(inner), _) => push(
                inner,
                &Span {
                    italic: true,
                    ..marks.clone()
                },
                out,
            ),
            (Inline::Underline(inner), _) => push(
                inner,
                &Span {
                    underline: true,
                    ..marks.clone()
                },
                out,
            ),
            (Inline::Highlight(inner), _) => push(
                inner,
                &Span {
                    highlight: true,
                    ..marks.clone()
                },
                out,
            ),
            (Inline::Strike(inner), _) => push(
                inner,
                &Span {
                    strike: true,
                    ..marks.clone()
                },
                out,
            ),
            (Inline::Link { content: inner, .. }, _) => push(inner, marks, out),
            (_, None) => {},
        }
    }
}

/// Join neighbouring spans of the same look; a span after a pause starts anew, and an empty one
/// (a `//` at the end of a run) passes its pause on.
fn merge(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for span in spans {
        let look = |s: &Span| (s.bold, s.italic, s.code, s.underline, s.highlight, s.strike);
        match out.last_mut() {
            Some(last) if last.pause && last.text.is_empty() => {
                *last = Span {
                    pause: true,
                    ..span
                };
            },
            Some(last) if !span.pause && look(last) == look(&span) => {
                last.text.push_str(&span.text);
            },
            _ => out.push(span),
        }
    }
    out
}

/// Inline markup of one line of text as spans: `**bold**`, `*italic*`, `` `code` ``,
/// `__underlined__`, `==highlighted==`, `~~struck~~`, `[text](url)` (the text); a `//` with
/// whitespace on both sides is a pause, gone from the text.
///
/// # Examples
///
/// ```
/// let spans = deckmark::deck::inline("a **bold** word");
/// assert_eq!(spans.len(), 3);
/// assert!(spans[1].bold);
/// assert_eq!(deckmark::deck::plain(&spans), "a bold word");
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
