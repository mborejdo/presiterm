//! Stepped diagrams: which node lights on which key press, for mermaid and for Typst.

use std::fmt::Write as _;

use crate::deck::Diagram;

/// Key presses of a stepped typst diagram: the number after `steps=` (`typst steps=6`).
pub(super) fn typst_steps(diagram: &Diagram) -> usize {
    match diagram.nodes.as_slice() {
        [one] if one.parse::<usize>().is_ok() => one.parse().unwrap_or(0),
        [] => number_nodes(&diagram.source, &[]).1,
        _ => step_map(diagram).into_iter().max().unwrap_or(0),
    }
}

/// The step each node of a plain Typst body lights in, from `steps=1,2,2,3`: the k-th node of the
/// source lights in step `map[k]`. Empty (or a single count) numbers the nodes 1, 2, 3 in source order.
pub(super) fn step_map(diagram: &Diagram) -> Vec<usize> {
    if diagram.nodes.len() < 2 {
        return Vec::new();
    }
    diagram
        .nodes
        .iter()
        .map(|n| n.parse().unwrap_or(0))
        .collect()
}

/// A body of plain `node(..)` calls with its nodes numbered as `stepnode(i, ..)`, and how many steps
/// there are: node k lights in step `map[k]`, or in step k + 1 when `map` has no entry for it. A body
/// that already uses `stepnode` is left alone.
pub(super) fn number_nodes(body: &str, map: &[usize]) -> (String, usize) {
    if body.contains("stepnode(") {
        return (body.to_string(), 0);
    }
    let mut out = String::with_capacity(body.len() + 32);
    let mut count = 0;
    let mut steps = 0;
    let mut rest = body;
    while let Some(at) = rest.find("node(") {
        let before = rest[..at].chars().next_back();
        let word = before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-');
        out.push_str(&rest[..at]);
        if word {
            out.push_str("node(");
        } else {
            let step = map.get(count).copied().unwrap_or(count + 1);
            steps = steps.max(step);
            count += 1;
            let _ = write!(out, "stepnode({step}, ");
        }
        rest = &rest[at + 5..];
    }
    out.push_str(rest);
    (out, steps)
}

/// List the node ids a `steps` diagram lights, in order.
#[must_use]
pub fn step_nodes(diagram: &Diagram) -> Vec<String> {
    if !diagram.nodes.is_empty() {
        return diagram.nodes.clone();
    }
    let mut nodes: Vec<String> = Vec::new();
    for line in diagram.source.lines() {
        let t = line.trim();
        let skip = [
            "flowchart",
            "graph",
            "subgraph",
            "end",
            "style",
            "classDef",
            "class ",
            "linkStyle",
            "%%",
            "direction",
        ];
        if t.is_empty() || skip.iter().any(|s| t.starts_with(s)) {
            continue;
        }
        for segment in split_arrows(t) {
            let id: String = segment
                .trim()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !id.is_empty() && !nodes.contains(&id) {
                nodes.push(id);
            }
        }
    }
    nodes
}

/// Cut a flowchart line at its arrows (`-->`, `---`, `-.->`, `==>`, with or without `|label|`).
pub(super) fn split_arrows(line: &str) -> Vec<&str> {
    let bytes = line.as_bytes();
    let mut parts = Vec::new();
    let (mut start, mut i) = (0, 0);
    let mut depth = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'[' | b'(' | b'{' => depth += 1,
            b']' | b')' | b'}' => depth = depth.saturating_sub(1),
            b'-' | b'=' | b'.' if depth == 0 => {
                let run = bytes[i..]
                    .iter()
                    .take_while(|b| matches!(**b, b'-' | b'=' | b'.' | b'>' | b'<' | b'o' | b'x'))
                    .count();
                let arrow = &line[i..i + run];
                if run >= 2 && (arrow.contains('-') || arrow.contains('=')) {
                    parts.push(&line[start..i]);
                    i += run;
                    // an edge label `|text|` belongs to the arrow
                    if bytes.get(i) == Some(&b'|')
                        && let Some(end) = line[i + 1..].find('|')
                    {
                        i += end + 2;
                    }
                    start = i;
                    continue;
                }
            },
            b'&' if depth == 0 => {
                parts.push(&line[start..i]);
                start = i + 1;
            },
            _ => {},
        }
        i += 1;
    }
    parts.push(&line[start..]);
    parts
}
