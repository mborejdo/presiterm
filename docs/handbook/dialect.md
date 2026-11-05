# The Markdown dialect

CommonMark with GitHub tables, plus a few marks for slides. Slides split on `#` and `##` headings
and on `---`. A `###` is a sub-heading inside a slide. A slide with only a heading, and the first
slide, use the centred title layout. A `---` line is always a slide break, also right under a line
of text (it is never a setext heading).

## Frontmatter

```
---
title: Container Orchestration
author: Jane Doe
theme: dark          # dark (default) or light
font-size: 54        # points; --font-size beats it
reveal: true         # every bullet is a key press
tags: long, demo     # the active tags for {only} blocks
---
```

## Text

* Every source line of a paragraph is a hard line break.
* Bullets `-` `*` `+`, numbers `1.`, nesting by indenting under the item's text. `+` bullets appear
  one per key press; `reveal: true` or `--reveal` does that for every bullet. Hidden bullets keep
  their rows, so nothing jumps.
* Inline: `**bold**`, `*italic*`, `` `inline code` ``, `==marked==` and `__marked__` (an underline
  in the warn colour, the text keeps its colour), `[links](show only their text)`. Headings take
  the same inline markup, so `# The ==one== thing` underlines one word of a title.
* Fenced code is coloured by its language tag (bash, yaml, toml, json, rust, md); the tag is never
  shown. Highlighting is per line (a string spanning lines is coloured on its first line only).
* Tables are drawn as a picture with the theme's accent header (`:--`, `:-:`, `--:` set the
  alignment). `>` is a block quote.
* Speaker notes: a `::: notes` ... `:::` block, or a `Note:` line (up to the next blank line).
  Never on the slide; key `n` shows them.
* Every slide has a footer with the deck title and `[n/N]`; a `logo` in the config file sits in
  the top left corner.

Pictures and diagrams: `presiterm handbook diagrams`.

## Directives

Colon fences in the MyST style group content: `:::{name} argument`, then `:key: value` option
lines, the body, and a closing `:::`. A fence with more colons holds one with fewer. presiterm
knows two names; any other directive shows its body as if the fence were not there.

* `::: notes` (or `:::{notes}`): speaker notes.
* `:::{only} expr`: the body is on the slide only when `expr` holds for the `tags:` of the
  frontmatter. `expr` is tags joined by `and`, `or`, `not` and parentheses (`long and not demo`).
  Any directive takes the same condition as an option: `:only: demo`.

```
:::{only} long
- the slower path, for the long version of the talk
:::
```

An expression that cannot be read is an error on the first slide.

## Layout

Derived from the grid of the window (cols x rows): content width = columns minus a 2-column margin
(at most 80), headings centred, body from column 3. Text that does not fit the rows is cut off at
the spare row and reported ("4 rows too many").

## Errors are visible

What presiterm cannot show (a missing file, a broken diagram) is a red line on the slide and an
error in `presiterm lint`. Nothing is dropped silently and nothing falls back.

## Reload

presiterm polls the deck and every file it points at (300 ms). On a change it reloads and jumps to
the slide of the first changed line; a changed picture or diagram file reloads too, even when the
deck text did not. `--no-watch` turns this off; key `r` and `presiterm ctl reload` still work.
