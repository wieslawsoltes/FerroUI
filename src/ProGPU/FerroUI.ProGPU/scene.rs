//! The retained scene: the owned builder over the engine's C ABI and the stream it builds
//! (docs/porting/progpu-backend.md, sections 1 and 2.3).

use progpu_native_sys::progpu::*;
use progpu_native_sys::scene_builder::*;
use std::ffi::c_void;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// No resource (`PROGPU_NATIVE_SCENE_NO_INDEX`).
pub(crate) const NO_INDEX: u32 = PROGPU_NATIVE_SCENE_NO_INDEX as u32;

static NEXT_SCENE_ID: AtomicU64 = AtomicU64::new(1);

/// A builder call the builder refused (its `scene_build_error`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneBuildError {
    /// The name of the call.
    pub call: &'static str,
    /// The builder's error code.
    pub code: u32,
}

impl fmt::Display for SceneBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the scene builder refused {} (error {})", self.call, self.code)
    }
}

impl std::error::Error for SceneBuildError {}

/// The retained scene builder of the engine, owned. Every scene has an identifier of its own;
/// its generation advances when its content changes.
pub struct SceneBuilder {
    raw: *mut progpu_native_scene_builder,
    scene_id: u64,
    generation: u64,
    stream: Vec<u8>,
}

impl SceneBuilder {
    /// A builder of a new scene, at its first generation.
    pub fn new() -> Self {
        let scene_id = NEXT_SCENE_ID.fetch_add(1, Ordering::Relaxed);
        // SAFETY: creates an owned builder; released in `drop`.
        let raw = unsafe { progpu_native_scene_builder_create(scene_id, 1) };
        assert!(!raw.is_null(), "the scene builder could not be allocated");
        Self { raw, scene_id, generation: 1, stream: Vec::new() }
    }

    /// The identifier of the scene.
    pub fn scene_id(&self) -> u64 {
        self.scene_id
    }

    /// The generation of the content being recorded.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn check(&self, call: &'static str, ok: i32) -> Result<(), SceneBuildError> {
        if ok != 0 {
            Ok(())
        } else {
            // SAFETY: the builder is alive.
            Err(SceneBuildError { call, code: unsafe { progpu_native_scene_builder_last_error(self.raw) } })
        }
    }

    /// Starts recording the next generation of the scene: everything recorded before is dropped.
    pub fn begin_generation(&mut self) -> Result<(), SceneBuildError> {
        self.generation += 1;
        // SAFETY: the builder is alive.
        let ok = unsafe { progpu_native_scene_builder_reset(self.raw, self.scene_id, self.generation) };
        self.check("reset", ok)
    }

    /// Replaces the target with `color` (through the current clip).
    pub fn clear_target(&mut self, color: progpu_native_color) -> Result<(), SceneBuildError> {
        // SAFETY: the builder is alive.
        let ok = unsafe { progpu_native_scene_builder_clear_target(self.raw, color) };
        self.check("clear_target", ok)
    }

    /// Adds a solid brush; returns its index.
    pub fn add_solid_brush(&mut self, color: progpu_native_color, opacity: f32) -> Result<u32, SceneBuildError> {
        let mut index = 0;
        // SAFETY: the builder is alive; `index` is written by the call.
        let ok = unsafe { progpu_native_scene_builder_add_solid_brush(self.raw, color, opacity, &mut index) };
        self.check("add_solid_brush", ok).map(|()| index)
    }

    /// Adds a brush with its gradient stops; returns its index.
    pub fn add_brush(
        &mut self,
        brush: &progpu_native_scene_brush,
        stops: &[progpu_native_scene_gradient_stop],
    ) -> Result<u32, SceneBuildError> {
        let mut index = 0;
        // SAFETY: the spans are borrowed for the call.
        let ok = unsafe { progpu_native_scene_builder_add_brush(self.raw, brush, stops.as_ptr(), stops.len(), &mut index) };
        self.check("add_brush", ok).map(|()| index)
    }

    /// Adds a state (transform, opacity, clip); returns its resource index.
    pub fn add_state(&mut self, state: &progpu_native_scene_state) -> Result<u32, SceneBuildError> {
        let mut index = 0;
        // SAFETY: the builder is alive; `index` is written by the call.
        let ok = unsafe { progpu_native_scene_builder_add_state(self.raw, state, &mut index) };
        self.check("add_state", ok).map(|()| index)
    }

    /// Opens a scope with the state of `state_index`.
    pub fn save(&mut self, state_index: u32) -> Result<(), SceneBuildError> {
        // SAFETY: the builder is alive.
        let ok = unsafe { progpu_native_scene_builder_save(self.raw, state_index) };
        self.check("save", ok)
    }

    /// Closes the scope opened last.
    pub fn restore(&mut self) -> Result<(), SceneBuildError> {
        // SAFETY: the builder is alive.
        let ok = unsafe { progpu_native_scene_builder_restore(self.raw) };
        self.check("restore", ok)
    }

    /// Opens a layer.
    pub fn push_layer(&mut self, layer: &progpu_native_scene_layer) -> Result<(), SceneBuildError> {
        // SAFETY: the builder is alive.
        let ok = unsafe { progpu_native_scene_builder_push_layer(self.raw, layer) };
        self.check("push_layer", ok)
    }

    /// Closes the layer opened last.
    pub fn pop_layer(&mut self) -> Result<(), SceneBuildError> {
        // SAFETY: the builder is alive.
        let ok = unsafe { progpu_native_scene_builder_pop_layer(self.raw) };
        self.check("pop_layer", ok)
    }

    /// Adds an axis-aligned clip mask; returns its resource index.
    pub fn add_axis_aligned_clip_mask(&mut self, bounds: progpu_native_image_rect) -> Result<u32, SceneBuildError> {
        let mut index = 0;
        // SAFETY: the builder is alive; `index` is written by the call.
        let ok = unsafe { progpu_native_scene_builder_add_axis_aligned_clip_mask(self.raw, bounds, &mut index) };
        self.check("add_axis_aligned_clip_mask", ok).map(|()| index)
    }

    /// Adds a rounded-rectangle mask; returns its resource index.
    pub fn add_rounded_rectangle_mask(&mut self, mask: &progpu_native_scene_layer_mask) -> Result<u32, SceneBuildError> {
        let mut index = 0;
        // SAFETY: the builder is alive; `index` is written by the call.
        let ok = unsafe { progpu_native_scene_builder_add_rounded_rectangle_mask(self.raw, mask, &mut index) };
        self.check("add_rounded_rectangle_mask", ok).map(|()| index)
    }

    /// Adds a vector clip mask of paths; returns its resource index.
    pub fn add_vector_clip_mask(
        &mut self,
        paths: &[progpu_native_scene_clip_path],
        segments: &[progpu_native_path_segment],
        opacity: f32,
    ) -> Result<u32, SceneBuildError> {
        let mut index = 0;
        // SAFETY: the spans are borrowed for the call.
        let ok = unsafe {
            progpu_native_scene_builder_add_vector_clip_mask(
                self.raw,
                paths.as_ptr(),
                paths.len(),
                segments.as_ptr(),
                segments.len(),
                opacity,
                &mut index,
            )
        };
        self.check("add_vector_clip_mask", ok).map(|()| index)
    }

    /// Draws analytic primitives (rectangles, rounded rectangles, ellipses, their strokes).
    pub fn draw_analytic(
        &mut self,
        primitives: &[progpu_native_analytic_primitive],
        brush_indices: &[u32],
        bounds: progpu_native_image_rect,
        state_index: u32,
    ) -> Result<(), SceneBuildError> {
        // SAFETY: the spans are borrowed for the call.
        let ok = unsafe {
            progpu_native_scene_builder_draw_analytic(
                self.raw,
                primitives.as_ptr(),
                primitives.len(),
                brush_indices.as_ptr(),
                brush_indices.len(),
                bounds,
                state_index,
            )
        };
        self.check("draw_analytic", ok)
    }

    /// Fills paths.
    pub fn draw_paths(
        &mut self,
        paths: &[progpu_native_scene_path_fill],
        segments: &[progpu_native_path_segment],
        brush_indices: &[u32],
        bounds: progpu_native_image_rect,
        state_index: u32,
    ) -> Result<(), SceneBuildError> {
        // SAFETY: the spans are borrowed for the call.
        let ok = unsafe {
            progpu_native_scene_builder_draw_paths(
                self.raw,
                paths.as_ptr(),
                paths.len(),
                segments.as_ptr(),
                segments.len(),
                brush_indices.as_ptr(),
                brush_indices.len(),
                bounds,
                state_index,
            )
        };
        self.check("draw_paths", ok)
    }

    /// Strokes figures.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_strokes(
        &mut self,
        strokes: &[progpu_native_scene_stroke],
        points: &[progpu_native_point],
        doubles: &[f64],
        brush_indices: &[u32],
        bounds: progpu_native_image_rect,
        state_index: u32,
    ) -> Result<(), SceneBuildError> {
        // SAFETY: the spans are borrowed for the call.
        let ok = unsafe {
            progpu_native_scene_builder_draw_strokes(
                self.raw,
                strokes.as_ptr(),
                strokes.len(),
                points.as_ptr(),
                points.len(),
                doubles.as_ptr(),
                doubles.len(),
                brush_indices.as_ptr(),
                brush_indices.len(),
                bounds,
                state_index,
            )
        };
        self.check("draw_strokes", ok)
    }

    /// Adds an image of pixels (four bytes per pixel, `RGBA` or `BGRA` with `bgra`), copied into
    /// the scene.
    pub fn add_pixel_image(&mut self, width: u32, height: u32, row_bytes: u32, pixels: &[u8], bgra: bool) -> Result<u32, SceneBuildError> {
        let mut index = NO_INDEX;
        // SAFETY: the pixels are borrowed for the call.
        let ok = unsafe {
            if bgra {
                progpu_native_scene_builder_add_bgra8_image(self.raw, width, height, row_bytes, pixels.as_ptr() as *const c_void, pixels.len(), &mut index)
            } else {
                progpu_native_scene_builder_add_rgba8_image(self.raw, width, height, row_bytes, pixels.as_ptr() as *const c_void, pixels.len(), &mut index)
            }
        };
        self.check("add_image", ok)?;
        Ok(index)
    }

    /// Adds an image the engine samples from a same-device texture view bound outside the stream
    /// (`ProGpuEngine::bind_external_images`). Returns the index of the image and the identity
    /// of its resource, an identifier and a generation: the view is bound under that pair.
    ///
    /// The identity is the one the builder gives every resource, the position of the resource
    /// in the scene and the generation of the scene. The engine holds a scene to both: the
    /// identifiers of a scene ascend with its resources, and the generation of an identifier
    /// never goes back from one generation of the scene to the next.
    pub fn add_external_image(&mut self, width: u32, height: u32) -> Result<(u32, u64, u64), SceneBuildError> {
        let mut index = NO_INDEX;
        // SAFETY: the builder is alive.
        let ok = unsafe { progpu_native_scene_builder_add_external_image(self.raw, width, height, &mut index) };
        self.check("add_external_image", ok)?;
        Ok((index, u64::from(index) + 1, self.generation))
    }

    /// Draws an image resource.
    pub fn draw_image(
        &mut self,
        image_index: u32,
        image: &progpu_native_scene_image_draw,
        bounds: progpu_native_image_rect,
        state_index: u32,
    ) -> Result<(), SceneBuildError> {
        // SAFETY: the record is borrowed for the call.
        let ok = unsafe { progpu_native_scene_builder_draw_image(self.raw, image_index, image, bounds, state_index, std::ptr::null()) };
        self.check("draw_image", ok)
    }

    /// Builds the stream of what was recorded into the builder's buffer (reused across
    /// generations) and returns it.
    pub fn build(&mut self) -> Result<&[u8], SceneBuildError> {
        // SAFETY: the builder is alive; the buffer is sized to the stream before the call.
        unsafe {
            let size = progpu_native_scene_builder_required_stream_size(self.raw);
            self.stream.resize(size, 0);
            let mut written = 0usize;
            let ok = progpu_native_scene_builder_build_into(
                self.raw,
                self.stream.as_mut_ptr() as *mut c_void,
                self.stream.len(),
                &mut written,
            );
            self.check("build_into", ok)?;
            self.stream.truncate(written);
        }
        Ok(&self.stream)
    }

    /// The identity transform of the engine.
    pub fn identity_transform() -> progpu_native_affine_2d {
        // SAFETY: a pure function of the ABI.
        unsafe { progpu_native_scene_builder_identity_transform() }
    }

    /// The identity state of the engine.
    pub fn identity_state() -> progpu_native_scene_state {
        // SAFETY: a pure function of the ABI.
        unsafe { progpu_native_scene_builder_identity_state() }
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SceneBuilder {
    fn drop(&mut self) {
        // SAFETY: the builder is owned here.
        unsafe { progpu_native_scene_builder_destroy(self.raw) };
    }
}
