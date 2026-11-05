//! The `presiterm` binary: dispatch the parsed command line and map the result to an exit code.

// The binary's output is the product: reports and messages go to stdout, the one-line failure to
// stderr.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the binary prints what each command returns"
)]

mod cli;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use presiterm::config::{Config, deck_dir, pair};
use presiterm::{handbook, lint, new_deck, pager, window};

use crate::cli::{Command, Given, PresentArgs};

fn run() -> Result<i32> {
    let (cli, given) = cli::parse();
    match cli.command {
        Some(Command::Lint {
            file,
            font_size,
            screen,
            grid,
        }) => {
            let config = Config::load(&deck_dir(&file))?;
            let grid = grid.map(|g| pair(&g, "--grid")).transpose()?;
            let screen = given
                .screen
                .then(|| pair(&screen, "--screen"))
                .transpose()?;
            let report = lint::run(&file, &config, font_size, screen, grid)?;
            println!("{report}");
            Ok(i32::from(report.errors() > 0))
        },
        Some(Command::Config { path }) => {
            let from = path.as_deref().unwrap_or_else(|| Path::new("."));
            println!("{}", Config::load(&deck_dir(from))?);
            Ok(0)
        },
        Some(Command::New { name }) => {
            println!("{}", new_deck::run(&name)?);
            Ok(0)
        },
        Some(Command::Ctl { control, command }) => {
            println!("{}", pager::ctl(control.as_deref(), &command)?);
            Ok(0)
        },
        Some(Command::Doctor) => {
            let diagnosis = window::doctor()?;
            println!("{diagnosis}");
            if diagnosis == window::Diagnosis::NeedsHandler {
                bail!(
                    "no `user-var-changed` handler for presiterm_font in the active WezTerm config"
                )
            }
            Ok(0)
        },
        Some(Command::ConfigSnippet) => {
            println!("{}", window::HANDLER);
            Ok(0)
        },
        Some(Command::Handbook { topic: None }) => {
            println!("{}", handbook::index());
            Ok(0)
        },
        Some(Command::Handbook { topic: Some(key) }) => {
            let Some(topic) = handbook::topic(&key) else {
                bail!("no handbook topic `{key}`; `presiterm handbook` lists them")
            };
            println!("{}", topic.body.trim_end());
            Ok(0)
        },
        Some(Command::Tldr { words }) => {
            let query = words.join(" ");
            println!(
                "{}",
                handbook::format_hits(&query, &handbook::search(&query, 3))
            );
            Ok(0)
        },
        None => present(cli.file, cli.present, given),
    }
}

/// Present the deck: in a dedicated WezTerm window (`--window`) or in the terminal we are in.
fn present(file: Option<PathBuf>, args: PresentArgs, given: Given) -> Result<i32> {
    let Some(file) = file else {
        bail!(
            "no deck given: run `presiterm talk/slides.md`, `presiterm new talk` to make one, or see `presiterm --help`"
        )
    };
    let config = Config::load(&deck_dir(&file))?;
    let cell_px = args
        .cell_px
        .as_deref()
        .map(|pair_text| pair(pair_text, "--cell-px"))
        .transpose()?;
    if args.window {
        window::open(&window::OpenOptions {
            file,
            cols: given.cols.then_some(args.cols),
            rows: given.rows.then_some(args.rows),
            config,
            font_size: args.font_size,
            slide: args.slide,
            reveal: args.reveal,
            no_watch: args.no_watch,
            control: args.control,
            cell_px: args.cell_px,
        })?;
        return Ok(0);
    }
    pager::run(&pager::Options {
        file,
        slide: args.slide,
        font_size: args.font_size,
        window_font: args.window_font,
        reveal: args.reveal,
        control: args.control,
        no_watch: args.no_watch,
        cell_px,
        config,
    })?;
    Ok(0)
}

fn main() {
    match run() {
        Ok(0) => {},
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("presiterm: {e:#}");
            std::process::exit(2);
        },
    }
}
