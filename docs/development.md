# Developing presiterm

## The gate

`cargo xtask <task>` runs the checks (the alias is in `.cargo/config.toml`). `cargo xtask qa` is
what must be green before a commit and what CI runs, plus `cargo deny check`.

| task | runs |
|---|---|
| `qa` | `fmt-check`, `lint`, `test`, `doc`, `no-shellout` |
| `fmt` / `fmt-check` | rustfmt over the workspace |
| `lint` | clippy `--all-targets --all-features -D warnings`, and the lean build (`--no-default-features`) |
| `test` | the tests of the full and the lean build |
| `doc` | rustdoc with warnings denied |
| `deny` | `cargo deny check` (needs cargo-deny) |
| `no-shellout` | every `Command::new` in `apps/presiterm/src/` starts `wezterm` and nothing else |
| `demo` | `presiterm lint` on every example deck; `--window` then presents `examples/intro` |

Toolchain: `rust-toolchain.toml` (1.99.1). Lints: `[workspace.lints]` in `Cargo.toml` (clippy
all, pedantic, nursery and cargo at warn, `-D warnings` in CI; no `unwrap`, `expect`, `panic` or
printing outside tests and the binaries; every `#[allow]` has a `reason`). Commit messages are
conventional (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).

## Tests

* Unit tests sit in `#[cfg(test)] mod tests` at the bottom of the file under test, or in a sibling
  `tests.rs` when they share fixtures. Names read `<unit>_<condition>_<expected>`.
* `apps/presiterm/tests/` holds the public-API tests: every deck in `examples/` parses and lays out.
* Pure functions (wrapping, inline markup, tables, `first_changed_line`, `number_nodes`, base64,
  key decoding, cell width, the `only` expressions, colonmark's parser) have proptest properties.
* The slide canvas text and the lint findings are insta snapshots (`snapshots/` next to the test).
  After a deliberate change: `INSTA_UPDATE=always cargo test`, then read the `.snap` diff.
* The byte stream is fed to a `vt100` screen (dev dependency) and must equal the canvas after slide
  changes and steps, after a size change mid-run, and a soft redraw neither rewrites an unchanged
  image nor its rows.
* No test writes to the real file system or the picture cache (`Assets::with_cache(.., None)`,
  `tempfile`) and none opens a WezTerm window. The Typst tests import fletcher from Typst Universe:
  the first run needs the network, later runs use the typst package cache.

## Adding a flag, a command or a dialect feature

1. Document the flag in its `#[arg]` doc comment with an example, and put an example line in the
   command's footer constant in `apps/presiterm/src/cli.rs`.
2. Add the feature to the right handbook topic in `docs/handbook/` (a new topic: add it to
   `TOPICS` in `apps/presiterm/src/handbook.rs`). The tests fail until you do.
3. Add a test for the behaviour, then `cargo xtask qa`.
