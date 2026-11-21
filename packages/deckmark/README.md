# deckmark

The presiterm deck dialect as a library: a Markdown file in, slides of blocks out.

- `deck::parse`, `deck::read`: frontmatter, `{only}` editions, slides cut at `#`, `##` and `---`,
  `+` bullets, `:::{metrics}`, `:::{notes}`, pictures and diagrams.
- `diagram::Assets`: mermaid and Typst diagrams and pictures as PNG in the deck's tokens, cached
  on disk.
- `faces::engine`: the Typst engine with the vendored faces and the deck's font files.
- `fragments`: the key presses a slide takes.

Feature `typst` (default) draws Typst fences and tables; without it they stay text.

The dialect: `presiterm handbook dialect`.
