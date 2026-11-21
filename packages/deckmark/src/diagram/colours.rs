//! The colours and the face a mermaid diagram is drawn in, from the design tokens.

use swatchbook::{Fonts, Palette};

/// Build the mermaid options for a palette: nodes on the surface with an accent edge, text in
/// the ink, lines muted, no ground behind the diagram, labels in the sans face.
#[must_use]
pub fn options(palette: &Palette, fonts: &Fonts) -> mermaid_rs_renderer::RenderOptions {
    let mut options = mermaid_rs_renderer::RenderOptions::modern();
    let theme = &mut options.theme;
    let hex = |colour: swatchbook::Rgb| colour.to_string();
    theme.font_family.clone_from(&fonts.sans);
    theme.primary_color = hex(palette.surface);
    theme.primary_border_color = hex(palette.accent);
    theme.primary_text_color = hex(palette.ink);
    theme.text_color = hex(palette.ink);
    theme.line_color = hex(palette.muted);
    theme.secondary_color = hex(palette.surface);
    theme.tertiary_color = hex(palette.ground);
    theme.edge_label_background = "transparent".into();
    theme.cluster_background = hex(palette.surface);
    theme.cluster_border = hex(palette.border);
    theme.background = "none".into();
    options
}
