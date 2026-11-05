//! Dialect tests: slides, blocks, notes, fences, tables and inline markup.

use super::*;

fn deck(src: &str) -> Deck {
    parse(src, "x", Path::new("."), false)
}

#[test]
fn splits_on_headings_and_rules() {
    let d = deck("---\ntitle: T\n---\n# A\n\ntext\n\n---\n\n## B\n- x\n- y\n");
    assert_eq!(d.title, "T");
    assert_eq!(d.slides.len(), 2);
    assert_eq!(d.slides[0].title().as_deref(), Some("A"));
    assert_eq!(d.slides[1].title().as_deref(), Some("B"));
    assert_eq!((d.slides[0].line, d.slides[1].line), (4, 10));
}

#[test]
fn rule_inside_a_fence_does_not_split() {
    let d = deck("# A\n```bash\n---\n# not a heading\n```\n");
    assert_eq!(d.slides.len(), 1);
    assert!(matches!(&d.slides[0].blocks[0], Block::Code { lines, .. } if lines.len() == 2));
}

#[test]
fn a_dashed_line_under_text_is_a_slide_break_not_a_heading() {
    let d = deck("# A\ntext\n---\nnext\n");
    assert_eq!(d.slides.len(), 2);
    assert_eq!(d.slides[1].line, 4);
}

#[test]
fn notes_are_collected_and_never_blocks() {
    let d = deck("# A\ntext\n\nNote: say this\nand this\n\n::: notes\nsecond\n:::\n");
    assert_eq!(d.slides[0].notes, "say this\nand this\nsecond");
    assert_eq!(d.slides[0].blocks.len(), 1);
}

#[test]
fn plus_bullets_reveal_and_reveal_flag_reveals_all() {
    let d = deck("# A\n- a\n+ b\n");
    let Block::List(items) = &d.slides[0].blocks[0] else {
        panic!("list")
    };
    assert_eq!([items[0].reveal, items[1].reveal], [false, true]);
    let all = parse("# A\n- a\n- b\n", "x", Path::new("."), true);
    let Block::List(items) = &all.slides[0].blocks[0] else {
        panic!("list")
    };
    assert!(items.iter().all(|i| i.reveal));
}

#[test]
fn lists_number_from_their_start_and_nest_by_level() {
    let d = deck("# A\n3. a\n4. b\n   - c\n     continued\n");
    let Block::List(items) = &d.slides[0].blocks[0] else {
        panic!("list")
    };
    assert_eq!(
        items
            .iter()
            .map(|i| (i.level, i.number))
            .collect::<Vec<_>>(),
        [(0, Some(3)), (0, Some(4)), (1, None)]
    );
    assert_eq!(plain(&items[2].spans), "c continued");
}

#[test]
fn marked_text_is_underlined() {
    let spans = inline("a ==b== c __d__ snake_case");
    assert_eq!(plain(&spans), "a b c d snake_case");
    assert!(spans[1].underline && spans[3].underline);
    assert!(!spans[0].underline && !spans[2].underline && !spans[4].underline);
}

#[test]
fn inline_markup_and_hard_breaks() {
    let spans = inline("a **b** *c* `d` [e](http://x)");
    assert_eq!(plain(&spans), "a b c d e");
    assert!(spans[1].bold && spans[3].italic && spans[5].code);
    let d = deck("# A\nline one\nline two\n");
    assert!(matches!(&d.slides[0].blocks[0], Block::Paragraph(lines) if lines.len() == 2));
}

#[test]
fn heading_markup_is_kept_as_spans() {
    let d = deck("# The ==one== thing\n");
    let heading = d.slides[0].heading.as_ref().expect("heading");
    assert!(heading[1].underline);
    assert_eq!(d.slides[0].title().as_deref(), Some("The one thing"));
}

#[cfg(feature = "typst")]
#[test]
fn a_table_is_a_typst_picture_with_its_alignment() {
    let d = deck("# A\n| a | b |\n|---|--:|\n| 1 | 2 |\n");
    let Block::Diagram(g) = &d.slides[0].blocks[0] else {
        panic!("table picture")
    };
    assert!(g.typst && !g.steps);
    assert!(g.source.contains("align: (left, right,)"), "{}", g.source);
    assert!(g.source.contains("[1], [2]"), "{}", g.source);
}

#[cfg(not(feature = "typst"))]
#[test]
fn a_table_is_text_without_typst() {
    let d = deck("# A\n| a | b |\n|---|--:|\n| 1 | 2 |\n");
    assert!(matches!(&d.slides[0].blocks[0], Block::Table { align, .. } if align == &['l', 'r']));
}

#[test]
fn a_typst_fence_takes_steps_and_sizes_from_its_info_string() {
    let d = deck("# A\n```typst steps=3 h=5 w=40\n#diagram(node((0,0), [x]))\n```\n");
    let Block::Diagram(g) = &d.slides[0].blocks[0] else {
        panic!("diagram")
    };
    assert!(g.typst && g.steps && g.height == Some(5) && g.width == Some(40));
    assert_eq!(g.nodes, ["3"]);
}

#[test]
fn a_quote_is_a_quote_of_its_lines() {
    let d = deck("# A\ntext\n\n> a quote\n> two lines\n");
    assert!(matches!(&d.slides[0].blocks[1], Block::Quote(lines) if lines.len() == 2));
    let odd = deck("# A\n> - a list in a quote\n");
    assert!(matches!(&odd.slides[0].blocks[0], Block::Error(_)));
}

#[test]
fn diagram_fences_and_attributes() {
    let d = deck(
        "# A\n```mermaid steps h=9 w=30\nflowchart LR\n a-->b\n```\n![k](a.png){h=7,w=20,c}\n![x](d.mmd)\n",
    );
    let Block::Diagram(g) = &d.slides[0].blocks[0] else {
        panic!("diagram")
    };
    assert!(g.steps && g.height == Some(9) && g.width == Some(30));
    assert!(
        matches!(&d.slides[0].blocks[1], Block::Picture(p) if p.height == Some(7) && p.width == Some(20))
    );
    assert!(matches!(&d.slides[0].blocks[2], Block::Error(e) if e.starts_with("d.mmd")));
}

#[test]
fn a_picture_between_lines_of_text_splits_the_paragraph() {
    let d = deck("# A\nbefore\n![p](p.png)\nafter\n");
    let kinds: Vec<&str> = d.slides[0]
        .blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph(_) => "text",
            Block::Picture(_) => "picture",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["text", "picture", "text"]);
}

#[test]
fn only_blocks_follow_the_tags_in_the_frontmatter() {
    let d = deck(
        "---\ntags: long\n---\n# A\n:::{only} long\nkept\n:::\n\n:::{only} short\ndropped\n:::\n",
    );
    assert_eq!(d.slides[0].blocks.len(), 1);
    let broken = deck("# A\n:::{only} (long\nx\n:::\n");
    assert!(matches!(&broken.slides[0].blocks[0], Block::Error(e) if e.starts_with("only:")));
}

#[test]
fn an_unknown_directive_shows_its_body() {
    let d = deck("# A\n:::{card} title\n- x\n:::\n");
    assert!(matches!(&d.slides[0].blocks[0], Block::List(items) if items.len() == 1));
}

#[test]
fn parse_frontmatter_sets_font_size_and_theme() {
    let d = deck("---\ntitle: \"X\"\nfont-size: 36\ntheme: light\n---\n# A\n");
    assert_eq!((d.font_size, d.theme), (Some(36.0), Theme::Light));
}

proptest::proptest! {
    #[test]
    fn inline_text_without_markup_survives_as_plain_text(text in "[a-zA-Z][a-zA-Z0-9 ,.]{0,59}") {
        proptest::prop_assert_eq!(plain(&inline(&text)), text.trim_end());
    }

    #[test]
    fn inline_never_panics_on_any_text(text in ".{0,80}") {
        let _ = inline(&text);
    }

    #[test]
    fn parse_never_panics_and_every_slide_starts_on_a_real_line(source in "[a-z#\\-+*>:{}`| \n]{0,160}") {
        let d = parse(&source, "x", Path::new("."), false);
        for slide in &d.slides {
            proptest::prop_assert!(slide.line >= 1 && slide.line <= source.lines().count() + 1);
        }
    }
}
