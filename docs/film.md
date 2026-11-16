# The film: a deck as GIF, MP4, DOCX and PDF

Design note, 06.11.2026, built the same day (`presiterm film`, `crates/filmstrip`,
`presiterm handbook film`). `presiterm film` renders a deck to files that play on their own: a
looping GIF for a README or a chat, an MP4 for a post, a Word file for a mail attachment, a PDF
that starts the film when the page opens in Acrobat. Every key press of the talk becomes a
build-in of the film.

## Why in presiterm

presiterm already owns everything the film needs except the encoders:

| need | presiterm has |
|---|---|
| slides from Markdown with steps | `deck`: `+` bullets (`reveal`), `steps` diagrams, `{only}` tags |
| design tokens and fonts | `swatchbook`: palettes, Inter, Fira Code |
| drawing without a browser | `typstkit`: in-process Typst to PNG |
| lint before export | `presiterm lint` |

One deck format, two outputs: the live terminal presentation and a rendered film of the same
file. Fits the design decisions: pure Rust, no program started at all for the export, no
fallbacks (a card that overflows is a lint error, not a cropped frame), help with examples, the
same tokens.

## Shape

### Crate `filmstrip` (`crates/filmstrip`)

Frames in, files out. Knows nothing about Markdown or Typst.

- `Frame { width, height, rgb }`, `Frame::png()`.
- `Timing { fps, fade, transition, ticks }`, `Scene { frames, hold }` and `sequence(scenes,
  timing) -> Vec<Frame>`: every scene's build-in frames, its own hold on the last one, a
  `Transition` into the next scene (fade, wipe, push, pixelize, circle, split, cut: CPU-drawn,
  readable at 10 fps).
- `gif::encode(frames, keyframes, fps, colours)`: one NeuQuant palette learned from the keyframes
  plus a transparent slot, delta frames (the changed rectangle only, unchanged pixels
  transparent), long holds split into legal delays. This is what keeps a 14-card film near 1 MB.
- `mp4::encode(frames, fps)`: H.264 through `rusty_h264-encoder`, boxed by `mp4`, no B-frames.
- `docx::wrap(poster, gif, links)`: an A4 landscape Word file with the GIF as its only picture and
  a line of links under it (docx-rs, then a zip pass that swaps the PNG part for the GIF). Word
  plays the GIF in place.
- `pdf::embed(page, mp4, name)`: a Screen annotation with a Rendition action over the poster,
  fired on page open; Acrobat plays and loops, other viewers show the poster (lopdf).

Features: `gif` (default), `mp4`, `docx`, `pdf`; each pulls only its encoder. Tests: the GIF's
frame count and delays, the MP4's sample table, the DOCX parts and relationships, the PDF's
annotation, all with the crate's own readers or a few lines of box parsing.

### `typstkit::Engine::raster`

`png()` compiles without inputs and encodes. The film needs the pixels of a compile with
`sys.inputs` (slide, step, frame), so: one new method returning width, height and RGB bytes;
`png()` becomes a wrapper. Pictures reach Typst by path under the engine's root, as they do for
diagrams today.

### `presiterm film` (feature `film`, needs `typst`)

    presiterm film talk/slides.md                 talk/slides.gif
    presiterm film talk/slides.md --mp4 --docx --pdf
    presiterm film talk/slides.md --cards out/    every card and build-in sample as PNG
    presiterm film talk/slides.md --preview       play the GIF inline in this WezTerm

A Typst layout embedded in the binary draws one slide at one build-in instant. Inputs: the slide
as JSON (`deck::Slide` serialised), `index`/`total`, `step`/`steps`, `frame`/`frames`, the
tokens as colours, the width. The look mirrors the terminal: heading in accent with its rule,
body from the left margin, footer with the title and `n / N`, the corner logo; the deck's
`theme:` picks the palette and the `[theme]` overlay applies.

How the deck's semantics become motion:

| in the terminal | in the film |
|---|---|
| a key press (a `+` bullet, a lit `steps` node) | one build-in step: `enter` seconds of animation, then `hold` |
| `- ` bullets shown at once | appear one by one along the ease curve of the step |
| `==marked==` / `__marked__` | the underline grows from left to right |
| a title slide | the heading lands word by word |
| a mermaid diagram with `steps` | drawn per step by the existing renderer, placed as a picture |
| a Typst diagram with `steps` | the same `stepnode` preamble with `step` set per frame |
| a table | the existing Typst table, rows appearing |
| `:::{notes}` | never in the film |
| `{only}` tags | the same `--tags` as the window |

Timing lives in `presiterm.toml` (`[film] fps = 10, width = 640, hold = 7, enter = 1.6,
fade = 0.4, transition = "fade", pace = 200, colours = 48`) and in the frontmatter with the same keys; a flag beats
both, as everywhere. `hold` is the least a scene stays; a scene with more words than that covers
at `pace` words per minute stays for its reading time. `presiterm lint --film` checks every slide at the film's geometry (a card that
overflows is an error, with the rows that do not fit).

Two directives join the dialect because talks want them and the film gives them motion:
`:::{metrics}` with `value | label` lines (big numbers in a row; the window draws them large,
the film counts them up) and `:::{columns}` for a picture beside text. Both are plain blocks in
the window and need no new rendering idea there.

## What was built

`filmstrip` with tests per container; `typstkit::Engine::raster` and `Input::Bytes`; the card
layout (`apps/presiterm/src/film/film.typ`) and the PDF page; `presiterm film` with
`--gif --mp4 --docx --pdf --cards --out --preview` and the eight timing flags; `presiterm lint
--film`; the `[film]` table and the frontmatter keys; `links:`; the `metrics` directive in the
dialect, the window and the film; `examples/film`, rendered by `cargo xtask demo`.

Left out on purpose: a `columns` directive (a picture above its text does the job), per-element
timing in the Markdown, audio, a transition per slide, GPU transitions, any browser.
