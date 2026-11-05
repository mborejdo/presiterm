//! Typst (fletcher) diagrams: the preamble, the pixel budget and the compile call.

use std::fmt::Write as _;

use anyhow::Result;

use super::raster::image_size;

/// Pixels per inch of a Typst diagram: the terminal scales it to the cell rectangle, and scaling down
/// stays sharp where scaling up goes blocky, so draw it big.
pub(super) const TYPST_PPI: u32 = 384;

/// Points of diagram height per slide row: the preamble's text size, times the slide's line height
/// over its font size, times a little under one so a diagram's text sits just below the slide's.
pub(super) const TYPST_ROW_PT: f64 = 20.0;

/// The fletcher release the preamble imports (from Typst Universe, cached after the first use).
const FLETCHER: &str = "@preview/fletcher:0.5.8";

/// Whether `body` uses fletcher, so the preamble imports it (a table does not need the network).
fn uses_fletcher(body: &str) -> bool {
    ["diagram(", "node(", "edge("]
        .iter()
        .any(|call| body.contains(call))
}

/// The user's Typst with a preamble: no page, no ground, the theme's ink, and fletcher drawn in the
/// design tokens. Every palette token (`ink`, `muted`, `accent`, `surface`, `border`, `ok`, `warn`,
/// `stop`, `ground`) is a colour the diagram can use (`node(.., fill: ok, stroke: none)`); nodes are
/// rounded boxes with an accent outline, edges muted, and nothing is filled behind the diagram.
pub(super) fn typst_source(body: &str, tokens: &super::Look, step: usize) -> String {
    let mut out = String::from("#set page(width: auto, height: auto, margin: 8pt, fill: none)\n");
    for (name, colour) in tokens.palette.named() {
        let _ = writeln!(out, "#let {name} = rgb(\"{colour}\")");
    }
    let _ = writeln!(
        out,
        "#set text(font: \"{}\", fill: ink, size: {}pt)",
        tokens.fonts.mono, tokens.sizes.text
    );
    if uses_fletcher(body) {
        let _ = writeln!(
            out,
            "#import \"{FLETCHER}\": diagram, node, edge\n\
             #let diagram = diagram.with(node-corner-radius: 7pt, node-inset: 9pt, \
             node-stroke: 1pt + accent, edge-stroke: 1.3pt + muted, label-size: {}pt, \
             spacing: (30pt, 26pt))",
            tokens.sizes.label
        );
        // `step` is the frame (0 = nothing lit); `stepnode(i, pos, label, ..)` is a node lit in
        // frame i. Plain `node(..)` calls of a `steps=N` fence are numbered in order of appearance.
        let _ = writeln!(
            out,
            "#let step = {step}\n\
             #let stepnode(i, pos, label, ..args) = node(pos, \
             text(fill: if step == i {{ ground }} else {{ ink }}, label), \
             fill: if step == i {{ accent }} else {{ none }}, \
             stroke: if step == i {{ none }} else {{ 1pt + accent }}, ..args)"
        );
    }
    out.push_str(body);
    out.push('\n');
    out
}

/// The most pixels a picture may have: WezTerm drops an inline image that needs more than 100 MB of RAM.
pub(super) const MAX_PIXELS: u64 = 20_000_000;

/// Whether a PNG is small enough for WezTerm to take.
pub(super) fn within_budget(png: &[u8]) -> bool {
    image_size(png).is_none_or(|(w, h)| u64::from(w) * u64::from(h) <= MAX_PIXELS)
}

/// Typst source as a PNG at `TYPST_PPI`, or at a lower density when that would pass `MAX_PIXELS` (a tall
/// diagram drawn at a fixed width gets very large).
pub(super) fn typst_within_budget(source: &str) -> Result<Vec<u8>> {
    let png = compile_typst(source, TYPST_PPI)?;
    let Some((w, h)) = image_size(&png) else {
        return Ok(png);
    };
    let pixels = u64::from(w) * u64::from(h);
    if pixels <= MAX_PIXELS {
        return Ok(png);
    }
    let scale = (MAX_PIXELS as f64 / pixels as f64).sqrt();
    compile_typst(source, ((f64::from(TYPST_PPI) * scale) as u32).max(24))
}

#[cfg(feature = "typst")]
fn compile_typst(source: &str, ppi: u32) -> Result<Vec<u8>> {
    use std::sync::OnceLock;

    // Fonts are parsed and packages resolved once per process.
    static ENGINE: OnceLock<typstkit::Engine> = OnceLock::new();
    ENGINE
        .get_or_init(|| typstkit::Engine::builder().build())
        .png(typstkit::Main::Source(source), ppi as f32)
}

#[cfg(not(feature = "typst"))]
fn compile_typst(_source: &str, _ppi: u32) -> Result<Vec<u8>> {
    anyhow::bail!("presiterm was built without the `typst` feature")
}
