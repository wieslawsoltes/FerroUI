//! Owned native resources behind the Windows SDK bridge.
use std::ffi::c_void;
pub type Ptr = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DrawCommand {
    pub kind: i32,
    pub blend: i32,
    pub aliased: i32,
    pub interpolation: i32,
    pub matrix: [f64; 6],
    pub params: [f64; 4],
    pub resource: Ptr,
    pub shift: f64,
    pub text_aliased: i32,
    pub fill_even_odd: i32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Command {
    pub kind: i32,
    pub p: [f64; 6],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub glyph: u16,
    pub cluster: i32,
    pub advance: f64,
    pub x: f64,
    pub y: f64,
}
extern "C" {
    #[cfg(test)]
    pub fn d2_shape_reference(
        font: Ptr,
        text: *const u16,
        n: usize,
        start: usize,
        length: usize,
        language: *const i8,
        size: f64,
        rtl: i32,
        tags: *const u32,
        values: *const i32,
        feature_starts: *const u32,
        feature_ends: *const u32,
        features: usize,
        info: Ptr,
        emit: extern "C" fn(Ptr, Glyph),
    ) -> i32;
    pub fn d2_glyph_run_bounds(
        font: Ptr,
        size: f64,
        ids: *const u16,
        positions: *const f64,
        count: usize,
        bounds: *mut f64,
    );
    pub fn d2_draw_batch(surface: Ptr, commands: *const DrawCommand, count: usize);
    pub fn d2_positioned_glyph_run(
        font: Ptr,
        size: f64,
        ids: *const u16,
        positions: *const f64,
        n: usize,
        advances: *const f64,
    ) -> Ptr;
    #[cfg(test)]
    pub fn d2_draw_positioned_glyph_run(
        surface: Ptr,
        run: Ptr,
        shift: f64,
        aliased: i32,
        r: f64,
        g: f64,
        b: f64,
        a: f64,
    );
    pub fn d2_supported() -> i32;
    pub fn d2_release(p: Ptr);
    pub fn d2_font(name: *const i8, weight: i32, italic: i32, stretch: i32) -> Ptr;
    pub fn d2_font_variations(font: Ptr, tags: *const u32, values: *const f64, count: usize)
        -> Ptr;
    pub fn d2_font_family_faces(family: *const i8) -> Ptr;
    pub fn d2_path_intercepts(path: Ptr, lower: f64, upper: f64) -> Ptr;
    pub fn d2_font_data(data: *const u8, n: usize) -> Ptr;
    pub fn d2_font_name(font: Ptr, out: *mut u8, n: usize);
    pub fn d2_font_table(font: Ptr, tag: u32) -> Ptr;
    pub fn d2_font_names() -> Ptr;
    pub fn d2_font_match(font: Ptr, text: *const u16, n: usize, locale: *const i8) -> Ptr;
    pub fn d2_data_len(data: Ptr) -> usize;
    pub fn d2_data_bytes(data: Ptr) -> *const u8;
    pub fn d2_shape(
        font: Ptr,
        text: *const u16,
        n: usize,
        start: usize,
        length: usize,
        language: *const i8,
        size: f64,
        rtl: i32,
        tags: *const u32,
        values: *const i32,
        feature_starts: *const u32,
        feature_ends: *const u32,
        features: usize,
        info: Ptr,
        emit: extern "C" fn(Ptr, Glyph),
    ) -> i32;
    pub fn d2_path(cmd: *const Command, n: usize) -> Ptr;
    pub fn d2_path_elements(path: Ptr, info: Ptr, emit: extern "C" fn(Ptr, Command));
    pub fn d2_glyph_path(font: Ptr, glyph: u16, size: f64) -> Ptr;
    pub fn d2_combine(a: Ptr, b: Ptr, op: i32, ea: i32, eb: i32) -> Ptr;
    pub fn d2_stroke(
        path: Ptr,
        width: f64,
        cap: i32,
        join: i32,
        miter: f64,
        dash: *const f64,
        n: usize,
        phase: f64,
    ) -> Ptr;
    pub fn d2_surface(w: usize, h: usize) -> Ptr;
    /// 1 when `len` bytes of premultiplied BGRA were read, 0 when the read failed.
    pub fn d2_read_pixels(ctx: Ptr, out: *mut u8, len: usize) -> i32;
    pub fn d2_write_pixels(ctx: Ptr, data: *const u8, len: usize);
    pub fn d2_retain(p: Ptr) -> Ptr;
    pub fn d2_snapshot(ctx: Ptr) -> Ptr;
    pub fn d2_save(ctx: Ptr);
    pub fn d2_restore(ctx: Ptr);
    pub fn d2_clear(ctx: Ptr, w: f64, h: f64, r: f64, g: f64, b: f64, a: f64);
    pub fn d2_clip(ctx: Ptr, path: Ptr, evenodd: i32);
    pub fn d2_gradient(
        ctx: Ptr,
        colors: *const f64,
        stops: *const f64,
        n: usize,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        radius: f64,
        radial: i32,
    );
    pub fn d2_begin_layer(ctx: Ptr, opacity: f64, blend: i32);
    pub fn d2_end_layer(ctx: Ptr);
    pub fn d2_draw_image(
        ctx: Ptr,
        image: Ptr,
        sx: f64,
        sy: f64,
        sw: f64,
        sh: f64,
        dx: f64,
        dy: f64,
        dw: f64,
        dh: f64,
        alpha: f64,
    );
    #[cfg(test)]
    pub fn d2_glyphs(
        ctx: Ptr,
        font: Ptr,
        size: f64,
        glyphs: *const u16,
        positions: *const f64,
        n: usize,
        r: f64,
        g: f64,
        b: f64,
        a: f64,
    );
    pub fn d2_decode(bytes: *const u8, n: usize) -> Ptr;
    pub fn d2_image_width(image: Ptr) -> usize;
    pub fn d2_image_height(image: Ptr) -> usize;
    pub fn d2_encode(image: Ptr, jpeg: i32, quality: f64) -> Ptr;
}
/// An owned native bridge reference. Its thread rules are enforced by its container:
/// fonts/paths/images are immutable; bitmap contexts are only used under Surface's mutex.
pub struct Owned(pub Ptr);
impl Owned {
    pub unsafe fn new(p: Ptr) -> Self {
        assert!(
            !p.is_null(),
            "Windows SDK could not create the native resource; see the Direct2D diagnostic"
        );
        Self(p)
    }
    /// The reference, or `None` when the native call failed: for the calls whose failure is a
    /// state the caller handles (a size out of range, a lost device), not a broken invariant.
    pub unsafe fn try_new(p: Ptr) -> Option<Self> {
        (!p.is_null()).then(|| Self(p))
    }
    pub fn bytes(&self) -> Vec<u8> {
        unsafe {
            let length = d2_data_len(self.0);
            if length == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(d2_data_bytes(self.0), length).to_vec()
            }
        }
    }
}
/// A null reference: the state of a container whose native resource could not be created.
/// Releasing it does nothing.
impl Default for Owned {
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { d2_release(self.0) }
    }
}
// SAFETY: ownership can move between threads; mutable drawing context access is mutex protected.
unsafe impl Send for Owned {}
extern "C" {
    pub fn d2_set_transform(
        ctx: Ptr,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        x: f64,
        y: f64,
        sx: f64,
        sy: f64,
    );
}

extern "C" {
    pub fn d2_gpu_draw_surface(
        ctx: Ptr,
        source: Ptr,
        sx: f64,
        sy: f64,
        sw: f64,
        sh: f64,
        dx: f64,
        dy: f64,
        dw: f64,
        dh: f64,
        alpha: f64,
    );
    pub fn d2_gpu_copy_surface(ctx: Ptr, source: Ptr);
    #[cfg(feature = "runtime-shaders")]
    pub fn d2_gpu_shader_compile(
        source: *const i8,
        vertex: *const i8,
        fragment: *const i8,
        error: *mut u8,
        length: usize,
    ) -> Ptr;
    #[cfg(feature = "runtime-shaders")]
    pub fn d2_gpu_shader_apply(
        surface: Ptr,
        pipeline: Ptr,
        uniforms: *const u8,
        size: usize,
        children: *const Ptr,
        count: usize,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    );
}

extern "C" {
    pub fn d2_gpu_blur(ctx: Ptr, sigma: f64, ax: f64, ay: f64, bx: f64, by: f64);
    pub fn d2_gpu_try_draw_box_shadow(ctx: Ptr, key: &[u64; 24], x: f32, y: f32) -> i32;
    pub fn d2_gpu_draw_box_shadow(
        ctx: Ptr,
        key: &[u64; 24],
        x: f32,
        y: f32,
        caster: Ptr,
        mask: Ptr,
        sigma: f64,
        axes: &[f64; 4],
        data: &[f32; 8],
    );
    pub fn d2_gpu_layer_snapshot(ctx: Ptr) -> Ptr;
    /// Drops the clips the current layer took over from its parent: the input of a blur or a
    /// shadow reaches past them, and the parent applies them when the layer is composed.
    pub fn d2_gpu_reset_effect_input_clip(ctx: Ptr);
    pub fn d2_gpu_filter(ctx: Ptr, kind: i32, data: *const f32, length: usize, secondary: Ptr);
    pub fn d2_gpu_defer_layer_filter(ctx: Ptr, kind: i32, data: *const f32, length: usize);
    pub fn d2_gpu_draw_lighting(
        ctx: Ptr,
        width: u32,
        height: u32,
        x: f64,
        y: f64,
        data: *const f32,
        length: usize,
    );
}

extern "C" {
    pub fn d2_gpu_gradient_options(ctx: Ptr, spread: i32, aspect: f64, angle: f64);
}

extern "C" {
    pub fn d2_gpu_draw_tile(
        ctx: Ptr,
        source: Ptr,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        alpha: f64,
        mode: i32,
    );
}

extern "C" {
    pub fn d2_gpu_render_options(ctx: Ptr, blend: i32, aliased: i32, interpolation: i32);
}

extern "C" {
    pub fn d2_gpu_take_layer(ctx: Ptr) -> Ptr;
}

extern "C" {
    pub fn d2_gpu_acrylic(ctx: Ptr, colors: *const f64, noise: Ptr, replace: i32);
}
extern "C" {
    pub fn d2_finish(surface: Ptr);
    pub fn d2_window(hwnd: isize) -> Ptr;
    pub fn d2_window_state(window: Ptr) -> i32;
    pub fn d2_window_dispose(window: Ptr);
    pub fn d2_window_frame_begin(window: Ptr);
    pub fn d2_present(window: Ptr, surface: Ptr) -> i32;
}
extern "C" {
    pub fn d2_native_device() -> Ptr;
    pub fn d2_device_luid(out: *mut u8);
    pub fn d2_texture(texture: Ptr, slice: u32) -> Ptr;
    pub fn d2_open_texture(handle: isize, nt: i32) -> Ptr;
    pub fn d2_texture_valid(texture: Ptr, width: u32, height: u32, rgba: i32) -> i32;
    pub fn d2_convert_frame(
        surface: Ptr,
        texture: Ptr,
        rec709: i32,
        flipped: i32,
        opaque: i32,
        context: Ptr,
        completed: extern "C" fn(Ptr, i32),
    ) -> i32;
}
extern "C" {
    pub fn d2_fences_supported() -> i32;
    pub fn d2_open_fence(handle: isize) -> Ptr;
    pub fn d2_copy_external(
        target: Ptr,
        source: Ptr,
        use_mutex: i32,
        acquire: u64,
        release: u64,
        wait: Ptr,
        wait_value: u64,
        signal: Ptr,
        signal_value: u64,
        flipped: i32,
    ) -> i32;
}
extern "C" {
    pub fn d2_device_generation() -> u64;
    pub fn d2_surface_lost(surface: Ptr) -> i32;
}
