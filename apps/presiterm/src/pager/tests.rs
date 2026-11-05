//! Pager tests: the terminal screen equals the canvas, resize, soft redraw, navigation and ctl.

use std::path::Path;

use super::*;
use crate::config::DEFAULT_GRID;
use crate::deck::parse;
use crate::render::Page;

const SRC: &str =
    "---\ntitle: Test\n---\n# One\n\n# Two\n+ a\n+ b\n\n# Three\n```bash\nls -la\n```\n";

#[test]
fn ctl_without_a_command_names_the_commands() {
    let message = format!("{:#}", ctl(None, &[]).expect_err("empty command"));
    assert!(
        message.contains("goto N") && message.contains("presiterm ctl --help"),
        "{message}"
    );
}

#[test]
fn ctl_on_a_dead_socket_says_which() {
    let path = std::env::temp_dir().join("presiterm-test-no-such.sock");
    let message = format!(
        "{:#}",
        ctl(Some(&path), &["status".to_string()]).expect_err("no listener")
    );
    assert!(message.starts_with("connect to "), "{message}");
    assert!(message.contains("presiterm-test-no-such.sock"), "{message}");
}

#[test]
fn an_edit_moves_to_its_slide() {
    let deck = parse(SRC, "t", Path::new("."), false);
    let edited = SRC.replace("ls -la", "ls -lah");
    let line = first_changed_line(SRC, &edited).unwrap();
    assert_eq!(slide_at(&deck, line), Some(2));
    assert_eq!(first_changed_line(SRC, SRC), None);
    assert_eq!(first_changed_line(SRC, &format!("{SRC}x\n")), Some(14));
    assert_eq!(slide_at(&deck, 2), None);
}

fn page(deck: &Deck, view: View, index: usize, step: usize) -> Page {
    let mut assets = Assets::with_cache(Path::new("."), deck, None);
    render::page(deck, index, step, view, &mut assets, Mode::Slide, "")
}

fn screen_rows(parser: &vt100::Parser) -> Vec<String> {
    let screen = parser.screen();
    (0..screen.size().0)
        .map(|r| {
            (0..screen.size().1)
                .map(|c| {
                    screen
                        .cell(r, c)
                        .map_or(' ', |cell| cell.contents().chars().next().unwrap_or(' '))
                })
                .collect()
        })
        .collect()
}

fn canvas_rows(page: &Page) -> Vec<String> {
    (0..page.canvas.rows)
        .map(|r| page.canvas.row_text(r))
        .collect()
}

#[test]
fn the_terminal_shows_exactly_the_canvas_after_slide_changes_and_steps() {
    let deck = parse(SRC, "x", Path::new("."), false);
    let view = View {
        cols: DEFAULT_GRID.0,
        rows: DEFAULT_GRID.1,
        cell: (50.0, 111.0),
    };
    let mut parser = vt100::Parser::new(20, 58, 0);
    let mut drawn = Drawn::default();
    for (index, step, full) in [
        (0, 0, true),
        (1, 0, true),
        (1, 1, false),
        (1, 2, false),
        (1, 1, false),
        (2, 0, true),
        (0, 0, true),
    ] {
        let p = page(&deck, view, index, step);
        let (bytes, now) = frame_bytes(&p, &drawn, full);
        parser.process(bytes.as_bytes());
        drawn = now;
        assert_eq!(
            screen_rows(&parser),
            canvas_rows(&p),
            "slide {index} step {step}"
        );
    }
}

#[test]
fn a_size_change_mid_run_leaves_no_stale_cells_and_the_same_slide() {
    let deck = parse(SRC, "x", Path::new("."), false);
    let big = View {
        cols: DEFAULT_GRID.0,
        rows: DEFAULT_GRID.1,
        cell: (50.0, 111.0),
    };
    let small = View {
        cols: 38,
        rows: 9,
        cell: (76.0, 220.0),
    };
    let mut parser = vt100::Parser::new(20, 58, 0);
    let (bytes, drawn) = frame_bytes(&page(&deck, big, 2, 0), &Drawn::default(), true);
    parser.process(bytes.as_bytes());
    parser.set_size(9, 38);
    let after = page(&deck, small, 2, 0);
    let (bytes, _) = frame_bytes(&after, &drawn, true);
    parser.process(bytes.as_bytes());
    assert_eq!(screen_rows(&parser), canvas_rows(&after));
    assert!(
        screen_rows(&parser).iter().any(|r| r.contains("ls -la")),
        "still slide 3"
    );
    assert_eq!(parser.screen().size(), (9, 38));
}

#[test]
fn a_soft_redraw_skips_the_rows_of_an_unchanged_image_and_resends_a_changed_one() {
    let deck = parse(
        "# A\n\n# B\n```mermaid steps\nflowchart LR\n a[One] --> b[Two]\n```\n",
        "x",
        Path::new("."),
        false,
    );
    let view = View {
        cols: DEFAULT_GRID.0,
        rows: DEFAULT_GRID.1,
        cell: (50.0, 111.0),
    };
    let first = page(&deck, view, 1, 0);
    let (full, drawn) = frame_bytes(&first, &Drawn::default(), true);
    assert!(full.contains("]1337;File=inline=1"));
    let (same, _) = frame_bytes(&first, &drawn, false);
    assert!(
        !same.contains("]1337;File"),
        "unchanged image must not be sent again"
    );
    let image = &first.images[0];
    assert!(
        !same.contains(&format!("\x1b[{};1H", image.row + 1)),
        "rows under the image are left alone"
    );
    let (changed, _) = frame_bytes(&page(&deck, view, 1, 1), &drawn, false);
    assert!(
        changed.contains("]1337;File"),
        "a lit node is a new picture"
    );
}

#[test]
fn navigation_walks_fragments_before_slides() {
    let deck = parse(SRC, "x", Path::new("."), false);
    assert_eq!(render::fragments(&deck.slides[1]), 2);
}

proptest::proptest! {
    #[test]
    fn first_changed_line_of_identical_text_is_none(text in "[a-z\\n]{0,60}") {
        proptest::prop_assert_eq!(first_changed_line(&text, &text), None);
    }

    #[test]
    fn first_changed_line_after_an_appended_line_is_the_one_after_the_old_last(
        lines in proptest::collection::vec("[a-z]{1,5}", 0..8),
        extra in "[a-z]{1,5}",
    ) {
        let old = lines.join("\n");
        let new = lines.iter().chain(std::iter::once(&extra)).cloned().collect::<Vec<_>>().join("\n");
        proptest::prop_assert_eq!(first_changed_line(&old, &new), Some(lines.len() + 1));
    }
}
