//! Keys from raw stdin bytes.

/// A key press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A printable character.
    Char(char),
    /// Control + letter (`Ctrl('c')`).
    Ctrl(char),
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
    /// Home.
    Home,
    /// End.
    End,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Return.
    Enter,
    /// Escape.
    Esc,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
}

/// Decode a chunk read from stdin in raw mode. Unknown sequences are dropped.
///
/// # Examples
///
/// ```
/// use presiterm::keys::{Key, parse};
///
/// assert_eq!(parse(b"j\x1b[A"), vec![Key::Char('j'), Key::Up]);
/// ```
#[must_use]
pub fn parse(bytes: &[u8]) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        match byte {
            0x1b => {
                let (key, used) = escape(&bytes[i..]);
                keys.extend(key);
                i += used;
            },
            b'\r' | b'\n' => {
                keys.push(Key::Enter);
                i += 1;
            },
            b'\t' => {
                keys.push(Key::Tab);
                i += 1;
            },
            0x7f | 0x08 => {
                keys.push(Key::Backspace);
                i += 1;
            },
            0x01..=0x1a => {
                keys.push(Key::Ctrl(char::from(byte - 1 + b'a')));
                i += 1;
            },
            _ => {
                let len = utf8_len(byte).min(bytes.len() - i);
                if let Ok(text) = std::str::from_utf8(&bytes[i..i + len]) {
                    keys.extend(text.chars().next().map(Key::Char));
                }
                i += len;
            },
        }
    }
    keys
}

const fn utf8_len(first: u8) -> usize {
    match first {
        0xf0..=0xff => 4,
        0xe0..=0xef => 3,
        0xc0..=0xdf => 2,
        _ => 1,
    }
}

/// One escape sequence at the start of `bytes`: the key (if known) and the bytes consumed.
fn escape(bytes: &[u8]) -> (Option<Key>, usize) {
    if bytes.len() == 1 {
        return (Some(Key::Esc), 1);
    }
    if bytes[1] != b'[' && bytes[1] != b'O' {
        return (Some(Key::Esc), 1);
    }
    // CSI / SS3: parameters, then one final byte in 0x40..=0x7e.
    let end = bytes[2..]
        .iter()
        .position(|b| (0x40..=0x7e).contains(b))
        .map_or(bytes.len(), |p| p + 3);
    let body = &bytes[2..end.min(bytes.len())];
    let key = match body {
        b"A" => Some(Key::Up),
        b"B" => Some(Key::Down),
        b"C" => Some(Key::Right),
        b"D" => Some(Key::Left),
        b"H" | b"1~" => Some(Key::Home),
        b"F" | b"4~" => Some(Key::End),
        b"5~" => Some(Key::PageUp),
        b"6~" => Some(Key::PageDown),
        _ => None,
    };
    (key, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plain_and_arrow_bytes_gives_keys() {
        assert_eq!(parse(b"q"), [Key::Char('q')]);
        assert_eq!(parse(b"\x1b[C"), [Key::Right]);
        assert_eq!(parse(b"\x1b[D\x1b[A"), [Key::Left, Key::Up]);
        assert_eq!(parse(b"\x1bOB"), [Key::Down]);
        assert_eq!(parse(b"\x1b[5~"), [Key::PageUp]);
    }

    #[test]
    fn parse_lone_escape_gives_esc() {
        assert_eq!(parse(b"\x1b"), [Key::Esc]);
    }

    #[test]
    fn parse_control_enter_and_unicode_bytes_gives_keys() {
        assert_eq!(parse(b"\x03"), [Key::Ctrl('c')]);
        assert_eq!(parse(b"\r"), [Key::Enter]);
        assert_eq!(parse("ä".as_bytes()), [Key::Char('ä')]);
        assert_eq!(
            parse(b"+-0"),
            [Key::Char('+'), Key::Char('-'), Key::Char('0')]
        );
    }

    #[test]
    fn parse_unknown_sequence_is_dropped() {
        assert_eq!(parse(b"\x1b[99~a"), [Key::Char('a')]);
    }

    proptest::proptest! {
        #[test]
        fn parse_arbitrary_bytes_never_panics(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..64)) {
            let _ = parse(&bytes);
        }

        #[test]
        fn parse_printable_ascii_gives_one_char_key_each(text in "[a-zA-Z0-9]{0,32}") {
            let keys = parse(text.as_bytes());
            proptest::prop_assert_eq!(keys.len(), text.len());
            proptest::prop_assert!(keys.iter().all(|key| matches!(key, Key::Char(_))));
        }
    }
}
