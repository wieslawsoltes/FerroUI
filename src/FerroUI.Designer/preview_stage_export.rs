//! Pixel-bounded screenshots of an already accepted native stage. Capture does not
//! arrange, reparent, refresh or pulse the preview; its host validates source currency.
use ferroui_base::layout::Layoutable;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelSize, Ref, Size, Vector, Visual};
use ferroui_controls::Control;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

pub const MAX_PREVIEW_STAGE_EXPORT_PIXELS: u64 = 16_777_216;
pub const MAX_PREVIEW_STAGE_EXPORT_BYTES: usize = 67_108_864;
const MAX_PIXEL_DIMENSION: f64 = 16_384.0;

/// Raster density relative to the stage's existing logical layout. This changes
/// the bitmap density, never the application's available layout space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewStageExportOptions {
    pub scale: f64,
}

impl Default for PreviewStageExportOptions {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl PreviewStageExportOptions {
    pub fn validate(&self) -> Result<(), String> {
        if !self.scale.is_finite() || !(0.25..=4.0).contains(&self.scale) {
            return Err("Preview screenshot scale must be between 0.25 and 4".into());
        }
        Ok(())
    }

    fn pixel_size(self, logical: Size) -> Result<PixelSize, String> {
        self.validate()?;
        if !logical.width.is_finite()
            || !logical.height.is_finite()
            || !(1.0..=16_384.0).contains(&logical.width)
            || !(1.0..=16_384.0).contains(&logical.height)
        {
            return Err(
                "Arrange the preview at a finite logical size before taking a screenshot".into(),
            );
        }
        let width = (logical.width * self.scale).ceil();
        let height = (logical.height * self.scale).ceil();
        if width > MAX_PIXEL_DIMENSION || height > MAX_PIXEL_DIMENSION {
            return Err("Preview screenshots support at most 16384 pixels on either axis; reduce the scale or preview size".into());
        }
        if width as u64 * height as u64 > MAX_PREVIEW_STAGE_EXPORT_PIXELS {
            return Err("Preview screenshots support at most 16777216 pixels; reduce the scale or preview size".into());
        }
        Ok(PixelSize::new(width as i32, height as i32))
    }
}

/// A captured PNG and the dimensions of the exact native frame that produced it.
/// The image is owned bytes, so a save picker may await without retaining controls.
#[derive(Debug)]
pub struct PreviewStageImage {
    pub png: Vec<u8>,
    pub logical_width: f64,
    pub logical_height: f64,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub scale: f64,
}

/// Capture a native stage at its existing arranged size, excluding all ancestors
/// such as the board's Viewbox, card title and commands. Pass the board's stage
/// control, not its card. No application input, layout, source or clock is changed.
///
/// An embedding host must first check that the accepted stage still represents
/// the current source and requested configuration. `DesignerWorkspace` performs
/// those checks in its screenshot API. This helper is also useful to custom hosts
/// which own their stage lifecycle directly.
pub fn capture_preview_stage_png(
    control: &Ref<Control>,
    options: PreviewStageExportOptions,
) -> Result<PreviewStageImage, String> {
    if !Dispatcher::ui_thread().check_access() {
        return Err("Capture preview screenshots on the UI thread".into());
    }
    let logical = control.bounds().size();
    let pixels = options.pixel_size(logical)?;
    if !control.is_measure_valid() || !control.is_arrange_valid() {
        return Err("Wait for the preview layout to settle before taking a screenshot".into());
    }
    validate_visible_layout(&control.clone().upcast())?;
    if FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .is_none()
    {
        return Err("This host has no native rendering backend for preview screenshots".into());
    }
    let png = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let bitmap = BitmapGuard(RenderTargetBitmap::with_dpi(
            pixels,
            Vector::new(96.0 * options.scale, 96.0 * options.scale),
        ));
        bitmap.0.render(&control.clone().upcast());
        let mut output = BoundedPng::default();
        bitmap
            .0
            .save(&mut output, &PngBitmapEncoderOptions::default().into())
            .map_err(|error| format!("Could not encode the preview screenshot: {error}"))?;
        if output.bytes.is_empty() {
            return Err("The native renderer returned an empty preview screenshot".into());
        }
        if control.bounds().size() != logical
            || !control.is_measure_valid()
            || !control.is_arrange_valid()
        {
            return Err(
                "The preview layout changed during capture; take the screenshot again".into(),
            );
        }
        validate_visible_layout(&control.clone().upcast())?;
        Ok(output.bytes)
    }))
    .unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|value| (*value).to_owned())
            })
            .unwrap_or_else(|| "The native rendering backend failed".into());
        Err(format!(
            "Could not render the preview screenshot: {message}"
        ))
    })?;
    Ok(PreviewStageImage {
        png,
        logical_width: logical.width,
        logical_height: logical.height,
        pixel_width: pixels.width as u32,
        pixel_height: pixels.height as u32,
        scale: options.scale,
    })
}

fn validate_visible_layout(root: &Ref<Visual>) -> Result<(), String> {
    let mut pending = vec![root.clone()];
    let mut examined = 0usize;
    while let Some(visual) = pending.pop() {
        examined += 1;
        if examined > 100_000 {
            return Err("Preview screenshots inspect at most 100000 visual nodes".into());
        }
        // Match the native immediate renderer's subtree visibility rules. Hidden
        // templates may legitimately retain invalid layout and are not captured.
        let opacity = visual.opacity();
        if !visual.is_visible()
            || visual.is_render_suppressed()
            || opacity.is_nan()
            || opacity <= 0.0
        {
            continue;
        }
        if let Some(layout) = visual.cast::<Layoutable>() {
            if !layout.is_measure_valid() || !layout.is_arrange_valid() {
                return Err(
                    "Wait for the preview layout to settle before taking a screenshot".into(),
                );
            }
        }
        if let Some(children) = visual.visual_children_snapshot() {
            if pending.len() + children.len() > 100_000 - examined {
                return Err("Preview screenshots inspect at most 100000 visual nodes".into());
            }
            pending.extend(children.iter().cloned());
        }
    }
    Ok(())
}

struct BitmapGuard(RenderTargetBitmap);
impl Drop for BitmapGuard {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

#[derive(Default)]
struct BoundedPng {
    bytes: Vec<u8>,
}
impl Write for BoundedPng {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_PREVIEW_STAGE_EXPORT_BYTES.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other(
                "The encoded preview screenshot exceeds 64 MiB",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "preview_stage_export_tests.rs"]
mod tests;
