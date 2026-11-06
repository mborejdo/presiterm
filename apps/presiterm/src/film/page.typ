// The PDF page under the film: the poster where the Screen annotation will sit, the title line
// under it. Inputs: poster (bytes), theme (JSON), title, author, links (one per line), note,
// geometry ("page-w page-h margin top poster-w poster-h" in points, from film/mod.rs).

#let theme = json(bytes(sys.inputs.theme))
#let title = sys.inputs.at("title", default: "")
#let author = sys.inputs.at("author", default: "")
#let links = sys.inputs.at("links", default: "").split("\n").filter(l => l != "")
#let note = sys.inputs.at("note", default: "")

#let geometry = sys.inputs.geometry.split(" ").map(float)
#let page-w = geometry.at(0) * 1pt
#let page-h = geometry.at(1) * 1pt
#let margin = geometry.at(2) * 1pt
#let top-off = geometry.at(3) * 1pt
#let pw = geometry.at(4) * 1pt
#let ph = geometry.at(5) * 1pt

#set page(width: page-w, height: page-h, margin: 0pt, fill: rgb(theme.ground))
#set text(font: theme.sans, size: 9pt, fill: rgb(theme.ink))
#set document(title: title, author: author)

#place(top + left, dx: margin, dy: top-off, box(width: pw, height: ph, clip: true, radius: 4pt, image(sys.inputs.poster, width: pw, height: ph, fit: "cover")))

#place(top + left, dx: margin, dy: top-off + ph + 14pt, box(width: pw, grid(
  columns: (1fr, auto),
  align: (left, right),
  text(weight: "bold", size: 12pt, title) + (if author != "" { h(0.8em) + text(fill: rgb(theme.muted), author) } else { none }),
  text(font: theme.mono, size: 8pt, fill: rgb(theme.accent), links.map(l => link(l, l.replace(regex("^(https?://|mailto:)"), ""))).join("   ")),
)))

#place(bottom + left, dx: margin, dy: -18pt, text(font: theme.mono, size: 7.5pt, fill: rgb(theme.muted), note))
