---
title: "presiterm in ten slides"
author: "Michael Borejdo"
theme: dark
tags: long
---

# presiterm

**Markdown in, a terminal presentation out**

Text is terminal text, pictures are inline images

Note: The first slide uses the centred title layout.

---

# Why a terminal

- the font is the terminal's: `+` and `-` resize it live
- no export step: save the file, the slide reloads
+ bullets with `+` appear one key press at a time
+ like this one

Note: Press n for these notes, n again to return.

---

# Inline marks

**bold**, *italic*, `code`, ==marked== and __marked__ text.

Every line of a paragraph
stays its own line on the slide.

> A quote is drawn with a bar
> and in the muted colour.

---

# Code

```rust
fn main() {
    let deck = std::fs::read_to_string("slides.md");
    println!("{} bytes", deck.map_or(0, |d| d.len()));
}
```

```bash
presiterm lint slides.md --font-size 54
```

---

# A table

| Key       | Does                     |
|:----------|:-------------------------|
| space     | next step, then slide    |
| backspace | previous step            |
| n         | speaker notes            |
| + / -     | font size                |

---

# Numbered and nested

1. parse the Markdown
2. cut it into slides
   - at `#` and `##`
   - at `---`
3. lay each slide out on the grid

---

# Conditional content

:::{only} long
Only the long version of the talk shows this:
the frontmatter says `tags: long`.
:::

:::{only} not long
The short version.
:::

::: notes
`:::{only} expr` takes and, or, not and parentheses.
:::

---

# Thanks
