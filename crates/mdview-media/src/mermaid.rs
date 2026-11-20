//! Mermaid diagram rendering with `merman`, a headless Rust port of Mermaid.
//!
//! `merman` follows Mermaid's own parsers, dagre layout, and SVG structure, so complex
//! diagrams (nested subgraphs, composite states, sequence fragments) lay out the way
//! GitHub shows them. Its resvg-safe pipeline replaces HTML labels with SVG text, and
//! text is measured with the face resvg draws so labels fit their boxes.

use std::sync::Arc;

use merman::svg::{
    DeterministicTextMeasurer, MeasurementProfileId, RootBackgroundPostprocessor, SvgPipeline, TextMeasurementPolicy,
    TextMeasurementProfile, TextMeasurementProfileIdentity,
};
use merman::{
    MermaidEngine, OperationControl, RenderOutput, RenderRequest, Renderer, SourcePresentationConfig, SvgEnvironment,
    SvgRequest,
};

use crate::mermaid_font::DiagramFont;
use crate::{MediaError, MediaKind, MediaRequest, Raster, rasterize_svg};

/// The tallest raster a diagram may produce, in pixels. Tall diagrams keep their
/// natural size up to this height instead of being shrunk to fit the width.
const MAX_DIAGRAM_HEIGHT: u32 = 4_096;

/// Render a Mermaid diagram to an RGBA raster.
pub fn render_mermaid(source: &str, dark: bool, max_width: u32) -> Result<Raster, MediaError> {
    let svg = mermaid_svg(source, dark)?;
    let mut request = MediaRequest::new(MediaKind::Svg, source);
    request.max_width = max_width.max(64);
    request.max_height = MAX_DIAGRAM_HEIGHT;
    request.dark = dark;
    rasterize_svg(svg.as_bytes(), &request).map_err(|error| match error {
        MediaError::Svg(message) => MediaError::Mermaid(message),
        other => other,
    })
}

/// Lay out a Mermaid diagram and return its resvg-safe SVG.
pub(crate) fn mermaid_svg(source: &str, dark: bool) -> Result<String, MediaError> {
    let source = source.trim();
    if source.is_empty() {
        return Err(MediaError::Empty);
    }
    let font = DiagramFont::shared(&crate::svg::system_fonts());
    let mut site = serde_json::json!({
        // GitHub renders Mermaid with its default theme in light mode and `dark` in dark mode.
        "theme": if dark { "dark" } else { "default" },
    });
    if let Some(font) = &font {
        site["fontFamily"] = serde_json::Value::String(font.css_family());
    }
    let renderer = Renderer::new()
        .with_presentation_engine(MermaidEngine::new().with_site_config(SourcePresentationConfig::from_value(site)));
    let pipeline = SvgPipeline::resvg_safe().with_postprocessor(RootBackgroundPostprocessor::new(
        // Match the reading surface. The exact app token is not available in this
        // egui-free crate; these are the two base surfaces.
        if dark { "#1e1e1e" } else { "#ffffff" },
    ));
    let mut request = SvgRequest {
        pipeline: Some(pipeline),
        ..SvgRequest::default()
    };
    if let Some(font) = font {
        request.environment = SvgEnvironment::deterministic().with_text_measurement_policy(measurement(font)?);
    }
    match renderer.render(RenderRequest::svg(source, OperationControl::new(), request)) {
        Ok(RenderOutput::Svg(Some(svg))) => Ok(svg.svg().to_owned()),
        Ok(_) => Err(MediaError::Mermaid(
            "This is not a supported Mermaid diagram type.".to_owned(),
        )),
        Err(error) => Err(MediaError::Mermaid(error.to_string())),
    }
}

fn measurement(font: Arc<DiagramFont>) -> Result<TextMeasurementPolicy, MediaError> {
    let invalid = |error: &dyn std::fmt::Display| MediaError::Mermaid(format!("text measurement: {error}"));
    let profile = MeasurementProfileId::new("mdview.diagram-font").map_err(|error| invalid(&error))?;
    let identity = TextMeasurementProfileIdentity::new(profile, font.identity()).map_err(|error| invalid(&error))?;
    let measurer = DeterministicTextMeasurer::default()
        .with_width_callback(move |text, style| font.width_px(text, style.font_size, style.font_weight.as_deref()));
    Ok(TextMeasurementPolicy::uniform(TextMeasurementProfile::new(
        identity, measurer,
    )))
}

#[cfg(test)]
#[path = "mermaid_tests.rs"]
mod tests;
