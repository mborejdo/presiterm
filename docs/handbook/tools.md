# lint, config, doctor, new, config-snippet

## presiterm lint DECK [--font-size PT] [--screen WxH] [--grid COLSxROWS]

Lays out every slide at the chosen size and reports, one line each:
`slide 4 (The chain): error: ...`.

* error: a picture or diagram file that is missing or broken, a slide that does not fit its rows.
* warning: more body rows than leave room to breathe, a line wider than the content (it wraps).
* note: no speaker notes.

Then the biggest font size at which the whole deck fits and the counts. Exit 1 when there is an
error (warnings and notes do not fail), 2 when the deck cannot be read or has no slides.

## presiterm config [DECK]

Prints the config files it found, closest first (`presiterm.toml` from the deck's folder upward, then
`~/.config/presiterm/config.toml`), what it did with each, and the effective font size, screen and
grid, the theme and the logo with their source: a file path or `default`. DECK is a deck file or its folder; default is
the current folder. Exit 0, or 2 when a file has an unknown key or a bad value. The rules are in
`presiterm handbook config`.

## presiterm doctor

Run it in the WezTerm you present in. It sets the font size to 30 pt, then back, and watches the
window size: a window that does not follow means the `user-var-changed` handler is missing, and
doctor prints it. Exit 0 when it follows, 2 when not.

## presiterm config-snippet

Prints the Lua handler for your own `wezterm.lua`. WezTerm config is Lua: comments are `--`.

## presiterm new NAME

Makes `NAME/slides.md` (an example deck with a title, bullets, notes, a stepped mermaid diagram,
code, a table, a diagram from file) and `NAME/assets/flow.mmd`. Fails when `NAME` exists.
