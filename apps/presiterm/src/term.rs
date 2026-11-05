//! The terminal: WezTerm detection, raw mode, size in cells and pixels, events, and the two
//! escape sequences presiterm needs beyond colours (OSC 1337 inline image and `SetUserVar`).

use std::io;
use std::time::Duration;

use rustix::event::{PollFd, PollFlags, Timespec, poll};
use rustix::stdio::{stdin, stdout};
use rustix::termios::{OptionalActions, Termios, tcgetattr, tcgetwinsize, tcsetattr};

use crate::keys::{self, Key};

/// Enter the alternate screen, hide the cursor, switch autowrap off (nothing wraps into the gutter).
pub const ENTER: &str = "\x1b[?1049h\x1b[?25l\x1b[?7l";
/// Undo [`ENTER`].
pub const LEAVE: &str = "\x1b[0m\x1b[?7h\x1b[?25h\x1b[?1049l";
/// Clear the whole screen (images included).
pub const CLEAR: &str = "\x1b[2J";

/// The window size in cells and, as WezTerm reports it, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    /// Columns.
    pub cols: usize,
    /// Rows.
    pub rows: usize,
    /// Width in pixels (0 if unknown).
    pub px_width: usize,
    /// Height in pixels (0 if unknown).
    pub px_height: usize,
}

impl Size {
    /// Return one cell in pixels, if the terminal reports pixel sizes.
    #[must_use]
    pub fn cell_px(&self) -> Option<(f64, f64)> {
        (self.cols > 0 && self.rows > 0 && self.px_width > 0 && self.px_height > 0).then(|| {
            (
                self.px_width as f64 / self.cols as f64,
                self.px_height as f64 / self.rows as f64,
            )
        })
    }
}

/// Something that happened on the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// A key.
    Key(Key),
    /// The size changed (a new font size counts).
    Resize(Size),
}

/// Fail unless this runs in WezTerm.
pub fn require_wezterm() -> anyhow::Result<()> {
    let in_wezterm = std::env::var("TERM_PROGRAM").is_ok_and(|value| value == "WezTerm")
        || std::env::var_os("WEZTERM_PANE").is_some()
        || std::env::var_os("WEZTERM_EXECUTABLE").is_some();
    anyhow::ensure!(
        in_wezterm,
        "presiterm runs in WezTerm only (inline images use its OSC 1337 path); start it with `presiterm --window deck.md`"
    );
    Ok(())
}

/// Read the size of the terminal: stdout's, or stdin's when stdout is redirected.
pub fn size() -> io::Result<Size> {
    let ws = tcgetwinsize(stdout()).or_else(|_| tcgetwinsize(stdin()))?;
    Ok(Size {
        cols: usize::from(ws.ws_col),
        rows: usize::from(ws.ws_row),
        px_width: usize::from(ws.ws_xpixel),
        px_height: usize::from(ws.ws_ypixel),
    })
}

/// Raw mode on stdin for as long as the value lives; restored on drop.
#[derive(Debug)]
pub struct RawMode {
    saved: Termios,
}

impl RawMode {
    /// Switch stdin to raw mode.
    pub fn enable() -> io::Result<Self> {
        let saved = tcgetattr(stdin())?;
        let mut raw = saved.clone();
        raw.make_raw();
        tcsetattr(stdin(), OptionalActions::Flush, &raw)?;
        Ok(Self { saved })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = tcsetattr(stdin(), OptionalActions::Flush, &self.saved);
    }
}

/// Polls stdin for keys and the window for size changes.
#[derive(Debug)]
pub struct Events {
    last: Option<Size>,
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

impl Events {
    /// Create a reader that treats the current size as known.
    #[must_use]
    pub fn new() -> Self {
        Self { last: size().ok() }
    }

    /// Wait up to `timeout` for input.
    pub fn next(&mut self, timeout: Duration) -> io::Result<Vec<Event>> {
        let mut events = Vec::new();
        let input = stdin();
        let mut fds = [PollFd::new(&input, PollFlags::IN)];
        let wait = Timespec {
            tv_sec: timeout.as_secs().cast_signed(),
            tv_nsec: i64::from(timeout.subsec_nanos()),
        };
        match poll(&mut fds, Some(&wait)) {
            Ok(_) | Err(rustix::io::Errno::INTR) => {},
            Err(e) => return Err(e.into()),
        }
        if fds[0].revents().contains(PollFlags::IN) {
            let mut buf = [0u8; 256];
            let count = rustix::io::read(input, &mut buf)?;
            events.extend(keys::parse(&buf[..count]).into_iter().map(Event::Key));
        }
        if let Ok(now) = size()
            && self.last != Some(now)
        {
            self.last = Some(now);
            events.push(Event::Resize(now));
        }
        Ok(events)
    }
}

/// Encode `bytes` as standard base64 with padding.
///
/// # Examples
///
/// ```
/// assert_eq!(presiterm::term::base64(b"presiterm"), "cHJlc2l0ZXJt");
/// ```
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(char::from(ALPHABET[(n >> 18) as usize & 63]));
        out.push(char::from(ALPHABET[(n >> 12) as usize & 63]));
        out.push(if chunk.len() > 1 {
            char::from(ALPHABET[(n >> 6) as usize & 63])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(ALPHABET[n as usize & 63])
        } else {
            '='
        });
    }
    out
}

/// Build the OSC 1337 inline image, `rows` cells high with the aspect kept (WezTerm plays a GIF itself).
#[must_use]
pub fn inline_image(bytes: &[u8], rows: usize) -> String {
    format!(
        "\x1b]1337;File=inline=1;height={rows};preserveAspectRatio=1;size={}:{}\x07",
        bytes.len(),
        base64(bytes)
    )
}

/// Tell WezTerm's `user-var-changed` handler something (the font size) with `SetUserVar`.
#[must_use]
pub fn set_user_var(name: &str, value: &str) -> String {
    format!(
        "\x1b]1337;SetUserVar={name}={}\x07",
        base64(value.as_bytes())
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_GRID;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn inline_image_carries_height_aspect_flag_size_and_payload() {
        assert_eq!(
            inline_image(&[1, 2, 3, 4], 7),
            "\x1b]1337;File=inline=1;height=7;preserveAspectRatio=1;size=4:AQIDBA==\x07"
        );
    }

    #[test]
    fn user_var_is_name_equals_base64_value() {
        assert_eq!(
            set_user_var("presiterm_font", "42"),
            "\x1b]1337;SetUserVar=presiterm_font=NDI=\x07"
        );
    }

    #[test]
    fn cell_px_with_missing_numbers_is_none() {
        assert_eq!(
            Size {
                cols: DEFAULT_GRID.0,
                rows: DEFAULT_GRID.1,
                px_width: 2900,
                px_height: 2220
            }
            .cell_px(),
            Some((50.0, 111.0))
        );
        assert_eq!(
            Size {
                cols: DEFAULT_GRID.0,
                rows: DEFAULT_GRID.1,
                px_width: 0,
                px_height: 0
            }
            .cell_px(),
            None
        );
    }

    /// Decode standard padded base64, the inverse of [`base64`].
    fn unbase64(text: &str) -> Vec<u8> {
        let value = |c: u8| match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            _ => 63,
        };
        let mut out = Vec::new();
        for chunk in text.as_bytes().chunks(4) {
            let sextets: Vec<u8> = chunk
                .iter()
                .filter(|c| **c != b'=')
                .map(|c| value(*c))
                .collect();
            let word = sextets
                .iter()
                .fold(0u32, |acc, s| (acc << 6) | u32::from(*s))
                << (6 * (4 - sextets.len()));
            let bytes = word.to_be_bytes();
            out.extend_from_slice(&bytes[1..sextets.len()]);
        }
        out
    }

    proptest::proptest! {
        #[test]
        fn base64_roundtrips_any_bytes(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..96)) {
            proptest::prop_assert_eq!(unbase64(&base64(&bytes)), bytes);
        }

        #[test]
        fn base64_length_is_four_thirds_rounded_up_to_four(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..96)) {
            proptest::prop_assert_eq!(base64(&bytes).len(), bytes.len().div_ceil(3) * 4);
        }
    }
}
