# colonmark

Markdown with MyST-style colon-fence directives, parsed by [comrak](https://crates.io/crates/comrak)
into a small owned tree that is easy to walk.

```markdown
---
title: Talk
---

# One

::::{card} A title
:class: wide

Text with **bold**.

:::{only} long and not draft
Shown only when the tag `long` is active and `draft` is not.
:::
::::
```

- `parse(source)` gives a `Document`: the frontmatter pairs and the blocks (headings, paragraphs,
  lists, code, tables, quotes, rules, HTML and directives), every block with its 1-based line.
- A directive is `:::{name} argument` (or `::: name argument`), its `:key: value` option lines
  directly after the opener, its body as blocks and as raw text. More colons on the outer fence
  nest one directive in another.
- `only::apply` keeps or drops content by tags: `:::{only} expr` blocks and any directive with an
  `:only: expr` option. `expr` is tags joined by `and`, `or`, `not` and parentheses.
- `sections` cuts blocks at headings and rules, for slides or chapters.

License: MIT OR Apache-2.0.
