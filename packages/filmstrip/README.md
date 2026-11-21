# filmstrip

Frames in, films out. A list of RGB frames (whatever drew them) becomes:

- an animated GIF with one palette learned from the stills and delta frames (only the rectangle
  that changed is stored, unchanged pixels are transparent), so a film of text cards stays small;
- an MP4 with H.264 video, pure Rust (`rusty_h264-encoder`, `mp4`), feature `mp4`;
- an A4 landscape Word file whose only content is the GIF and a line of links (Word plays the
  GIF in place), feature `docx`;
- a Screen annotation on a PDF page that plays the MP4 when the page opens in Acrobat; other
  viewers show the page as it is, feature `pdf`.

`sequence` turns scenes (each its build-in frames and a hold of its own) into the frame list:
every scene's build-in, the hold on its last frame, a `Transition` into the next scene (fade,
wipe, push, pixelize, circle, split or cut, each drawn on the CPU and readable at 10 fps), the
and the last scene going back into the first, so the loop has no jump.

```rust
use filmstrip::{Frame, Scene, Timing, Transition};

let one = [Frame::solid(4, 2, [0; 3]), Frame::solid(4, 2, [40; 3])];
let two = [Frame::solid(4, 2, [200; 3])];
let scenes = [Scene { frames: &one, hold: 1.0 }, Scene { frames: &two, hold: 2.0 }];
let timing = Timing { fps: 10, fade: 0.2, transition: Transition::Wipe, ticks: 2 };
let frames = filmstrip::sequence(&scenes, timing)?;
let stills = [Frame::solid(4, 2, [40; 3]), Frame::solid(4, 2, [200; 3])];
let gif = filmstrip::gif::encode(&frames, &stills, 10, 16)?;
assert!(gif.starts_with(b"GIF89a"));
# Ok::<(), anyhow::Error>(())
```

Nothing is started: no ffmpeg, no browser. License: MIT OR Apache-2.0.
