//! Conditional content: `:::{only} expr` blocks and the `:only: expr` option of any directive.
//!
//! An expression is tags joined by `and`, `or` and `not`, with parentheses; `not` binds tightest,
//! then `and`, then `or`. A tag is letters, digits, `-` and `_`. `,` reads as `or`, so
//! `:only: web, print` is `web or print`.

use std::collections::BTreeSet;
use std::fmt;

use crate::tree::{Block, Kind};

/// A parsed condition over tags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// True when the tag is active.
    Tag(String),
    /// True when the inner expression is false.
    Not(Box<Self>),
    /// True when both are.
    And(Box<Self>, Box<Self>),
    /// True when either is.
    Or(Box<Self>, Box<Self>),
}

/// An expression that cannot be read, with what was wrong.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprError {
    /// The expression as written.
    pub expr: String,
    /// What is wrong with it.
    pub reason: String,
}

impl fmt::Display for ExprError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "`{}`: {}", self.expr, self.reason)
    }
}

impl std::error::Error for ExprError {}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Tag(String),
    Not,
    And,
    Or,
    Open,
    Close,
}

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {},
            '(' => out.push(Token::Open),
            ')' => out.push(Token::Close),
            ',' => out.push(Token::Or),
            c if is_tag_char(c) => {
                let mut word = String::from(c);
                while let Some(next) = chars.next_if(|n| is_tag_char(*n)) {
                    word.push(next);
                }
                out.push(match word.as_str() {
                    "not" => Token::Not,
                    "and" => Token::And,
                    "or" => Token::Or,
                    _ => Token::Tag(word),
                });
            },
            other => return Err(format!("unexpected `{other}`")),
        }
    }
    Ok(out)
}

const fn is_tag_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

impl Parser {
    fn next_if(&mut self, token: &Token) -> bool {
        let hit = self.tokens.get(self.at) == Some(token);
        self.at += usize::from(hit);
        hit
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut left = self.and()?;
        while self.next_if(&Token::Or) {
            left = Expr::Or(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        while self.next_if(&Token::And) {
            left = Expr::And(Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.next_if(&Token::Not) {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        if self.next_if(&Token::Open) {
            let inner = self.or()?;
            return if self.next_if(&Token::Close) {
                Ok(inner)
            } else {
                Err("a `(` is not closed".into())
            };
        }
        match self.tokens.get(self.at).cloned() {
            Some(Token::Tag(tag)) => {
                self.at += 1;
                Ok(Expr::Tag(tag))
            },
            Some(other) => Err(format!("expected a tag, got {other:?}")),
            None => Err("expected a tag, got the end".into()),
        }
    }
}

impl Expr {
    /// Parse an expression such as `print and not draft`.
    ///
    /// # Examples
    ///
    /// ```
    /// use colonmark::only::Expr;
    ///
    /// let expr = Expr::parse("web or (print and not draft)").expect("valid");
    /// assert!(expr.eval(&["print"]));
    /// assert!(!expr.eval(&["print", "draft"]));
    /// assert!(Expr::parse("web and").is_err());
    /// ```
    pub fn parse(text: &str) -> Result<Self, ExprError> {
        let error = |reason: String| ExprError {
            expr: text.trim().to_string(),
            reason,
        };
        let mut parser = Parser {
            tokens: tokens(text).map_err(error)?,
            at: 0,
        };
        let expr = parser.or().map_err(error)?;
        if parser.at < parser.tokens.len() {
            return Err(error("unexpected words after the expression".into()));
        }
        Ok(expr)
    }

    /// Evaluate against the active tags.
    #[must_use]
    pub fn eval<S: AsRef<str>>(&self, active: &[S]) -> bool {
        match self {
            Self::Tag(tag) => active.iter().any(|a| a.as_ref() == tag),
            Self::Not(inner) => !inner.eval(active),
            Self::And(left, right) => left.eval(active) && right.eval(active),
            Self::Or(left, right) => left.eval(active) || right.eval(active),
        }
    }

    /// Collect the tags the expression names.
    #[must_use]
    pub fn tags(&self) -> BTreeSet<&str> {
        let mut out = BTreeSet::new();
        self.collect(&mut out);
        out
    }

    fn collect<'a>(&'a self, out: &mut BTreeSet<&'a str>) {
        match self {
            Self::Tag(tag) => {
                out.insert(tag);
            },
            Self::Not(inner) => inner.collect(out),
            Self::And(left, right) | Self::Or(left, right) => {
                left.collect(out);
                right.collect(out);
            },
        }
    }
}

/// The condition a block carries, as written: the argument of `{only}` or an `:only:` option.
fn condition(block: &Block) -> Option<&str> {
    let Kind::Directive(directive) = &block.kind else {
        return None;
    };
    if directive.name == "only" {
        Some(directive.argument.as_str())
    } else {
        directive.option("only")
    }
}

/// Keep the blocks whose condition holds for `active` and drop the rest, at every depth.
///
/// An `{only}` directive that holds is replaced by its body; any other directive keeps its place.
/// The first unreadable expression is an error.
///
/// # Examples
///
/// ```
/// let doc = colonmark::parse("a\n\n:::{only} long\nb\n:::\n");
/// let short = colonmark::only::apply(doc.blocks.clone(), &["short"]).expect("valid");
/// assert_eq!(short.len(), 1);
/// let long = colonmark::only::apply(doc.blocks, &["long"]).expect("valid");
/// assert_eq!(long.len(), 2);
/// ```
pub fn apply<S: AsRef<str>>(blocks: Vec<Block>, active: &[S]) -> Result<Vec<Block>, ExprError> {
    let mut out = Vec::with_capacity(blocks.len());
    for mut block in blocks {
        if let Some(text) = condition(&block)
            && !Expr::parse(text)?.eval(active)
        {
            continue;
        }
        let unwrap = matches!(&block.kind, Kind::Directive(d) if d.name == "only");
        block.map_children(|children| apply(children, active))?;
        if unwrap && let Kind::Directive(directive) = block.kind {
            out.extend(directive.blocks);
        } else {
            out.push(block);
        }
    }
    Ok(out)
}

/// A problem with a condition, found by [`check`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// The 1-based line of the directive.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

/// Find unreadable conditions, and tags that are not in `known` (skipped when `known` is empty).
///
/// # Examples
///
/// ```
/// let doc = colonmark::parse(":::{only} web or prnt\nx\n:::\n");
/// let findings = colonmark::only::check(&doc.blocks, &["web", "print"]);
/// assert_eq!(findings[0].message, "unknown tag `prnt`");
/// ```
#[must_use]
pub fn check<S: AsRef<str>>(blocks: &[Block], known: &[S]) -> Vec<Finding> {
    let mut out = Vec::new();
    for block in blocks {
        if let Some(text) = condition(block) {
            match Expr::parse(text) {
                Ok(expr) if !known.is_empty() => out.extend(
                    expr.tags()
                        .into_iter()
                        .filter(|tag| !known.iter().any(|k| k.as_ref() == *tag))
                        .map(|tag| Finding {
                            line: block.line,
                            message: format!("unknown tag `{tag}`"),
                        }),
                ),
                Ok(_) => {},
                Err(e) => out.push(Finding {
                    line: block.line,
                    message: e.to_string(),
                }),
            }
        }
        block.for_each_children(|children| out.extend(check(children, known)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest::proptest! {
        #[test]
        fn not_not_is_the_tag_itself(tag in "[a-z]{1,8}", on in proptest::bool::ANY) {
            proptest::prop_assume!(!matches!(tag.as_str(), "and" | "or" | "not"));
            let expr = Expr::parse(&format!("not not {tag}")).expect("valid");
            let active: Vec<&str> = if on { vec![tag.as_str()] } else { vec![] };
            proptest::prop_assert_eq!(expr.eval(&active), on);
        }

        #[test]
        fn parse_never_panics(text in ".{0,40}") {
            let _ = Expr::parse(&text);
        }
    }

    #[test]
    fn and_binds_tighter_than_or_and_a_comma_is_or() {
        let expr = Expr::parse("a or b and c").expect("valid");
        assert!(expr.eval(&["a"]));
        assert!(!expr.eval(&["b"]));
        assert!(Expr::parse("a, b").expect("valid").eval(&["b"]));
        assert!(Expr::parse("(a").is_err());
        assert!(Expr::parse("a b").is_err());
        assert!(Expr::parse("a ! b").is_err());
    }

    #[test]
    fn apply_filters_nested_directives_and_the_only_option() {
        let doc = crate::parse(
            "::::{card}\n:::{only} x\none\n:::\n\ntwo\n::::\n\n:::{card}\n:only: y\nthree\n:::\n",
        );
        let kept = apply(doc.blocks, &["x"]).expect("valid");
        assert_eq!(kept.len(), 1);
        let Kind::Directive(card) = &kept[0].kind else {
            panic!("card")
        };
        assert_eq!(card.blocks.len(), 2, "{card:?}");
    }

    #[test]
    fn check_reports_a_broken_expression_with_its_line() {
        let doc = crate::parse("text\n\n:::{only} (a\nx\n:::\n");
        let findings = check(&doc.blocks, &[] as &[&str]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 3);
    }
}
