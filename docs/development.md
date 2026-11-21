# Developing presiterm

## The gate

`cargo xtask <task>` (alias in `.cargo/config.toml`). `qa` is green before every commit; CI runs
it, then `cargo deny check` and `cargo xtask demo`.

| task | runs |
|---|---|
| `qa` | `fmt-check`, `lint`, `test`, `doc`, `no-shellout` |
| `fmt` / `fmt-check` | rustfmt over the workspace |
| `lint` | clippy `--all-targets --all-features -D warnings`, and the lean build (`--no-default-features`) |
| `test` | the tests of the full and the lean build |
| `doc` | rustdoc, warnings denied |
| `deny` | `cargo deny check` (needs cargo-deny) |
| `no-shellout` | every `Command::new` in `apps/presiterm/src/` starts `wezterm`, nothing else |
| `demo` | `presiterm lint` on every example deck, then presents `examples/intro` with `--window` |

Toolchain 1.99.1 (`rust-toolchain.toml`). `[workspace.lints]`: clippy all, pedantic, nursery,
cargo at warn, `-D warnings` in CI; no `unwrap`, `expect`, `panic` or printing outside tests and
binaries; every `#[allow]` has a `reason`. Conventional commits (`feat:`, `fix:`, `refactor:`,
`docs:`, `test:`, `chore:`).

## Tests

* Unit tests at the bottom of the file (`#[cfg(test)] mod tests`), or a sibling `tests.rs` for
  shared fixtures. Names: `<unit>_<condition>_<expected>`.
* `apps/presiterm/tests/`: public-API tests, every deck in `examples/` parses and lays out.
* Pure functions (wrapping, inline markup, tables, `first_changed_line`, `number_nodes`, base64,
  key decoding, cell width, `only` expressions, colonmark's parser) have proptest properties.
* Canvas text and lint findings are insta snapshots (`snapshots/`). After a deliberate change:
  `INSTA_UPDATE=always cargo test`, then read the `.snap` diff.
* The byte stream, fed to a `vt100` screen, must equal the canvas after slide changes, steps and
  a size change mid-run; a soft redraw rewrites no unchanged image or row.
* No test writes to the real file system or picture cache (`Assets::with_cache(.., None)`,
  `tempfile`) or opens WezTerm. The Typst tests fetch fletcher: the first run needs the network.

## Adding a flag, a command or a dialect feature

1. An example in the flag's `#[arg]` doc comment, and an example line in the command's footer in
   `apps/presiterm/src/cli.rs`.
2. The right topic in `docs/handbook/` (a new topic goes into `TOPICS` in
   `apps/presiterm/src/handbook.rs`). Tests fail until both are done.
3. A test for the behaviour, then `cargo xtask qa`.
