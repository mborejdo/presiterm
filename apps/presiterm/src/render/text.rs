//! How a block of the slide becomes lines of styled cells: inline spans, wrapping, lists, code and tables.

use super::{Geometry, Palette, text_style};
use crate::canvas::{Canvas, Style, width};
use crate::deck::{Block, Item, Span};
use crate::highlight::{self, Kind};

#[derive(Clone, Debug)]
pub(super) struct Line {
    pub(super) cells: Vec<(char, Style)>,
    pub(super) indent: usize,
    pub(super) fill: Option<Style>,
    pub(super) centre: bool,
    pub(super) frag: Option<usize>,
}

impl Line {
    pub(super) const fn new(cells: Vec<(char, Style)>) -> Self {
        Self {
            cells,
            indent: 0,
            fill: None,
            centre: false,
            frag: None,
        }
    }
}

pub(super) fn paint(canvas: &mut Canvas, line: &Line, row: usize, geo: Geometry) {
    if let Some(fill) = line.fill {
        canvas.fill(geo.left, row, geo.width, 1, fill);
    }
    let w: usize = line
        .cells
        .iter()
        .map(|(c, _)| unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0))
        .sum();
    let col = if line.centre {
        geo.left + geo.width.saturating_sub(w) / 2
    } else {
        geo.left + line.indent
    };
    put_cells(canvas, col, row, &line.cells);
}

pub(super) fn put_cells(canvas: &mut Canvas, col: usize, row: usize, cells: &[(char, Style)]) {
    let mut x = col;
    for (ch, style) in cells {
        let w = unicode_width::UnicodeWidthChar::width(*ch).unwrap_or(0);
        if w == 0 {
            continue;
        }
        canvas.put(x, row, *ch, *style);
        x += w;
    }
}

pub(super) fn spans_to_cells(spans: &[Span], pal: &Palette, base: Style) -> Vec<(char, Style)> {
    let mut cells = Vec::new();
    for span in spans {
        let mut style = base;
        if span.bold {
            style = style.bold();
        }
        if span.italic {
            style = style.italic();
        }
        if span.code {
            style = style.fg(pal.warn).bg(pal.surface);
        }
        if span.underline {
            style = style.underline(pal.warn);
        }
        cells.extend(span.text.chars().map(|c| (c, style)));
    }
    cells
}

/// Greedy word wrap on cells; a word longer than `max` is cut.
/// A word of cells and the style of the space that preceded it.
pub(super) type Word<'a> = (Option<Style>, &'a [(char, Style)]);

pub(super) fn wrap(cells: &[(char, Style)], max: usize) -> Vec<Vec<(char, Style)>> {
    let max = max.max(1);
    let mut lines: Vec<Vec<(char, Style)>> = Vec::new();
    let mut current: Vec<(char, Style)> = Vec::new();
    let mut current_w = 0;
    let cell_w = |c: char| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
    // Words, each with the style of the space before it: a space keeps the look it had in the text, so
    // a code span does not paint the gap after it.
    let mut words: Vec<Word<'_>> = Vec::new();
    let (mut start, mut sep) = (0, None);
    for (i, (c, style)) in cells.iter().enumerate() {
        if *c == ' ' {
            words.push((sep, &cells[start..i]));
            (start, sep) = (i + 1, Some(*style));
        }
    }
    words.push((sep, &cells[start..]));
    for (sep, word) in words {
        let word_w: usize = word.iter().map(|(c, _)| cell_w(*c)).sum();
        let needs_space = !current.is_empty();
        if needs_space && current_w + 1 + word_w > max {
            lines.push(std::mem::take(&mut current));
            current_w = 0;
        }
        if let Some(&(_, last)) = current.last() {
            current.push((' ', sep.unwrap_or(last)));
            current_w += 1;
        }
        for cell in word {
            let w = cell_w(cell.0);
            if current_w + w > max {
                lines.push(std::mem::take(&mut current));
                current_w = 0;
            }
            current.push(*cell);
            current_w += w;
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

pub(super) fn block_lines(
    block: &Block,
    pal: &Palette,
    geo: Geometry,
    title: bool,
    counter: &mut usize,
    problems: &mut Vec<String>,
) -> Vec<Line> {
    let base = text_style(pal);
    match block {
        Block::Paragraph(lines) => lines
            .iter()
            .flat_map(|spans| wrap(&spans_to_cells(spans, pal, base), geo.width))
            .map(|cells| Line {
                centre: title,
                ..Line::new(cells)
            })
            .collect(),
        Block::SubHeading(text) => {
            vec![Line {
                centre: title,
                ..Line::new(
                    text.chars()
                        .map(|c| (c, base.fg(pal.accent).bold()))
                        .collect(),
                )
            }]
        },
        Block::Quote(lines) => {
            let mut out = Vec::new();
            for spans in lines {
                let cells = spans_to_cells(spans, pal, base.fg(pal.muted).italic());
                for part in wrap(&cells, geo.width.saturating_sub(2)) {
                    let mut line = vec![('▎', base.fg(pal.accent)), (' ', base)];
                    line.extend(part);
                    out.push(Line::new(line));
                }
            }
            out
        },
        Block::List(items) => items
            .iter()
            .flat_map(|item| item_lines(item, pal, geo, counter))
            .collect(),
        Block::Code { lang, lines } => code_lines(lang, lines, pal, geo, problems),
        Block::Table { head, rows, align } => table_lines(head, rows, align, pal, geo, problems),
        Block::Error(message) => {
            problems.push(message.clone());
            let style = base.fg(pal.stop).bold();
            let cells: Vec<(char, Style)> =
                format!("⚠ {message}").chars().map(|c| (c, style)).collect();
            wrap(&cells, geo.width).into_iter().map(Line::new).collect()
        },
        Block::Picture(_) | Block::Diagram(_) => Vec::new(),
    }
}

pub(super) fn item_lines(
    item: &Item,
    pal: &Palette,
    geo: Geometry,
    counter: &mut usize,
) -> Vec<Line> {
    let base = text_style(pal);
    let frag = item.reveal.then(|| {
        *counter += 1;
        *counter
    });
    let marker = item.number.map_or_else(
        || ["• ", "◦ ", "▪ "][item.level.min(2)].to_string(),
        |n| format!("{n}. "),
    );
    let indent = (2 * item.level).min(geo.width.saturating_sub(6));
    let mw = width(&marker);
    let cells = spans_to_cells(&item.spans, pal, base);
    wrap(&cells, geo.width.saturating_sub(indent + mw))
        .into_iter()
        .enumerate()
        .map(|(n, part)| {
            let mut line: Vec<(char, Style)> = if n == 0 {
                marker
                    .chars()
                    .map(|c| (c, base.fg(pal.accent).bold()))
                    .collect()
            } else {
                " ".repeat(mw).chars().map(|c| (c, base)).collect()
            };
            line.extend(part);
            Line {
                indent,
                frag,
                ..Line::new(line)
            }
        })
        .collect()
}

pub(super) fn code_lines(
    lang: &str,
    lines: &[String],
    pal: &Palette,
    geo: Geometry,
    problems: &mut Vec<String>,
) -> Vec<Line> {
    let panel = text_style(pal).bg(pal.surface);
    let room = geo.width.saturating_sub(2);
    lines
        .iter()
        .map(|source| {
            let mut cells: Vec<(char, Style)> = vec![(' ', panel)];
            for (kind, text) in highlight::line(lang, &source.replace('\t', "  ")) {
                let style = match kind {
                    Kind::Plain => panel,
                    Kind::Keyword => panel.fg(pal.accent).bold(),
                    Kind::Str => panel.fg(pal.ok),
                    Kind::Comment => panel.fg(pal.muted).italic(),
                    Kind::Value => panel.fg(pal.warn),
                    Kind::Flag => panel.fg(pal.muted.mix(pal.ink, 0.5)),
                };
                cells.extend(text.chars().map(|c| (c, style)));
            }
            let w: usize = cells
                .iter()
                .map(|(c, _)| unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0))
                .sum();
            if w > room + 1 {
                problems.push(format!(
                    "code line is {} columns, {} fit: {}",
                    w - 1,
                    room,
                    clip(source, 30)
                ));
                let mut kept = 0;
                cells.retain(|(c, _)| {
                    kept += unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0);
                    kept < room + 1
                });
                cells.push(('…', panel.fg(pal.muted)));
            }
            Line {
                fill: Some(panel),
                ..Line::new(cells)
            }
        })
        .collect()
}

pub(super) fn table_lines(
    head: &[String],
    rows: &[Vec<String>],
    align: &[char],
    pal: &Palette,
    geo: Geometry,
    problems: &mut Vec<String>,
) -> Vec<Line> {
    let base = text_style(pal);
    let n = head.len();
    let mut widths: Vec<usize> = (0..n)
        .map(|c| {
            std::iter::once(head)
                .chain(rows.iter().map(Vec::as_slice))
                .map(|r| r.get(c).map_or(0, |t| width(t)))
                .max()
                .unwrap_or(1)
        })
        .collect();
    let gutter = 3;
    let total = |w: &[usize]| w.iter().sum::<usize>() + gutter * n.saturating_sub(1);
    let before = total(&widths);
    while total(&widths) > geo.width {
        let Some((widest, _)) = widths.iter().enumerate().max_by_key(|(_, w)| **w) else {
            break;
        };
        if widths[widest] <= 3 {
            break;
        }
        widths[widest] -= 1;
    }
    if total(&widths) < before {
        problems.push(format!("table is {before} columns, {} fit", geo.width));
    }
    let cell_cells = |text: &str, w: usize, a: char, style: Style| -> Vec<(char, Style)> {
        let shown = clip(text, w);
        let pad = w.saturating_sub(width(&shown));
        let (left, right) = match a {
            'r' => (pad, 0),
            'c' => (pad / 2, pad - pad / 2),
            _ => (0, pad),
        };
        let mut out: Vec<(char, Style)> = " ".repeat(left).chars().map(|c| (c, base)).collect();
        out.extend(shown.chars().map(|c| (c, style)));
        out.extend(" ".repeat(right).chars().map(|c| (c, base)));
        out
    };
    let sep = base.fg(pal.muted);
    let build = |row: &[String], style: Style| -> Line {
        let mut cells = Vec::new();
        for (c, column) in widths.iter().enumerate() {
            if c > 0 {
                cells.extend(" │ ".chars().map(|ch| (ch, sep)));
            }
            cells.extend(cell_cells(
                row.get(c).map_or("", String::as_str),
                *column,
                align.get(c).copied().unwrap_or('l'),
                style,
            ));
        }
        Line::new(cells)
    };
    let mut out = vec![build(head, base.fg(pal.accent).bold())];
    let rule: Vec<(char, Style)> = widths
        .iter()
        .enumerate()
        .flat_map(|(c, w)| {
            let mut part: Vec<(char, Style)> = Vec::new();
            if c > 0 {
                part.extend("─┼─".chars().map(|ch| (ch, sep)));
            }
            part.extend("─".repeat(*w).chars().map(|ch| (ch, sep)));
            part
        })
        .collect();
    out.push(Line::new(rule));
    out.extend(rows.iter().map(|r| build(r, base)));
    out
}

pub(super) fn clip(text: &str, max: usize) -> String {
    if width(text) <= max {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        if width(&out) + 1 >= max {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}
