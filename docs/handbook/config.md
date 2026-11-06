# The config file: presiterm.toml

presiterm has a few settings you may want to fix once instead of typing a flag each time. They go
in a TOML file; every key is optional.

```toml
font_size = 48          # points
screen    = "1728x1117" # points; what `presiterm lint` checks the deck against
grid      = "58x20"     # the window in cells, columns x rows (`--window`; fixed grid for lint)
logo      = "logo.svg"  # a picture in the top left corner of every slide, relative to this file
fonts     = ["fonts"]   # font files or folders Typst draws with, relative to this file

[theme.dark]            # the design tokens; any key left out keeps its default
accent = "#ff8800"

[theme.light]
ink = "#101010"

[theme.fonts]
sans = "Inter"          # text on the film's cards
mono = "Fira Code"      # typst diagrams, tables, code
display = "Inter"       # headings on the film's cards, in the heaviest weight the family has

[theme.sizes]
text = 13               # points of diagram text
label = 11              # points of edge labels

[film]                  # presiterm film (presiterm handbook film); every key optional
width = 640
fps = 10
hold = 7
enter = 1.6
fade = 0.4
pace = 200
colours = 48
```

The palette tokens of `[theme.dark]` and `[theme.light]` are `ground`, `ink`, `muted`, `accent`,
`surface`, `border`, `ok`, `warn` and `stop`, each a `"#rrggbb"` colour. The whole `[theme]` table
is one key for the rule below: the closest file that has one decides all of it; so are `[film]`
and `fonts`.

Inter (regular, bold, italic) and Fira Code (regular, bold) are built in; a family `[theme.fonts]`
names beyond those must come from `fonts`: a file is loaded as it is, a folder by its `.otf`,
`.ttf` and `.ttc` files. A path that does not exist is an error. Typst picks the closest weight a
family has, so `display = "Inter"` without extra files draws headings in Inter Bold.

An unknown key is an error that names the file and the key (`font_sise` does not silently fall
back). A value in the wrong form is an error that names the file, the key and the form it expects.
A file that does not exist is skipped.

## Closest wins

presiterm looks at these levels, strongest first:

1. A command-line flag (`--font-size`, `--cols`, `--rows`, `--screen`, `--grid`).
2. The deck's frontmatter (`font-size:`).
3. The closest `presiterm.toml`: presiterm starts in the deck's folder and walks up to the filesystem root;
   the first `presiterm.toml` it finds is the one. Farther ones are ignored.
4. The user file `$XDG_CONFIG_HOME/presiterm/config.toml`, else `~/.config/presiterm/config.toml`.
5. The built-in default: 42 pt, a 1728x1117 screen, a 58x20 grid.

Files are not merged as wholes. Each key is decided on its own: a key the closest `presiterm.toml`
does not set falls through to the user file, and from there to the default. So a workshop folder can
hold `presiterm.toml` with `font_size = 54` and the user file can set the `grid` for the laptop.

Only the keys of the settings above exist; the frontmatter's other fields (title, theme, tags)
stay in the deck. The frontmatter's `theme: light` picks the light palette of the `[theme]` table.

## Which file did presiterm use

`presiterm config` prints the files it found, closest first, with what it did with each, and the
effective value of every setting with its source: the file path, or `default`.

```
presiterm config                     look up from the current folder
presiterm config talk/slides.md      look up from the deck's folder
```

Flags and the frontmatter are not shown there: they belong to one run. The `[default: ...]` that
`presiterm --help` prints for `--cols`, `--rows` and `--screen` is the built-in default; a config file
changes what is used when the flag is not typed.

## lint and the grid

`presiterm lint` computes the grid from the screen and the font size. When a config file sets `grid`,
or `--grid` is typed, lint checks on exactly that grid instead, so it checks the window you will
open.
