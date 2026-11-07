//! The renderer of FerroUI's 3D scenes (#608, `docs/porting/media3d.md`): what every backend
//! shares to draw the [`Scene3DSnapshot`](ferroui_base::rendering::scene3d::Scene3DSnapshot) of a
//! `Viewport3D`.
//!
//! - [`cpu`]: the CPU reference rasterizer, deterministic and in safe Rust: the reference of the
//!   comparison tests, the renderer of the surfaces without a GPU (Skia raster, headless, the CPU
//!   mode of Vello) and, until a backend has a GPU executor, the counted last resort on its GPU
//!   surfaces ([`report_scene_drawn_on_cpu`]).
//! - [`uniforms`]: the constants of a render (the camera, the lights, the matrices and colours of
//!   each draw) as `#[repr(C)]`, std140-compatible structures, the one layout the CPU reference and
//!   the shaders of the GPU executors (step D4) share.
//! - [`lighting`]: the lighting of WPF's lights and materials per pixel, ported from ProGPU's
//!   `ComputeWpfLighting`.
//! - [`output_cache`]: the output of a scene kept while the scene, the size and the scale are
//!   unchanged.
//!
//! The crate depends on the base library and std only; its GPU executors come with their steps
//! (D4: wgpu, D5: Metal, D8: GL, D9: Vulkan) as features.

#![forbid(unsafe_code)]

pub mod cpu;
pub mod lighting;
mod math;
pub mod output_cache;
pub mod uniforms;

#[cfg(test)]
mod test_scenes;

pub use cpu::{CpuRasterizer, CpuRenderOptions, Scene3DImage};
pub use output_cache::Scene3DOutputCache;

use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::{PixelSize, Rect};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The scenes drawn by the CPU reference on a GPU surface.
static SCENES_DRAWN_ON_CPU: AtomicU64 = AtomicU64::new(0);
static SCENES_DRAWN_ON_CPU_REPORTED: AtomicBool = AtomicBool::new(false);
/// The lights left out of renders, past [`uniforms::MAX_LIGHTS`].
static LIGHTS_LEFT_OUT: AtomicU64 = AtomicU64::new(0);
static LIGHTS_LEFT_OUT_REPORTED: AtomicBool = AtomicBool::new(false);
/// The layers drawn with their colour because their brush cannot be sampled yet.
static UNSAMPLED_BRUSHES: AtomicU64 = AtomicU64::new(0);
static UNSAMPLED_BRUSHES_REPORTED: AtomicBool = AtomicBool::new(false);

/// How many times a scene was drawn by the CPU reference on a GPU surface since the process
/// started: the gap of the backends without a GPU executor (`docs/porting/gpu-rendering.md`), which
/// the GPU executors of steps D4 and later close. A surface without a GPU (Skia raster, the CPU
/// mode of Vello) draws the CPU reference by definition and is not counted.
pub fn scenes_drawn_on_cpu() -> u64 {
    SCENES_DRAWN_ON_CPU.load(Ordering::Relaxed)
}

/// Counts a scene drawn by the CPU reference on a GPU surface of `backend` (a name for the
/// log, such as "Skia (GPU)"), and reports the first one in the log (area `Visual`, level
/// warning).
pub fn report_scene_drawn_on_cpu(backend: &str) {
    SCENES_DRAWN_ON_CPU.fetch_add(1, Ordering::Relaxed);
    if SCENES_DRAWN_ON_CPU_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
        logger.log(
            None,
            &format!(
                "A 3D scene (Viewport3D) is drawn by the CPU reference on a GPU surface of {backend}: the backend has no \
                 GPU executor for 3D scenes yet (docs/porting/media3d.md, docs/porting/gpu-rendering.md)."
            ),
        );
    }
}

/// How many lights were left out of renders since the process started: a render takes the
/// first [`uniforms::MAX_LIGHTS`] lights of a scene in tree order (`docs/porting/media3d.md`,
/// section 6.1).
pub fn lights_left_out() -> u64 {
    LIGHTS_LEFT_OUT.load(Ordering::Relaxed)
}

pub(crate) fn note_lights_left_out(count: usize) {
    LIGHTS_LEFT_OUT.fetch_add(count as u64, Ordering::Relaxed);
    if LIGHTS_LEFT_OUT_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
        logger.log(
            None,
            "A 3D scene has more than 16 lights: the lights after the 16th in tree order are left out \
             (docs/porting/media3d.md, section 6.1).",
        );
    }
}

/// How many material layers were drawn with their `Color` alone because the renderer cannot
/// sample their brush yet (a gradient, an image, a visual or drawing brush: step D6 of the port),
/// since the process started.
pub fn layers_with_unsampled_brushes() -> u64 {
    UNSAMPLED_BRUSHES.load(Ordering::Relaxed)
}

pub(crate) fn note_unsampled_brush() {
    UNSAMPLED_BRUSHES.fetch_add(1, Ordering::Relaxed);
    if UNSAMPLED_BRUSHES_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
        logger.log(
            None,
            "A material of a 3D scene paints with a brush the 3D renderer cannot sample yet (only solid colour brushes \
             are drawn before step D6): the layer is drawn with its Color (docs/porting/media3d.md).",
        );
    }
}

/// The size in device pixels of the image of a viewport drawn over `bounds` (in the coordinates of
/// a drawing context) whose transform to the device maps the unit x axis to `x_axis` and the
/// unit y axis to `y_axis`, and the scale (device pixels per unit, the larger of the two axes):
/// the bounds at the resolution of the device, also under a rotation or a skew
/// (`docs/porting/media3d.md`, section 3.6). The perspective of a 2D transform is not taken into
/// account. Each side is at least one pixel and at most [`cpu::MAX_SIDE`].
pub fn scene_pixel_size(bounds: Rect, x_axis: (f64, f64), y_axis: (f64, f64)) -> (PixelSize, f64) {
    let x_scale = x_axis.0.hypot(x_axis.1);
    let y_scale = y_axis.0.hypot(y_axis.1);
    let side = |length: f64| {
        if length.is_finite() {
            // A tolerance below a pixel, so that 100 units at a scale of 1.5 are 150 pixels.
            (length - 1e-6).ceil().clamp(1.0, f64::from(cpu::MAX_SIDE)) as i32
        } else {
            1
        }
    };
    let size = PixelSize::new(side(bounds.width * x_scale), side(bounds.height * y_scale));
    let scale = x_scale.max(y_scale);
    (size, if scale.is_finite() && scale > 0.0 { scale } else { 1.0 })
}
