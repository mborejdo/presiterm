# The config file: presiterm.toml

Settings you would otherwise type as flags each time. TOML, every key optional.

```toml
font-size = 48          # points
screen    = "1728x1117" # points; what `presiterm lint` checks the deck against
grid      = "58x20"     # the window in cells, columns x rows (`--window`; fixed grid for lint)
logo      = "logo.svg"  # a picture top left on every slide, relative to this file
fonts     = ["fonts"]   # font files or folders Typst draws with, relative to this file

[theme.dark]            # the design tokens; a key left out keeps its default
accent = "#ff8800"

[theme.light]
ink = "#101010"

[theme.fonts]
sans = "Inter"          # text in a film of the deck (flipcard)
mono = "Fira Code"      # typst diagrams, tables, code
display = "Inter"       # headings in a film of the deck, in the family's heaviest weight

[theme.sizes]
text = 13               # points of diagram text
label = 11              # points of edge labels
```

Palette tokens (`[theme.dark]`, `[theme.light]`): `ground`, `ink`, `muted`, `accent`,
`surface`, `border`, `ok`, `warn`, `stop`, each `"#rrggbb"`. `[theme.series]` (`dark`, `light`
colour lists), `[theme.radii]` (`sm`, `md`, `lg`) and `[theme.space]` (`gap`) are read too, so a
web token file fits unchanged. `[theme]` and `fonts` each count as one key below: the closest
file that has one decides all of it.

Built in: Inter (regular, bold, italic), Fira Code (regular, bold). Other families come from
`fonts`: a file as is, a folder by its `.otf`, `.ttf`, `.ttc` files; a missing path is an error.
Typst picks the closest weight, so plain `display = "Inter"` is Inter Bold.

An unknown key (`font-sise`) is an error naming file and key; a malformed value names file,
key and expected form. A missing file is skipped. `font_size`, the spelling up to 0.4, is still
read as `font-size`.

## Closest wins, strongest first

1. A flag (`--font-size`, `--cols`, `--rows`, `--screen`, `--grid`).
2. The deck's frontmatter (`font-size:`).
3. The first `presiterm.toml` from the deck's folder up to the root; farther ones are ignored.
4. The user file `$XDG_CONFIG_HOME/presiterm/config.toml`, else `~/.config/presiterm/config.toml`.
5. The default: 42 pt, a 1728x1117 screen, a 58x20 grid.

Each key is decided on its own: one the closest `presiterm.toml` does not set falls through to
the user file, then the default. So a workshop folder can set `font-size = 54` and the user file
the laptop's `grid`. Frontmatter fields (title, theme, tags) stay in the deck; `theme: light`
picks the light palette of `[theme]`.

## Which file did presiterm use

`presiterm config` prints the files found, closest first, what it did with each, and every
effective value with its source (a path or `default`).

```
presiterm config                     look up from the current folder
presiterm config talk/slides.md      look up from the deck's folder
```

Flags and frontmatter belong to one run and are not shown. The `[default: ...]` in
`presiterm --help` for `--cols`, `--rows` and `--screen` is the built-in one; a config file
changes what is used when the flag is not typed.

## lint and the grid

`presiterm lint` computes the grid from screen and font size, unless `grid` or `--grid` gives
it: then it checks the window you will open.
