//! The Typst engine a deck draws with: the vendored Inter and Fira Code plus the font files its
//! config names (`fonts = ["fonts", "extra/Face.otf"]`). Fonts are parsed once per distinct list
//! and the engine is shared by everything that draws with it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use anyhow::{Context, Result, bail, ensure};
use typstkit::Engine;

/// File extensions that count as a font in a folder.
const EXTENSIONS: [&str; 3] = ["otf", "ttf", "ttc"];

/// The engine for `faces`: a file is loaded as it is, a folder by its font files in name order.
/// A path that does not exist, or a folder without a font file, is an error.
pub fn engine(faces: &[PathBuf]) -> Result<Arc<Engine>> {
    static ENGINES: OnceLock<Mutex<HashMap<Vec<PathBuf>, Arc<Engine>>>> = OnceLock::new();
    let engines = ENGINES.get_or_init(Mutex::default);
    if let Some(engine) = engines
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(faces)
    {
        return Ok(Arc::clone(engine));
    }
    let mut builder = Engine::builder();
    for file in files(faces)? {
        let bytes = std::fs::read(&file).with_context(|| format!("fonts: {}", file.display()))?;
        builder = builder.font(bytes);
    }
    let engine = Arc::new(builder.build());
    engines
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(faces.to_vec(), Arc::clone(&engine));
    Ok(engine)
}

/// The font files `faces` name: files as they are, folders expanded to their font files.
fn files(faces: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for face in faces {
        if face.is_file() {
            files.push(face.clone());
        } else if face.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(face)
                .with_context(|| format!("fonts: {}", face.display()))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| is_font(path))
                .collect();
            ensure!(
                !found.is_empty(),
                "fonts: {}: no font file (.otf, .ttf, .ttc) in this folder",
                face.display()
            );
            found.sort();
            files.append(&mut found);
        } else {
            bail!("fonts: {}: missing", face.display());
        }
    }
    Ok(files)
}

/// Whether `path` has a font file's extension.
fn is_font(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            EXTENSIONS
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_lists_its_font_files_in_name_order_and_a_file_stays_as_it_is() {
        let dir = tempfile::tempdir().expect("tempdir");
        for name in ["b.ttf", "a.OTF", "notes.txt", "c.ttc"] {
            std::fs::write(dir.path().join(name), b"").expect("write");
        }
        let single = dir.path().join("notes.txt");
        let listed = files(&[dir.path().to_path_buf(), single]).expect("listed");
        let names: Vec<_> = listed
            .iter()
            .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            .collect();
        assert_eq!(names, ["a.OTF", "b.ttf", "c.ttc", "notes.txt"]);
    }

    #[test]
    fn a_missing_path_and_a_folder_without_fonts_are_errors_naming_the_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("nowhere");
        let error = files(std::slice::from_ref(&missing))
            .expect_err("missing")
            .to_string();
        assert!(
            error.contains("nowhere") && error.contains("missing"),
            "{error}"
        );
        let error = files(&[dir.path().to_path_buf()])
            .expect_err("empty")
            .to_string();
        assert!(error.contains("no font file"), "{error}");
    }

    #[test]
    fn the_engine_for_the_same_list_is_shared() {
        let a = engine(&[]).expect("engine");
        let b = engine(&[]).expect("engine");
        assert!(Arc::ptr_eq(&a, &b));
    }
}
