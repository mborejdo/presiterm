//! Frames in, films out: RGB frames become an animated GIF ([`gif`]), an MP4 ([`video`]), a Word
//! file that plays the GIF ([`docx`]) or a PDF page that plays the MP4 ([`pdf`]). Pure Rust,
//! nothing started.
//!
//! [`sequence`] builds the frame list of a film from scenes: every scene's build-in frames, a
//! hold on its last frame, a [`Transition`] into the next scene.

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

    /// Whether this frame has the size of `first` and a buffer to match: what every encoder asks
    /// of the frames after the first.
    const fn matches(&self, first: &Self) -> bool {
        self.width == first.width && self.height == first.height && self.is_consistent()
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

    /// A frame drawn pixel by pixel.
    fn drawn(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 3]) -> Self {
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                rgb.extend_from_slice(&pixel(x, y));
            }
        }
        Self { width, height, rgb }
    }

    /// The pixel at `x`, `y`; both inside the frame.
    fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let at = ((y * self.width + x) * 3) as usize;
        [self.rgb[at], self.rgb[at + 1], self.rgb[at + 2]]
    }

    /// This frame with `other` showing wherever `take` says so.
    fn composed(&self, other: &Self, take: impl Fn(u32, u32) -> bool) -> Self {
        Self::drawn(self.width, self.height, |x, y| {
            if take(x, y) {
                other.pixel(x, y)
            } else {
                self.pixel(x, y)
            }
        })
    }

    /// This frame in blocks of `block` pixels, each the colour of its centre.
    fn mosaic(&self, block: u32) -> Self {
        let block = block.max(1);
        let centre = |v: u32, end: u32| (v - v % block + block / 2).min(end - 1);
        Self::drawn(self.width, self.height, |x, y| {
            self.pixel(centre(x, self.width), centre(y, self.height))
        })
    }
}

/// How one scene gives way to the next. Every one reads at a low frame rate: four frames of
/// it at 10 fps are enough.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum Transition {
    /// A crossfade.
    #[default]
    Fade,
    /// The next scene wipes in from the left.
    Wipe,
    /// The next scene pushes this one out to the left.
    Push,
    /// This scene breaks into blocks that grow, the next one sharpens out of them.
    Pixelize,
    /// The next scene opens in a circle from the centre.
    Circle,
    /// The next scene opens from the centre line outwards, like barn doors.
    Split,
    /// No frames between: the next scene is just there.
    Cut,
}

impl Transition {
    /// Every transition by its name, in the order of the docs.
    pub const ALL: [Self; 7] = [
        Self::Fade,
        Self::Wipe,
        Self::Push,
        Self::Pixelize,
        Self::Circle,
        Self::Split,
        Self::Cut,
    ];

    /// The name the config, the frontmatter and the flag use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fade => "fade",
            Self::Wipe => "wipe",
            Self::Push => "push",
            Self::Pixelize => "pixelize",
            Self::Circle => "circle",
            Self::Split => "split",
            Self::Cut => "cut",
        }
    }

    /// The frame `t` of the way from `from` to `to`, `t` strictly between 0 and 1; both frames
    /// the same size.
    #[must_use]
    pub fn frame(self, from: &Frame, to: &Frame, t: f32) -> Frame {
        let (w, h) = (from.width as f32, from.height as f32);
        match self {
            Self::Fade => from.blend(to, t),
            Self::Cut => to.clone(),
            Self::Wipe => from.composed(to, |x, _| (x as f32) < t * w),
            Self::Split => {
                let (mid, half) = (w / 2.0, t * w / 2.0);
                from.composed(to, |x, _| (x as f32 + 0.5 - mid).abs() < half)
            },
            Self::Circle => {
                let (cx, cy) = (w / 2.0, h / 2.0);
                let r = t * cx.hypot(cy);
                from.composed(to, |x, y| {
                    (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy) < r
                })
            },
            Self::Push => {
                let shift = (t * w).round() as u32;
                Frame::drawn(from.width, from.height, |x, y| {
                    if x + shift < from.width {
                        from.pixel(x + shift, y)
                    } else {
                        to.pixel(x + shift - from.width, y)
                    }
                })
            },
            Self::Pixelize => {
                // Blocks grow towards the middle of the way, then shrink on the next scene.
                let size = (1.0 - 2.0f32.mul_add(t, -1.0).abs()) * w / 16.0;
                let source = if t < 0.5 { from } else { to };
                source.mosaic(size.round() as u32)
            },
        }
    }
}

impl std::str::FromStr for Transition {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|t| t.name().eq_ignore_ascii_case(name.trim()))
            .ok_or_else(|| {
                let names: Vec<_> = Self::ALL.iter().map(|t| t.name()).collect();
                anyhow::anyhow!(
                    "transition: expected one of {}, got \"{name}\"",
                    names.join(", ")
                )
            })
    }
}

impl std::fmt::Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// How the film runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timing {
    /// Frames per second of the film.
    pub fps: u16,
    /// Seconds a transition between scenes takes; a cut takes none.
    pub fade: f32,
    /// How one scene gives way to the next.
    pub transition: Transition,
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
/// last one until the scene's `hold` is used up, then the transition into the next scene, the
/// last into the first so the loop has no jump.
///
/// # Examples
///
/// ```
/// use filmstrip::{Frame, Scene, Timing, Transition};
///
/// let a = [Frame::solid(2, 2, [0, 0, 0])];
/// let b = [Frame::solid(2, 2, [255, 255, 255])];
/// let timing = Timing { fps: 10, fade: 0.0, transition: Transition::Fade, ticks: 1 };
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
        .find(|f| !f.matches(first))
    {
        bail!(
            "scene frames differ in size: {w}x{h} and {}x{}",
            odd.width,
            odd.height
        );
    }
    let fade = match timing.transition {
        Transition::Cut => 0,
        _ => timing.frames(timing.fade),
    };
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
            out.push(timing.transition.frame(last, next, t));
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
            transition: Transition::Fade,
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
            transition: Transition::Fade,
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
    fn a_cut_has_no_frames_between_and_the_others_reach_the_next_scene_from_the_left_or_centre() {
        let black = Frame::solid(8, 4, [0; 3]);
        let white = Frame::solid(8, 4, [255; 3]);
        let left = |f: &Frame| f.pixel(0, 1)[0];
        let centre = |f: &Frame| f.pixel(4, 2)[0];
        let right = |f: &Frame| f.pixel(7, 1)[0];
        for t in Transition::ALL {
            let frame = t.frame(&black, &white, 0.5);
            assert!(frame.is_consistent(), "{t}");
            match t {
                Transition::Cut | Transition::Pixelize => assert_eq!(frame, white),
                Transition::Fade => assert_eq!(frame.pixel(3, 3), [128; 3]),
                Transition::Wipe => assert_eq!((left(&frame), right(&frame)), (255, 0)),
                Transition::Push => assert_eq!((left(&frame), right(&frame)), (0, 255)),
                Transition::Circle | Transition::Split => {
                    assert_eq!((left(&frame), centre(&frame)), (0, 255), "{t}");
                },
            }
        }
        let scene = [black];
        let scenes = [Scene {
            frames: &scene,
            hold: 0.5,
        }];
        let cut = Timing {
            fps: 10,
            fade: 1.0,
            transition: Transition::Cut,
            ticks: 1,
        };
        assert_eq!(sequence(&scenes, cut).unwrap().len(), 5);
        let wipe = Timing {
            transition: Transition::Wipe,
            ..cut
        };
        assert_eq!(sequence(&scenes, wipe).unwrap().len(), 15);
    }

    #[test]
    fn a_mosaic_takes_the_colour_of_every_block_centre_and_a_name_reads_back() {
        let mut frame = Frame::solid(4, 2, [0; 3]);
        frame.rgb[15..18].copy_from_slice(&[9; 3]); // (1, 1): the centre of the first block
        let mosaic = frame.mosaic(2);
        assert_eq!(mosaic.pixel(0, 0), [9; 3]);
        assert_eq!(mosaic.pixel(2, 0), [0; 3]);
        assert_eq!(" Push ".parse::<Transition>().unwrap(), Transition::Push);
        let error = "slide".parse::<Transition>().unwrap_err().to_string();
        assert!(
            error.contains("fade, wipe") && error.contains("slide"),
            "{error}"
        );
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
