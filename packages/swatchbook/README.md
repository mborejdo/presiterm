# swatchbook

Design tokens for terminal and document tools: a dark and a light palette (ground, ink, muted,
accent, surface, border, ok, warn, stop), font families (`sans`, `mono`, `display` for
headings), diagram text sizes, a six-colour chart series per palette, radii (`sm`, `md`, `lg`)
and the gap. Open defaults; a serde overlay changes any single token from TOML (a series list
replaces the whole list):

```toml
[dark]
accent = "#ff8800"

[fonts]
mono = "JetBrains Mono"
display = "Inter Display"

[radii]
md = "6px"
```

## The token file

The defaults come from this crate's `tokens.toml`: `build.rs` reads it into `Palette::DARK`,
`Palette::LIGHT`, `Series::DARK`, `Series::LIGHT` and the other `Default`s, and it loads as a
`Theme` unchanged. Radii and spacing are CSS lengths (`"8px"`), kept as written.

It is a copy of `packages/panelkit/tokens.toml` in [onepage](https://github.com/mborejdo/onepage).
To sync, copy it over and run the tests:

```sh
cp ../onepage/packages/panelkit/tokens.toml packages/swatchbook/tokens.toml
cargo test -p swatchbook
```

With an onepage checkout next to this one, a test fails while the files differ; without it the
test is skipped.

Feature `fonts` (default) carries the faces the defaults name, as bytes: Inter (regular, bold,
italic) and Fira Code (regular, bold), SIL Open Font License 1.1 (`fonts/OFL-*.txt`).

License: code MIT OR Apache-2.0, fonts OFL-1.1.
