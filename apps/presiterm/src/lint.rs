//! `presiterm lint`: does every slide fit, and is everything it points at there?

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::canvas::width;
use crate::config::Config;
use crate::deck::{self, Block, Deck, plain};
use crate::diagram::Assets;
use crate::render::{self, Mode, View};
use crate::window;

/// How bad a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Cannot be shown as written.
    Error,
    /// Shown, but crowded or clipped.
    Warning,
    /// Worth knowing.
    Note,
}

/// One finding.
#[derive(Clone, Debug)]
pub struct Finding {
    /// 1-based slide number.
    pub slide: usize,
    /// The slide's heading, if it has one.
    pub heading: Option<String>,
    /// Severity.
    pub level: Level,
    /// What is wrong.
    pub message: String,
}

fn has_auto_media(slide: &deck::Slide) -> bool {
    slide.blocks.iter().any(|b| match b {
        Block::Diagram(d) => d.height.is_none(),
        Block::Picture(p) => p.height.is_none(),
        _ => false,
    })
}

fn findings(deck: &Deck, view: View, base: &Path) -> Vec<Finding> {
    let mut assets = Assets::new(base, deck);
    let limit = render::content_width(view);
    let mut out = Vec::new();
    for (index, slide) in deck.slides.iter().enumerate() {
        let mut add = |level, message: String| {
            out.push(Finding {
                slide: index + 1,
                heading: slide.title(),
                level,
                message,
            });
        };
        let steps = render::fragments(slide);
        let page = render::page(deck, index, steps, view, &mut assets, Mode::Slide, "");
        for problem in &page.problems {
            add(Level::Error, problem.clone());
        }
        if page.overflow > 0 {
            add(
                Level::Error,
                format!(
                    "{} rows too many for {} rows ({} body rows used)",
                    page.overflow, view.rows, page.body_rows
                ),
            );
        } else if page.body_rows > render::comfortable_rows(view)
            && !slide.is_title(index == 0)
            && !has_auto_media(slide)
        {
            add(
                Level::Warning,
                format!(
                    "{} body rows, more than {} leave room to breathe",
                    page.body_rows,
                    render::comfortable_rows(view)
                ),
            );
        }
        for block in &slide.blocks {
            let lines: Vec<String> = match block {
                Block::Paragraph(lines) | Block::Quote(lines) => {
                    lines.iter().map(|l| plain(l)).collect()
                },
                Block::List(items) => items.iter().map(|i| plain(&i.spans)).collect(),
                _ => Vec::new(),
            };
            for line in lines.iter().filter(|l| width(l) > limit) {
                add(
                    Level::Warning,
                    format!(
                        "line over {limit} columns wraps: {}",
                        line.chars().take(30).collect::<String>()
                    ),
                );
            }
        }
        if slide.notes.is_empty() {
            add(Level::Note, "no speaker notes".into());
        }
    }
    out
}

/// The biggest font size (whole points) at which every slide fits `screen`, if any.
fn biggest_font(deck: &Deck, screen: (f64, f64), base: &Path) -> Option<u32> {
    let fits = |points: u32| {
        let (cols, rows, cell) = window::grid_for(points as f32, screen);
        let view = View { cols, rows, cell };
        let mut assets = Assets::new(base, deck);
        (0..deck.slides.len()).all(|i| {
            let page = render::page(
                deck,
                i,
                render::fragments(&deck.slides[i]),
                view,
                &mut assets,
                Mode::Slide,
                "",
            );
            page.overflow == 0 && page.problems.is_empty()
        })
    };
    let (mut low, mut high) = (8u32, 120u32);
    if !fits(low) {
        return None;
    }
    while low < high {
        let mid = (low + high).div_ceil(2);
        if fits(mid) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    Some(low)
}

/// The result of linting a deck: the facts of the run, every finding, and the biggest font that fits.
///
/// [`Display`](fmt::Display) renders the text `presiterm lint` prints, without a trailing newline.
#[derive(Clone, Debug)]
pub struct Report {
    /// The deck that was checked.
    pub file: PathBuf,
    /// How many slides it has.
    pub slides: usize,
    /// The font size in points the deck was checked at.
    pub font_size: f32,
    /// Columns of the grid it was checked on.
    pub cols: usize,
    /// Rows of the grid it was checked on.
    pub rows: usize,
    /// Screen size in points the biggest font was searched for.
    pub screen: (f64, f64),
    /// Every finding, in slide order.
    pub findings: Vec<Finding>,
    /// The biggest font size (whole points) at which the whole deck fits the screen, if any.
    pub biggest_font: Option<u32>,
}

impl Report {
    /// Count the findings of `level`.
    #[must_use]
    pub fn count(&self, level: Level) -> usize {
        self.findings.iter().filter(|f| f.level == level).count()
    }

    /// Count the errors, which fail `presiterm lint`.
    #[must_use]
    pub fn errors(&self) -> usize {
        self.count(Level::Error)
    }

    /// Count the warnings, which do not.
    #[must_use]
    pub fn warnings(&self) -> usize {
        self.count(Level::Warning)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            out,
            "{}: {} slides, {} pt, {}x{} cells",
            self.file.display(),
            self.slides,
            self.font_size,
            self.cols,
            self.rows
        )?;
        for finding in &self.findings {
            let tag = match finding.level {
                Level::Error => "error",
                Level::Warning => "warning",
                Level::Note => "note",
            };
            let title = finding.heading.as_deref().unwrap_or("-");
            writeln!(
                out,
                "  slide {} ({title}): {tag}: {}",
                finding.slide, finding.message
            )?;
        }
        match self.biggest_font {
            Some(points) => writeln!(
                out,
                "biggest font size at which the whole deck fits a {}x{} pt screen: {points} pt",
                self.screen.0, self.screen.1
            )?,
            None => writeln!(
                out,
                "the deck does not fit a {}x{} pt screen even at 8 pt",
                self.screen.0, self.screen.1
            )?,
        }
        write!(
            out,
            "{} errors, {} warnings",
            self.errors(),
            self.warnings()
        )
    }
}

/// Lint `file` and return what was found; the caller prints it and decides the exit code.
///
/// Each of `font_size`, `screen` and `grid` is a command-line flag; what a flag leaves open comes
/// from the deck's frontmatter, then `config`.
pub fn run(
    file: &Path,
    config: &Config,
    font_size: Option<f32>,
    screen: Option<(f64, f64)>,
    grid: Option<(usize, usize)>,
) -> Result<Report> {
    let deck::Loaded { deck, base, .. } = deck::read(file, false, config)?;
    let font = config.resolve_font_size(font_size, deck.font_size, None);
    let screen = config.resolve_screen(screen);
    let (mut cols, mut rows, cell) = window::grid_for(font, screen);
    if let Some((c, r)) = config.lint_grid(grid) {
        (cols, rows) = (c, r);
    }
    let view = View { cols, rows, cell };
    Ok(Report {
        file: file.to_path_buf(),
        slides: deck.slides.len(),
        font_size: font,
        cols,
        rows,
        screen,
        findings: findings(&deck, view, &base),
        biggest_font: biggest_font(&deck, screen, &base),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DEFAULT_GRID, DEFAULT_SCREEN};

    #[test]
    fn a_missing_image_and_a_crowded_slide_are_found() {
        let src = "# A\n\n# B\n![x](nope.png)\n\n# C\n- 1\n- 2\n- 3\n- 4\n- 5\n- 6\n- 7\n- 8\n- 9\n- 10\n- 11\n- 12\n";
        let deck = deck::parse(src, "t", Path::new("."), false);
        let view = View {
            cols: DEFAULT_GRID.0,
            rows: DEFAULT_GRID.1,
            cell: (50.0, 111.0),
        };
        let all = findings(&deck, view, Path::new("."));
        assert!(all.iter().any(|f| f.slide == 2 && f.level == Level::Error));
        assert!(
            all.iter()
                .any(|f| f.slide == 3 && f.level == Level::Warning)
        );
        assert!(all.iter().all(|f| f.slide != 1 || f.level == Level::Note));
    }

    #[test]
    fn a_bigger_font_stops_fitting() {
        let src = "# A\n\n# B\n- one\n- two\n- three\n- four\n- five\n- six\n";
        let deck = deck::parse(src, "t", Path::new("."), false);
        let best = biggest_font(&deck, DEFAULT_SCREEN, Path::new(".")).expect("fits at 8 pt");
        assert!((9..120).contains(&best), "{best}");
    }

    #[test]
    fn findings_of_a_missing_image_and_a_crowded_slide_match_snapshot() {
        let src = "# A\n\n# B\n![x](nope.png)\n\n# C\n- 1\n- 2\n- 3\n- 4\n- 5\n- 6\n- 7\n- 8\n- 9\n- 10\n- 11\n- 12\n";
        let deck = deck::parse(src, "t", Path::new("."), false);
        let view = View {
            cols: DEFAULT_GRID.0,
            rows: DEFAULT_GRID.1,
            cell: (50.0, 111.0),
        };
        insta::assert_debug_snapshot!(findings(&deck, view, Path::new(".")));
    }

    const BROKEN: &str = "# A\n\n# B\n![x](nope.png)\n\n# C\n- 1\n- 2\n- 3\n- 4\n- 5\n- 6\n- 7\n- 8\n- 9\n- 10\n- 11\n- 12\n";

    fn report_of(source: &str) -> Report {
        let root = tempfile::tempdir().expect("temp dir");
        let file = root.path().join("slides.md");
        std::fs::write(&file, source).expect("write deck");
        run(&file, &Config::default(), None, None, Some(DEFAULT_GRID)).expect("lint")
    }

    #[test]
    fn run_on_a_broken_deck_reports_facts_findings_and_counts() {
        let report = report_of(BROKEN);
        assert_eq!(report.slides, 3);
        assert_eq!((report.cols, report.rows), DEFAULT_GRID);
        assert!((report.font_size - crate::config::DEFAULT_FONT_SIZE).abs() < 1e-6);
        assert_eq!(report.errors(), 1);
        assert_eq!(report.warnings(), 1);
        let missing = report
            .findings
            .iter()
            .find(|f| f.level == Level::Error)
            .expect("an error");
        assert_eq!((missing.slide, missing.heading.as_deref()), (2, Some("B")));
    }

    /// A deck folder with a `presiterm.toml` and a deck with the given frontmatter lines.
    fn deck_with_config(toml: &str, frontmatter: &str) -> (tempfile::TempDir, PathBuf, Config) {
        let root = tempfile::tempdir().expect("temp dir");
        std::fs::write(root.path().join("presiterm.toml"), toml).expect("write config");
        let file = root.path().join("slides.md");
        std::fs::write(&file, format!("{frontmatter}# A\n\n- one\n")).expect("write deck");
        let config = Config::load_from(root.path(), None).expect("load config");
        (root, file, config)
    }

    #[test]
    fn run_takes_font_size_screen_and_grid_from_the_config_file() {
        let (_root, file, config) = deck_with_config(
            "font_size = 50\nscreen = \"1440x900\"\ngrid = \"70x22\"\n",
            "",
        );

        let report = run(&file, &config, None, None, None).expect("lint");

        assert!((report.font_size - 50.0).abs() < 1e-6);
        assert_eq!(report.screen, (1440.0, 900.0));
        assert_eq!((report.cols, report.rows), (70, 22));
    }

    #[test]
    fn run_font_size_prefers_flag_then_deck_then_config_file() {
        let (_root, file, config) =
            deck_with_config("font_size = 50\n", "---\nfont-size: 36\n---\n");

        let from_deck = run(&file, &config, None, None, None).expect("lint");
        let from_flag = run(&file, &config, Some(60.0), None, None).expect("lint");

        assert!(
            (from_deck.font_size - 36.0).abs() < 1e-6,
            "deck beats the config"
        );
        assert!(
            (from_flag.font_size - 60.0).abs() < 1e-6,
            "flag beats the deck"
        );
    }

    #[test]
    fn run_grid_and_screen_flags_beat_the_config_file() {
        let (_root, file, config) =
            deck_with_config("screen = \"1440x900\"\ngrid = \"70x22\"\n", "");

        let report = run(&file, &config, None, Some((800.0, 600.0)), Some((40, 12))).expect("lint");

        assert_eq!(report.screen, (800.0, 600.0));
        assert_eq!((report.cols, report.rows), (40, 12));
    }

    #[test]
    fn run_on_a_deck_without_slides_errors() {
        let root = tempfile::tempdir().expect("temp dir");
        let file = root.path().join("slides.md");
        std::fs::write(&file, "").expect("write deck");
        let error = run(&file, &Config::default(), None, None, None).expect_err("no slides");
        assert!(
            format!("{error:#}").ends_with("no slides found"),
            "{error:#}"
        );
    }

    #[test]
    fn report_display_prints_header_findings_biggest_font_and_counts() {
        let report = Report {
            file: PathBuf::from("talk/slides.md"),
            slides: 2,
            font_size: 42.0,
            cols: 58,
            rows: 20,
            screen: (1728.0, 1117.0),
            findings: vec![Finding {
                slide: 1,
                heading: None,
                level: Level::Note,
                message: "no speaker notes".into(),
            }],
            biggest_font: Some(46),
        };
        assert_eq!(
            report.to_string(),
            "talk/slides.md: 2 slides, 42 pt, 58x20 cells\n  slide 1 (-): note: no speaker notes\nbiggest font size at which the whole deck fits a 1728x1117 pt screen: 46 pt\n0 errors, 0 warnings"
        );
        let none = Report {
            biggest_font: None,
            ..report
        };
        assert!(
            none.to_string()
                .contains("the deck does not fit a 1728x1117 pt screen even at 8 pt")
        );
    }
}
