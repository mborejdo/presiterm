//! A matrix of styled cells for one screen, turned into ANSI lines.

use std::fmt::Write as _;

use unicode_width::UnicodeWidthChar;

pub use swatchbook::Rgb;

/// Colours and attributes of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools, reason = "independent SGR switches")]
pub struct Style {
    /// Text colour.
    pub fg: Rgb,
    /// Background colour.
    pub bg: Rgb,
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Dim.
    pub dim: bool,
    /// Underline in this colour, independent of the text colour.
    pub underline: Option<Rgb>,
}

impl Style {
    /// Plain text in `fg` on `bg`.
    #[must_use]
    pub const fn new(fg: Rgb, bg: Rgb) -> Self {
        Self {
            fg,
            bg,
            bold: false,
            italic: false,
            dim: false,
            underline: None,
        }
    }

    /// Return the style in bold.
    #[must_use]
    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Return the style in italic.
    #[must_use]
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Return the style dimmed.
    #[must_use]
    pub const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    /// Return the style underlined in `colour`.
    #[must_use]
    pub const fn underline(mut self, colour: Rgb) -> Self {
        self.underline = Some(colour);
        self
    }

    /// Return the style with another foreground.
    #[must_use]
    pub const fn fg(mut self, fg: Rgb) -> Self {
        self.fg = fg;
        self
    }

    /// Return the style with another background.
    #[must_use]
    pub const fn bg(mut self, bg: Rgb) -> Self {
        self.bg = bg;
        self
    }

    fn sgr(self) -> String {
        let mut out = String::from("\x1b[0");
        for (on, code) in [(self.bold, 1), (self.dim, 2), (self.italic, 3)] {
            if on {
                let _ = write!(out, ";{code}");
            }
        }
        if let Some(Rgb(r, g, b)) = self.underline {
            let _ = write!(out, ";4;58;2;{r};{g};{b}");
        }
        let _ = write!(
            out,
            ";38;2;{};{};{};48;2;{};{};{}m",
            self.fg.0, self.fg.1, self.fg.2, self.bg.0, self.bg.1, self.bg.2
        );
        out
    }
}

/// Cells of one screen; writes outside it are clipped, so nothing can overflow a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canvas {
    /// Columns.
    pub cols: usize,
    /// Rows.
    pub rows: usize,
    cells: Vec<(char, Style)>,
}

impl Canvas {
    /// Create a blank canvas of `cols` x `rows` cells in the `base` style.
    #[must_use]
    pub fn new(cols: usize, rows: usize, base: Style) -> Self {
        Self {
            cols,
            rows,
            cells: vec![(' ', base); cols * rows],
        }
    }

    /// Write one char (a wide char claims the next cell too).
    pub fn put(&mut self, col: usize, row: usize, ch: char, style: Style) {
        let width = ch.width().unwrap_or(1).max(1);
        if col + width > self.cols || row >= self.rows {
            return;
        }
        self.cells[row * self.cols + col] = (ch, style);
        if width == 2 {
            self.cells[row * self.cols + col + 1] = ('\0', style);
        }
    }

    /// Write `text` from `col`; returns the cells used (clipped at the right edge).
    pub fn text(&mut self, col: usize, row: usize, text: &str, style: Style) -> usize {
        let mut x = col;
        for ch in text.chars() {
            let width = ch.width().unwrap_or(0);
            if width == 0 {
                continue;
            }
            if x + width > self.cols {
                break;
            }
            self.put(x, row, ch, style);
            x += width;
        }
        x - col
    }

    /// Fill a rectangle with blanks.
    pub fn fill(&mut self, col: usize, row: usize, cols: usize, rows: usize, style: Style) {
        for y in row..(row + rows).min(self.rows) {
            for x in col..(col + cols).min(self.cols) {
                self.put(x, y, ' ', style);
            }
        }
    }

    /// Return the text of a row without styles (wide-char halves skipped).
    #[must_use]
    pub fn row_text(&self, row: usize) -> String {
        self.cells[row * self.cols..(row + 1) * self.cols]
            .iter()
            .filter(|(c, _)| *c != '\0')
            .map(|(c, _)| *c)
            .collect()
    }

    /// Return the style of one cell.
    #[must_use]
    pub fn style_at(&self, col: usize, row: usize) -> Style {
        self.cells[row * self.cols + col].1
    }

    /// Render every row as an ANSI string of exactly `cols` cells, ending in a reset.
    #[must_use]
    pub fn ansi_rows(&self) -> Vec<String> {
        (0..self.rows)
            .map(|row| {
                let mut out = String::new();
                let mut current: Option<Style> = None;
                for (ch, style) in &self.cells[row * self.cols..(row + 1) * self.cols] {
                    if *ch == '\0' {
                        continue;
                    }
                    if current != Some(*style) {
                        out.push_str(&style.sgr());
                        current = Some(*style);
                    }
                    out.push(*ch);
                }
                out.push_str("\x1b[0m");
                out
            })
            .collect()
    }
}

/// Measure the display width of a string in cells.
///
/// # Examples
///
/// ```
/// assert_eq!(presiterm::canvas::width("abc"), 3);
/// assert_eq!(presiterm::canvas::width("日本"), 4);
/// ```
#[must_use]
pub fn width(text: &str) -> usize {
    text.chars().map(|c| c.width().unwrap_or(0)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Style = Style::new(Rgb(255, 255, 255), Rgb(0, 0, 0));

    #[test]
    fn writes_clip_at_the_right_edge() {
        let mut canvas = Canvas::new(5, 1, S);
        assert_eq!(canvas.text(2, 0, "abcdef", S), 3);
        assert_eq!(canvas.row_text(0), "  abc");
    }

    #[test]
    fn every_ansi_row_is_exactly_cols_wide() {
        let mut canvas = Canvas::new(6, 2, S);
        canvas.text(0, 0, "ä中x", S.bold());
        for row in canvas.ansi_rows() {
            let plain: String = strip(&row);
            assert_eq!(width(&plain), 6, "{plain:?}");
        }
    }

    fn strip(text: &str) -> String {
        let (mut out, mut esc) = (String::new(), false);
        for c in text.chars() {
            if esc {
                esc = !c.is_ascii_alphabetic();
            } else if c == '\x1b' {
                esc = true;
            } else {
                out.push(c);
            }
        }
        out
    }

    proptest::proptest! {
        #[test]
        fn width_of_printable_ascii_is_its_length(text in "[ -~]{0,60}") {
            proptest::prop_assert_eq!(width(&text), text.len());
        }

        #[test]
        fn width_of_two_ascii_strings_adds_up(a in "[ -~]{0,30}", b in "[ -~]{0,30}") {
            proptest::prop_assert_eq!(width(&format!("{a}{b}")), width(&a) + width(&b));
        }
    }
}
