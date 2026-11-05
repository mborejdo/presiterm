//! The frontmatter: `key: value` lines between a leading `---` and the next `---`.

/// The flat `key: value` pairs of a document's frontmatter.
///
/// Only one level is read: a line without a colon, or an indented one, is skipped; values lose
/// one pair of surrounding quotes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frontmatter {
    /// The pairs in source order.
    pub pairs: Vec<(String, String)>,
    /// How many source lines the frontmatter takes, delimiters included (0 without one).
    pub lines: usize,
}

impl Frontmatter {
    /// Return the value of `key`, if the frontmatter sets it.
    ///
    /// # Examples
    ///
    /// ```
    /// let doc = colonmark::parse("---\ntitle: \"Talk\"\n---\n# One\n");
    /// assert_eq!(doc.frontmatter.get("title"), Some("Talk"));
    /// assert_eq!(doc.frontmatter.get("author"), None);
    /// ```
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Return the 1-based line the body starts on.
    #[must_use]
    pub const fn body_line(&self) -> usize {
        self.lines + 1
    }

    /// Read the frontmatter at the start of `source` (an empty one when there is none).
    #[must_use]
    pub fn read(source: &str) -> Self {
        let mut lines = source.lines();
        if lines.next().map(str::trim_end) != Some("---") {
            return Self::default();
        }
        let mut pairs = Vec::new();
        for (index, line) in lines.enumerate() {
            if matches!(line.trim_end(), "---" | "...") {
                return Self {
                    pairs,
                    lines: index + 2,
                };
            }
            if line.starts_with([' ', '\t', '#']) {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                pairs.push((key.trim().to_string(), unquote(value.trim()).to_string()));
            }
        }
        Self::default()
    }
}

/// `value` without one pair of matching surrounding quotes.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner;
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_without_a_closing_delimiter_is_no_frontmatter() {
        assert_eq!(
            Frontmatter::read("---\ntitle: x\n# body"),
            Frontmatter::default()
        );
        assert_eq!(Frontmatter::read("# body\n"), Frontmatter::default());
    }

    #[test]
    fn read_skips_nested_lines_and_counts_the_delimiters() {
        let front = Frontmatter::read("---\na: 'one'\nlist:\n  - x\nb: two: three\n---\nbody\n");
        assert_eq!(
            front.pairs,
            [
                ("a".to_string(), "one".to_string()),
                ("list".to_string(), String::new()),
                ("b".to_string(), "two: three".to_string()),
            ]
        );
        assert_eq!(front.body_line(), 7);
    }
}
