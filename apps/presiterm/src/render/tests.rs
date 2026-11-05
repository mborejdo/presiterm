//! Layout tests: grid sizes, reveal, notes, overflow, error slides and diagram steps.

use std::path::Path;

use super::text::{spans_to_cells, wrap};
use super::*;
use crate::canvas::width as w;
use crate::config::DEFAULT_GRID;
use crate::deck::parse;

const VIEW: View = View {
    cols: DEFAULT_GRID.0,
    rows: DEFAULT_GRID.1,
    cell: (50.0, 111.0),
};

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

fn render(src: &str, index: usize, step: usize, view: View) -> Page {
    let deck = parse(src, "Deck", Path::new("."), false);
    let mut assets = Assets::with_cache(Path::new("."), &deck, None);
    page(&deck, index, step, view, &mut assets, Mode::Slide, "")
}

#[test]
fn every_row_is_exactly_the_grid_wide_at_any_size() {
    let src = "# Title\n\nText with **bold** and a very long sentence that has to wrap because it does not fit.\n\n```bash\nssh root@host 'docker compose ps --format \"{{.Names}} {{.Status}} and much more\"'\n```\n\n| a | b |\n|---|---|\n| 1 | two |\n";
    for (cols, rows) in [(58, 20), (38, 9), (80, 24), (120, 40), (20, 6)] {
        let page = render(
            src,
            0,
            0,
            View {
                cols,
                rows,
                cell: (50.0, 111.0),
            },
        );
        for row in page.canvas.ansi_rows() {
            assert_eq!(w(&strip(&row)), cols, "{cols}x{rows}: {row:?}");
        }
    }
}

#[test]
fn body_sits_in_column_three_and_heading_is_centred() {
    let page = render("# One\n\n# Two\n- point\n", 1, 0, VIEW);
    let heading = page.canvas.row_text(2);
    let lead = heading.len() - heading.trim_start().len();
    let trail = heading.len() - heading.trim_end().len();
    assert!(lead.abs_diff(trail) <= 1, "{heading:?}");
    assert!(
        page.canvas.row_text(5).starts_with("  • point"),
        "{:?}",
        page.canvas.row_text(5)
    );
}

#[test]
fn a_code_span_does_not_paint_the_space_after_it() {
    let pal = Palette::DARK;
    let cells = spans_to_cells(&crate::deck::inline("a `b c` d"), &pal, text_style(&pal));
    let line = &wrap(&cells, 20)[0];
    let at = |i: usize| line[i].1.bg;
    assert_eq!(at(0), pal.ground);
    assert_eq!(at(1), pal.ground, "space before the code");
    assert_eq!(at(3), pal.surface, "space inside the code");
    assert_eq!(at(5), pal.ground, "space after the code");
}

#[test]
fn heading_markup_is_drawn_not_shown() {
    let page = render("# One\n\n# The ==word== here\ntext\n", 1, 0, VIEW);
    let rows: Vec<String> = (0..VIEW.rows).map(|r| page.canvas.row_text(r)).collect();
    let row = rows
        .iter()
        .position(|r| r.contains("The word here"))
        .expect("heading");
    assert!(!rows[row].contains("=="));
    let col = rows[row].find("word").unwrap();
    assert!(page.canvas.style_at(col, row).underline.is_some());
    assert!(page.canvas.style_at(col - 2, row).underline.is_none());
}

#[test]
fn footer_has_title_and_counter() {
    let page = render("---\ntitle: My Deck\n---\n# A\n\n# B\ntext\n", 1, 0, VIEW);
    let footer = page.canvas.row_text(19);
    assert!(footer.starts_with("  My Deck"), "{footer:?}");
    assert!(footer.trim_end().ends_with("[2/2]"), "{footer:?}");
}

#[test]
fn footer_counter_survives_a_narrow_grid_and_a_long_note() {
    let deck = parse(
        "# A\n\n# B\ntext\n",
        "A very long title",
        Path::new("."),
        false,
    );
    let mut assets = Assets::with_cache(Path::new("."), &deck, None);
    let view = View {
        cols: 34,
        rows: 12,
        cell: (76.0, 160.0),
    };
    let page = page(
        &deck,
        1,
        0,
        view,
        &mut assets,
        Mode::Slide,
        "reload failed: something went wrong",
    );
    let footer = page.canvas.row_text(11);
    assert!(footer.trim_end().ends_with("[2/2]"), "{footer:?}");
}

#[test]
fn nothing_reaches_the_gutter() {
    let page = render(
        "# A\n\n# B\na really very long word: Pneumonoultramicroscopicsilicovolcanoconiosisxyzä and more text behind it\n",
        1,
        0,
        VIEW,
    );
    for row in 0..20 {
        let text = page.canvas.row_text(row);
        assert!(
            text.chars().skip(56).all(char::is_whitespace),
            "row {row}: {text:?}"
        );
        assert!(
            text.chars().take(2).all(char::is_whitespace),
            "row {row}: {text:?}"
        );
    }
}

#[test]
fn reveal_bullets_appear_one_per_step_and_keep_their_rows() {
    let src = "# A\n\n# B\n+ one\n+ two\n+ three\n";
    let at = |step| render(src, 1, step, VIEW);
    assert_eq!(at(0).fragments, 3);
    assert!(!at(0).canvas.row_text(5).contains("one"));
    assert!(at(1).canvas.row_text(5).contains("one") && !at(1).canvas.row_text(6).contains("two"));
    assert!(at(3).canvas.row_text(7).contains("three"));
}

#[test]
fn notes_never_show_on_the_slide_but_on_the_notes_page() {
    let src = "# A\n\n# B\ntext\n\nNote: sag das laut\n";
    let deck = parse(src, "D", Path::new("."), false);
    let mut assets = Assets::with_cache(Path::new("."), &deck, None);
    let slide = page(&deck, 1, 0, VIEW, &mut assets, Mode::Slide, "");
    assert!((0..20).all(|r| !slide.canvas.row_text(r).contains("sag das")));
    let notes = page(&deck, 1, 0, VIEW, &mut assets, Mode::Notes, "");
    assert!((0..20).any(|r| notes.canvas.row_text(r).contains("sag das laut")));
}

#[test]
fn overflow_is_reported_and_clipped_not_wrapped_into_the_footer() {
    let body = "- punkt\n".repeat(30);
    let page = render(&format!("# A\n\n# B\n{body}"), 1, 0, VIEW);
    assert!(page.overflow > 0);
    assert!(
        page.canvas.row_text(18).trim().is_empty(),
        "spare row: {:?}",
        page.canvas.row_text(18)
    );
    assert!(page.canvas.row_text(19).contains("rows too many"));
}

#[test]
fn missing_picture_becomes_a_visible_error() {
    let page = render("# A\n\n# B\n![x](nope.png)\n", 1, 0, VIEW);
    assert!(
        page.problems[0].starts_with("nope.png: missing image"),
        "{:?}",
        page.problems
    );
    assert!((0..20).any(|r| page.canvas.row_text(r).contains('⚠')));
}

#[test]
fn diagram_fills_the_rest_and_steps_light_one_node_each() {
    let src = "# A\n\n# B\n```mermaid steps\nflowchart LR\n a[One] --> b[Two] --> c[Three]\n```\n";
    let at = |step| render(src, 1, step, VIEW);
    let p0 = at(0);
    assert_eq!(p0.fragments, 3);
    assert_eq!(p0.images.len(), 1, "the diagram, no logo without a config");
    let image = &p0.images[0];
    assert!(
        image.row >= 4 && image.row + image.rows <= 18,
        "{:?}",
        (image.row, image.rows)
    );
    assert!(image.col + image.cols <= 56 && image.col >= 2);
    assert_ne!(p0.images[0].bytes, at(1).images[0].bytes);
    assert_ne!(at(1).images[0].bytes, at(2).images[0].bytes);
    assert_eq!(at(3).images[0].bytes, at(9).images[0].bytes);
}

#[test]
fn a_configured_logo_sits_one_row_high_in_the_top_left_corner() {
    let folder = tempfile::tempdir().expect("temp dir");
    let logo = folder.path().join("logo.svg");
    std::fs::write(
        &logo,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40\" height=\"10\"><rect width=\"40\" height=\"10\" fill=\"#7cc4fa\"/></svg>",
    )
    .expect("write logo");
    let mut deck = parse("# A\ntext\n", "Deck", Path::new("."), false);
    deck.logo = Some(logo);
    let mut assets = Assets::with_cache(Path::new("."), &deck, None);
    let page = page(&deck, 0, 0, VIEW, &mut assets, Mode::Slide, "");
    let placed = page.images.last().expect("logo");
    assert_eq!((placed.row, placed.rows, placed.col), (0, 1, 2));
    deck.logo = Some(folder.path().join("missing.svg"));
    let mut assets = Assets::with_cache(Path::new("."), &deck, None);
    let broken = super::page(&deck, 0, 0, VIEW, &mut assets, Mode::Slide, "");
    assert!(
        broken.problems.iter().any(|p| p.starts_with("logo:")),
        "{:?}",
        broken.problems
    );
}

proptest::proptest! {
    #[test]
    fn wrap_keeps_every_line_within_the_width_and_loses_no_letter(
        text in "[a-z ]{0,80}",
        max in 1usize..30,
    ) {
        let style = Style::new(crate::canvas::Rgb(0, 0, 0), crate::canvas::Rgb(1, 1, 1));
        let cells: Vec<(char, Style)> = text.chars().map(|c| (c, style)).collect();
        let lines = wrap(&cells, max);
        for line in &lines {
            proptest::prop_assert!(line.iter().map(|(c, _)| w(&c.to_string())).sum::<usize>() <= max);
        }
        let kept: String = lines.iter().flatten().map(|(c, _)| *c).filter(|c| *c != ' ').collect();
        let expected: String = text.chars().filter(|c| *c != ' ').collect();
        proptest::prop_assert_eq!(kept, expected);
    }
}

const SAMPLE: &str = "---\ntitle: Sample\n---\n# Title\n\n# List\nText with **bold** and `code`.\n\n- one\n- two\n\n```bash\nssh root@host ls\n```\n\n| a | b |\n|---|---|\n| 1 | two |\n";

fn canvas_text(page: &Page) -> String {
    (0..page.canvas.rows)
        .map(|row| page.canvas.row_text(row).trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn title_slide_canvas_matches_snapshot() {
    insta::assert_snapshot!(canvas_text(&render(SAMPLE, 0, 0, VIEW)));
}

// The table is a picture with Typst and text without it: the snapshot is of the default build.
#[cfg(feature = "typst")]
#[test]
fn list_code_and_table_slide_canvas_matches_snapshot() {
    insta::assert_snapshot!(canvas_text(&render(SAMPLE, 1, 2, VIEW)));
}
