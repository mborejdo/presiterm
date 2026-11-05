# swatchbook

Design tokens for terminal and document tools: a dark and a light palette (ground, ink, muted,
accent, surface, border, ok, warn, stop), the font families and the diagram text sizes, with open
defaults and a serde overlay so a TOML table can change any single token:

```toml
[dark]
accent = "#ff8800"

[fonts]
mono = "JetBrains Mono"
```

The `fonts` feature (on by default) carries the faces the defaults name, as bytes: Inter (regular,
bold, italic) and Fira Code (regular, bold), both under the SIL Open Font License 1.1
(`fonts/OFL-*.txt`).

License: the code is MIT OR Apache-2.0; the fonts are OFL-1.1.
