//! The source as comrak sees it: frontmatter and directive option lines blanked out, so every
//! line keeps its number and comrak reads only Markdown.

use std::borrow::Cow;
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
    /// The info of each opener whose info has a `:` (comrak refuses those), by its line.
    pub(crate) infos: HashMap<usize, String>,
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
    let mut out: Vec<Cow<'_, str>> = Vec::new();
    let mut options = HashMap::new();
    let mut infos = HashMap::new();
    // The open code fence: its character and length.
    let mut code: Option<(char, usize)> = None;
    // The opener whose option lines are being read.
    let mut opener: Option<usize> = None;
    for (index, line) in source.lines().enumerate() {
        let number = index + 1;
        if number <= frontmatter.lines {
            out.push(Cow::Borrowed(""));
            continue;
        }
        let trimmed = line.trim_start();
        if let Some((fence, length)) = code {
            if fence_run(trimmed, fence).is_some_and(|run| run >= length)
                && trimmed.trim_start_matches(fence).trim().is_empty()
            {
                code = None;
            }
            out.push(Cow::Borrowed(line));
            continue;
        }
        if let Some(at) = opener
            && let Some(pair) = option_line(trimmed)
        {
            options.entry(at).or_insert_with(Vec::new).push(pair);
            out.push(Cow::Borrowed(""));
            continue;
        }
        opener = None;
        if is_opener(trimmed) {
            opener = Some(number);
            let info = trimmed.trim_start_matches(':');
            if info.contains(':') {
                infos.insert(number, info.trim().to_string());
                let fence = &line[..line.len() - info.len()];
                out.push(Cow::Owned(format!("{fence}{}", info.replace(':', " "))));
                continue;
            }
        } else if let Some(fence) = ['`', '~']
            .into_iter()
            .find(|f| fence_run(trimmed, *f).is_some())
        {
            code = fence_run(trimmed, fence).map(|run| (fence, run));
        }
        out.push(Cow::Borrowed(line));
    }
    let mut text = out.join("\n");
    text.push('\n');
    Prepared {
        text,
        frontmatter,
        options,
        infos,
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
    fn prepare_keeps_an_info_with_a_colon_aside() {
        let doc = crate::parse(":::{project} AI: chat\n:only: x\nBody\n:::\n");
        let crate::Kind::Directive(d) = &doc.blocks[0].kind else {
            panic!("a directive: {doc:?}")
        };
        assert_eq!(
            (d.name.as_str(), d.argument.as_str()),
            ("project", "AI: chat")
        );
        assert_eq!((d.option("only"), d.raw.as_str()), (Some("x"), "Body\n"));
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
