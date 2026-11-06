//! Frames in, films out: RGB frames become an animated GIF ([`gif`]), an MP4 ([`video`]), a Word
//! file that plays the GIF ([`docx`]) or a PDF page that plays the MP4 ([`pdf`]). Pure Rust,
//! nothing started.
//!
//! [`sequence`] builds the frame list of a film from scenes: every scene's build-in frames, a
//! hold on its last frame, a crossfade into the next scene.

// Pixel and timing math: frames are a few hundred pixels wide and a film a few hundred frames
// long, far inside every range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "pixel and timing math on small frames"
)]

#[cfg(feature = "docx")]
pub mod docx;
#[cfg(feature = "gif")]
pub mod gif;
#[cfg(feature = "pdf")]
pub mod pdf;
#[cfg(feature = "mp4")]
pub mod video;

use anyhow::{Result, bail};

/// One RGB frame, row-major, 3 bytes per pixel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 3` bytes.
    pub rgb: Vec<u8>,
}

impl Frame {
    /// A frame of one colour.
    #[must_use]
    pub fn solid(width: u32, height: u32, rgb: [u8; 3]) -> Self {
        Self {
            width,
            height,
            rgb: rgb.repeat((width * height) as usize),
        }
    }

    /// Whether the buffer has the size the dimensions promise.
    #[must_use]
    pub const fn is_consistent(&self) -> bool {
        self.rgb.len() == (self.width * self.height * 3) as usize
    }

    /// The frame as a PNG.
    pub fn png(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.width, self.height);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header()?;
            w.write_image_data(&self.rgb)?;
        }
        Ok(out)
    }

    /// Mix this frame with `other`: `t` = 0 is this frame, 1 is the other.
    #[must_use]
    pub fn blend(&self, other: &Self, t: f32) -> Self {
        let rgb = self
            .rgb
            .iter()
            .zip(&other.rgb)
            .map(|(&x, &y)| f32::from(x).mul_add(1.0 - t, f32::from(y) * t).round() as u8)
            .collect();
        Self {
            width: self.width,
            height: self.height,
            rgb,
        }
    }
}

/// How the film runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    /// Frames per second of the film.
    pub fps: u16,
    /// Seconds of crossfade between scenes.
    pub fade: f32,
    /// Frames each build-in frame stays (2 at 10 fps gives the stop-motion look).
    pub ticks: usize,
}

impl Timing {
    fn frames(self, seconds: f32) -> usize {
        (seconds * f32::from(self.fps)).round() as usize
    }
}

/// One scene of the film: its build-in frames in order (one frame for a still) and how long it
/// stays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene<'a> {
    /// The build-in frames, the last one fully built.
    pub frames: &'a [Frame],
    /// Seconds the scene stays, build-in included.
    pub hold: f32,
}

/// The frames of the whole film. Every scene's build-in frames stay `ticks` frames each, the
/// last one until the scene's `hold` is used up, then the scene crossfades into the next, the
/// last into the first so the loop is seamless.
///
/// # Examples
///
/// ```
/// use filmstrip::{Frame, Scene, Timing};
///
/// let a = [Frame::solid(2, 2, [0, 0, 0])];
/// let b = [Frame::solid(2, 2, [255, 255, 255])];
/// let timing = Timing { fps: 10, fade: 0.0, ticks: 1 };
/// let scenes = [Scene { frames: &a, hold: 1.0 }, Scene { frames: &b, hold: 2.0 }];
/// let frames = filmstrip::sequence(&scenes, timing).expect("frames");
/// assert_eq!(frames.len(), 30);
/// ```
pub fn sequence(scenes: &[Scene<'_>], timing: Timing) -> Result<Vec<Frame>> {
    let Some(first) = scenes.first().and_then(|s| s.frames.first()) else {
        bail!("no scene frames");
    };
    let (w, h) = (first.width, first.height);
    if let Some(odd) = scenes
        .iter()
        .flat_map(|s| s.frames)
        .find(|f| f.width != w || f.height != h || !f.is_consistent())
    {
        bail!(
            "scene frames differ in size: {w}x{h} and {}x{}",
            odd.width,
            odd.height
        );
    }
    let fade = timing.frames(timing.fade);
    let ticks = timing.ticks.max(1);
    let mut out = Vec::new();
    for (i, scene) in scenes.iter().enumerate() {
        let Some((last, build)) = scene.frames.split_last() else {
            bail!("scene {} has no frames", i + 1);
        };
        for f in build {
            for _ in 0..ticks {
                out.push(f.clone());
            }
        }
        let hold = timing.frames(scene.hold).max(1);
        let rest = hold.saturating_sub(build.len() * ticks).max(ticks);
        for _ in 0..rest {
            out.push(last.clone());
        }
        let next = &scenes[(i + 1) % scenes.len()].frames[0];
        for k in 1..=fade {
            let t = k as f32 / (fade + 1) as f32;
            out.push(last.blend(next, t));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_in_then_a_hold_then_a_fade() {
        let a = Frame::solid(2, 1, [0, 0, 0]);
        let b = Frame::solid(2, 1, [100, 100, 100]);
        let c = Frame::solid(2, 1, [200, 200, 200]);
        let timing = Timing {
            fps: 10,
            fade: 0.2,
            ticks: 2,
        };
        let one = [a.clone(), b.clone()];
        let two = [c.clone()];
        let scenes = [
            Scene {
                frames: &one,
                hold: 1.0,
            },
            Scene {
                frames: &two,
                hold: 1.5,
            },
        ];
        let frames = sequence(&scenes, timing).unwrap();
        // scene 1: a a (build-in, 2 ticks), b x 8 (hold 10 - 2), 2 blends; scene 2: c x 15, 2 blends
        assert_eq!(frames.len(), 2 + 8 + 2 + 15 + 2);
        assert_eq!(frames[0], a);
        assert_eq!(frames[2], b);
        assert_eq!(frames[9], b);
        assert!(frames[10].rgb[0] > 100 && frames[10].rgb[0] < 200);
        assert_eq!(frames[12], c);
        assert_eq!(frames[26], c);
        assert!(frames[28].rgb[0] < 200);
    }

    #[test]
    fn frames_of_different_sizes_are_refused() {
        let a = Frame::solid(2, 1, [0; 3]);
        let b = Frame::solid(1, 1, [0; 3]);
        let timing = Timing {
            fps: 10,
            fade: 0.0,
            ticks: 1,
        };
        let (a, b) = ([a], [b]);
        let scenes = [
            Scene {
                frames: &a,
                hold: 1.0,
            },
            Scene {
                frames: &b,
                hold: 1.0,
            },
        ];
        assert!(sequence(&scenes, timing).is_err());
        assert!(sequence(&[], timing).is_err());
    }

    #[test]
    fn png_round_trip_keeps_the_size() {
        let f = Frame::solid(3, 2, [1, 2, 3]);
        let png = f.png().unwrap();
        assert!(png.starts_with(b"\x89PNG"));
        let w = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(png[20..24].try_into().unwrap());
        assert_eq!((w, h), (3, 2));
    }
}
