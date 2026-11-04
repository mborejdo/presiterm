//! Parse tests: blocks, inlines, directives and their nesting, line numbers.

use crate::{Align, Inline, Kind, parse, plain_text};

fn kinds(source: &str) -> Vec<Kind> {
    parse(source).blocks.into_iter().map(|b| b.kind).collect()
}

#[test]
fn a_dashed_line_under_text_is_a_rule_not_a_heading() {
    let doc = parse("text\n---\nmore\n");
    assert!(matches!(doc.blocks[1].kind, Kind::Rule));
    assert_eq!(doc.blocks[2].line, 3);
}

#[test]
fn lines_count_from_the_top_of_the_file_frontmatter_included() {
    let doc = parse("---\ntitle: T\n---\n\n# A\n");
    assert_eq!(doc.frontmatter.get("title"), Some("T"));
    assert_eq!(doc.blocks[0].line, 5);
}

#[test]
fn inline_markup_maps_to_its_own_variant() {
    let Kind::Paragraph(content) = &kinds("a **b** *c* `d` __e__ ==f== ~~g~~\nh")[0] else {
        panic!("paragraph")
    };
    assert!(matches!(&content[1], Inline::Strong(_)));
    assert!(matches!(&content[3], Inline::Emph(_)));
    assert!(matches!(&content[5], Inline::Code(c) if c == "d"));
    assert!(matches!(&content[7], Inline::Underline(_)));
    assert!(matches!(&content[9], Inline::Highlight(_)));
    assert!(matches!(&content[11], Inline::Strike(_)));
    assert!(matches!(&content[12], Inline::SoftBreak));
    assert_eq!(plain_text(content), "a b c d e f g h");
}

#[test]
fn snake_case_stays_text_and_escapes_are_text() {
    let Kind::Paragraph(content) = &kinds("snake_case_word \\*x\\*")[0] else {
        panic!("paragraph")
    };
    assert_eq!(content, &[Inline::Text("snake_case_word *x*".into())]);
}

#[test]
fn lists_keep_marker_start_and_nesting() {
    let Kind::List(list) = &kinds("3. a\n4. b\n   - c\n")[0] else {
        panic!("list")
    };
    assert_eq!(
        (list.start, list.marker, list.items.len()),
        (Some(3), '.', 2)
    );
    assert!(matches!(&list.items[1].blocks[1].kind, Kind::List(inner) if inner.marker == '-'));
    assert_eq!(list.items[1].line, 2);
}

#[test]
fn a_table_has_head_rows_and_alignment() {
    let Kind::Table(table) = &kinds("| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |\n")[0] else {
        panic!("table")
    };
    assert_eq!(table.align, [Align::Left, Align::Center, Align::Right]);
    assert_eq!(plain_text(&table.head[0]), "a");
    assert_eq!(plain_text(&table.rows[0][2]), "3");
}

#[test]
fn code_keeps_info_and_text_and_hides_directives() {
    let kinds = kinds("```mermaid steps h=4\n:::{x}\n```\n");
    assert_eq!(
        kinds,
        [Kind::Code {
            info: "mermaid steps h=4".into(),
            text: ":::{x}\n".into()
        }]
    );
}

#[test]
fn directives_nest_by_colon_count_and_keep_their_raw_body() {
    let doc = parse("::::{outer} arg\n:k: v\n:::{inner}\n- a\n:::\ntext\n::::\nafter\n");
    let Kind::Directive(outer) = &doc.blocks[0].kind else {
        panic!("outer")
    };
    assert_eq!(outer.argument, "arg");
    assert_eq!(outer.option("k"), Some("v"));
    assert_eq!(outer.raw, ":::{inner}\n- a\n:::\ntext\n");
    assert!(matches!(&outer.blocks[0].kind, Kind::Directive(inner) if inner.name == "inner"));
    assert!(matches!(&outer.blocks[1].kind, Kind::Paragraph(_)));
    assert!(matches!(&doc.blocks[1].kind, Kind::Paragraph(_)));
}

#[test]
fn a_directive_without_braces_takes_its_first_word_as_the_name() {
    let Kind::Directive(notes) = &kinds("::: notes\nsay this\n:::\n")[0] else {
        panic!("directive")
    };
    assert_eq!(notes.name, "notes");
    assert_eq!(notes.raw, "say this\n");
}

#[test]
fn an_unclosed_directive_runs_to_the_end() {
    let Kind::Directive(open) = &kinds(":::{x}\na\n\nb\n")[0] else {
        panic!("directive")
    };
    assert_eq!(open.blocks.len(), 2);
    assert_eq!(open.raw, "a\n\nb\n");
}

#[test]
fn images_carry_url_title_and_alt() {
    let Kind::Paragraph(content) = &kinds("![a *b*](p.png \"t\"){h=3}")[0] else {
        panic!("paragraph")
    };
    assert_eq!(
        content[0],
        Inline::Image {
            url: "p.png".into(),
            title: "t".into(),
            alt: "a b".into()
        }
    );
    assert_eq!(content[1], Inline::Text("{h=3}".into()));
}

proptest::proptest! {
    #[test]
    fn parse_never_panics(source in "[a-z:#*_`>|{}\\- \n]{0,120}") {
        let _ = parse(&source);
    }

    #[test]
    fn plain_words_survive_as_plain_text(text in "[a-zA-Z][a-zA-Z0-9 ,.]{0,59}") {
        let doc = parse(&text);
        let joined: String = doc
            .blocks
            .iter()
            .filter_map(|b| match &b.kind {
                Kind::Paragraph(content) => Some(plain_text(content)),
                _ => None,
            })
            .collect();
        proptest::prop_assert_eq!(joined, text.trim());
    }
}
