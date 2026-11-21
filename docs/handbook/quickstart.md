# Quickstart

```sh
presiterm new talk                      # talk/slides.md and talk/assets/flow.mmd
presiterm --window talk/slides.md       # own WezTerm window, 58x20 cells at 42 pt
presiterm lint talk/slides.md           # does every slide fit, is every file there
```

Save `talk/slides.md` and the window reloads at the slide of the first line you changed.
Without `--window` it presents in the current WezTerm; other terminals get one line and exit.

```sh
presiterm --window talk/slides.md --font-size 54     # bigger text (also keys + - 0, or font-size: 54)
presiterm --window talk/slides.md --slide 7          # start at slide 7
presiterm --window talk/slides.md --reveal           # every bullet is a key press
presiterm --window talk/slides.md --cols 80 --rows 24
presiterm talk/slides.md --no-watch                  # no reload on save (key r still does)
```

Font size, screen and grid can go in a `presiterm.toml` next to the deck; `presiterm config`
shows which applies.

Next: `presiterm handbook keys`, `dialect`, `diagrams`, `window`, `config`, `tools`,
`troubleshooting`. `presiterm tldr <words>` searches them, `presiterm <command> --help` has
examples and exit codes.

Exit codes: 0 done, 1 `presiterm lint` found errors, 2 presiterm could not do what you asked (one
line on stderr, `presiterm: <what failed>: <why>`).
