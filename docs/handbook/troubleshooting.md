# Troubleshooting

| you see | it means | do |
|---|---|---|
| `presiterm runs in WezTerm only` | not started from WezTerm | `presiterm --window deck.md` |
| `+` / `-` show a footer hint | this WezTerm has no `presiterm_font` handler | `presiterm doctor`, add `presiterm config-snippet` to wezterm.lua |
| a red line on a slide | a missing file or a diagram that does not parse | fix the deck; `presiterm lint` lists them all |
| `N rows too many` | the slide does not fit the grid at this font | fewer lines, a smaller `--font-size`; `presiterm lint` gives the biggest that fits |
| a blocky picture | fewer pixels than the cells it fills | a bigger source or a smaller `h=` |
| `no running presenter found` | `presiterm ctl` finds no live socket | start the deck, or pass `--control PATH` |
| a typst fence is an error | built with `--no-default-features` | build with the default `typst` feature |
| `failed to download package` in a typst diagram | fletcher comes from Typst Universe on first use | go online once, it is cached after |
| WezTerm shows a configuration error | a `//` comment in a Lua config | Lua comments are `--` |
| the window does not reload | started with `--no-watch` | key `r`, or `presiterm ctl reload` |

Failures print one line to stderr, `presiterm: <what failed>: <why>`, and exit 2.
