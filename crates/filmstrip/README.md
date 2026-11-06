# filmstrip

Frames in, films out. A list of RGB frames (whatever drew them) becomes:

- an animated GIF with one palette learned from the stills and delta frames (only the rectangle
  that changed is stored, unchanged pixels are transparent), so a film of text cards stays small;
- an MP4 with H.264 video, pure Rust (`rusty_h264-encoder`, `mp4`), feature `mp4`;
- an A4 landscape Word file whose only content is the GIF and a line of links (Word plays the
  GIF in place), feature `docx`;
- a Screen annotation on a PDF page that plays the MP4 when the page opens in Acrobat; other
  viewers show the page as it is, feature `pdf`.

`sequence` turns scenes (each a list of build-in frames) into the frame list: every scene's
build-in, a hold on its last frame, a crossfade into the next scene, the last scene fading back
into the first so the loop is seamless.

```rust
use filmstrip::{Frame, Timing};

let scenes = vec![
    vec![Frame::solid(4, 2, [0; 3]), Frame::solid(4, 2, [40; 3])],
    vec![Frame::solid(4, 2, [200; 3])],
];
let timing = Timing { fps: 10, hold: 1.0, fade: 0.2, ticks: 2 };
let frames = filmstrip::sequence(&scenes, timing)?;
let stills = [Frame::solid(4, 2, [40; 3]), Frame::solid(4, 2, [200; 3])];
let gif = filmstrip::gif::encode(&frames, &stills, 10, 16)?;
assert!(gif.starts_with(b"GIF89a"));
# Ok::<(), anyhow::Error>(())
```

Nothing is started: no ffmpeg, no browser. License: MIT OR Apache-2.0.
