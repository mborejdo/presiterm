# Diagrams and pictures

All of them are inline images drawn to the pixel size of their cell rectangle (cached by content
hash in `~/.cache/presiterm`), in the deck's design tokens (see `presiterm handbook config` for
the `[theme]` table). `h=N` is the height in cells, `w=N` the largest width in cells; without `h` a
picture takes the rest of the slide. Fitted, centred, never wider than the text.

## Pictures

`![alt](file.png){h=7}`: PNG, JPEG, GIF (an animated GIF plays), SVG. The picture must be alone on
its line. WezTerm goes blocky when it scales an image up, so give a picture at least as many pixels
as it is shown at (a full-height picture at 42 pt is about 1700 px tall) or a smaller `h`. See
`examples/media`.

## Mermaid

A ```` ```mermaid ```` fence, or `![alt](flow.mmd){h=8}` for a file. Drawn in-process by
mermaid-rs-renderer: nodes on the surface colour with an accent outline, text in the ink, labels in
the sans face (Inter by default).

* `steps` (```` ```mermaid steps ````, `{steps}` on a file, or a ```` ```steps ```` fence) draws the
  diagram once more per node with that node lit; every lit node is one key press.
* `steps=a,b,c` names the nodes and their order (default: order of appearance).
* `h=9 w=30` size it: ```` ```mermaid steps h=9 w=30 ````.
* Not supported by the renderer: `sequenceDiagram`. `~~~` links may reorder nodes.

## Typst / fletcher

A ```` ```typst ```` fence is Typst source, drawn in-process. When it calls `diagram(`, `node(` or
`edge(`, the preamble imports [fletcher](https://typst.app/universe/package/fletcher) (downloaded
from Typst Universe once and cached where the typst CLI caches packages):

```typst
#diagram(node((0,0), [a]), edge("->"), node((1,0), [b]))
```

* The preamble sets no page, no ground, the mono face and the theme's ink, and gives `diagram`
  rounded accent-outlined nodes and muted edges. Every palette token is a Typst colour you can use:
  `ground`, `ink`, `muted`, `accent`, `surface`, `border`, `ok`, `warn`, `stop`
  (`node((0,0), [ok], fill: ok, stroke: none)`).
* `steps=N` (```` ```typst steps=3 h=6 ````): N frames, key press i lights the i-th `node`.
  `steps=1,2,2,3` gives the step of each node when two nodes light together or out of order.
* `stepnode(i, pos, label, ..)` lights a node in frame i explicitly; a body that uses it is left
  as written.
* A Markdown table is drawn as a Typst picture, so no cell is cut at a column edge.
* A Typst picture that would pass 20 megapixels is redrawn at a lower density, because WezTerm
  drops bigger inline images.
* Cargo feature `typst` (default): about a thousand crates. Built without it
  (`--no-default-features`) a typst fence is an error on the slide and tables are aligned text.

## D2

There is no in-process D2 engine: a ```` ```d2 ```` fence or a `.d2` file is an error on the slide
that names the alternative (mermaid or a typst fence).
