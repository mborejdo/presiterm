# The Markdown dialect

CommonMark with GitHub tables, plus a few marks. Slides split on `#`, `##` and `---`; `###` is
a sub-heading. The first slide and heading-only slides get the centred title layout. `---` is
always a break, also under text (never a setext heading).

## Frontmatter

```
---
title: Container Orchestration
author: Jane Doe
theme: dark          # dark (default) or light
font-size: 54        # points; --font-size beats it
reveal: true         # every bullet is a key press
tags: long, demo     # the active tags for {only} blocks
links: https://example.org, mailto:me@example.org   # under the picture in a film of the deck (flipcard)
---
```

## Text

* Each source line of a paragraph is a hard line break.
* Bullets `-` `*` `+`, numbers `1.`, nest by indenting under the item's text. `+` bullets appear
  one per key press (`reveal: true` or `--reveal`: all bullets). Hidden bullets keep their rows,
  nothing jumps.
* Inline: `**bold**`, `*italic*`, `` `inline code` ``, `==marked==` and `__marked__` (an
  underline in the warn colour, the text keeps its colour), `~~struck~~` (shown plain),
  `[links](show only their text)`. A `//` with spaces around it is a pause for a film; not shown.
  Headings take the same markup: `# The ==one== thing` underlines one word of a title.
* Fenced code is coloured by its tag (bash, yaml, toml, json, rust, md), per line: a multi-line
  string is coloured on its first line only.
* Tables are a picture with the theme's accent header (`:--`, `:-:`, `--:` align). `>` is a
  block quote.
* Speaker notes: a `:::{notes}` ... `:::` block, or a `Note:` line (up to the next blank line).
  Never on the slide, key `n` shows them.
* Footer: deck title and `[n/N]`. A config `logo` sits top left.

Pictures and diagrams: `presiterm handbook diagrams`.

## Directives

MyST colon fences: `:::{name} argument`, `:key: value` option lines, the body, `:::`. More
colons hold fewer. Three names are known; any other shows its body as is.

* `:::{notes}`: speaker notes.
* `:::{metrics}`: one `value | label` per line, big numbers in a row (`1 200 | commits`).
* `:::{only} expr`: the body shows when `expr` (tags with `and`, `or`, `not`, parentheses:
  `long and not demo`) holds for the frontmatter's `tags:`. Any directive takes it as an option:
  `:only: demo`.

```
:::{only} long
- the slower path, for the long version of the talk
:::
```

An expression that cannot be read is an error on the first slide.

## Layout

Content width is the window's columns minus a 2-column margin (at most 80), headings centred,
body from column 3. Text past the rows is cut at the spare row and reported ("4 rows too
many").

## Errors are visible

A missing file or broken diagram is a red line on the slide and an error in `presiterm lint`.
Nothing is dropped silently, nothing falls back.

## Reload

The deck and every file it points at are polled every 300 ms. A change reloads and jumps to the
slide of the first changed line. `--no-watch` turns it off; key `r` and `presiterm ctl reload`
still work.
