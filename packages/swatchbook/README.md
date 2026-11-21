# swatchbook

Design tokens for terminal and document tools: a dark and a light palette (ground, ink, muted,
accent, surface, border, ok, warn, stop), the font families (`sans`, `mono`, `display` for
headings), the diagram text sizes, a six-colour chart series per palette, the corner radii
(`sm`, `md`, `lg`) and the gap. The defaults are open, and a serde overlay lets a TOML table
change any single token (a series list replaces the whole list):

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

The defaults come from `tokens.toml` in this crate: `build.rs` reads it into `Palette::DARK`,
`Palette::LIGHT`, `Series::DARK`, `Series::LIGHT` and the other `Default`s, and the file loads
as a `Theme` unchanged. Radii and spacing are CSS lengths (`"8px"`), kept as written.

The canonical file is `packages/panelkit/tokens.toml` in
[onepage](https://github.com/mborejdo/onepage); this one is a copy. To sync, copy it over and
run the tests:

```sh
cp ../onepage/packages/panelkit/tokens.toml packages/swatchbook/tokens.toml
cargo test -p swatchbook
```

With a checkout of onepage next to this one, a test fails while the two files differ; without
it, the test is skipped.

The `fonts` feature (on by default) carries the faces the defaults name, as bytes: Inter (regular,
bold, italic) and Fira Code (regular, bold), both under the SIL Open Font License 1.1
(`fonts/OFL-*.txt`).

License: the code is MIT OR Apache-2.0; the fonts are OFL-1.1.
