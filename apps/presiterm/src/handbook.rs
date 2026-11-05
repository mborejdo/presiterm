//! The manual inside the binary: `presiterm handbook [TOPIC]` and `presiterm tldr WORDS`.
//!
//! The topics are the Markdown files of `docs/handbook/`, embedded at build time, so the manual
//! always matches the binary.

use std::fmt::Write as _;

/// One topic of the manual.
#[derive(Clone, Copy, Debug)]
pub struct Topic {
    /// The name `presiterm handbook NAME` takes.
    pub key: &'static str,
    /// One line on what the topic covers.
    pub title: &'static str,
    /// The topic in Markdown.
    pub body: &'static str,
}

/// What `presiterm handbook` prints above the list of topics.
pub const OVERVIEW: &str = "presiterm turns a Markdown file into a presentation in a WezTerm window. Text is real terminal text (WezTerm's own font, so the font size is yours to change); diagrams and pictures are inline images. Start with `presiterm new talk`, present with `presiterm --window talk/slides.md`, check with `presiterm lint talk/slides.md`. The file reloads on save and jumps to the slide you edited.";

macro_rules! topic {
    ($key:literal, $title:literal) => {
        Topic {
            key: $key,
            title: $title,
            body: include_str!(concat!("../../../docs/handbook/", $key, ".md")),
        }
    };
}

/// Every topic, in reading order.
pub const TOPICS: &[Topic] = &[
    topic!(
        "quickstart",
        "Start here: new, present, lint, flags, exit codes"
    ),
    topic!("keys", "Keys and presiterm ctl"),
    topic!(
        "dialect",
        "The Markdown dialect: frontmatter, text, marks, notes, directives, reload"
    ),
    topic!(
        "diagrams",
        "Pictures and diagrams: mermaid, typst, steps, w= h="
    ),
    topic!("window", "The window and the font size"),
    topic!(
        "config",
        "The config file presiterm.toml: closest wins, per key, theme and logo"
    ),
    topic!("tools", "lint, config, doctor, new, config-snippet"),
    topic!(
        "troubleshooting",
        "Troubleshooting: what you see, what it means"
    ),
];

/// Find a topic by its key.
#[must_use]
pub fn topic(key: &str) -> Option<&'static Topic> {
    TOPICS.iter().find(|t| t.key.eq_ignore_ascii_case(key))
}

/// The overview and the list of topics, as `presiterm handbook` prints it.
#[must_use]
pub fn index() -> String {
    let mut out = format!("{OVERVIEW}\n\nTopics (presiterm handbook TOPIC):\n");
    for t in TOPICS {
        let _ = writeln!(out, "  {:<16} {}", t.key, t.title);
    }
    out.push_str("\nSearch them all: presiterm tldr WORDS");
    out
}

/// A topic that matched a search, with its score and the first line that matched.
#[derive(Clone, Debug)]
pub struct Hit {
    /// The topic.
    pub topic: &'static Topic,
    /// Higher is better: matches in the key and title count three times.
    pub score: usize,
    /// The first line of the body that holds one of the words.
    pub line: &'static str,
}

/// The lowercase words of `text`: letters, digits, `-` and `_`.
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
}

/// Search the topics for `query`, best first, at most `limit` hits; a topic without any of the
/// words is left out.
///
/// # Examples
///
/// ```
/// let hits = presiterm::handbook::search("font size", 3);
/// assert!(hits.iter().any(|h| h.topic.key == "window"));
/// assert!(presiterm::handbook::search("zzzz", 3).is_empty());
/// ```
#[must_use]
pub fn search(query: &str, limit: usize) -> Vec<Hit> {
    let query: Vec<String> = words(query).collect();
    let mut hits: Vec<Hit> = TOPICS
        .iter()
        .filter_map(|topic| {
            let count = |text: &str| words(text).filter(|w| query.contains(w)).count();
            let score = 3 * count(&format!("{} {}", topic.key, topic.title)) + count(topic.body);
            let line = topic
                .body
                .lines()
                .find(|l| count(l) > 0)
                .unwrap_or_default()
                .trim();
            (score > 0).then_some(Hit { topic, score, line })
        })
        .collect();
    hits.sort_by_key(|hit| std::cmp::Reverse(hit.score));
    hits.truncate(limit);
    hits
}

/// The hits of a search as `presiterm tldr` prints them.
#[must_use]
pub fn format_hits(query: &str, hits: &[Hit]) -> String {
    if hits.is_empty() {
        return format!(
            "nothing in the handbook matches \"{query}\"; `presiterm handbook` lists the topics"
        );
    }
    let mut out = String::new();
    for hit in hits {
        let _ = writeln!(out, "{}: {}", hit.topic.key, hit.topic.title);
        let _ = writeln!(out, "    {}", hit.line);
    }
    let _ = write!(out, "read one: presiterm handbook {}", hits[0].topic.key);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_topic_is_found_by_its_own_title_and_by_its_key() {
        for t in TOPICS {
            assert!(!t.body.trim().is_empty(), "{} is empty", t.key);
            assert!(
                search(t.title, 3).iter().any(|h| h.topic.key == t.key),
                "tldr does not find {} by its title",
                t.key
            );
            assert_eq!(topic(&t.key.to_uppercase()).map(|f| f.key), Some(t.key));
        }
    }

    #[test]
    fn the_index_names_every_topic() {
        let index = index();
        assert!(TOPICS.iter().all(|t| index.contains(t.key)));
    }

    #[test]
    fn format_hits_says_so_when_nothing_matches() {
        assert!(format_hits("zzz", &[]).starts_with("nothing in the handbook"));
        let hits = search("speaker notes", 2);
        assert!(format_hits("speaker notes", &hits).ends_with(hits[0].topic.key));
    }
}
