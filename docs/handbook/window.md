# The window and the font size

presiterm runs in WezTerm only: images use its OSC 1337 inline-image path (which also plays GIFs). In
any other terminal it exits with one line and does not degrade.

`presiterm --window deck.md` starts `wezterm --config-file <generated> start --always-new-process`:
no tab bar, JetBrains Mono (bundled with WezTerm), no ligatures, the theme's colours as background,
padding, no close prompt, 58x20 cells at 42 pt. `--cols`, `--rows` and `--font-size` change that,
and so do `grid` and `font_size` in a `presiterm.toml` (`presiterm handbook config`). A
bigger font means fewer cells, not a bigger window.

Two flags for odd setups: `--control PATH` puts the control socket where you want it (default
`<tmp>/presiterm-<pid>.sock`; `presiterm ctl` finds the newest one), and `--cell-px 21x42` gives the cell
size in pixels for a terminal that does not report it.

## Font size

First-class and live: `--font-size PT`, `font-size:` in the frontmatter (the flag wins), keys
`+ - 0`, `presiterm ctl font ...`. All of them send the WezTerm user variable `presiterm_font`; a
`user-var-changed` handler in the WezTerm config applies it, the window's grid changes, and presiterm
lays the slide out again (diagrams are drawn again at the new pixel size), keeping slide and step.
The generated window config has the handler. In your own `wezterm.lua` add what
`presiterm config-snippet` prints (before `return config`) and check with `presiterm doctor`.

Without the handler `+` and `-` show a footer hint; they never silently do nothing.

## Fitting a deck

`presiterm lint deck.md --font-size 54` checks every slide at that size on a screen of
`--screen 1728x1117` points and prints the biggest font size at which the whole deck fits. Use
`--grid 80x24` to check on a fixed grid.
