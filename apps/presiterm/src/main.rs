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

use crate::cli::{Command, FilmArgs, Given, PresentArgs};

fn run() -> Result<i32> {
    let (cli, given) = cli::parse();
    match cli.command {
        Some(Command::Lint {
            file,
            font_size,
            screen,
            grid,
            film,
        }) => {
            let config = Config::load(&deck_dir(&file))?;
            let grid = grid.map(|g| pair(&g, "--grid")).transpose()?;
            let screen = given
                .screen
                .then(|| pair(&screen, "--screen"))
                .transpose()?;
            let mut report = lint::run(&file, &config, font_size, screen, grid)?;
            if film {
                report.findings.extend(film_findings(&file, &config)?);
            }
            println!("{report}");
            Ok(i32::from(report.errors() > 0))
        },
        Some(Command::Film(args)) => film(&args),
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

/// What `presiterm film` and `presiterm lint --film` say in a build without the feature.
#[cfg(not(feature = "film"))]
const NO_FILM: &str = "this presiterm was built without the `film` feature";

/// `presiterm lint --film`: the cards that overflow, at the deck's own film settings.
#[cfg(feature = "film")]
fn film_findings(file: &Path, config: &Config) -> Result<Vec<lint::Finding>> {
    let source = std::fs::read_to_string(file)?;
    let options = presiterm::film::Options::resolve(
        presiterm::config::FilmSettings::default(),
        &source,
        config,
    )?;
    lint::film(file, config, &options)
}

#[cfg(not(feature = "film"))]
fn film_findings(_file: &Path, _config: &Config) -> Result<Vec<lint::Finding>> {
    bail!(NO_FILM)
}

/// `presiterm film`: render the deck and write the files asked for (the GIF when none is).
#[cfg(feature = "film")]
fn film(args: &FilmArgs) -> Result<i32> {
    use presiterm::film::{self, output_path};

    let config = Config::load(&deck_dir(&args.file))?;
    let loaded = presiterm::deck::read(&args.file, false, &config)?;
    let options = film::Options::resolve(args.settings(), &loaded.source, &config)?;
    let rendered = film::render(&loaded, &options)?;
    let out = args.out.as_deref();
    let write = |path: PathBuf, bytes: &[u8]| -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, bytes)
            .map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
        println!("{}  {} KB", path.display(), bytes.len() / 1024);
        Ok(())
    };
    let any_other = args.mp4 || args.docx || args.pdf;
    let gif = (args.gif || args.docx || args.preview || !any_other)
        .then(|| rendered.gif())
        .transpose()?;
    if let Some(gif) = &gif {
        if args.gif || !any_other {
            write(output_path(&args.file, out, "gif"), gif)?;
        }
        if args.docx {
            write(
                output_path(&args.file, out, "docx"),
                &rendered.docx(&loaded.deck, gif)?,
            )?;
        }
    }
    if args.mp4 || args.pdf {
        let mp4 = rendered.mp4()?;
        if args.mp4 {
            write(output_path(&args.file, out, "mp4"), &mp4)?;
        }
        if args.pdf {
            let note =
                "The film plays in Acrobat when the page opens; other viewers show the still.";
            write(
                output_path(&args.file, out, "pdf"),
                &rendered.pdf(&loaded.deck, &mp4, note)?,
            )?;
        }
    }
    if let Some(dir) = &args.cards {
        for (n, scene) in rendered.scenes.iter().enumerate() {
            if let Some(still) = scene.last() {
                let slide = rendered.slide_of[n] + 1;
                write(
                    dir.join(format!("card{slide:02}-{n:03}.png")),
                    &still.png()?,
                )?;
            }
        }
    }
    if args.preview {
        presiterm::term::require_wezterm()?;
        if let Some(gif) = &gif {
            println!("{}", presiterm::term::inline_image(gif, 18));
        }
    }
    Ok(0)
}

#[cfg(not(feature = "film"))]
fn film(_args: &FilmArgs) -> Result<i32> {
    bail!(NO_FILM)
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
