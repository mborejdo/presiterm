# Design decisions

For whoever changes the code. The manual is `presiterm handbook` (`docs/handbook/`).

## Decisions

* **Workspace:** `apps/presiterm` (a library, a thin `main.rs`, `cli.rs` for clap and help) and
  four reusable crates: `colonmark`, `deckmark`, `swatchbook`, `typstkit`. The film is
  flipcard's, on deckmark.
* **WezTerm only, OSC 1337 (iTerm2 inline images):** WezTerm's own path, sizes by cell height
  with the aspect kept, plays GIFs. Its kitty graphics are partial, so no kitty.
* **Pure Rust, one program started:** comrak, `mermaid-rs-renderer`, `resvg`, Typst via
  typstkit; an own highlighter for six languages (syntect: about 1.5 MB and two regex engines);
  `rustix` for raw mode and window size; a 300 ms mtime poll, not `notify`.
  `cargo xtask no-shellout` fails on any `Command::new` but `wezterm`.
* **No fallbacks:** a missing image, a broken diagram or another terminal is an error (red line
  plus lint error, or one line and exit), never a degraded mode.
* **Theme:** dark (default) and light, open defaults in `swatchbook`, any token changed by
  `[theme]`. Window text is JetBrains Mono, bundled with WezTerm.
* **Help is part of the product:** every `--help` ends in Examples and Exit codes, the handbook
  is embedded from `docs/handbook/*.md`, and tests in `apps/presiterm/src/cli.rs` fail on a flag
  without an example or a flag or command missing from the handbook. Undocumented is not done.
* **Exit codes:** 0 done, 1 lint errors, 2 could not do it (`presiterm: <what failed>: <why>`
  on stderr).

## The dialect on CommonMark

CommonMark with GitHub tables, but:

* A `---` line is always a slide break: setext headings are off.
* Each source line of a paragraph is its own line on the slide.
* `- a` then `+ b` is one list (CommonMark would start a new one); `+` appears on a key press.
* `__text__` is underlined, not bold; so is `==text==`. A spaced `//` is a pause, not text.
* A picture alone on its line takes `{h=7,w=20}` after it.

## Supported

* The highlighter colours a string on the line it starts on.
* Mermaid: the kinds mermaid-rs-renderer draws (`presiterm handbook diagrams` lists them);
  `steps=a,b,c` fixes the lighting order.
* `+`, `-` and the font size at start share the code path of `presiterm ctl font`.
