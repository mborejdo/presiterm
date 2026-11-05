//! The pager: alternate screen, keys, auto-reload, the control socket and the font size.
//!
//! This file is the spine: the `Pager` state, its event loop and `run`. What goes to the terminal
//! each frame is `frame`; `presiterm ctl`, the client of the control socket, is `control`.

mod control;
mod frame;
#[cfg(test)]
mod tests;

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};

use crate::config::Config;
use crate::deck::{self, Block, Deck};
use crate::diagram::Assets;
use crate::keys::Key;
use crate::render::{self, Mode, View};
use crate::term::{self, Event, Events, RawMode};

pub use control::ctl;
pub use frame::{Drawn, frame_bytes};

/// Smallest and largest font size the keys reach, in points.
const FONT_RANGE: (f32, f32) = (8.0, 120.0);

/// Points per `+` / `-`.
const FONT_STEP: f32 = 2.0;

/// What the user asked for on the command line.
#[derive(Debug, Clone)]
pub struct Options {
    /// The deck file.
    pub file: PathBuf,
    /// Start at this slide (1-based).
    pub slide: Option<usize>,
    /// Font size in points (flag beats frontmatter).
    pub font_size: Option<f32>,
    /// Font size the window was started with (`--window` tells the inner presiterm).
    pub window_font: Option<f32>,
    /// Every bullet is a reveal.
    pub reveal: bool,
    /// Control socket path.
    pub control: Option<PathBuf>,
    /// Do not reload on file changes.
    pub no_watch: bool,
    /// Cell size in pixels when the terminal does not report it.
    pub cell_px: Option<(f64, f64)>,
    /// What the flags and the deck leave open: the config files and the defaults.
    pub config: Config,
}

/// Where the deck was last seen: the file and what it points at.
fn stamp(file: &Path, deck: &Deck, base: &Path) -> Vec<Option<SystemTime>> {
    let mtime = |f: &Path| std::fs::metadata(f).and_then(|m| m.modified()).ok();
    let mut out = vec![mtime(file)];
    out.extend(referenced(deck).iter().map(|p| mtime(&base.join(p))));
    out
}

/// List the files a deck points at: pictures and `.mmd` diagrams.
#[must_use]
pub fn referenced(deck: &Deck) -> Vec<String> {
    deck.slides
        .iter()
        .flat_map(|s| s.blocks.iter())
        .filter_map(|b| match b {
            Block::Picture(p) => Some(p.path.clone()),
            Block::Diagram(d) if deck::extension_is(&d.origin, &["mmd", "mermaid"]) => {
                Some(d.origin.clone())
            },
            _ => None,
        })
        .collect()
}

struct Pager {
    options: Options,
    base: PathBuf,
    deck: Deck,
    /// The deck file as last loaded, to find where an edit happened.
    source: String,
    assets: Assets,
    view: View,
    current: usize,
    step: usize,
    mode: Mode,
    pending: String,
    status: String,
    status_until: Option<Instant>,
    drawn: Drawn,
    seen: Option<(usize, usize, Mode, View)>,
    font_base: f32,
    font_now: f32,
    font_check: Option<(Instant, term::Size)>,
    quit: bool,
}

impl Pager {
    fn view_of(&self, size: term::Size) -> Result<View> {
        let cell = size.cell_px().or(self.options.cell_px).context(
            "the terminal reports no pixel size (needed to size diagrams); pass --cell-px WxH",
        )?;
        Ok(View {
            cols: size.cols,
            rows: size.rows,
            cell,
        })
    }

    fn say(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_until = Some(Instant::now() + Duration::from_secs(3));
    }

    fn fragments(&self) -> usize {
        render::fragments(&self.deck.slides[self.current])
    }

    fn next(&mut self) {
        if self.step < self.fragments() {
            self.step += 1;
        } else if self.current + 1 < self.deck.slides.len() {
            self.current += 1;
            self.step = 0;
        }
    }

    fn prev(&mut self) {
        if self.step > 0 {
            self.step -= 1;
        } else if self.current > 0 {
            self.current -= 1;
            self.step = self.fragments();
        }
    }

    fn goto(&mut self, number: usize) {
        self.current = number.clamp(1, self.deck.slides.len()) - 1;
        self.step = 0;
    }

    fn reload(&mut self) {
        match deck::read(
            &self.options.file,
            self.options.reveal,
            &self.options.config,
        ) {
            Ok(deck::Loaded { deck, base, source }) => {
                let changed = deck != self.deck;
                let edit = first_changed_line(&self.source, &source);
                self.deck = deck;
                self.base = base;
                self.source = source;
                // A picture or diagram file may have changed without the deck text changing: always
                // read the files again.
                self.assets = Assets::new(&self.base, &self.deck);
                self.current = self.current.min(self.deck.slides.len() - 1);
                if changed {
                    // Follow the edit: show the slide the first changed line is on.
                    if let Some(slide) = edit.and_then(|line| slide_at(&self.deck, line))
                        && slide != self.current
                    {
                        self.current = slide;
                        self.step = 0;
                    }
                }
                self.step = self.step.min(self.fragments());
                self.seen = None;
                self.say("reloaded");
            },
            Err(e) => self.say(format!("reload failed: {e:#}")),
        }
    }

    fn set_font(&mut self, points: f32, out: &mut impl Write) -> io::Result<()> {
        let points = points.clamp(FONT_RANGE.0, FONT_RANGE.1);
        self.font_now = points;
        out.write_all(term::set_user_var("presiterm_font", &format!("{points}")).as_bytes())?;
        out.flush()?;
        if let Ok(size) = term::size() {
            self.font_check = Some((Instant::now(), size));
        }
        self.say(format!("font {points} pt"));
        Ok(())
    }

    fn font_command(&mut self, argument: &str, out: &mut impl Write) -> Result<String, String> {
        let target = match argument {
            "reset" => self.font_base,
            a if a.starts_with('+') || a.starts_with('-') => {
                let delta: f32 = a
                    .parse()
                    .map_err(|_| format!("font: {a} is not a number"))?;
                self.font_now + delta
            },
            a => a.parse().map_err(|_| format!("font: {a} is not a size"))?,
        };
        self.set_font(target, out).map_err(|e| e.to_string())?;
        Ok(format!("font {} pt", self.font_now))
    }

    fn status_line(&self) -> String {
        let size = term::size()
            .map(|s| format!("{}x{}", s.cols, s.rows))
            .unwrap_or_default();
        format!(
            "slide {}/{} step {}/{} font {} grid {size}",
            self.current + 1,
            self.deck.slides.len(),
            self.step,
            self.fragments(),
            self.font_now
        )
    }

    fn draw(&mut self, out: &mut impl Write) -> Result<()> {
        let note = if self.pending.is_empty() {
            self.status.clone()
        } else {
            format!(":{}", self.pending)
        };
        let page = render::page(
            &self.deck,
            self.current,
            self.step,
            self.view,
            &mut self.assets,
            self.mode,
            &note,
        );
        let identity = (self.current, self.step, self.mode, self.view);
        let slide_changed = self
            .seen
            .is_none_or(|s| (s.0, s.2, s.3) != (identity.0, identity.2, identity.3));
        let (bytes, drawn) = frame_bytes(&page, &self.drawn, slide_changed);
        out.write_all(bytes.as_bytes())?;
        out.flush()?;
        self.drawn = drawn;
        self.seen = Some(identity);
        Ok(())
    }

    /// Returns true to quit.
    fn key(&mut self, key: Key, out: &mut impl Write) -> io::Result<bool> {
        self.status.clear();
        self.status_until = None;
        match key {
            Key::Char(c) if c.is_ascii_digit() && (c != '0' || !self.pending.is_empty()) => {
                self.pending.push(c);
            },
            Key::Char('0') => {
                let base = self.font_base;
                self.set_font(base, out)?;
            },
            Key::Char(':') => self.pending.clear(),
            Key::Enter if !self.pending.is_empty() => {
                if let Ok(n) = self.pending.parse::<usize>() {
                    self.goto(n);
                }
                self.pending.clear();
            },
            Key::Esc if !self.pending.is_empty() => self.pending.clear(),
            Key::Esc | Key::Char('q') | Key::Ctrl('c') => return Ok(true),
            Key::Char('r') => self.reload(),
            Key::Char('n') => {
                self.mode = if self.mode == Mode::Notes {
                    Mode::Slide
                } else {
                    Mode::Notes
                }
            },
            Key::Char('+' | '=') => self.set_font(self.font_now + FONT_STEP, out)?,
            Key::Char('-') => self.set_font(self.font_now - FONT_STEP, out)?,
            Key::Char('g') | Key::Home => self.goto(1),
            Key::Char('G') | Key::End => self.goto(self.deck.slides.len()),
            Key::Right | Key::Down | Key::Char('l' | 'j' | ' ') | Key::PageDown | Key::Enter => {
                self.pending.clear();
                self.next();
            },
            Key::Left | Key::Up | Key::Char('h' | 'k') | Key::PageUp | Key::Backspace => {
                self.pending.clear();
                self.prev();
            },
            _ => {},
        }
        Ok(false)
    }

    fn control(&mut self, listener: &UnixListener, out: &mut impl Write) {
        while let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let mut command = String::new();
            let _ = BufReader::new(&stream).read_line(&mut command);
            let mut words = command.split_whitespace();
            let reply = match (words.next(), words.next()) {
                (Some("goto"), Some(n)) => match n.parse::<usize>() {
                    Ok(n) if (1..=self.deck.slides.len()).contains(&n) => {
                        self.goto(n);
                        self.status_line()
                    },
                    _ => format!("error: slide must be 1..{}", self.deck.slides.len()),
                },
                (Some("next"), _) => {
                    self.next();
                    self.status_line()
                },
                (Some("prev"), _) => {
                    self.prev();
                    self.status_line()
                },
                (Some("reload"), _) => {
                    self.reload();
                    self.status_line()
                },
                (Some("status"), _) => self.status_line(),
                (Some("quit"), _) => {
                    self.quit = true;
                    "bye".to_string()
                },
                (Some("font"), Some(a)) => self.font_command(a, out).unwrap_or_else(|e| format!("error: {e}")),
                _ => "error: use goto N | next | prev | reload | status | quit | font <pt|+N|-N|reset>".to_string(),
            };
            self.pending.clear();
            let _ = writeln!(stream, "{reply}");
        }
    }
}

/// Present the deck until the user quits.
pub fn run(options: &Options) -> Result<()> {
    term::require_wezterm()?;
    let deck::Loaded { deck, base, source } =
        deck::read(&options.file, options.reveal, &options.config)?;
    let font_base =
        options
            .config
            .resolve_font_size(options.font_size, deck.font_size, options.window_font);
    let size = term::size()
        .context("read the terminal size (stdout is not a terminal; run presiterm in WezTerm)")?;
    let mut pager = Pager {
        options: options.clone(),
        assets: Assets::new(&base, &deck),
        base,
        view: View {
            cols: size.cols,
            rows: size.rows,
            cell: (1.0, 2.0),
        },
        current: options
            .slide
            .map_or(0, |n| n.saturating_sub(1))
            .min(deck.slides.len() - 1),
        deck,
        source,
        step: 0,
        mode: Mode::Slide,
        pending: String::new(),
        status: String::new(),
        status_until: None,
        drawn: Drawn::default(),
        seen: None,
        font_base,
        font_now: options.window_font.unwrap_or(font_base),
        font_check: None,
        quit: false,
    };
    pager.view = pager.view_of(size)?;

    let socket = options.control.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("presiterm-{}.sock", std::process::id()))
    });
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)
        .and_then(|l| l.set_nonblocking(true).map(|()| l))
        .context("open the control socket")?;

    let _raw = RawMode::enable().context("switch the terminal to raw mode")?;
    let mut out = io::stdout();
    out.write_all(term::ENTER.as_bytes())?;
    let result = event_loop(&mut pager, &listener, &mut out);
    let _ = out.write_all(term::LEAVE.as_bytes());
    let _ = out.flush();
    let _ = std::fs::remove_file(&socket);
    result
}

fn event_loop(pager: &mut Pager, listener: &UnixListener, out: &mut io::Stdout) -> Result<()> {
    let mut events = Events::new();
    let mut stamped = stamp(&pager.options.file, &pager.deck, &pager.base);
    let mut last_poll = Instant::now();
    if let Some(target) = pager
        .options
        .font_size
        .or(pager.deck.font_size)
        .or_else(|| pager.options.config.font_override())
        && pager.options.window_font != Some(target)
    {
        pager.set_font(target, out)?;
        pager.status.clear();
    }
    pager.draw(out)?;
    loop {
        let mut quit = false;
        for event in events.next(Duration::from_millis(60))? {
            match event {
                Event::Key(key) => quit |= pager.key(key, out)?,
                Event::Resize(size) => {
                    pager.view = pager.view_of(size)?;
                    pager.font_check = None;
                    pager.seen = None;
                },
            }
        }
        if quit || pager.quit {
            return Ok(());
        }
        pager.control(listener, out);
        if let Some((at, before)) = pager.font_check
            && at.elapsed() > Duration::from_millis(1500)
        {
            pager.font_check = None;
            if term::size().is_ok_and(|now| now == before) {
                pager.say("font: no WezTerm handler (run `presiterm doctor`)");
            }
        }
        if pager.status_until.is_some_and(|t| Instant::now() > t) {
            pager.status.clear();
            pager.status_until = None;
        }
        if !pager.options.no_watch && last_poll.elapsed() >= Duration::from_millis(300) {
            last_poll = Instant::now();
            let now = stamp(&pager.options.file, &pager.deck, &pager.base);
            if now != stamped {
                stamped = now;
                pager.reload();
            }
        }
        pager.draw(out)?;
    }
}

/// 1-based number of the first line that differs between two versions of the file.
fn first_changed_line(old: &str, new: &str) -> Option<usize> {
    let mut a = old.lines();
    let mut b = new.lines();
    let mut number = 1;
    loop {
        match (a.next(), b.next()) {
            (None, None) => return None,
            (x, y) if x == y => number += 1,
            _ => return Some(number),
        }
    }
}

/// Index of the slide a source line belongs to; `None` above the first slide (frontmatter).
fn slide_at(deck: &Deck, line: usize) -> Option<usize> {
    deck.slides.iter().rposition(|s| s.line <= line)
}
