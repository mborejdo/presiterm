//! Diagram tests: stepping, Typst fences, mermaid errors and picture sizes.

use super::steps::number_nodes;
use super::*;

#[test]
fn plain_nodes_are_numbered_in_order_and_stepnodes_left_alone() {
    let (body, n) = number_nodes("#diagram(node((0,0), [a]), edge(), node((1,0), [b]))", &[]);
    assert_eq!(n, 2);
    assert!(
        body.contains("stepnode(1, (0,0)") && body.contains("stepnode(2, (1,0)"),
        "{body}"
    );
    assert!(body.starts_with("#diagram("));
    let (same, none) = number_nodes("stepnode(1, (0,0), [a])", &[]);
    assert_eq!((same.as_str(), none), ("stepnode(1, (0,0), [a])", 0));
    let (grouped, steps) = number_nodes(
        "node((0,0), [a]) node((1,0), [b]) node((2,0), [c])",
        &[1, 1, 2],
    );
    assert_eq!(steps, 2);
    assert!(
        grouped.contains("stepnode(1, (1,0)") && grouped.contains("stepnode(2, (2,0)"),
        "{grouped}"
    );
}

fn dark() -> Deck {
    crate::deck::parse("", "t", Path::new("."), false)
}

/// Whether the diagram cannot be drawn.
fn broken(d: &Diagram) -> bool {
    Assets::with_cache(Path::new("."), &dark(), None)
        .aspect(d)
        .is_err()
}

fn diagram(source: &str, steps: bool) -> Diagram {
    Diagram {
        source: source.into(),
        typst: false,
        height: None,
        width: None,
        steps,
        nodes: Vec::new(),
        origin: "test".into(),
    }
}

#[test]
fn step_nodes_follow_the_chain_in_order() {
    let d = diagram(
        "flowchart LR\n  q[Container] --> a(Alloy)\n  a -->|logs| l[(Loki)] --> g{Grafana}\n  p --> g\n",
        true,
    );
    assert_eq!(step_nodes(&d), ["q", "a", "l", "g", "p"]);
}

#[test]
fn image_size_png_header_gives_width_and_height() {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&5846u32.to_be_bytes());
    png.extend_from_slice(&900u32.to_be_bytes());
    assert_eq!(image_size(&png), Some((5846, 900)));
    assert_eq!(image_size(b"GIF89a\x80\x02\x68\x01rest"), Some((640, 360)));
    let jpg = [
        0xff, 0xd8, 0xff, 0xc0, 0, 17, 8, 1, 0x68, 2, 0x80, 3, 0, 0, 0, 0, 0, 0,
    ];
    assert_eq!(image_size(&jpg), Some((640, 360)));
}

#[test]
fn mermaid_draws_to_a_png_of_the_requested_height_and_lit_frames_differ() {
    let d = diagram("flowchart LR\n  a[One] --> b[Two]\n", true);
    let mut assets = Assets::with_cache(Path::new("."), &dark(), None);
    let plain = assets.diagram_png(&d, 0, 300).expect("draws");
    let lit = assets.diagram_png(&d, 1, 300).expect("draws lit");
    assert_eq!(image_size(&plain).map(|s| s.1), Some(300));
    assert_ne!(plain, lit, "a lit node must change the picture");
    assert!(assets.aspect(&d).expect("aspect") > 1.0);
}

#[cfg(feature = "typst")]
#[test]
fn a_typst_fence_draws_a_wide_transparent_png_and_a_broken_one_is_an_error() {
    let mut d = diagram(
        "#diagram(node((0,0), [a]), edge(\"->\"), node((1,0), [b]))",
        false,
    );
    d.typst = true;
    let mut assets = Assets::with_cache(Path::new("."), &dark(), None);
    let png = assets.diagram_png(&d, 0, 300).expect("draws");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(png[25], 6, "RGBA, so the ground shows through");
    assert!(assets.aspect(&d).expect("aspect") > 1.0);
    assert_eq!(Assets::steps(&d), 0);
    d.source = "#diagram(stepnode(1, (0,0), [a]), edge(\"->\"), stepnode(2, (1,0), [b]))".into();
    d.steps = true;
    d.nodes = vec!["2".into()];
    assert_eq!(Assets::steps(&d), 2);
    let (plain, lit) = (
        assets.diagram_png(&d, 0, 300).expect("draws"),
        assets.diagram_png(&d, 1, 300).expect("draws lit"),
    );
    assert_ne!(plain, lit);
    d.source = "#diagram(".into();
    assert!(broken(&d));
}

#[test]
fn broken_mermaid_is_an_error_not_a_picture() {
    assert!(broken(&diagram("this is not mermaid at all", false)));
}

proptest::proptest! {
    #[test]
    fn number_nodes_numbers_every_plain_node_once_and_a_second_pass_changes_nothing(count in 0usize..8) {
        let body = "node((0,0), [x]) ".repeat(count);
        let (numbered, steps) = number_nodes(&body, &[]);
        proptest::prop_assert_eq!(steps, count);
        proptest::prop_assert_eq!(numbered.matches("stepnode(").count(), count);
        let (again, again_steps) = number_nodes(&numbered, &[]);
        proptest::prop_assert_eq!(again, numbered);
        proptest::prop_assert_eq!(again_steps, 0);
    }
}
