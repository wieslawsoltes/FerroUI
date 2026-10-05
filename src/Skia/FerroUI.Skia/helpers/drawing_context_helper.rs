use crate::drawing_context_impl::{CanvasSource, CreateInfo, DrawingContextImpl};
use ferroui_base::media::IPen;
use ferroui_base::Vector;
use skia_safe::{PathEffect, Surface};

/// Wraps a Skia surface in a drawing context that draws onto its canvas.
///
/// Text is drawn without subpixel antialiasing.
pub fn wrap_skia_surface(surface: &Surface, dpi: Vector) -> DrawingContextImpl {
    let create_info = CreateInfo {
        canvas: Some(CanvasSource::Surface(surface.clone())),
        dpi,
        disable_subpixel_text_rendering: true,
        ..CreateInfo::default()
    };

    DrawingContextImpl::new(create_info, Vec::new())
}

/// Tries to create a dash effect for the pen's dash style.
///
/// The dash lengths are relative to the pen thickness. An odd number of
/// dashes is repeated to produce an even number of on/off intervals.
pub fn try_create_dash_effect(pen: Option<&dyn IPen>) -> Option<PathEffect> {
    let pen = pen?;
    let dash_style = pen.dash_style()?;
    let src_dashes = dash_style.dashes()?;

    if src_dashes.is_empty() {
        return None;
    }

    let count = if src_dashes.len() % 2 == 0 { src_dashes.len() } else { src_dashes.len() * 2 };
    let thickness = pen.thickness() as f32;
    let dashes_array: Vec<f32> = (0..count).map(|i| src_dashes[i % src_dashes.len()] as f32 * thickness).collect();

    let offset = (dash_style.offset() * pen.thickness()) as f32;

    PathEffect::dash(&dashes_array, offset)
}
