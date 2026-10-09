use crate::drawing_context_impl::{CanvasSource, CreateInfo, DrawingContextImpl};
use crate::SkiaPlatform;
use ferroui_base::media::{DrawingContext, IPen, PlatformDrawingContext};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::rendering::ImmediateRenderer;
use ferroui_base::{Rect, Ref, Vector, Visual};
use skia_safe::{PathEffect, Surface};
use std::future::{ready, Ready};

/// Renders a visual onto the canvas of a Skia surface, the whole of it at
/// the default DPI; see [`render_async_clipped`].
pub fn render_async(surface: &Surface, visual: &Ref<Visual>) -> Ready<()> {
    render_async_clipped(surface, visual, visual.bounds(), SkiaPlatform::default_dpi())
}

/// Renders a visual onto the canvas of a Skia surface.
/// This is useful in scenarios where the surface is not controlled by the
/// application, but received from another API.
///
/// `clip_rect` is the clipping rectangle and `dpi` the DPI of the drawings.
/// The visual is rendered when the function returns: the result is a
/// completed future, as the task of the original is a completed one.
// Deviation (DEVIATIONS.md, Skia backend): upstream takes an `SKCanvas`;
// a drawing context owns what it draws to here, so the function takes the
// surface the canvas belongs to, as `wrap_skia_surface` does.
pub fn render_async_clipped(surface: &Surface, visual: &Ref<Visual>, clip_rect: Rect, dpi: Vector) -> Ready<()> {
    let mut drawing_context_impl = wrap_skia_surface(surface, dpi);
    {
        let mut core = PlatformDrawingContext::borrowed(&mut drawing_context_impl);
        let mut drawing_context = DrawingContext::new(&mut core);
        ImmediateRenderer::render_clipped(&mut drawing_context, visual, clip_rect);
        drawing_context.dispose();
    }
    drawing_context_impl.dispose();
    ready(())
}

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
