//! The decks in `examples/` through the public API: parse, lay out, and nothing on the way breaks.
//!
//! Read-only: the picture cache is switched off (`Assets::with_cache(.., None)`), so no test writes
//! outside its own process.

#![allow(
    clippy::expect_used,
    reason = "test helpers fail the test on a broken example"
)]

use std::path::{Path, PathBuf};

use presiterm::deck::{self, Block};
use presiterm::diagram::Assets;
use presiterm::render::{self, Mode, View};

const VIEW: View = View {
    cols: 120,
    rows: 40,
    cell: (50.0, 111.0),
};

fn example_decks() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut decks: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("examples/ is next to apps/")
        .flatten()
        .map(|entry| entry.path().join("slides.md"))
        .filter(|path| path.exists())
        .collect();
    decks.sort();
    decks
}

fn parse(path: &Path) -> deck::Deck {
    let source = std::fs::read_to_string(path).expect("read the example deck");
    deck::parse(&source, "example", path.parent().expect("deck dir"), false)
}

#[test]
fn examples_exist_so_the_other_tests_here_check_something() {
    assert!(example_decks().len() >= 3, "{:?}", example_decks());
}

#[test]
fn every_example_deck_parses_without_an_error_block() {
    for path in example_decks() {
        let deck = parse(&path);
        let errors: Vec<_> = deck
            .slides
            .iter()
            .flat_map(|slide| &slide.blocks)
            .filter(|block| matches!(block, Block::Error(_)))
            .collect();
        assert!(errors.is_empty(), "{}: {errors:?}", path.display());
    }
}

#[test]
fn every_example_slide_lays_out_exactly_the_grid_wide() {
    for path in example_decks() {
        let deck = parse(&path);
        let mut assets = Assets::with_cache(path.parent().expect("deck dir"), &deck, None);
        for index in 0..deck.slides.len() {
            let page = render::page(&deck, index, 0, VIEW, &mut assets, Mode::Slide, "");
            assert_eq!(
                page.canvas.rows,
                VIEW.rows,
                "{} slide {index}",
                path.display()
            );
            assert_eq!(
                page.canvas.cols,
                VIEW.cols,
                "{} slide {index}",
                path.display()
            );
        }
    }
}
