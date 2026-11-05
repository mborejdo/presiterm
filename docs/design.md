# Design decisions

Why presiterm looks the way it does. The user manual is `presiterm handbook` (sources in
`docs/handbook/`); this page is for whoever changes the code.

## Decisions

* **Workspace:** the binary in `apps/presiterm` (a library plus a thin `main.rs` for dispatch and
  `cli.rs` for the clap CLI and its help texts) and three library crates meant to be reused:
  `colonmark` (Markdown and MyST colon-fence directives), `swatchbook` (design tokens and the
  vendored fonts) and `typstkit` (in-process Typst).
* **WezTerm only, OSC 1337 (iTerm2 inline image protocol):** WezTerm's native, robust image path;
  it sizes an image by cell height with the aspect kept, and plays animated GIFs through it. Its
  kitty graphics support is partial, so kitty is not used. Not WezTerm: exit with one line, no
  fallback.
* **Pure Rust, one program started:** Markdown by comrak (through colonmark), mermaid in-process
  (`mermaid-rs-renderer`, coloured from the tokens), SVG to PNG with `resvg`, Typst and fletcher
  through typstkit; an own token highlighter (bash, yaml, toml, json, rust, md; syntect would add
  about 1.5 MB and two regex engines for six languages); `rustix` for raw mode and window size; a
  300 ms mtime poll instead of `notify`. `cargo xtask no-shellout` fails if any `Command::new`
  other than `wezterm` appears in the app.
* **No fallbacks:** a missing image, a broken diagram or a non-WezTerm terminal is an error (a red
  line on the slide plus a lint error), never a degraded mode.
* **Theme:** two palettes (dark, the default, and light) with open defaults in `swatchbook`;
  `theme: light` in the frontmatter picks one, a `[theme]` table in `presiterm.toml` changes any
  token. The window text uses JetBrains Mono, which WezTerm bundles.
* **Help is part of the product:** every command's `--help` ends in Examples and Exit codes, the
  handbook is embedded from `docs/handbook/*.md`, and tests in `apps/presiterm/src/cli.rs` fail when
  a flag has no example or a flag or command is missing from the handbook. Adding a flag therefore
  means documenting it.
* **Exit codes:** 0 done, 1 `presiterm lint` found errors, 2 presiterm could not do what was asked
  (the reason is one line on stderr: `presiterm: <what failed>: <why>`).

## The dialect on CommonMark

The slide dialect sits on CommonMark with GitHub tables. Where it differs:

* A `---` line is always a slide break: setext headings are off.
* Every source line of a paragraph is its own line on the slide.
* `- a` followed by `+ b` is one list on the slide (CommonMark starts a new list at a new marker);
  `+` marks a bullet that appears on a key press.
* `__text__` is underlined, not bold; `==text==` is underlined too.
* A picture alone on its line takes `{h=7,w=20}` after it.

## Not done

* An export (pptx, pdf, html). No window screenshot command.
* D2: there is no in-process D2 engine in Rust; a d2 fence is an error that names the alternatives.
* The highlighter colours a string spanning lines on its first line only.
* Mermaid: `~~~` may reorder nodes and `sequenceDiagram` is not supported by the renderer.
* The `+` and `-` keys and the first-start font application were exercised through the same code
  path as `presiterm ctl font`, not by typing in a window.
