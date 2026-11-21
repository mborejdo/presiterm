# Diagrams and pictures

Inline images at the pixel size of their cells, in the deck's tokens (`[theme]`, see
`presiterm handbook config`), cached by content hash in `~/.cache/presiterm`. `h=N` is the height
in cells, `w=N` the largest width; without `h` a picture fills the rest of the slide. Fitted,
centred, never wider than the text.

## Pictures

`![alt](file.png){h=7}`, alone on its line: PNG, JPEG, GIF (animated ones play), SVG. Scaled up
it goes blocky, so give it at least the pixels it is shown at (full height at 42 pt is about
1700 px) or a smaller `h`. See `examples/media`.

## Mermaid

A ```` ```mermaid ```` fence, or `![alt](flow.mmd){h=8}`. mermaid-rs-renderer draws it
in-process: surface nodes with an accent outline, ink text, labels in the sans face (Inter).

* `steps` (```` ```mermaid steps ````, `{steps}` on a file, or a ```` ```steps ```` fence): one
  frame per node with that node lit, one key press each.
* `steps=a,b,c` names the nodes and their order (default: the renderer's layout order, so name
  them for a fixed order).
* `h=9 w=30` sizes it: ```` ```mermaid steps h=9 w=30 ````.
* Kinds: flowchart, sequenceDiagram, classDiagram, stateDiagram, erDiagram, pie, gantt, journey,
  timeline, mindmap, gitGraph, quadrantChart, sankey, xychart, kanban, block, architecture,
  requirementDiagram, packet, radar and treemap.

## Typst / fletcher

A ```` ```typst ```` fence is Typst source, drawn in-process. If it calls `diagram(`, `node(` or
`edge(`, the preamble imports [fletcher](https://typst.app/universe/package/fletcher) (fetched
once, cached where the typst CLI caches packages):

```typst
#diagram(node((0,0), [a]), edge("->"), node((1,0), [b]))
```

* The preamble: no page, no ground, mono face, theme ink; `diagram` gets rounded accent-outlined
  nodes and muted edges. Every palette token is a Typst colour:
  `ground`, `ink`, `muted`, `accent`, `surface`, `border`, `ok`, `warn`, `stop`
  (`node((0,0), [ok], fill: ok, stroke: none)`).
* `steps=N` (```` ```typst steps=3 h=6 ````): N frames, key press i lights the i-th `node`.
  `steps=1,2,2,3` sets each node's step (together or out of order).
* `stepnode(i, pos, label, ..)` lights a node in frame i; a body using it is left as written.
* Markdown tables are Typst pictures, so no cell is cut at a column edge.
* Over 20 megapixels a picture is redrawn at a lower density: WezTerm drops bigger images.
* Cargo feature `typst` (default): about a thousand crates. Without it (`--no-default-features`)
  a typst fence is an error on the slide and tables are aligned text.
