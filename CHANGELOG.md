# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## Unreleased

### Added

- typstkit `Engine::query`: every `metadata("...") <label>` of a compiled document with its page
  and position, for a caller that needs to know where things landed.

## 0.3.0 - 2026-11-21

### Changed

- deckmark keeps the marks apart: `Span::highlight` (`==x==`), `Span::underline` (`__x__` only)
  and `Span::strike` (`~~x~~`). The terminal draws `==` and `__` underlined, as before.
- typstkit compiles side by side on one `Engine` and keeps Typst's cache until
  `Engine::evict(max_age)`; deckmark evicts at 10 after a diagram, as the typst CLI does.

### Added

- `//` with spaces around it is a pause: gone from the text, `Span::pause` on what follows.
  A URL or code keeps its `//`.

### Removed

- The film (`presiterm film`, `presiterm lint --film`, `[film]`, `packages/filmstrip`,
  `examples/film`) moved to [flipcard](https://github.com/mborejdo/flipcard), which reads the
  same decks. `presiterm film` is a hidden stub that says so, for one release.

## 0.2.0 - 2026-11-21

A rewrite, now a Cargo workspace: the `presiterm` binary and five library crates.

### Added

- Slides in a WezTerm window: terminal text, OSC 1337 pictures and diagrams, live font size
  (`+`, `-`, `--font-size`), reload on save at the edited slide, speaker notes, `+` bullets, a
  control socket (`presiterm ctl`), `lint`, `new`, `doctor`, `config`, a built-in manual
  (`handbook`, `tldr`).
- Mermaid diagrams in-process, `steps` lights one node per key press.
- Typst diagrams with fletcher (feature `typst`, default on); tables drawn as Typst pictures.
- `presiterm.toml` (closest wins, then `~/.config/presiterm/config.toml`): font size, screen,
  grid, a `[theme]` table over the default tokens, a corner `logo`.
- `fonts = [...]`: extra font files and folders for Typst; `[theme.fonts] display` for film
  headings.
- MyST colon fences: `:::{notes}`, `:::{only} expr` / `:only: expr` on the frontmatter's `tags:`.
  A fence without `{name}` is no directive, its body shows.
- `:::{metrics}`: `value | label` lines as big numbers in a row.
- `presiterm film`: a looping GIF, an MP4 (pure-Rust H.264), an A4 landscape DOCX playing the
  GIF, a PDF playing the MP4 on open. Each key press is a build-in (bullets appear, marks
  underline, metrics count up, titles land word by word, `steps` light up). `lint --film`,
  `[film]` and the same frontmatter keys, `links:`.
- Film `pace` (words per minute: a scene with more words than `hold` covers stays its reading
  time; per-scene hold in `filmstrip::Scene`) and `transition` (`fade`, `wipe`, `push`,
  `pixelize`, `circle`, `split`, `cut`; flag, frontmatter or `[film]`; `filmstrip::Transition`).
- Crates: `colonmark` (Markdown with MyST colon fences on comrak, as an owned tree), `deckmark`
  (the dialect: slides of blocks, diagrams and pictures in the deck's tokens, the Typst faces),
  `swatchbook` (tokens: two palettes, font families, sizes, a TOML overlay; Inter and Fira Code
  vendored, OFL-1.1), `typstkit` (in-process Typst to PNG, SVG, PDF with the vendored fonts and
  Typst Universe packages), `filmstrip` (frames to GIF with delta frames and a learned palette,
  MP4, DOCX, PDF).
- `typstkit::Engine::raster` (pixels, string and byte inputs) and `Input::Bytes`.

### Removed

- The 2022 version (RON slides through termimad and figlet).

## 0.1.0 - 2022

- First version: RON slides rendered with termimad.
