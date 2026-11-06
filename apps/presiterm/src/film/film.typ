// One card of the film: a slide at one instant of one build-in step, drawn in the deck's tokens.
// Inputs (sys.inputs): card (JSON, see film/mod.rs), theme (JSON: colours and fonts), title (the
// deck's), index/total, step/steps, frame/frames, width (logical pt; rendered at 2 px/pt), logo
// (bytes, optional), img<N> (bytes, the pictures and diagrams of this card at this step).

#let card = json(bytes(sys.inputs.card))
#let theme = json(bytes(sys.inputs.theme))
#let deck-title = sys.inputs.at("title", default: "")
#let index = int(sys.inputs.at("index", default: "0"))
#let total = int(sys.inputs.at("total", default: "1"))
#let step = int(sys.inputs.at("step", default: "0"))
#let frame = int(sys.inputs.at("frame", default: "0"))
#let frames = int(sys.inputs.at("frames", default: "1"))
#let W = float(sys.inputs.at("width", default: "320"))
#let H = W * 9 / 16
#let s = W / 320 // every size below is set for a 320 pt card
#let px = 1pt

#let ground = rgb(theme.ground)
#let ink = rgb(theme.ink)
#let muted = rgb(theme.muted)
#let accent = rgb(theme.accent)
#let surface = rgb(theme.surface)
#let border = rgb(theme.border)
#let warn = rgb(theme.warn)
#let stop = rgb(theme.stop)
#let sans = theme.sans
#let mono = theme.mono
#let display = theme.display // headings, in the heaviest weight the family has

// ---- the build-in: progress 0..1 with an ease-out cubic ----
#let t = if frames > 1 { frame / (frames - 1) } else { 1 }
#let ease = 1 - calc.pow(1 - t, 3)
// the i-th of n staggered elements is there once the build-in passed its slot
#let appear(i, n, body) = if ease >= (i + 0.35) / calc.max(n, 1) { body } else { hide(body) }
// an element that belongs to key press k: there after it, building in during it, held before it
#let at-step(k, i, n, body) = if step > k { body } else if step == k { appear(i, n, body) } else { hide(body) }
// marked text gets an accent underline that grows with the build-in
#let marked(body) = context {
  let w = measure(body).width
  box(body + place(bottom + left, dy: 0.2em, line(length: w * ease, stroke: 1.3 * s * px + accent)))
}
// count-up: a value like "1 200" or "60 %" runs from 0 to its number, its grouping kept
#let group(n) = {
  let d = str(n)
  if d.len() <= 3 { d } else { group(int(d.slice(0, d.len() - 3))) + " " + d.slice(d.len() - 3) }
}
#let counted(value) = {
  let digits = value.replace(" ", "").replace("\u{202f}", "").replace(",", "")
  let m = digits.match(regex("^([0-9]+)(.*)$"))
  if m == none { value } else {
    let n = int(m.captures.at(0))
    group(calc.round(n * ease)) + (if m.captures.at(1) == "%" { " %" } else { m.captures.at(1) })
  }
}

// ---- inline markup ----
#let spans(xs) = xs.map(sp => {
  let body = if sp.code { raw(sp.text) } else { text(sp.text) }
  if sp.bold { body = strong(body) }
  if sp.italic { body = emph(body) }
  if sp.underline { body = marked(body) }
  body
}).join()

#set page(width: W * px, height: H * px, margin: (x: 24 * s * px, top: 16 * s * px, bottom: 26 * s * px), fill: ground)
#set text(font: sans, size: 8.4 * s * px, fill: ink, hyphenate: false, number-type: "lining")
#set par(leading: 0.55em, spacing: 0.8em, justify: false)
#set smartquote(enabled: false)
#show raw: set text(font: mono, size: 0.92em)
#show raw.where(block: false): it => box(fill: surface, inset: (x: 0.3em, y: 0.1em), radius: 2 * s * px, it)

#let small(body, fill: muted, size: 5.4) = text(font: mono, size: size * s * px, fill: fill, body)
#let heading-text(body, size) = text(font: display, weight: "black", size: size * s * px, fill: accent, body)

// ---- how many elements build in during this step, so they can be staggered ----
#let fresh = {
  let n = 0
  let rc = 0
  for b in card.blocks {
    if b.kind == "list" {
      for it in b.items {
        if it.reveal { rc += 1; if rc == step { n += 1 } } else if step == 0 { n += 1 }
      }
    } else if b.kind == "metrics" {
      if step == 0 { n += b.items.len() }
    } else if step == 0 { n += 1 }
  }
  calc.max(n, 1)
}

// ---- the blocks ----
#let marker(level, number) = if number != none { small(str(number) + ".", fill: accent, size: 7) } else {
  box(baseline: -0.2em, circle(radius: (1.7 - 0.35 * calc.min(level, 2)) * s * px, fill: if level == 0 { accent } else { muted }))
}

#let body-blocks = {
  let slot = 0
  let rc = 0
  for b in card.blocks {
    if b.kind == "paragraph" {
      at-step(0, slot, fresh, par(b.lines.map(spans).join(linebreak())))
      if step == 0 { slot += 1 }
    } else if b.kind == "subheading" {
      at-step(0, slot, fresh, block(above: 1.1em, below: 0.5em, text(font: display, weight: "bold", size: 9.5 * s * px, fill: accent, b.text)))
      if step == 0 { slot += 1 }
    } else if b.kind == "quote" {
      at-step(0, slot, fresh, block(inset: (left: 0.7em, y: 0.2em), stroke: (left: 1.6 * s * px + accent), text(fill: muted, style: "italic", b.lines.map(spans).join(linebreak()))))
      if step == 0 { slot += 1 }
    } else if b.kind == "list" {
      block(width: 100%, {
        set par(spacing: 0.45em)
        for it in b.items {
          let line = box(inset: (left: it.level * 1.2em), [#marker(it.level, it.number) #h(0.5em) #spans(it.spans)])
          let k = if it.reveal { rc += 1; rc } else { 0 }
          at-step(k, slot, fresh, line)
          if k == step { slot += 1 }
          parbreak()
        }
      })
    } else if b.kind == "code" {
      at-step(0, slot, fresh, block(width: 100%, fill: surface, inset: 0.7em, radius: 3 * s * px, raw(b.lines.join("\n"), block: true, lang: if b.lang == "" { none } else { b.lang })))
      if step == 0 { slot += 1 }
    } else if b.kind == "metrics" {
      block(above: 1em, below: 1.1em, grid(
        columns: b.items.map(it => auto),
        column-gutter: 10 * s * px,
        ..b.items.enumerate().map(((i, m)) => block(inset: (left: 0.5em, y: 0.1em), stroke: (left: (1.6 * s * px + accent)), {
          set par(leading: 0.7em)
          text(font: display, weight: "bold", size: 17 * s * px, fill: ink, counted(m.value))
          linebreak()
          at-step(0, slot + i, fresh, small(m.label))
        })),
      ))
      if step == 0 { slot += b.items.len() }
    } else if b.kind == "image" {
      let h = if b.height != none { calc.min(b.height / 15, 0.75) * H } else { 0.45 * H }
      at-step(0, slot, fresh, block(width: 100%, height: h * px, align(center + horizon, image(sys.inputs.at(b.key), width: 100%, height: 100%, fit: "contain"))))
      if step == 0 { slot += 1 }
    } else if b.kind == "error" {
      at-step(0, slot, fresh, text(fill: stop, weight: "bold", b.text))
      if step == 0 { slot += 1 }
    }
  }
}

// ---- the heading: word by word on a title card, whole otherwise ----
#let title-words = if card.title == none { () } else {
  let words = ()
  for sp in card.title {
    for w in sp.text.split(" ") {
      if w != "" { words.push((text: w, bold: sp.bold, italic: sp.italic, code: sp.code, underline: sp.underline)) }
    }
  }
  words
}
#let heading-block = if card.title == none { none } else if card.first {
  let n = title-words.len()
  block(width: 100%, {
    set par(leading: 0.42em)
    for (i, w) in title-words.enumerate() {
      appear(i, n + 1, heading-text(spans((w,)), 19))
      if i + 1 < n and not title-words.at(i + 1).text.starts-with(regex("[,.;:!?)]")) { h(0.3em) }
    }
  })
} else {
  block(width: 100%, below: 7 * s * px, {
    heading-text(spans(card.title), 13)
    v(3 * s * px)
    line(length: 100%, stroke: 0.5 * s * px + border)
  })
}

// ---- the page ----
#if sys.inputs.at("logo", default: none) != none {
  place(top + left, dy: -8 * s * px, image(sys.inputs.logo, height: 12 * s * px))
}
#if card.first {
  align(horizon, {
    heading-block
    v(6 * s * px)
    body-blocks
  })
} else {
  heading-block
  body-blocks
}

// ---- footer: the deck's title, the counter ----
#place(bottom + left, dy: 14 * s * px, grid(
  columns: (1fr, auto),
  align: (left + horizon, right + horizon),
  small(deck-title),
  text(font: mono, weight: "bold", size: 5.4 * s * px, fill: accent, str(index + 1) + " / " + str(total)),
))
