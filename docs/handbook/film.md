# presiterm film: the deck as GIF, MP4, DOCX and PDF

```sh
presiterm film talk/slides.md                      # talk/slides.gif
presiterm film talk/slides.md --mp4 --docx --pdf   # the MP4, a Word file, a PDF
presiterm film talk/slides.md --out dist           # into dist/ instead of next to the deck
presiterm film talk/slides.md --cards dist/cards   # every card, fully built, as PNG
presiterm film talk/slides.md --preview            # play the GIF inline in this WezTerm
presiterm lint talk/slides.md --film               # which cards do not fit
```

The same deck that presents in the window renders as a film: every slide is a 16:9 card in the
deck's tokens (the `[theme]` table, `theme: light`), drawn by Typst in process, no browser.
Every key press of the talk is a build-in step of the film:

| in the window | in the film |
|:--|:--|
| a key press (a `+` bullet, a lit `steps` node) | one scene: `enter` seconds of build-in, then `hold` seconds of rest |
| `-` bullets shown at once | appear one after the other during the build-in |
| `==marked==` and `__marked__` | an underline in the accent colour that grows from the left |
| the first slide, or a heading alone | the heading lands word by word |
| a `steps` diagram, mermaid or Typst | the node of the step lights up, as in the window |
| `:::{metrics}` | the numbers count up from zero |
| a picture, a table, a code block | drawn as in the window, in the card |
| `::: notes` | never in the film |
| `{only}` blocks | follow the frontmatter's `tags:` |

## Files

* `slides.gif`: one palette learned from the stills, only the pixels that change are stored, so
  a film of text cards is small (a 14-card film at 640 px is about 1 MB). Loops.
* `slides.mp4` (`--mp4`): H.264, pure Rust. Same frames.
* `slides.docx` (`--docx`): an A4 landscape Word file whose only content is the GIF and a line
  of links under it. Word for Mac and current Word for Windows play the GIF in place.
* `slides.pdf` (`--pdf`): the first card as a poster on an A4 landscape page with the title line
  and the links; Acrobat plays the MP4 over it when the page opens and loops it. Other viewers
  show the poster.
* `--cards DIR`: every scene's fully built card as `card03-007.png` (slide 3, scene 7), to look
  at the cards without opening anything.

## Timing and size

Six values, each a flag, a frontmatter key or a `[film]` key in `presiterm.toml`; a flag beats
the frontmatter, which beats the file:

| value | flag | default | what |
|:--|:--|--:|:--|
| fps | `--fps` | 10 | frames per second |
| width | `--width` | 640 | pixels; the height follows 16:9; even numbers only (the MP4 needs them) |
| hold | `--hold` | 7 | seconds every scene stays, build-in included |
| enter | `--enter` | 1.6 | seconds of build-in at the start of every scene |
| fade | `--fade` | 0.4 | seconds of crossfade between scenes |
| colours | `--colours` | 48 | colours of the GIF palette, 2 to 255 |

```markdown
---
title: My talk
links: https://example.org/talk, mailto:me@example.org
hold: 5
enter: 1.2
---
```

```toml
[film]
width = 960
fps = 12
```

`links:` are comma-separated URLs; the Word and PDF editions show them under the picture, the
scheme (`https://`, `mailto:`) stripped from the label.

## Fitting

A card that does not fit is never cropped: `presiterm film` stops with
`slide 4 (The chain): the card overflows onto 2 pages; trim it`, and `presiterm lint --film`
reports every such card as an error. Fewer words, a smaller picture (`{h=5}`), or a second
slide.

## Fonts

The cards use the theme's `sans` for text and `mono` for code and the footer (`[theme.fonts]`
in presiterm.toml). Inter and Fira Code are built in and the only faces the film draws with.
