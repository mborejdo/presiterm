# Quickstart

```sh
presiterm new talk                      # talk/slides.md and talk/assets/flow.mmd
presiterm --window talk/slides.md       # a dedicated WezTerm window, 58x20 cells at 42 pt
presiterm lint talk/slides.md           # does every slide fit, is every file there
```

Edit `talk/slides.md` in your editor and save: the window reloads and jumps to the slide of the
first line you changed. `presiterm deck.md` without `--window` presents in the WezTerm you ran it in;
in any other terminal it exits with one line saying so.

Variations:

```sh
presiterm --window talk/slides.md --font-size 54     # bigger text (also keys + - 0, or font-size: 54)
presiterm --window talk/slides.md --slide 7          # start at slide 7
presiterm --window talk/slides.md --reveal           # every bullet is a key press
presiterm --window talk/slides.md --cols 80 --rows 24
presiterm talk/slides.md --no-watch                  # do not reload on save (key r still does)
```

Settings you always want (font size, screen, grid) can live in a `presiterm.toml` next to the deck
instead of in flags; `presiterm config` shows which file applies (`presiterm handbook config`).

Where to look next: `presiterm handbook keys`, `dialect`, `diagrams`, `window`, `config`, `tools`,
`troubleshooting`; `presiterm tldr <words>` searches all of it; `presiterm <command> --help` has examples
and exit codes for each command.

Exit codes: 0 done, 1 `presiterm lint` found errors, 2 presiterm could not do what you asked (one line on
stderr, `presiterm: <what failed>: <why>`).
