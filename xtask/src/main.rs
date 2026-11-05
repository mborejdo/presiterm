//! presiterm's task runner: `cargo xtask <task>`.
//!
//! Each task is a list of cargo calls run from the workspace root; the first that fails stops the
//! run with its exit code. `qa` is the gate CI runs.

// A task runner reports what it runs.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a task runner reports what it runs"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

/// The tasks.
#[derive(Parser)]
#[command(name = "xtask", about = "presiterm's tasks: cargo xtask <task>")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand, Clone, Copy)]
enum Task {
    /// Format the workspace.
    Fmt,
    /// Check the formatting without writing.
    FmtCheck,
    /// Clippy with warnings as errors, the full and the lean (no Typst) build.
    Lint,
    /// The tests of the full and the lean build.
    Test,
    /// Rustdoc with warnings as errors.
    Doc,
    /// cargo deny: advisories, licences, bans, sources (needs cargo-deny).
    Deny,
    /// The only program presiterm starts is `wezterm`.
    NoShellout,
    /// Lint every example deck; with --window, present examples/intro.
    Demo {
        /// Open examples/intro/slides.md in a WezTerm window afterwards.
        #[arg(long)]
        window: bool,
    },
    /// Everything CI runs: fmt-check, lint, test, doc, no-shellout.
    Qa,
}

/// The workspace root: the folder above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Run `cargo` with `args` in the workspace root, with extra environment `env`.
fn cargo(args: &[&str], env: &[(&str, &str)]) -> Result<()> {
    println!("$ cargo {}", args.join(" "));
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(args)
        .envs(env.iter().copied())
        .current_dir(root())
        .status()
        .with_context(|| format!("start cargo {}", args.join(" ")))?;
    if !status.success() {
        bail!("cargo {} failed ({status})", args.join(" "));
    }
    Ok(())
}

/// Every `Command::new` in the app's sources that does not start `wezterm`.
fn shellouts() -> Result<Vec<String>> {
    let mut found = Vec::new();
    let mut folders = vec![root().join("apps/presiterm/src")];
    while let Some(folder) = folders.pop() {
        for entry in
            std::fs::read_dir(&folder).with_context(|| format!("read {}", folder.display()))?
        {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path)?;
                found.extend(
                    text.lines()
                        .enumerate()
                        .filter(|(_, l)| {
                            l.contains("Command::new(") && !l.contains("Command::new(\"wezterm\")")
                        })
                        .map(|(n, l)| format!("{}:{}: {}", path.display(), n + 1, l.trim())),
                );
            }
        }
    }
    Ok(found)
}

fn run(task: Task) -> Result<()> {
    match task {
        Task::Fmt => cargo(&["fmt", "--all"], &[]),
        Task::FmtCheck => cargo(&["fmt", "--all", "--check"], &[]),
        Task::Lint => {
            cargo(
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                    "--",
                    "-D",
                    "warnings",
                ],
                &[],
            )?;
            cargo(
                &[
                    "clippy",
                    "-p",
                    "presiterm",
                    "--all-targets",
                    "--no-default-features",
                    "--",
                    "-D",
                    "warnings",
                ],
                &[],
            )
        },
        Task::Test => {
            cargo(&["test", "--workspace", "--all-features"], &[])?;
            cargo(&["test", "-p", "presiterm", "--no-default-features"], &[])
        },
        Task::Doc => cargo(
            &["doc", "--workspace", "--no-deps", "--all-features"],
            &[("RUSTDOCFLAGS", "-D warnings")],
        ),
        Task::Deny => cargo(&["deny", "check"], &[]),
        Task::NoShellout => {
            let found = shellouts()?;
            if !found.is_empty() {
                bail!(
                    "a process other than wezterm is started:\n{}",
                    found.join("\n")
                );
            }
            println!("no-shellout: ok (only wezterm)");
            Ok(())
        },
        Task::Demo { window } => {
            for deck in ["intro", "diagrams", "media"] {
                cargo(
                    &[
                        "run",
                        "-q",
                        "-p",
                        "presiterm",
                        "--",
                        "lint",
                        &format!("examples/{deck}/slides.md"),
                    ],
                    &[],
                )?;
            }
            if window {
                cargo(
                    &[
                        "run",
                        "-q",
                        "-p",
                        "presiterm",
                        "--",
                        "--window",
                        "examples/intro/slides.md",
                    ],
                    &[],
                )?;
            }
            Ok(())
        },
        Task::Qa => [
            Task::FmtCheck,
            Task::Lint,
            Task::Test,
            Task::Doc,
            Task::NoShellout,
        ]
        .into_iter()
        .try_for_each(run),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse().task) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e:#}");
            ExitCode::FAILURE
        },
    }
}
