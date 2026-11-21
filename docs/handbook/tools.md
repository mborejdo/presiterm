# lint, config, doctor, new, config-snippet

## presiterm lint DECK [--font-size PT] [--screen WxH] [--grid COLSxROWS]

Lays out every slide at that size and reports one line each: `slide 4 (The chain): error: ...`.

* error: a picture or diagram file missing or broken, a slide that does not fit its rows.
* warning: more body rows than leave room to breathe, a line wider than the content (it wraps).
* note: no speaker notes.

Then the biggest font size at which the whole deck fits, and the counts. Exit 1 on an error
(warnings and notes do not fail), 2 when the deck cannot be read or has no slides.

## presiterm config [DECK]

Prints the config files, closest first (`presiterm.toml` from the deck's folder up, then
`~/.config/presiterm/config.toml`), what it did with each, and font size, screen, grid, theme and
logo with their source (a path or `default`). DECK is a deck or its folder, default the current
folder. Exit 0, or 2 on an unknown key or bad value. Rules: `presiterm handbook config`.

## presiterm doctor

Run it in the WezTerm you present in. It sets 30 pt and back and watches the window size; if it
does not follow, the `user-var-changed` handler is missing and doctor prints it. Exit 0 when it
follows, 2 when not.

## presiterm config-snippet

Prints the Lua handler for your own `wezterm.lua`. Lua comments are `--`.

## presiterm new NAME

Makes `NAME/slides.md` (an example deck: title, bullets, notes, a stepped mermaid diagram, code,
a table, a diagram from file) and `NAME/assets/flow.mmd`. Fails when `NAME` exists.
