//! Tiny line-wise token colouring for the few languages a talk shows.
//!
//! No grammar engine: bash, yaml, toml, json, rust and markdown get comments, strings, numbers,
//! keywords/keys and flags; any other tag gets comments, strings and numbers. Stateless per line,
//! so a string that runs over several lines is only coloured on the line that opens it.

/// What a stretch of code is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Everything else.
    Plain,
    /// Keywords, commands, keys, section names.
    Keyword,
    /// String literals.
    Str,
    /// Comments.
    Comment,
    /// Numbers, booleans, variables.
    Value,
    /// Options and punctuation of note (`--flag`, `|`, `&&`).
    Flag,
}

const RUST: &[&str] = &[
    "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait", "match", "if",
    "else", "for", "while", "loop", "return", "const", "static", "async", "await", "move", "where",
    "self", "Self", "crate", "type", "as", "in", "ref", "dyn", "unsafe",
];
const BASH: &[&str] = &[
    "if", "then", "else", "fi", "for", "do", "done", "while", "case", "esac", "in", "function",
];

/// End of the word that starts at `start`: letters, digits and `_-./@`, and a `:` inside a word.
fn word_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() {
        let c = chars[end];
        let inner_colon = c == ':' && chars.get(end + 1).is_some_and(|n| !n.is_whitespace());
        if !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '@') || inner_colon) {
            break;
        }
        end += 1;
    }
    end.max(start + 1)
}

/// Colour one line of `lang` as `(kind, text)` runs that concatenate back to `line`.
///
/// # Examples
///
/// ```
/// let runs = presiterm::highlight::line("bash", "ls -la");
/// let joined: String = runs.iter().map(|(_, text)| text.as_str()).collect();
/// assert_eq!(joined, "ls -la");
/// ```
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "one scanner whose state (command position, quotes) lives across its arms"
)]
pub fn line(lang: &str, line: &str) -> Vec<(Kind, String)> {
    let lang = lang.to_lowercase();
    let comment = match lang.as_str() {
        "rust" | "rs" | "js" | "ts" | "go" | "c" | "cpp" | "java" => "//",
        "json" | "md" | "markdown" => "",
        _ => "#",
    };
    let chars: Vec<char> = line.chars().collect();
    let mut out: Vec<(Kind, String)> = Vec::new();
    let mut push = |kind: Kind, text: String| {
        if text.is_empty() {
            return;
        }
        match out.last_mut() {
            Some((k, t)) if *k == kind => t.push_str(&text),
            _ => out.push((kind, text)),
        }
    };
    if matches!(lang.as_str(), "md" | "markdown") && line.starts_with('#') {
        push(Kind::Keyword, line.to_string());
        return out;
    }
    let mut command_next = matches!(lang.as_str(), "bash" | "sh" | "shell" | "zsh" | "console");
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let rest: String = chars[i..].iter().collect();
        if !comment.is_empty()
            && rest.starts_with(comment)
            && (i == 0 || chars[i - 1].is_whitespace() || comment == "//")
        {
            push(Kind::Comment, rest);
            break;
        }
        if c == '"' || c == '\'' {
            let end = chars[i + 1..]
                .iter()
                .position(|x| *x == c)
                .map_or(chars.len(), |p| i + p + 2);
            let text: String = chars[i..end].iter().collect();
            let is_key = matches!(lang.as_str(), "json" | "yaml" | "yml" | "toml")
                && chars.get(end) == Some(&':');
            push(if is_key { Kind::Keyword } else { Kind::Str }, text);
            i = end;
            command_next = false;
        } else if c == '$' {
            let end = i
                + 1
                + chars[i + 1..]
                    .iter()
                    .take_while(|x| x.is_alphanumeric() || **x == '_' || **x == '{' || **x == '}')
                    .count();
            push(Kind::Value, chars[i..end].iter().collect());
            i = end;
        } else if c.is_ascii_digit()
            && (i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_')
        {
            let end = i + chars[i..]
                .iter()
                .take_while(|x| x.is_ascii_alphanumeric() || **x == '.')
                .count();
            push(Kind::Value, chars[i..end].iter().collect());
            i = end;
        } else if c.is_alphabetic()
            || c == '_'
            || c == '-'
                && chars
                    .get(i + 1)
                    .is_some_and(|n| n.is_alphabetic() || *n == '-')
        {
            let end = word_end(&chars, i);
            let word: String = chars[i..end].iter().collect();
            let key_like = matches!(lang.as_str(), "yaml" | "yml" | "toml")
                && chars.get(end) == Some(&':')
                || matches!(lang.as_str(), "toml")
                    && chars[end..].iter().find(|x| !x.is_whitespace()) == Some(&'=')
                    && line[..].trim_start().starts_with(word.as_str());
            let kind = if word.starts_with('-') {
                Kind::Flag
            } else if key_like
                || (command_next && !word.contains('=') && !word.starts_with('-'))
                || matches!(lang.as_str(), "rust" | "rs") && RUST.contains(&word.as_str())
                || matches!(lang.as_str(), "bash" | "sh" | "shell" | "zsh")
                    && BASH.contains(&word.as_str())
            {
                Kind::Keyword
            } else if matches!(word.as_str(), "true" | "false" | "null" | "yes" | "no") {
                Kind::Value
            } else {
                Kind::Plain
            };
            push(kind, word);
            command_next = false;
            i = end;
        } else {
            if matches!(lang.as_str(), "bash" | "sh" | "shell" | "zsh" | "console")
                && matches!(c, '|' | ';' | '&')
            {
                command_next = true;
                push(Kind::Flag, c.to_string());
            } else if c == '[' && lang == "toml" && line.trim_start().starts_with('[') {
                push(Kind::Keyword, chars[i..].iter().collect());
                break;
            } else {
                push(Kind::Plain, c.to_string());
            }
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(lang: &str, text: &str) -> Vec<(Kind, String)> {
        line(lang, text)
    }

    #[test]
    fn line_runs_concatenate_back_to_the_line() {
        for (lang, text) in [
            ("bash", "docker logs --since 10m -f printsvc # hi"),
            ("yaml", "key: \"v\" # c"),
            ("rust", "let x: u32 = 5; // n"),
            ("json", "{\"a\": [1, true]}"),
            ("toml", "[a.b]"),
            ("", "ärger – ünï"),
        ] {
            let joined: String = line(lang, text).into_iter().map(|(_, t)| t).collect();
            assert_eq!(joined, text, "{lang}");
        }
    }

    #[test]
    fn bash_marks_command_flag_string_and_comment() {
        let k = kinds("bash", "journalctl -u docker --since \"10 min ago\" # x");
        assert_eq!(k[0], (Kind::Keyword, "journalctl".into()));
        assert!(k.iter().any(|(kind, t)| *kind == Kind::Flag && t == "-u"));
        assert!(
            k.iter()
                .any(|(kind, t)| *kind == Kind::Str && t == "\"10 min ago\"")
        );
        assert_eq!(k.last().map(|(kind, _)| *kind), Some(Kind::Comment));
    }

    #[test]
    fn yaml_key_is_a_keyword() {
        assert_eq!(kinds("yaml", "name: x")[0], (Kind::Keyword, "name".into()));
    }
}
