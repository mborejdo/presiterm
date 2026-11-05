use std::ffi::OsString;

use proptest::prelude::*;

use super::*;

/// Write `text` to `folder/name`, making the folders on the way.
fn write(folder: &Path, name: &str, text: &str) -> PathBuf {
    let path = folder.join(name);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, text).expect("write");
    path
}

/// A temp tree `root/a/b` with the deck folder `b`.
fn tree() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().expect("tempdir");
    let deck = root.path().join("a").join("b");
    std::fs::create_dir_all(&deck).expect("mkdir");
    (root, deck)
}

#[test]
fn resolve_font_size_prefers_flag_then_deck_then_window_then_default() {
    let config = Config::default();
    assert!((config.resolve_font_size(Some(1.0), Some(2.0), Some(3.0)) - 1.0).abs() < 1e-6);
    assert!((config.resolve_font_size(None, Some(2.0), Some(3.0)) - 2.0).abs() < 1e-6);
    assert!((config.resolve_font_size(None, None, Some(3.0)) - 3.0).abs() < 1e-6);
    assert!((config.resolve_font_size(None, None, None) - DEFAULT_FONT_SIZE).abs() < 1e-6);
}

#[test]
fn load_without_any_file_gives_the_defaults() {
    let (root, deck) = tree();
    let user = root.path().join("home").join("config.toml");

    let config = Config::load_from(&deck, Some(&user)).expect("load");

    assert!((config.font_size - DEFAULT_FONT_SIZE).abs() < 1e-6);
    assert_eq!(config.screen, DEFAULT_SCREEN);
    assert_eq!(config.grid, DEFAULT_GRID);
    assert_eq!(config.sources, Sources::default());
    assert_eq!(
        config.files,
        vec![ConfigFile {
            path: user,
            state: FileState::Missing
        }]
    );
}

#[test]
fn load_closest_presiterm_toml_wins_over_a_farther_one() {
    let (root, deck) = tree();
    write(root.path(), "presiterm.toml", "font_size = 20\n");
    let near = write(&root.path().join("a"), "presiterm.toml", "font_size = 30\n");

    let config = Config::load_from(&deck, None).expect("load");

    assert!((config.font_size - 30.0).abs() < 1e-6);
    assert_eq!(config.sources.font_size, Source::File(near.clone()));
    assert_eq!(config.files[0].state, FileState::Used);
    assert_eq!(config.files[0].path, near);
    assert_eq!(config.files[1].state, FileState::Shadowed);
}

#[test]
fn load_closest_file_does_not_borrow_keys_from_a_farther_presiterm_toml() {
    let (root, deck) = tree();
    write(root.path(), "presiterm.toml", "grid = \"90x30\"\n");
    write(&root.path().join("a"), "presiterm.toml", "font_size = 30\n");

    let config = Config::load_from(&deck, None).expect("load");

    assert_eq!(config.grid, DEFAULT_GRID, "a farther file is not read");
}

#[test]
fn load_key_missing_in_the_closest_file_falls_through_to_the_user_file() {
    let (root, deck) = tree();
    let project = write(&deck, "presiterm.toml", "font_size = 54\n");
    let user = write(
        root.path(),
        "home/config.toml",
        "font_size = 20\nscreen = \"1440x900\"\n",
    );

    let config = Config::load_from(&deck, Some(&user)).expect("load");

    assert!((config.font_size - 54.0).abs() < 1e-6);
    assert_eq!(config.sources.font_size, Source::File(project));
    assert_eq!(config.screen, (1440.0, 900.0));
    assert_eq!(config.sources.screen, Source::File(user));
    assert_eq!(config.grid, DEFAULT_GRID);
    assert_eq!(config.sources.grid, Source::Default);
}

#[test]
fn load_reads_font_size_given_as_integer_or_float() {
    let (_root, deck) = tree();
    write(&deck, "presiterm.toml", "font_size = 36.5\n");
    let config = Config::load_from(&deck, None).expect("load");
    assert!((config.font_size - 36.5).abs() < 1e-6);
}

#[test]
fn load_reads_screen_and_grid() {
    let (_root, deck) = tree();
    write(
        &deck,
        "presiterm.toml",
        "screen = \"1440x900\"\ngrid = \"80x24\"\n",
    );
    let config = Config::load_from(&deck, None).expect("load");
    assert_eq!(config.screen, (1440.0, 900.0));
    assert_eq!(config.grid, (80, 24));
}

#[test]
fn load_unknown_key_is_an_error_naming_the_file_and_the_key() {
    let (_root, deck) = tree();
    let file = write(&deck, "presiterm.toml", "font_size = 40\nfont_sise = 12\n");

    let message = format!(
        "{:#}",
        Config::load_from(&deck, None).expect_err("unknown key")
    );

    assert!(message.contains(&file.display().to_string()), "{message}");
    assert!(message.contains("font_sise"), "{message}");
    assert!(message.contains("unknown field"), "{message}");
}

#[test]
fn load_unknown_key_in_the_user_file_is_an_error_naming_that_file() {
    let (root, deck) = tree();
    let user = write(root.path(), "home/config.toml", "colour = \"red\"\n");

    let message = format!(
        "{:#}",
        Config::load_from(&deck, Some(&user)).expect_err("unknown key")
    );

    assert!(message.contains(&user.display().to_string()), "{message}");
    assert!(message.contains("colour"), "{message}");
}

#[test]
fn load_bad_value_is_an_error_naming_file_key_and_expected_form() {
    let cases = [
        (
            "font_size = \"big\"",
            "font_size: expected a positive number of points like 42, got \"big\"",
        ),
        (
            "font_size = -3",
            "font_size: expected a positive number of points like 42, got integer",
        ),
        (
            "font_size = true",
            "font_size: expected a positive number of points like 42, got boolean",
        ),
        (
            "screen = 1728",
            "screen: expected a string like \"1728x1117\", got integer",
        ),
        (
            "screen = \"wide\"",
            "screen: expected two numbers like 80x24, got \"wide\"",
        ),
        (
            "screen = \"0x900\"",
            "screen: expected two positive numbers like 1728x1117, got \"0x900\"",
        ),
        (
            "grid = \"80x24.5\"",
            "grid: expected two whole numbers like 58x20, got \"80x24.5\"",
        ),
        (
            "grid = \"80\"",
            "grid: expected two numbers like 80x24, got \"80\"",
        ),
    ];
    for (text, expected) in cases {
        let (_root, deck) = tree();
        let file = write(&deck, "presiterm.toml", text);

        let message = format!("{:#}", Config::load_from(&deck, None).expect_err(text));

        assert!(
            message.contains(&file.display().to_string()),
            "{text}: {message}"
        );
        assert!(message.contains(expected), "{text}: {message}");
    }
}

#[test]
fn load_syntax_error_is_an_error_naming_the_file() {
    let (_root, deck) = tree();
    let file = write(&deck, "presiterm.toml", "font_size = = 3\n");

    let message = format!("{:#}", Config::load_from(&deck, None).expect_err("syntax"));

    assert!(message.contains(&file.display().to_string()), "{message}");
}

#[test]
fn load_a_folder_named_presiterm_toml_is_skipped() {
    let (_root, deck) = tree();
    std::fs::create_dir_all(deck.join("presiterm.toml")).expect("mkdir");

    let config = Config::load_from(&deck, None).expect("load");

    assert_eq!(config.sources, Sources::default());
}

#[test]
fn resolve_precedence_is_flag_then_deck_then_file_then_default() {
    let (_root, deck) = tree();
    write(&deck, "presiterm.toml", "font_size = 50\n");
    let config = Config::load_from(&deck, None).expect("load");

    assert!((config.resolve_font_size(Some(60.0), Some(40.0), None) - 60.0).abs() < 1e-6);
    assert!((config.resolve_font_size(None, Some(40.0), None) - 40.0).abs() < 1e-6);
    assert!((config.resolve_font_size(None, None, None) - 50.0).abs() < 1e-6);
    assert!((Config::default().resolve_font_size(None, None, None) - 42.0).abs() < 1e-6);
}

#[test]
fn resolve_grid_and_screen_prefer_the_flag_over_the_file() {
    let (_root, deck) = tree();
    write(
        &deck,
        "presiterm.toml",
        "grid = \"80x24\"\nscreen = \"1440x900\"\n",
    );
    let config = Config::load_from(&deck, None).expect("load");

    assert_eq!(config.resolve_grid(None, None), (80, 24));
    assert_eq!(config.resolve_grid(Some(100), None), (100, 24));
    assert_eq!(config.resolve_grid(None, Some(30)), (80, 30));
    assert_eq!(config.resolve_screen(None), (1440.0, 900.0));
    assert_eq!(config.resolve_screen(Some((800.0, 600.0))), (800.0, 600.0));
}

#[test]
fn lint_grid_is_none_until_a_file_or_a_flag_sets_it() {
    let (_root, deck) = tree();
    let plain = Config::load_from(&deck, None).expect("load");
    assert_eq!(plain.lint_grid(None), None);
    assert_eq!(plain.lint_grid(Some((70, 20))), Some((70, 20)));

    write(&deck, "presiterm.toml", "grid = \"80x24\"\n");
    let set = Config::load_from(&deck, None).expect("load");
    assert_eq!(set.lint_grid(None), Some((80, 24)));
    assert_eq!(set.lint_grid(Some((70, 20))), Some((70, 20)));
}

#[test]
fn font_override_is_set_only_by_a_file() {
    let (_root, deck) = tree();
    assert_eq!(Config::default().font_override(), None);
    write(&deck, "presiterm.toml", "font_size = 50\n");
    let config = Config::load_from(&deck, None).expect("load");
    assert_eq!(config.font_override(), Some(50.0));
}

#[test]
fn display_lists_files_closest_first_and_the_source_of_each_value() {
    let (root, deck) = tree();
    let near = write(&deck, "presiterm.toml", "font_size = 50\n");
    let far = write(root.path(), "presiterm.toml", "font_size = 20\n");
    let user = root.path().join("home").join("config.toml");

    let text = Config::load_from(&deck, Some(&user))
        .expect("load")
        .to_string();

    let near_at = text.find(&near.display().to_string()).expect("near");
    let far_at = text.find(&far.display().to_string()).expect("far");
    assert!(near_at < far_at, "{text}");
    assert!(text.contains("not found"), "{text}");
    assert!(
        text.contains(&format!("font_size = 50  ({})", near.display())),
        "{text}"
    );
    assert!(text.contains("screen    = 1728x1117  (default)"), "{text}");
    assert!(text.contains("grid      = 58x20  (default)"), "{text}");
}

#[test]
fn user_config_path_prefers_xdg_then_home() {
    let xdg = Some(OsString::from("/x"));
    let home = Some(OsString::from("/h"));
    assert_eq!(
        user_config_path_from(xdg, home.clone()),
        Some(PathBuf::from("/x/presiterm/config.toml"))
    );
    assert_eq!(
        user_config_path_from(None, home.clone()),
        Some(PathBuf::from("/h/.config/presiterm/config.toml"))
    );
    assert_eq!(
        user_config_path_from(Some(OsString::new()), home),
        Some(PathBuf::from("/h/.config/presiterm/config.toml"))
    );
    assert_eq!(user_config_path_from(None, None), None);
}

#[test]
fn deck_dir_is_the_folder_of_a_file_and_a_folder_itself() {
    let (_root, deck) = tree();
    assert_eq!(deck_dir(&deck), deck);
    assert_eq!(deck_dir(&deck.join("slides.md")), deck);
    assert_eq!(deck_dir(Path::new("slides.md")), PathBuf::from("."));
}

#[test]
fn pair_reads_two_numbers_and_names_the_flag_when_it_cannot() {
    assert_eq!(
        pair::<f64>("1728x1117", "--screen").expect("pair"),
        DEFAULT_SCREEN
    );
    assert_eq!(
        pair::<f64>("80 X 24", "--grid").expect("pair"),
        (80.0, 24.0)
    );
    assert_eq!(pair("80x24", "--grid").expect("pair"), (80_usize, 24_usize));
    assert!(pair::<usize>("80.5x24", "--grid").is_err());
    for bad in ["80", "axb", "80x"] {
        let message = format!("{:#}", pair::<f64>(bad, "--grid").expect_err(bad));
        assert!(
            message.starts_with("--grid: expected two numbers like 80x24"),
            "{message}"
        );
    }
}

proptest! {
    #[test]
    fn pair_roundtrips_any_two_numbers(a in 1_usize..100_000, b in 1_usize..100_000) {
        prop_assert_eq!(pair::<usize>(&format!("{a}x{b}"), "t").expect("pair"), (a, b));
    }
}

#[test]
fn load_reads_a_theme_table_over_the_defaults_and_a_logo_next_to_the_file() {
    let (_root, deck) = tree();
    write(
        &deck,
        "presiterm.toml",
        "logo = \"art/logo.svg\"\n[theme.dark]\naccent = \"#ff8800\"\n[theme.fonts]\nmono = \"Hack\"\n",
    );
    let config = Config::load_from(&deck, None).expect("load");
    assert_eq!(config.theme.dark.accent, swatchbook::Rgb(0xff, 0x88, 0x00));
    assert_eq!(config.theme.light, swatchbook::Palette::LIGHT);
    assert_eq!(config.theme.fonts.mono, "Hack");
    let logo = config.logo.as_deref().expect("logo");
    assert!(
        logo.ends_with("art/logo.svg") && logo.starts_with(&deck),
        "{logo:?}"
    );
    let mut parsed = crate::deck::parse("# A\n", "t", &deck, false);
    config.dress(&mut parsed);
    assert_eq!(parsed.palette().accent, swatchbook::Rgb(0xff, 0x88, 0x00));
}

#[test]
fn load_bad_theme_names_the_file_and_the_key() {
    let (_root, deck) = tree();
    let file = write(
        &deck,
        "presiterm.toml",
        "[theme.dark]\naccent = \"orange\"\n",
    );
    let error = format!(
        "{:#}",
        Config::load_from(&deck, None).expect_err("bad colour")
    );
    assert!(
        error.contains(&file.display().to_string()) && error.contains("theme"),
        "{error}"
    );
    write(&deck, "presiterm.toml", "logo = 3\n");
    let error = format!(
        "{:#}",
        Config::load_from(&deck, None).expect_err("bad logo")
    );
    assert!(error.contains("logo: expected a path"), "{error}");
}
