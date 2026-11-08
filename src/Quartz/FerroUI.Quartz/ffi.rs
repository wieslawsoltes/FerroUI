//! Narrow C ABI to Apple's public framework APIs. No bundled drawing or shaping engine.
use std::ffi::c_void;
pub type Ptr = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Command {
    pub kind: i32,
    pub p: [f64; 6],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Glyph {
    pub glyph: u16,
    pub cluster: i32,
    pub advance: f64,
    pub x: f64,
    pub y: f64,
}
extern "C" {
    pub fn q_supported() -> i32;
    pub fn q_release(p: Ptr);
    pub fn q_font(name: *const i8, weight: i32, italic: i32, stretch: i32) -> Ptr;
    pub fn q_font_variations(font: Ptr, tags: *const u32, values: *const f64, count: usize) -> Ptr;
    pub fn q_font_family_faces(family: *const i8) -> Ptr;
    pub fn q_path_intercepts(path: Ptr, lower: f64, upper: f64) -> Ptr;
    pub fn q_font_data(data: *const u8, n: usize) -> Ptr;
    pub fn q_font_name(font: Ptr, out: *mut u8, n: usize);
    pub fn q_font_table(font: Ptr, tag: u32) -> Ptr;
    pub fn q_font_names() -> Ptr;
    pub fn q_font_traits(font: Ptr) -> u32;
    pub fn q_font_info(font: Ptr, info: *mut i32);
    pub fn q_font_match(font: Ptr, text: *const u16, n: usize, language: *const i8) -> Ptr;
    pub fn q_data_len(data: Ptr) -> usize;
    pub fn q_data_bytes(data: Ptr) -> *const u8;
    pub fn q_shape(
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
        starts: *const u32,
        ends: *const u32,
        features: usize,
        info: Ptr,
        emit: extern "C" fn(Ptr, Glyph),
    );
    pub fn q_path(cmd: *const Command, n: usize) -> Ptr;
    pub fn q_path_elements(path: Ptr, info: Ptr, emit: extern "C" fn(Ptr, Command));
    pub fn q_glyph_bounds(font: Ptr, glyph: u16, size: f64, out: *mut f64);
    pub fn q_glyph_path(font: Ptr, glyph: u16, size: f64) -> Ptr;
    pub fn q_combine(a: Ptr, b: Ptr, op: i32, ea: i32, eb: i32) -> Ptr;
    pub fn q_stroke(
        path: Ptr,
        width: f64,
        cap: i32,
        join: i32,
        miter: f64,
        dash: *const f64,
        n: usize,
        phase: f64,
    ) -> Ptr;
    pub fn q_surface(w: usize, h: usize) -> Ptr;
    pub fn q_read_pixels(ctx: Ptr, out: *mut u8, len: usize);
    pub fn q_write_pixels(ctx: Ptr, data: *const u8, len: usize);
    #[cfg(feature = "metal")]
    pub fn q_retain(p: Ptr) -> Ptr;
    pub fn q_snapshot(ctx: Ptr) -> Ptr;
    pub fn q_save(ctx: Ptr);
    pub fn q_restore(ctx: Ptr);
    pub fn q_clear(ctx: Ptr, w: f64, h: f64, r: f64, g: f64, b: f64, a: f64);
    pub fn q_clip(ctx: Ptr, path: Ptr, evenodd: i32);
    pub fn q_fill(ctx: Ptr, path: Ptr, evenodd: i32, r: f64, g: f64, b: f64, a: f64);
    pub fn q_gradient(
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
    pub fn q_begin_layer(ctx: Ptr, opacity: f64, blend: i32);
    pub fn q_end_layer(ctx: Ptr);
    pub fn q_draw_image(
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
    pub fn q_glyphs(
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
    pub fn q_decode(bytes: *const u8, n: usize) -> Ptr;
    pub fn q_image_width(image: Ptr) -> usize;
    pub fn q_image_height(image: Ptr) -> usize;
    pub fn q_encode(image: Ptr, jpeg: i32, quality: f64) -> Ptr;
}
/// An owned +1 Core Foundation reference, or NULL after a native allocation failure.
/// Fonts/paths/images are immutable; bitmap contexts are only used under Surface's mutex.
#[derive(Default)]
pub struct Owned(pub Ptr);
impl Owned {
    pub unsafe fn new(p: Ptr) -> Self {
        Self(p)
    }
    pub fn bytes(&self) -> Vec<u8> {
        unsafe {
            if self.0.is_null() {
                return Vec::new();
            }
            let length = q_data_len(self.0);
            if length == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(q_data_bytes(self.0), length).to_vec()
            }
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { q_release(self.0) }
    }
}
// SAFETY: ownership can move between threads; mutable CGContext access is mutex protected.
unsafe impl Send for Owned {}
extern "C" {
    pub fn q_set_transform(
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
    #[cfg(any(not(feature = "metal"), test))]
    pub fn q_shadow(ctx: Ptr, x: f64, y: f64, blur: f64, r: f64, g: f64, b: f64, a: f64);
}

#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_text_options(surface: Ptr, hinting: i32);
    pub fn qm_configure(device: Ptr) -> u64;
    pub fn qm_validate_texture(texture: Ptr, width: i32, height: i32, bgra: i32) -> i32;
    pub fn qm_reset_effect_input_clip(surface: Ptr);
    pub fn qm_finish_context(generation: u64);
    pub fn qm_generation() -> u64;
    pub fn qm_is_lost(generation: u64) -> i32;
    #[cfg(test)]
    pub fn qm_record_fault(generation: u64);
    pub fn qm_draw_surface(
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
    pub fn qm_copy_surface(ctx: Ptr, source: Ptr);
    pub fn qm_present(ctx: Ptr, texture: Ptr, platform_queue: Ptr, flipped: i32) -> i32;
    pub fn qm_snapshot_external(
        surface: Ptr,
        source: Ptr,
        flipped: i32,
        wait: Ptr,
        wait_value: u64,
        signal: Ptr,
        signal_value: u64,
    ) -> i32;
    #[cfg(feature = "runtime-shaders")]
    pub fn qm_shader_compile(
        source: *const i8,
        vertex: *const i8,
        fragment: *const i8,
        error: *mut u8,
        length: usize,
    ) -> Ptr;
    #[cfg(feature = "runtime-shaders")]
    pub fn qm_shader_apply(
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
    pub fn qm_statistics(out: *mut u64, count: usize);
}
#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_convert_frame(
        surface: Ptr,
        texture: Ptr,
        rec709: i32,
        flipped: i32,
        opaque: i32,
        context: Ptr,
        completed: extern "C" fn(Ptr, i32),
    ) -> i32;
}

#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_blur(ctx: Ptr, sigma: f64, ax: f64, ay: f64, bx: f64, by: f64);
    pub fn qm_layer_snapshot(ctx: Ptr) -> Ptr;
    pub fn qm_filter(ctx: Ptr, kind: i32, data: *const f32, length: usize, secondary: Ptr);
}

#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_gradient_options(ctx: Ptr, spread: i32, aspect: f64, angle: f64);
}

#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_draw_tile(
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

#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_render_options(ctx: Ptr, blend: i32, aliased: i32, interpolation: i32);
}

#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_take_layer(ctx: Ptr) -> Ptr;
}

#[cfg(feature = "metal")]
extern "C" {
    pub fn qm_acrylic(ctx: Ptr, colors: *const f64, noise: Ptr, replace: i32);
}
