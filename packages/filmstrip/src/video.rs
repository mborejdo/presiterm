//! The MP4: the frames as H.264 (`rusty_h264-encoder`) in an MP4 container (`mp4`), one video
//! track, no audio, no B-frames so decode order is display order.

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use rusty_h264_common::YuvFrame;
use rusty_h264_encoder::{Encoder, EncoderConfig};

use crate::Frame;

/// Encode `frames` at `fps` into MP4 bytes. Width and height must be even.
///
/// # Examples
///
/// ```
/// use filmstrip::Frame;
///
/// let frames = vec![Frame::solid(32, 18, [0; 3]), Frame::solid(32, 18, [255; 3])];
/// let mp4 = filmstrip::video::encode(&frames, 10).expect("mp4");
/// assert_eq!(&mp4[4..8], b"ftyp");
/// ```
pub fn encode(frames: &[Frame], fps: u16) -> Result<Vec<u8>> {
    let Some(first) = frames.first() else {
        bail!("no frames");
    };
    if fps == 0 {
        bail!("fps must be at least 1");
    }
    let (w, h) = (first.width as usize, first.height as usize);
    if w % 2 != 0 || h % 2 != 0 {
        bail!("video needs even dimensions, got {w}x{h}");
    }

    let mut cfg = EncoderConfig::new(w, h);
    cfg.framerate = f32::from(fps);
    cfg.qp = 23;
    cfg.bframes = 0;
    cfg.lookahead = 0;
    let mut enc = Encoder::new(cfg).map_err(|e| anyhow::anyhow!("h264 encoder: {e:?}"))?;

    // Annex-B out of the encoder; collect it, then split into access units for the muxer.
    let mut stream = Vec::new();
    for f in frames {
        if !f.matches(first) {
            bail!("frames differ in size");
        }
        stream.extend_from_slice(&enc.encode(&i420(f)));
    }
    stream.extend_from_slice(&enc.flush());
    let (sps, pps, samples) = split_annex_b(&stream)?;
    if samples.len() != frames.len() {
        bail!(
            "the encoder returned {} pictures for {} frames",
            samples.len(),
            frames.len()
        );
    }

    let config = mp4::Mp4Config {
        major_brand: "isom".parse().context("brand")?,
        minor_version: 512,
        compatible_brands: ["isom", "iso2", "avc1", "mp41"]
            .iter()
            .map(|b| b.parse().context("brand"))
            .collect::<Result<_>>()?,
        timescale: 1000,
    };
    let mut writer = mp4::Mp4Writer::write_start(std::io::Cursor::new(Vec::new()), &config)?;
    writer.add_track(&mp4::TrackConfig {
        track_type: mp4::TrackType::Video,
        timescale: u32::from(fps),
        language: "und".to_owned(),
        media_conf: mp4::MediaConfig::AvcConfig(mp4::AvcConfig {
            width: u16::try_from(w)?,
            height: u16::try_from(h)?,
            seq_param_set: sps,
            pic_param_set: pps,
        }),
    })?;
    for (i, (is_sync, bytes)) in samples.into_iter().enumerate() {
        writer.write_sample(
            1,
            &mp4::Mp4Sample {
                start_time: i as u64,
                duration: 1,
                rendering_offset: 0,
                is_sync,
                bytes: Bytes::from(bytes),
            },
        )?;
    }
    writer.write_end()?;
    Ok(writer.into_writer().into_inner())
}

/// RGB to I420 (BT.601, limited range), chroma averaged over 2x2.
fn i420(frame: &Frame) -> YuvFrame {
    let (width, height) = (frame.width as usize, frame.height as usize);
    let mut luma = vec![0u8; width * height];
    let (chroma_w, chroma_h) = (width / 2, height / 2);
    let mut cb = vec![0u8; chroma_w * chroma_h];
    let mut cr = vec![0u8; chroma_w * chroma_h];
    let pixel = |col: usize, row: usize| {
        let i = (row * width + col) * 3;
        (
            f32::from(frame.rgb[i]),
            f32::from(frame.rgb[i + 1]),
            f32::from(frame.rgb[i + 2]),
        )
    };
    for row in 0..height {
        for col in 0..width {
            let (red, green, blue) = pixel(col, row);
            let sum = 24.966_f32.mul_add(blue, 128.553_f32.mul_add(green, 65.481 * red));
            luma[row * width + col] = (16.0 + sum / 255.0).round() as u8;
        }
    }
    for row in 0..chroma_h {
        for col in 0..chroma_w {
            let mut sum = (0.0_f32, 0.0_f32, 0.0_f32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (red, green, blue) = pixel(col * 2 + dx, row * 2 + dy);
                sum.0 += red;
                sum.1 += green;
                sum.2 += blue;
            }
            let (red, green, blue) = (sum.0 / 4.0, sum.1 / 4.0, sum.2 / 4.0);
            let blue_diff = 112.0_f32.mul_add(blue, 74.203_f32.mul_add(-green, -37.797 * red));
            let red_diff = 18.214_f32.mul_add(-blue, 93.786_f32.mul_add(-green, 112.0 * red));
            cb[row * chroma_w + col] = (128.0 + blue_diff / 255.0).round() as u8;
            cr[row * chroma_w + col] = (128.0 + red_diff / 255.0).round() as u8;
        }
    }
    YuvFrame {
        width,
        height,
        y: luma,
        u: cb,
        v: cr,
    }
}

/// One picture for the muxer: whether it is a keyframe, and its length-prefixed NALs.
type Sample = (bool, Vec<u8>);

/// Split an Annex-B stream into SPS, PPS and one AVCC sample (length-prefixed NALs) per
/// picture; a new picture starts at every slice NAL (types 1 and 5).
fn split_annex_b(stream: &[u8]) -> Result<(Vec<u8>, Vec<u8>, Vec<Sample>)> {
    let mut nals: Vec<&[u8]> = Vec::new();
    let mut i = 0;
    let mut start: Option<usize> = None;
    while i + 3 <= stream.len() {
        let code = if stream[i..].starts_with(&[0, 0, 0, 1]) {
            Some(4)
        } else if stream[i..].starts_with(&[0, 0, 1]) {
            Some(3)
        } else {
            None
        };
        if let Some(len) = code {
            if let Some(s) = start {
                nals.push(&stream[s..i]);
            }
            i += len;
            start = Some(i);
        } else {
            i += 1;
        }
    }
    if let Some(s) = start {
        nals.push(&stream[s..]);
    }
    let (mut sps, mut pps) = (Vec::new(), Vec::new());
    let mut samples: Vec<Sample> = Vec::new();
    for nal in nals {
        let Some(&header) = nal.first() else { continue };
        let kind = header & 0x1f;
        match kind {
            7 => sps = nal.to_vec(),
            8 => pps = nal.to_vec(),
            9 | 6 => {}, // access unit delimiter, SEI: not needed in MP4
            1 | 5 => {
                let mut sample = Vec::with_capacity(nal.len() + 4);
                sample.extend_from_slice(&u32::try_from(nal.len())?.to_be_bytes());
                sample.extend_from_slice(nal);
                samples.push((kind == 5, sample));
            },
            _ => {
                // other NALs travel with the current picture
                if let Some((_, sample)) = samples.last_mut() {
                    sample.extend_from_slice(&u32::try_from(nal.len())?.to_be_bytes());
                    sample.extend_from_slice(nal);
                }
            },
        }
    }
    if sps.is_empty() || pps.is_empty() {
        bail!("the encoder stream has no SPS/PPS");
    }
    Ok((sps, pps, samples))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_frame_becomes_a_sample_and_the_first_is_a_keyframe() {
        let frames: Vec<Frame> = (0..5)
            .map(|i| Frame::solid(32, 18, [i * 40, 0, 255 - i * 40]))
            .collect();
        let bytes = encode(&frames, 10).unwrap();
        let size = bytes.len() as u64;
        let mut reader = mp4::Mp4Reader::read_header(std::io::Cursor::new(bytes), size).unwrap();
        let (&id, track) = reader.tracks().iter().next().unwrap();
        assert_eq!(track.sample_count(), 5);
        assert_eq!((track.width(), track.height()), (32, 18));
        let first = reader.read_sample(id, 1).unwrap().unwrap();
        assert!(first.is_sync);
        let last = reader.read_sample(id, 5).unwrap().unwrap();
        assert_eq!(last.start_time, 4);
    }

    #[test]
    fn odd_sizes_and_empty_input_are_errors() {
        assert!(encode(&[], 10).is_err());
        assert!(encode(&[Frame::solid(31, 18, [0; 3])], 10).is_err());
    }

    #[test]
    fn grey_stays_grey_in_yuv() {
        let f = Frame::solid(2, 2, [128, 128, 128]);
        let yuv = i420(&f);
        assert_eq!(yuv.u, vec![128]);
        assert_eq!(yuv.v, vec![128]);
        assert!(yuv.y.iter().all(|&y| (125..=128).contains(&y)));
    }
}
