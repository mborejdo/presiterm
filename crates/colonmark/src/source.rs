//! The source as comrak sees it: frontmatter and directive option lines blanked out, so every
//! line keeps its number and comrak reads only Markdown.

use std::collections::HashMap;

use crate::Frontmatter;

/// The source prepared for comrak, and what was taken out of it.
#[derive(Debug)]
pub(crate) struct Prepared {
    /// The source with the frontmatter and option lines replaced by empty lines.
    pub(crate) text: String,
    /// The frontmatter that was taken out.
    pub(crate) frontmatter: Frontmatter,
    /// The option lines of each directive, by the 1-based line of its opener.
    pub(crate) options: HashMap<usize, Vec<(String, String)>>,
}

/// The length of a run of `fence` characters at the start of `line`, if it is at least 3.
fn fence_run(line: &str, fence: char) -> Option<usize> {
    let run = line.chars().take_while(|c| *c == fence).count();
    (run >= 3).then_some(run)
}

/// Whether `line` (trimmed) opens a colon-fence directive: three or more colons and a name.
pub(crate) fn is_opener(line: &str) -> bool {
    fence_run(line, ':').is_some_and(|run| !line[run..].trim().is_empty())
}

/// A `:key: value` option line, trimmed: the key is letters, digits, `-` and `_`.
pub(crate) fn option_line(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix(':')?;
    let (key, value) = rest.split_once(':')?;
    let valid = !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valid.then(|| (key.to_string(), value.trim().to_string()))
}

/// Prepare `source` for comrak.
pub(crate) fn prepare(source: &str) -> Prepared {
    let frontmatter = Frontmatter::read(source);
    let mut out: Vec<&str> = Vec::new();
    let mut options = HashMap::new();
    // The open code fence: its character and length.
    let mut code: Option<(char, usize)> = None;
    // The opener whose option lines are being read.
    let mut opener: Option<usize> = None;
    for (index, line) in source.lines().enumerate() {
        let number = index + 1;
        if number <= frontmatter.lines {
            out.push("");
            continue;
        }
        let trimmed = line.trim_start();
        if let Some((fence, length)) = code {
            if fence_run(trimmed, fence).is_some_and(|run| run >= length)
                && trimmed.trim_start_matches(fence).trim().is_empty()
            {
                code = None;
            }
            out.push(line);
            continue;
        }
        if let Some(at) = opener
            && let Some(pair) = option_line(trimmed)
        {
            options.entry(at).or_insert_with(Vec::new).push(pair);
            out.push("");
            continue;
        }
        opener = None;
        if is_opener(trimmed) {
            opener = Some(number);
        } else if let Some(fence) = ['`', '~']
            .into_iter()
            .find(|f| fence_run(trimmed, *f).is_some())
        {
            code = fence_run(trimmed, fence).map(|run| (fence, run));
        }
        out.push(line);
    }
    let mut text = out.join("\n");
    text.push('\n');
    Prepared {
        text,
        frontmatter,
        options,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_blanks_frontmatter_and_option_lines_but_keeps_line_numbers() {
        let source =
            "---\na: b\n---\n:::{x} arg\n:k: v\n:other-key: w x\nbody :not: an option\n:::\n";
        let prepared = prepare(source);
        assert_eq!(prepared.text.lines().count(), source.lines().count());
        assert_eq!(prepared.text.lines().nth(4), Some(""));
        assert_eq!(
            prepared.options[&4],
            [
                ("k".to_string(), "v".to_string()),
                ("other-key".to_string(), "w x".to_string())
            ]
        );
    }

    #[test]
    fn prepare_leaves_option_like_lines_in_code_alone() {
        let prepared = prepare("```\n:::{x}\n:k: v\n```\n");
        assert!(prepared.options.is_empty());
        assert!(prepared.text.contains(":k: v"));
    }

    #[test]
    fn option_line_needs_a_plain_key() {
        assert_eq!(option_line(":a b: c"), None);
        assert_eq!(option_line("::: c"), None);
        assert_eq!(
            option_line(":only: x or y"),
            Some(("only".into(), "x or y".into()))
        );
    }
}
