//! The animated GIF: one palette learned from the stills (`NeuQuant`), every frame after the first
//! a delta against the screen (the changed rectangle, unchanged pixels transparent), identical
//! neighbours folded into one longer delay.

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{Result, bail};
use color_quant::NeuQuant;
use gif::{DisposalMethod, Encoder, Frame as GifFrame, Repeat};

use crate::Frame;

/// Some players cap a single frame delay around a second, so a longer hold is written as the
/// frame plus 1x1 transparent "keep" frames that only carry time.
const MAX_DELAY_CS: u32 = 100;

/// Encode `frames` at `fps`. The palette (`colours` entries, 2 to 255) is learned from
/// `keyframes`, the stills a viewer looks at longest (empty: from every frame); blends map onto it.
///
/// # Examples
///
/// ```
/// use filmstrip::Frame;
///
/// let a = Frame::solid(4, 4, [0, 0, 0]);
/// let b = Frame::solid(4, 4, [255, 255, 255]);
/// let gif = filmstrip::gif::encode(&[a.clone(), a.clone(), b.clone()], &[a, b], 10, 8).expect("gif");
/// assert!(gif.starts_with(b"GIF89a"));
/// ```
pub fn encode(frames: &[Frame], keyframes: &[Frame], fps: u16, colours: usize) -> Result<Vec<u8>> {
    let Some(first) = frames.first() else {
        bail!("no frames");
    };
    let (Ok(w), Ok(h)) = (u16::try_from(first.width), u16::try_from(first.height)) else {
        bail!(
            "frame too large for a GIF: {}x{}",
            first.width,
            first.height
        );
    };
    if fps == 0 {
        bail!("fps must be at least 1");
    }
    let stills = if keyframes.is_empty() {
        frames
    } else {
        keyframes
    };
    // NeuQuant wants RGBA samples; every third pixel keeps the training quick.
    let mut samples = Vec::new();
    for f in stills {
        for px in f.rgb.as_chunks::<3>().0.iter().step_by(3) {
            samples.extend_from_slice(&[px[0], px[1], px[2], 255]);
        }
    }
    // One slot stays free for the "unchanged" (transparent) pixels of delta frames.
    let colours = colours.clamp(2, 255);
    let nq = NeuQuant::new(10, colours, &samples);
    let mut palette: Vec<u8> = nq
        .color_map_rgba()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|c| [c[0], c[1], c[2]])
        .collect();
    let transparent = u8::try_from(palette.len() / 3)?;
    palette.extend_from_slice(&[0, 0, 0]);

    let tick = (100.0 / f32::from(fps)).round().max(1.0) as u32; // centiseconds per frame
    let mut out = Vec::new();
    {
        let mut enc = Encoder::new(&mut out, w, h, &palette)?;
        enc.set_repeat(Repeat::Infinite)?;
        let mut cache: HashMap<[u8; 3], u8> = HashMap::new();
        let mut quantize = |f: &Frame| -> Vec<u8> {
            f.rgb
                .as_chunks::<3>()
                .0
                .iter()
                .map(|px| {
                    let key = *px;
                    *cache
                        .entry(key)
                        .or_insert_with(|| nq.index_of(&[px[0], px[1], px[2], 255]) as u8)
                })
                .collect()
        };
        // Frames are written late: identical neighbours fold into the delay of the frame on
        // screen, and every frame after the first is a delta against the previous.
        let mut put = |buf: &[u8], shown: Option<&[u8]>, count: u32| {
            write(&mut enc, w, h, buf, shown, transparent, count * tick)
        };
        let mut shown: Option<Vec<u8>> = None;
        let mut pending: Option<(Vec<u8>, u32)> = None;
        for f in frames {
            if !f.matches(first) {
                bail!("frames differ in size");
            }
            let indexed = quantize(f);
            match pending.as_mut() {
                Some((buf, n)) if *buf == indexed => *n += 1,
                _ => {
                    if let Some((buf, n)) = pending.take() {
                        put(&buf, shown.as_deref(), n)?;
                        shown = Some(buf);
                    }
                    pending = Some((indexed, 1));
                },
            }
        }
        if let Some((buf, n)) = pending.take() {
            put(&buf, shown.as_deref(), n)?;
        }
    }
    Ok(out)
}

/// Write one frame as the smallest rectangle that differs from what is on screen; unchanged
/// pixels inside it are transparent. The first frame (no `prev`) is written whole.
fn write<W: std::io::Write>(
    enc: &mut Encoder<W>,
    w: u16,
    h: u16,
    indexed: &[u8],
    prev: Option<&[u8]>,
    transparent: u8,
    delay_cs: u32,
) -> Result<()> {
    let first = delay_cs.min(MAX_DELAY_CS);
    let (wu, hu) = (usize::from(w), usize::from(h));
    let frame = match prev {
        Some(prev) => {
            let (mut x0, mut y0, mut x1, mut y1) = (wu, hu, 0usize, 0usize);
            for y in 0..hu {
                let row = &indexed[y * wu..(y + 1) * wu];
                let old = &prev[y * wu..(y + 1) * wu];
                if row == old {
                    continue;
                }
                let lx = row.iter().zip(old).position(|(a, b)| a != b).unwrap_or(0);
                let rx = wu
                    - row
                        .iter()
                        .zip(old)
                        .rev()
                        .position(|(a, b)| a != b)
                        .unwrap_or(0);
                x0 = x0.min(lx);
                x1 = x1.max(rx);
                y0 = y0.min(y);
                y1 = y1.max(y + 1);
            }
            if x1 <= x0 || y1 <= y0 {
                // nothing changed (callers fold identical frames, so this is a safety net)
                (x0, y0, x1, y1) = (0, 0, 1, 1);
            }
            let (cw, ch) = (x1 - x0, y1 - y0);
            let mut buffer = Vec::with_capacity(cw * ch);
            for y in y0..y1 {
                for x in x0..x1 {
                    let i = y * wu + x;
                    buffer.push(if indexed[i] == prev[i] {
                        transparent
                    } else {
                        indexed[i]
                    });
                }
            }
            GifFrame {
                left: u16::try_from(x0)?,
                top: u16::try_from(y0)?,
                width: u16::try_from(cw)?,
                height: u16::try_from(ch)?,
                delay: first as u16,
                dispose: DisposalMethod::Keep,
                transparent: Some(transparent),
                buffer: Cow::Owned(buffer),
                ..GifFrame::default()
            }
        },
        None => GifFrame {
            width: w,
            height: h,
            delay: first as u16,
            dispose: DisposalMethod::Keep,
            buffer: Cow::Owned(indexed.to_vec()),
            ..GifFrame::default()
        },
    };
    enc.write_frame(&frame)?;
    let mut rest = delay_cs - first;
    while rest > 0 {
        let d = rest.min(MAX_DELAY_CS);
        let keep = GifFrame {
            width: 1,
            height: 1,
            delay: d as u16,
            dispose: DisposalMethod::Keep,
            transparent: Some(0),
            buffer: Cow::Owned(vec![0]),
            ..GifFrame::default()
        };
        enc.write_frame(&keep)?;
        rest -= d;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (left, top, width, height, delay in cs) of every frame in the file.
    fn frames_of(bytes: &[u8]) -> Vec<(u16, u16, u16, u16, u16)> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::Indexed);
        let mut decoder = options.read_info(std::io::Cursor::new(bytes)).unwrap();
        let mut out = Vec::new();
        while let Some(f) = decoder.read_next_frame().unwrap() {
            out.push((f.left, f.top, f.width, f.height, f.delay));
        }
        out
    }

    #[test]
    fn identical_neighbours_fold_and_long_holds_split() {
        let a = Frame::solid(8, 4, [10, 20, 30]);
        let frames = vec![a.clone(); 25]; // 2.5 s at 10 fps
        let gif = encode(&frames, &[a], 10, 4).unwrap();
        let frames = frames_of(&gif);
        // one full frame (100 cs) + keep frames 100 + 50
        assert_eq!(frames[0], (0, 0, 8, 4, 100));
        assert_eq!(frames[1], (0, 0, 1, 1, 100));
        assert_eq!(frames[2], (0, 0, 1, 1, 50));
        assert_eq!(frames.len(), 3);
    }

    #[test]
    fn a_change_is_written_as_its_rectangle_only() {
        let a = Frame::solid(8, 4, [0, 0, 0]);
        let mut b = a.clone();
        // change the pixels at (5, 2) and (6, 2)
        for x in 5..7 {
            let i = (2 * 8 + x) * 3;
            b.rgb[i..i + 3].copy_from_slice(&[255, 255, 255]);
        }
        let gif = encode(&[a.clone(), b.clone()], &[a, b], 10, 4).unwrap();
        let frames = frames_of(&gif);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1], (5, 2, 2, 1, 10));
    }

    #[test]
    fn empty_input_and_zero_fps_are_errors() {
        assert!(encode(&[], &[], 10, 4).is_err());
        let a = Frame::solid(2, 2, [0; 3]);
        assert!(encode(&[a], &[], 0, 4).is_err());
    }
}
