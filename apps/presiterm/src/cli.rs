//! The command line definition: `Cli`, `Command`, the `--help` texts and the embedded handbook.

use std::path::PathBuf;
use std::sync::LazyLock;

use clap::parser::ValueSource;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand};
use presiterm::config::{DEFAULT_GRID, DEFAULT_SCREEN};

/// The default of `--screen` as the text clap shows and parses, built from [`DEFAULT_SCREEN`].
static SCREEN_TEXT: LazyLock<String> =
    LazyLock::new(|| format!("{}x{}", DEFAULT_SCREEN.0, DEFAULT_SCREEN.1));

/// What the top of `--help` says: the idea in a few lines.
const ABOUT: &str = "Markdown in, a WezTerm presentation out

The text is real terminal text (WezTerm's own font, so the font size is yours to change); diagrams \
and pictures are inline images. Pure Rust: the only program presiterm starts is `wezterm`.";

/// The end of the root `--help`: examples, the manual, exit codes.
const ROOT_FOOTER: &str = "\
Examples:
  presiterm new talk                          make talk/slides.md and an example diagram
  presiterm --window talk/slides.md           present in a dedicated WezTerm window
  presiterm talk/slides.md --slide 7          present in the WezTerm you are in, from slide 7
  presiterm --window talk/slides.md --font-size 54 --cols 80 --rows 24
                                              bigger text on a bigger grid
  presiterm --window talk/slides.md --reveal  every bullet is a key press
  presiterm --window talk/slides.md --no-watch --control /tmp/talk.sock
                                              no reload on save, a socket of your own
  presiterm talk/slides.md --cell-px 21x42    cell size in pixels, for a terminal that does not report it
  presiterm lint talk/slides.md               does every slide fit, is every file there
  presiterm config talk/slides.md             which presiterm.toml applies, and the effective settings
  presiterm ctl goto 5                        drive the running window from another terminal
  presiterm doctor                            does this WezTerm follow the font-size variable
  presiterm handbook                          the manual (keys, dialect, window); `presiterm tldr font` searches it

Defaults: 42 pt, a 1728x1117 screen, a 58x20 grid. A `presiterm.toml` in the deck's folder (or above it),
or ~/.config/presiterm/config.toml, changes them; a flag typed here beats both. See `presiterm handbook config`.

Exit codes:
  0  done
  1  `presiterm lint` found errors in the deck
  2  presiterm could not do what you asked (the reason is the one line on stderr)";

const LINT_FOOTER: &str = "\
Examples:
  presiterm lint talk/slides.md                          check at the deck's font size (else presiterm.toml, else 42 pt)
  presiterm lint talk/slides.md --font-size 54           check at 54 pt
  presiterm lint talk/slides.md --screen 1440x900        a smaller screen, in points
  presiterm lint talk/slides.md --grid 80x24             check on exactly this grid
  presiterm lint talk/slides.md --film                   also: does every card of the film fit

Prints one line per finding (`slide N (heading): error|warning|note: ...`), then the biggest font
size at which the whole deck fits the screen.

Exit codes:
  0  no errors (warnings and notes do not fail)
  1  at least one error: a missing picture, a broken diagram, a slide that cannot be laid out
  2  the deck could not be read or has no slides";

const FILM_FOOTER: &str = "\
Examples:
  presiterm film talk/slides.md                      talk/slides.gif
  presiterm film talk/slides.md --gif --mp4 --docx --pdf
                                                     all four: the GIF, the MP4, a Word file that plays the GIF, a PDF that plays the MP4
  presiterm film talk/slides.md --out dist           write into dist/ instead of next to the deck
  presiterm film talk/slides.md --cards dist/cards   every card, fully built, as PNG (card03-007.png: slide 3, scene 7)
  presiterm film talk/slides.md --preview            play the GIF inline in this WezTerm
  presiterm film talk/slides.md --width 960 --fps 12 --hold 5 --enter 1.2 --fade 0.3 --colours 64
  presiterm film talk/slides.md --transition wipe   wipe instead of fade (or push, pixelize, circle, split, cut)

Every key press of the talk is a build-in of the film: `enter` seconds of animation, then `hold`
seconds of rest. The deck's frontmatter (fps, width, hold, enter, fade, transition, pace,
colours) and a `[film]` table in presiterm.toml set the same values; a flag typed here beats both. `links:` in the
frontmatter go under the picture in the Word and PDF editions. A card that does not fit is an
error naming the slide; `presiterm lint --film` finds them all first.

Exit codes:
  0  written
  2  a card overflows, a file cannot be drawn or written, or presiterm was built without `film`";

const CONFIG_FOOTER: &str = "\
Examples:
  presiterm config                     look up from the current folder
  presiterm config talk/slides.md      look up from the deck's folder

Prints the files it found, closest first (the closest presiterm.toml wins, farther ones are ignored,
then ~/.config/presiterm/config.toml), and each setting with where its value came from: a file or
`default`. Per key, the closest file that sets it wins.

Exit codes:
  0  printed
  2  a config file cannot be read, has an unknown key or a malformed value (the file and key are named)";

const NEW_FOOTER: &str = "\
Examples:
  presiterm new talk        makes talk/slides.md and talk/assets/flow.mmd
  presiterm --window talk/slides.md

Exit codes:
  0  the folder is made
  2  the folder exists already, or cannot be written";

const CTL_FOOTER: &str = "\
Commands:
  goto N             go to slide N (1-based)
  next | prev        one step forward or back (a bullet, a lit node, then a slide)
  reload             read the deck again now
  status             print the slide, step and font size
  font PT|+N|-N|reset   set the font size in points, change it by N, or back to the deck's
  quit               close the presentation

Examples:
  presiterm ctl goto 5
  presiterm ctl font +4
  presiterm ctl --control /tmp/presiterm-4711.sock status

Exit codes:
  0  the presenter answered
  2  no running presenter, or it refused the command (slide out of range, unknown command)";

const DOCTOR_FOOTER: &str = "\
Run it inside the WezTerm you present in: it sets the font size to 30 pt, then back, and watches
the window size. If nothing changed it prints the Lua handler to add to your wezterm.lua.

Examples:
  presiterm doctor

Exit codes:
  0  this WezTerm follows the font-size variable
  2  it does not (the handler is printed), or this is not WezTerm";

const SNIPPET_FOOTER: &str = "\
Examples:
  presiterm config-snippet >> ~/.config/wezterm/wezterm.lua   then move it above `return config`

Exit codes:
  0  printed";

const HANDBOOK_FOOTER: &str = "\
Examples:
  presiterm handbook            overview and the list of topics
  presiterm handbook keys       one topic in full

Exit codes:
  0  printed
  2  no such topic";

const TLDR_FOOTER: &str = "\
Examples:
  presiterm tldr font size      topics that match, best first, with a snippet
  presiterm tldr \"speaker notes\"

Exit codes:
  0  searched (no match is still 0)";

/// Which flags that carry a visible default were really given: the default shown in `--help` must
/// not hide the config file's value.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Given {
    /// `--cols` was given.
    pub(crate) cols: bool,
    /// `--rows` was given.
    pub(crate) rows: bool,
    /// `presiterm lint --screen` was given.
    pub(crate) screen: bool,
}

/// Parse the command line, and note which defaulted flags the user really typed.
pub(crate) fn parse() -> (Cli, Given) {
    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    let typed =
        |matches: &ArgMatches, id: &str| matches.value_source(id) == Some(ValueSource::CommandLine);
    let given = Given {
        cols: typed(&matches, "cols"),
        rows: typed(&matches, "rows"),
        screen: matches
            .subcommand_matches("lint")
            .is_some_and(|lint| typed(lint, "screen")),
    };
    (cli, given)
}

/// The `presiterm` command line: a deck to present, or one of the subcommands.
#[derive(Parser)]
#[command(
    name = "presiterm",
    version,
    about = ABOUT,
    after_help = ROOT_FOOTER,
    args_conflicts_with_subcommands = true
)]
pub(crate) struct Cli {
    /// The deck (a markdown file) to present, e.g. talk/slides.md.
    pub(crate) file: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) present: PresentArgs,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

/// The flags that present a deck.
#[derive(clap::Args)]
pub(crate) struct PresentArgs {
    /// Start at slide N, 1-based (--slide 7).
    #[arg(long, value_name = "N")]
    pub(crate) slide: Option<usize>,
    /// Font size in points; beats `font-size:` in the frontmatter (--font-size 54).
    #[arg(long, value_name = "PT")]
    pub(crate) font_size: Option<f32>,
    /// Every bullet appears on a key press, as if all were `+` bullets.
    #[arg(long)]
    pub(crate) reveal: bool,
    /// Open a dedicated WezTerm window for the deck (presiterm --window talk/slides.md).
    #[arg(long)]
    pub(crate) window: bool,
    /// Window width in cells, with --window (--cols 80).
    #[arg(long, default_value_t = DEFAULT_GRID.0, value_name = "COLS")]
    pub(crate) cols: usize,
    /// Window height in cells, with --window (--rows 24).
    #[arg(long, default_value_t = DEFAULT_GRID.1, value_name = "ROWS")]
    pub(crate) rows: usize,
    /// Control socket path (--control /tmp/talk.sock); default: <tmp>/presiterm-<pid>.sock.
    #[arg(long, value_name = "PATH")]
    pub(crate) control: Option<PathBuf>,
    /// Do not reload when the deck or a file it points at changes; key r still works.
    #[arg(long)]
    pub(crate) no_watch: bool,
    /// Cell size in pixels when the terminal does not report one (--cell-px 21x42).
    #[arg(long, value_name = "WxH")]
    pub(crate) cell_px: Option<String>,
    /// Font size the window was started with (set by --window).
    #[arg(long, hide = true)]
    pub(crate) window_font: Option<f32>,
}

/// The flags of `presiterm film`.
#[derive(clap::Args)]
#[allow(clippy::struct_excessive_bools, reason = "independent output switches")]
pub(crate) struct FilmArgs {
    /// The deck, e.g. talk/slides.md.
    pub(crate) file: PathBuf,
    /// Write the animated GIF (the default when no format is given).
    #[arg(long)]
    pub(crate) gif: bool,
    /// Write the MP4 (pure-Rust H.264).
    #[arg(long)]
    pub(crate) mp4: bool,
    /// Write an A4 landscape Word file with the GIF and the deck's links under it.
    #[arg(long)]
    pub(crate) docx: bool,
    /// Write a PDF whose page plays the MP4 when it opens in Acrobat.
    #[arg(long)]
    pub(crate) pdf: bool,
    /// Also write every card, fully built, as PNG into this folder (--cards dist/cards).
    #[arg(long, value_name = "DIR")]
    pub(crate) cards: Option<PathBuf>,
    /// Write into this folder instead of next to the deck (--out dist).
    #[arg(long, value_name = "DIR")]
    pub(crate) out: Option<PathBuf>,
    /// Play the GIF inline in this WezTerm after writing.
    #[arg(long)]
    pub(crate) preview: bool,
    /// Frames per second (--fps 12); default 10.
    #[arg(long, value_name = "N")]
    pub(crate) fps: Option<u16>,
    /// Width in pixels, even (--width 960); default 640, the height follows 16:9.
    #[arg(long, value_name = "PX")]
    pub(crate) width: Option<u32>,
    /// Seconds every scene stays, build-in included (--hold 5); default 7.
    #[arg(long, value_name = "S")]
    pub(crate) hold: Option<f32>,
    /// Seconds of build-in at the start of every scene (--enter 1.2); default 1.6.
    #[arg(long, value_name = "S")]
    pub(crate) enter: Option<f32>,
    /// Seconds a transition between scenes takes (--fade 0.3); default 0.4.
    #[arg(long, value_name = "S")]
    pub(crate) fade: Option<f32>,
    /// How one scene gives way to the next: fade, wipe, push, pixelize, circle, split or cut
    /// (--transition wipe); default fade.
    #[arg(long, value_name = "NAME")]
    pub(crate) transition: Option<filmstrip::Transition>,
    /// Words per minute a wordy scene is held for, 0 for a fixed hold (--pace 160); default 200.
    #[arg(long, value_name = "WPM")]
    pub(crate) pace: Option<f32>,
    /// Colours of the GIF palette, 2 to 255 (--colours 64); default 48.
    #[arg(long, value_name = "N")]
    pub(crate) colours: Option<usize>,
}

#[cfg(feature = "film")]
impl FilmArgs {
    /// The timing and size flags as settings, to lay over the frontmatter and the config.
    pub(crate) const fn settings(&self) -> presiterm::config::FilmSettings {
        presiterm::config::FilmSettings {
            fps: self.fps,
            width: self.width,
            hold: self.hold,
            enter: self.enter,
            fade: self.fade,
            transition: self.transition,
            pace: self.pace,
            colours: self.colours,
        }
    }
}

/// The subcommands.
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Check that every slide fits and everything it points at exists.
    #[command(after_help = LINT_FOOTER)]
    Lint {
        /// The deck, e.g. talk/slides.md.
        file: PathBuf,
        /// Font size in points to check at (--font-size 54); default: the deck's, else presiterm.toml, else 42.
        #[arg(long, value_name = "PT")]
        font_size: Option<f32>,
        /// Screen size in points a window must fit (--screen 1440x900).
        #[arg(long, default_value = SCREEN_TEXT.as_str(), value_name = "WxH")]
        screen: String,
        /// Check on exactly this grid of cells (--grid 80x24), not the one the screen gives.
        #[arg(long, value_name = "COLSxROWS")]
        grid: Option<String>,
        /// Also draw every card of the film and report the ones that overflow.
        #[arg(long)]
        film: bool,
    },
    /// Render the deck as a film: GIF, MP4, a Word file that plays the GIF, a PDF that plays the MP4.
    #[command(after_help = FILM_FOOTER)]
    Film(FilmArgs),
    /// Show which config files apply and the effective settings with their source.
    #[command(after_help = CONFIG_FOOTER)]
    Config {
        /// A deck (or its folder) to look up from; default: the current folder.
        path: Option<PathBuf>,
    },
    /// Make a deck folder: slides.md and an example diagram.
    #[command(after_help = NEW_FOOTER)]
    New {
        /// Folder to make, e.g. talk; it must not exist.
        name: String,
    },
    /// Drive a running presentation from another terminal.
    #[command(after_help = CTL_FOOTER)]
    Ctl {
        /// Control socket (--control /tmp/talk.sock); default: the newest running presenter.
        #[arg(long, value_name = "PATH")]
        control: Option<PathBuf>,
        /// goto N | next | prev | reload | status | quit | font <pt|+N|-N|reset>
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            value_name = "COMMAND"
        )]
        command: Vec<String>,
    },
    /// Check that WezTerm follows the font-size variable; print the Lua if not.
    #[command(after_help = DOCTOR_FOOTER)]
    Doctor,
    /// Print the Lua handler for your own wezterm.lua.
    #[command(after_help = SNIPPET_FOOTER)]
    ConfigSnippet,
    /// The manual: the list of topics, or one topic in full.
    #[command(after_help = HANDBOOK_FOOTER)]
    Handbook {
        /// A topic to print in full, e.g. keys; without one, the overview and the topics.
        topic: Option<String>,
    },
    /// Search the manual, best topic first.
    #[command(after_help = TLDR_FOOTER)]
    Tldr {
        /// What you are looking for, e.g. font size.
        #[arg(required = true)]
        words: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn help_of(command: &clap::Command) -> String {
        command.clone().render_long_help().to_string()
    }

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_command_help_has_examples_and_exit_codes() {
        let root = Cli::command();
        let mut commands = vec![root.clone()];
        commands.extend(
            root.get_subcommands()
                .filter(|c| c.get_name() != "help")
                .cloned(),
        );
        for command in commands {
            let help = help_of(&command);
            assert!(
                help.contains("Examples:"),
                "{}: no Examples",
                command.get_name()
            );
            assert!(
                help.contains("Exit codes:"),
                "{}: no Exit codes",
                command.get_name()
            );
        }
    }

    #[test]
    fn every_flag_of_the_present_lint_and_film_commands_has_an_example_in_its_help() {
        let root = Cli::command();
        let lint = root.find_subcommand("lint").expect("lint");
        let film = root.find_subcommand("film").expect("film");
        for command in [&root, lint, film] {
            for arg in command.get_arguments().filter(|a| !a.is_hide_set()) {
                let Some(long) = arg.get_long() else {
                    continue;
                };
                if matches!(long, "help" | "version") {
                    continue;
                }
                let flag = format!("--{long}");
                let in_flag_help = arg
                    .get_help()
                    .is_some_and(|h| h.to_string().contains(&flag));
                let in_footer = help_of(command)
                    .lines()
                    .filter(|l| l.trim_start().starts_with("presiterm "))
                    .any(|l| l.contains(&flag));
                assert!(in_flag_help || in_footer, "{flag} has no example in --help");
            }
        }
    }

    fn handbook_text() -> String {
        presiterm::handbook::TOPICS
            .iter()
            .map(|t| t.body)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn handbook_names_every_flag_and_command() {
        let text = handbook_text();
        let root = Cli::command();
        let lint = root.find_subcommand("lint").expect("lint");
        let film = root.find_subcommand("film").expect("film");
        for command in [&root, lint, film] {
            for arg in command.get_arguments().filter(|a| !a.is_hide_set()) {
                if let Some(long) = arg.get_long().filter(|l| !matches!(*l, "help" | "version")) {
                    assert!(
                        text.contains(&format!("--{long}")),
                        "handbook lacks --{long}"
                    );
                }
            }
        }
        for sub in root.get_subcommands().filter(|c| c.get_name() != "help") {
            let name = format!("presiterm {}", sub.get_name());
            assert!(text.contains(&name), "handbook lacks `{name}`");
        }
    }

    #[test]
    fn handbook_covers_the_dialect_and_the_fences() {
        let text = handbook_text();
        for needle in [
            "==marked==",
            "__marked__",
            "steps=a,b,c",
            "steps=N",
            "stepnode",
            "{only}",
            "theme",
            "logo",
            "w=",
            "h=",
            ":::{notes}",
            ":::{metrics}",
            "presiterm film",
            "presiterm ctl",
            "presiterm doctor",
            "presiterm lint",
            "first changed line",
            ":only:",
        ] {
            assert!(text.contains(needle), "handbook lacks {needle}");
        }
    }

    #[test]
    fn handbook_uses_keyboard_characters_only() {
        assert!(
            handbook_text().is_ascii(),
            "non-ASCII character in the handbook"
        );
    }

    #[test]
    fn help_uses_keyboard_characters_only() {
        let root = Cli::command();
        let mut text = help_of(&root);
        for sub in root.get_subcommands() {
            text.push_str(&help_of(sub));
        }
        assert!(text.is_ascii(), "non-ASCII character in --help");
    }
}
