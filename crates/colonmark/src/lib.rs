//! Markdown with MyST-style colon-fence directives, as a small owned tree.
//!
//! [`parse`] reads CommonMark plus GitHub tables and strikethrough, `__underline__`,
//! `==highlight==` and colon-fence directives (comrak does the Markdown) into a [`Document`] of
//! [`Block`]s, each with its 1-based source line. A directive is
//!
//! ```markdown
//! :::{name} argument
//! :key: value
//! body, any Markdown, nested directives on fences with more colons
//! :::
//! ```
//!
//! [`only`] keeps or drops content by active tags, [`sections`] cuts a document at headings and
//! rules. A `---` line under text is always a rule, never a setext heading.

mod convert;
mod frontmatter;
pub mod only;
mod sections;
mod source;
mod tree;

pub use convert::parse;
pub use frontmatter::Frontmatter;
pub use sections::{Section, sections};
pub use tree::{Align, Block, Directive, Document, Inline, Item, Kind, List, Table, plain_text};
