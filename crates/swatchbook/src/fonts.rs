//! The vendored faces: Inter and Fira Code, SIL Open Font License 1.1 (`fonts/OFL-*.txt`).

/// One font face and its bytes.
#[derive(Clone, Copy, Debug)]
pub struct Face {
    /// The family name the face registers under.
    pub family: &'static str,
    /// The file it came from, under `fonts/`.
    pub file: &'static str,
    /// The OpenType or TrueType data.
    pub bytes: &'static [u8],
}

macro_rules! face {
    ($family:literal, $file:literal) => {
        Face {
            family: $family,
            file: $file,
            bytes: include_bytes!(concat!("../fonts/", $file)),
        }
    };
}

/// Every vendored face: the families [`Fonts::default`](crate::Fonts::default) names.
pub const FACES: &[Face] = &[
    face!("Inter", "Inter-Regular.otf"),
    face!("Inter", "Inter-Bold.otf"),
    face!("Inter", "Inter-Italic.otf"),
    face!("Fira Code", "FiraCode-Regular.ttf"),
    face!("Fira Code", "FiraCode-Bold.ttf"),
];

/// Iterate over the faces of `family`.
///
/// # Examples
///
/// ```
/// assert_eq!(swatchbook::fonts::family("Inter").count(), 3);
/// assert_eq!(swatchbook::fonts::family("Comic Sans").count(), 0);
/// ```
pub fn family(family: &str) -> impl Iterator<Item = &'static Face> {
    FACES.iter().filter(move |face| face.family == family)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_is_an_opentype_or_truetype_file() {
        for face in FACES {
            let magic = &face.bytes[..4];
            assert!(
                magic == b"OTTO" || magic == [0, 1, 0, 0],
                "{}: {magic:?}",
                face.file
            );
        }
    }
}
