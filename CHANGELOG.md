# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## 0.2.0 - unreleased

A rewrite. presiterm is now a Cargo workspace: the `presiterm` binary and three library crates.

### Added

- Markdown slides in a WezTerm window: real terminal text, pictures and diagrams as OSC 1337
  inline images, a live font size (`+`, `-`, `--font-size`), reload on save that jumps to the
  edited slide, speaker notes, `+` bullets that appear per key press, a control socket
  (`presiterm ctl`), `presiterm lint`, `presiterm new`, `presiterm doctor`, `presiterm config`
  and a built-in manual (`presiterm handbook`, `presiterm tldr`).
- Mermaid diagrams in-process, with `steps` that light one node per key press.
- Typst diagrams with fletcher (feature `typst`, on by default), tables drawn as Typst pictures.
- `presiterm.toml` (closest wins, then `~/.config/presiterm/config.toml`): font size, screen,
  grid, a `[theme]` table over the default design tokens, and a corner `logo`.
- MyST colon-fence directives: `::: notes`, and `:::{only} expr` / `:only: expr` with the
  frontmatter's `tags:`.
- `colonmark`: Markdown with MyST colon-fence directives on comrak, as an owned tree.
- `swatchbook`: design tokens (two palettes, font families, sizes) with a TOML overlay, Inter and
  Fira Code vendored (OFL-1.1).
- `typstkit`: in-process Typst to PNG, SVG and PDF with the vendored fonts and Typst Universe
  packages.

### Removed

- The 2022 version (RON slides through termimad and figlet).

## 0.1.0 - 2022

- First version: RON-described slides rendered with termimad.
