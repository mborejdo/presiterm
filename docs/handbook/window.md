# The window and the font size

WezTerm only: images use its OSC 1337 inline-image path (GIFs play). Elsewhere presiterm exits
with one line, no degraded mode.

`presiterm --window deck.md` starts `wezterm --config-file <generated> start --always-new-process`:
no tab bar, JetBrains Mono (bundled), no ligatures, theme background, padding, no close prompt,
58x20 cells at 42 pt. `--cols`, `--rows`, `--font-size` change that, so do `grid` and `font_size`
in `presiterm.toml`. A bigger font means fewer cells, not a bigger window.

Odd setups: `--control PATH` moves the control socket (default `<tmp>/presiterm-<pid>.sock`,
`presiterm ctl` finds the newest); `--cell-px 21x42` sets the cell size for a terminal that does
not report it.

## Font size

Live: `--font-size PT`, `font-size:` in the frontmatter (the flag wins), keys `+ - 0`,
`presiterm ctl font ...`. Each sets the WezTerm user variable `presiterm_font`; a
`user-var-changed` handler applies it, the grid changes, presiterm lays out the slide again
(diagrams at the new pixel size), same slide and step. The `--window` config has the handler; for
your own `wezterm.lua` add what `presiterm config-snippet` prints (before `return config`) and
check with `presiterm doctor`. Without it `+` and `-` show a footer hint.

## Fitting a deck

`presiterm lint deck.md --font-size 54` checks every slide at that size on a
`--screen 1728x1117` (points) and prints the biggest size the deck fits at. `--grid 80x24` checks
a fixed grid.
