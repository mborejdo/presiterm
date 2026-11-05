# Keys and presiterm ctl

| key | what |
|---|---|
| right, down, space, l, j, enter, page down | next step (a revealed bullet or a lit diagram node), then next slide |
| left, up, h, k, backspace, page up | previous step, then previous slide (shown fully revealed) |
| g, home / G, end | first / last slide |
| 12 then enter, or :12 then enter | go to slide 12 (esc cancels) |
| n | speaker notes of this slide (again to return) |
| r | reload now (the deck also reloads when it or a file it points at changes) |
| + / - / 0 | font size up by 2 pt / down by 2 pt / back to the deck's size |
| q, esc, ctrl-c | quit |

## presiterm ctl

`presiterm ctl <command>` drives a running presentation over its unix socket, from another terminal
(or a script, or a hotkey). It picks the newest running presenter; `--control PATH` picks one.

| command | what |
|---|---|
| goto N | go to slide N (1-based) |
| next, prev | one step forward or back |
| reload | read the deck again now |
| status | print slide, step and font size |
| font PT, font +N, font -N, font reset | set the font size, change it, or back to the deck's |
| quit | close the presentation |

```sh
presiterm ctl goto 5
presiterm ctl font +4
presiterm ctl --control /tmp/talk.sock status
```

Exit code 0 when the presenter answered, 2 when none runs or it refused (slide out of range,
unknown command).
