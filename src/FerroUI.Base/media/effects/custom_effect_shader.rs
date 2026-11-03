//! The runtime shaders of custom effects (#455, E4; not from upstream): the
//! description of [`CustomEffect::shader`], loaded once per type, and the
//! uniform convention the backends hand the parameters of a snapshot to the
//! program with (docs/porting/custom-effects.md, section 3.4). The convention
//! is documented on [`ShaderEffectDescription`].

use super::{CustomEffect, EffectParameterValue, ImmutableCustomEffect};
use crate::media::imaging::BitmapBlendingMode;
use std::any::TypeId;
use std::borrow::Cow;
use std::sync::{Arc, RwLock};

/// The name of the uniform of the size of the layer in device pixels, a
/// `float2`, the first uniform of a shader effect.
pub const SHADER_LAYER_SIZE: &str = "layer_size";

/// The name of the uniform of the scale of the layer (device pixels per
/// device independent pixel), a `float`, the second uniform of a shader
/// effect.
pub const SHADER_LAYER_SCALE: &str = "layer_scale";

/// What the program of a shader effect computes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShaderEffectMode {
    /// The program samples the content (the child `content`) and its output
    /// replaces the content over the area of the effect (a pixelation, a
    /// distortion). Skia draws it as the runtime shader image filter of the
    /// layer of the content, so the content stays a texture of the GPU.
    #[default]
    ContentSampling,
    /// The program does not sample the content: its output is drawn over the
    /// content with the [blend mode](ShaderEffectDescription::blend_mode) of
    /// the description (a procedural overlay: scan lines, a spotlight), inside
    /// the layer of the content, so it blends with the content only.
    Overlay,
}

/// How far the output of a shader effect reaches beyond its content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShaderEffectPadding {
    /// The padding of the effect ([`CustomEffect::padding`]) and nothing more:
    /// the area of the effect is its content and that padding.
    #[default]
    Effect,
    /// The padding of the effect and, on every side, the
    /// [sample radius](ShaderEffectDescription::sample_radius): for a program
    /// that moves the content outward (a wave, an aberration of the colours)
    /// by at most that radius.
    SampleRadius,
}

/// The description of a runtime shader effect (#455, E4): what
/// [`CustomEffect::shader`] returns, read once per type.
///
/// It carries two programs with one convention: SkSL for Skia, drawn on
/// every surface of the backend, and WGSL for Vello, drawn by a pass of the
/// device in its hybrid and GPU modes (E5; the module of the pass is
/// [`wgsl_module`](Self::wgsl_module)). Vello's CPU mode draws the CPU
/// reference of the effect ([`CustomEffect::cpu_reference`]) instead.
///
/// # The uniform convention
///
/// The uniform block of a shader effect is, in this order:
///
/// 1. `layer_size` ([`SHADER_LAYER_SIZE`]), a `float2` (`vec2<f32>`): the
///    width and the height of the area of the effect in device pixels (the
///    content and the padding of the effect, rounded out to whole pixels);
/// 2. `layer_scale` ([`SHADER_LAYER_SCALE`]), a `float` (`f32`): the device
///    pixels per device independent pixel of the layer;
/// 3. the parameters of the effect, in the order of
///    [`CustomEffect::for_each_parameter`], each named as the parameter with
///    every `-` written `_` (`cell-size` is `cell_size`): a `Float` is a
///    `float`, a `Point`, `Vector` or `Size` a `float2`, a `Color` a
///    premultiplied `float4` (red, green and blue times the alpha, 0 to 1), a
///    `Rect` a `float4` (x, y, width, height), a `Bool` (0 or 1) or an `Int` an
///    `int`. A `Text` parameter has no uniform.
///
/// The values of the parameters are passed as they are: a length in device
/// independent pixels is turned into device pixels by the program, with
/// `layer_scale`. The program is given the coordinates of the pixel it
/// colours in device pixels of the layer: `(0, 0)` is the top left corner of
/// the area of the effect, the centre of the first pixel `(0.5, 0.5)`.
///
/// # The programs
///
/// The SkSL program of Skia declares the uniforms above, in that order and
/// with those types, and, when it samples the content
/// ([`ShaderEffectMode::ContentSampling`]), the child `uniform shader content;`
/// ([`CUSTOM_EFFECT_CONTENT`](super::CUSTOM_EFFECT_CONTENT)), which it
/// samples with `content.eval(coord)` (premultiplied, transparent beyond the
/// area); its entry point is `half4 main(float2 coord)`, which returns a
/// premultiplied colour.
///
/// The WGSL program of Vello declares the uniforms as
/// the members of `struct Uniforms`, in the same order, and the function
/// `fn effect(coord: vec2<f32>) -> vec4<f32>`. The pass of the backend adds
/// the bindings: `uniforms`, the block, and `fn sample_content(coord:
/// vec2<f32>) -> vec4<f32>`, the content at `coord` (content-sampling
/// programs only), and calls `effect` at the centre of each pixel.
///
/// Each backend checks its program against the parameters when it compiles
/// it (decision 1 of the plan): a program whose uniforms are not those of the
/// convention is reported once and the effect is drawn as an effect without
/// a factory (its content unchanged), as is a program that does not compile.
///
/// ```ignore
/// fn shader() -> Option<ShaderEffectDescription> {
///     Some(ShaderEffectDescription::content_sampling(PIXELATE_SKSL, PIXELATE_WGSL).with_sample_radius(64.0))
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShaderEffectDescription {
    /// The SkSL program of Skia: `half4 main(float2 coord)`.
    pub sksl: Cow<'static, str>,
    /// The WGSL program of Vello: `fn effect(coord: vec2<f32>) -> vec4<f32>`
    /// and `struct Uniforms`, wrapped by [`wgsl_module`](Self::wgsl_module).
    pub wgsl: Cow<'static, str>,
    /// Whether the program samples the content or is an overlay.
    pub mode: ShaderEffectMode,
    /// How an overlay is drawn over the content (source over by default);
    /// not used by a content-sampling program, whose output replaces the
    /// content.
    pub blend_mode: BitmapBlendingMode,
    /// How far from the pixel it colours the program samples the content at
    /// most, in device independent pixels (0 by default): the backend
    /// provides the content that far beyond the part of the area it draws
    /// (a dirty rectangle), so that the result does not depend on what is
    /// redrawn.
    pub sample_radius: f64,
    /// How far the output reaches beyond the content.
    pub padding: ShaderEffectPadding,
    /// The names of further children the program samples besides the
    /// content: reserved for the secondary images of step E6 and **not yet
    /// supported**; a description with children is reported once and drawn
    /// as an effect without a factory.
    pub children: Vec<Cow<'static, str>>,
}

impl ShaderEffectDescription {
    /// A program that samples the content and replaces it
    /// ([`ShaderEffectMode::ContentSampling`]).
    pub fn content_sampling(sksl: impl Into<Cow<'static, str>>, wgsl: impl Into<Cow<'static, str>>) -> Self {
        Self {
            sksl: sksl.into(),
            wgsl: wgsl.into(),
            mode: ShaderEffectMode::ContentSampling,
            blend_mode: BitmapBlendingMode::SourceOver,
            sample_radius: 0.0,
            padding: ShaderEffectPadding::Effect,
            children: Vec::new(),
        }
    }

    /// A program drawn over the content with `blend_mode`
    /// ([`ShaderEffectMode::Overlay`]).
    pub fn overlay(
        sksl: impl Into<Cow<'static, str>>,
        wgsl: impl Into<Cow<'static, str>>,
        blend_mode: BitmapBlendingMode,
    ) -> Self {
        Self { mode: ShaderEffectMode::Overlay, blend_mode, ..Self::content_sampling(sksl, wgsl) }
    }

    /// The description with the [sample radius](Self::sample_radius)
    /// `radius` (device independent pixels; a negative or not finite value is
    /// 0).
    pub fn with_sample_radius(mut self, radius: f64) -> Self {
        self.sample_radius = if radius.is_finite() { radius.max(0.0) } else { 0.0 };
        self
    }

    /// The description with the [padding](Self::padding) `padding`.
    pub fn with_padding(mut self, padding: ShaderEffectPadding) -> Self {
        self.padding = padding;
        self
    }

    /// The description with the children `children` besides the content
    /// (step E6, not yet supported).
    pub fn with_children<S: Into<Cow<'static, str>>>(mut self, children: impl IntoIterator<Item = S>) -> Self {
        self.children = children.into_iter().map(Into::into).collect();
        self
    }

    /// Whether the program samples the content.
    pub fn samples_content(&self) -> bool {
        self.mode == ShaderEffectMode::ContentSampling
    }

    /// The WGSL module of the pass that draws this description on the GPU
    /// (Vello's hybrid and GPU modes, E5; the source of the GLSL ES of
    /// WebGL2 that `ferroui-build` translates at build time): the program
    /// ([`wgsl`](Self::wgsl)) followed by what the pass adds to it.
    ///
    /// - `@group(0) @binding(0) var<uniform> uniforms: Uniforms;`
    ///   ([`SHADER_WGSL_UNIFORMS_BINDING`]): the block of the convention,
    ///   whose type is the `struct Uniforms` of the program.
    /// - `@group(0) @binding(1) var<uniform> ferroui_pass: FerroUiShaderPass;`
    ///   ([`SHADER_WGSL_PASS_BINDING`]): one `vec4<f32>` `area`, the device
    ///   pixel of the area of the effect at the top left texel of the
    ///   textures (`x`, `y`, whole pixels) and the width and the height of
    ///   the part of the area they hold (`z`, `w`).
    /// - `@group(0) @binding(2) var ferroui_content: texture_2d<f32>;`
    ///   ([`SHADER_WGSL_CONTENT_BINDING`]): the content, premultiplied, one
    ///   texel per device pixel, read with `textureLoad` (no sampler).
    /// - For a content-sampling program, `fn sample_content(coord:
    ///   vec2<f32>) -> vec4<f32>`: the texel under `coord` (device pixels of
    ///   the area), as Skia samples the child `content` of a runtime shader
    ///   image filter at the centre of a texel; transparent beyond the area.
    ///   An overlay has no `sample_content`: a call of it does not compile.
    /// - The entry points [`SHADER_WGSL_VERTEX_ENTRY`] (a triangle over the
    ///   target) and [`SHADER_WGSL_FRAGMENT_ENTRY`], which calls `effect` at
    ///   the centre of each pixel of the area: a content-sampling program's
    ///   output is the result; an overlay's output, clamped to 0..1, is
    ///   composed over the content with the [blend mode](Self::blend_mode)
    ///   (every mode of `BitmapBlendingMode`, premultiplied, the formulas of
    ///   Skia's blend modes; `Unspecified` is source over). Pixels beyond the
    ///   part of the area the textures hold are transparent.
    ///
    /// The names the pass adds start with `ferroui_` (and `FerroUi`), but
    /// `uniforms`, `sample_content`, `vs_main` and `fs_main`: a program that
    /// declares one of them does not compile.
    pub fn wgsl_module(&self) -> String {
        shader_wgsl_module(&self.wgsl, self.mode, self.blend_mode)
    }
}

/// The binding (group 0) of the uniform block of the effect, `uniforms`, in
/// the WGSL module of a shader effect
/// ([`ShaderEffectDescription::wgsl_module`]).
pub const SHADER_WGSL_UNIFORMS_BINDING: u32 = 0;

/// The binding (group 0) of the uniforms of the pass, `ferroui_pass`, in the
/// WGSL module of a shader effect.
pub const SHADER_WGSL_PASS_BINDING: u32 = 1;

/// The binding (group 0) of the content, `ferroui_content`, a
/// `texture_2d<f32>`, in the WGSL module of a shader effect.
pub const SHADER_WGSL_CONTENT_BINDING: u32 = 2;

/// The vertex entry point of the WGSL module of a shader effect.
pub const SHADER_WGSL_VERTEX_ENTRY: &str = "vs_main";

/// The fragment entry point of the WGSL module of a shader effect.
pub const SHADER_WGSL_FRAGMENT_ENTRY: &str = "fs_main";

/// The bindings, the content and the vertex stage of the pass of a shader
/// effect (`ShaderEffectDescription::wgsl_module`).
const SHADER_PASS_WGSL: &str = r#"

// --- The pass of the backend (docs/porting/custom-effects.md, section 3.4) ---

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// What the pass gives the module besides the uniforms of the effect.
struct FerroUiShaderPass {
    // x, y: the device pixel of the area of the effect at the top left texel
    // of the textures; z, w: the width and the height of the part of the
    // area the textures hold.
    area: vec4<f32>,
}

@group(0) @binding(1) var<uniform> ferroui_pass: FerroUiShaderPass;

// The content of the effect: premultiplied, one texel per device pixel.
@group(0) @binding(2) var ferroui_content: texture_2d<f32>;

// The texel of the content at `pixel` of the textures; transparent beyond
// the part of the area they hold.
fn ferroui_content_texel(pixel: vec2<i32>) -> vec4<f32> {
    let extent = vec2<i32>(ferroui_pass.area.zw);
    if (any(pixel < vec2<i32>(0)) || any(pixel >= extent)) {
        return vec4<f32>(0.0);
    }
    return textureLoad(ferroui_content, pixel, 0);
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}
"#;

/// `sample_content` and the fragment stage of a content-sampling program.
const CONTENT_SAMPLING_WGSL: &str = r#"
// The content at `coord` (device pixels of the area): the texel under it, as
// Skia samples the child `content` at the centre of a texel; transparent
// beyond the area.
fn sample_content(coord: vec2<f32>) -> vec4<f32> {
    return ferroui_content_texel(vec2<i32>(floor(coord - ferroui_pass.area.xy)));
}

@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    if (any(fragment.xy >= ferroui_pass.area.zw)) {
        return vec4<f32>(0.0);
    }
    // The centre of the pixel in the coordinates of the convention.
    return effect(fragment.xy + ferroui_pass.area.xy);
}
"#;

/// The fragment stage of an overlay: its output composed over the content
/// by `ferroui_blend`.
const OVERLAY_WGSL: &str = r#"
@fragment
fn fs_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    if (any(fragment.xy >= ferroui_pass.area.zw)) {
        return vec4<f32>(0.0);
    }
    let source = clamp(effect(fragment.xy + ferroui_pass.area.xy), vec4<f32>(0.0), vec4<f32>(1.0));
    let destination = ferroui_content_texel(vec2<i32>(floor(fragment.xy)));
    return ferroui_blend(source, destination);
}
"#;

/// The blend modes of the Compositing and Blending specification that are
/// not Porter-Duff operators, on colours that are not premultiplied: the
/// mixed colour composed source over (`ferroui_compose`).
const BLEND_COMPOSE_WGSL: &str = r#"
// A premultiplied colour without its alpha.
fn ferroui_straight(c: vec4<f32>) -> vec3<f32> {
    if (c.a > 0.0) {
        return min(c.rgb / c.a, vec3<f32>(1.0));
    }
    return vec3<f32>(0.0);
}

// The colour `mixed` of a blend mode where both colours are, the source
// and the destination where only one is.
fn ferroui_compose(s: vec4<f32>, d: vec4<f32>, mixed: vec3<f32>) -> vec4<f32> {
    let rgb = s.rgb * (1.0 - d.a) + d.rgb * (1.0 - s.a) + s.a * d.a * mixed;
    return vec4<f32>(rgb, s.a + d.a - s.a * d.a);
}

fn ferroui_blend(s: vec4<f32>, d: vec4<f32>) -> vec4<f32> {
    return ferroui_compose(s, d, ferroui_mix(ferroui_straight(d), ferroui_straight(s)));
}
"#;

/// A separable blend mode: `ferroui_channel(b, s)` for each channel of the
/// colours of the destination (`b`) and the source (`s`).
const SEPARABLE_MIX_WGSL: &str = r#"
fn ferroui_mix(b: vec3<f32>, s: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(ferroui_channel(b.r, s.r), ferroui_channel(b.g, s.g), ferroui_channel(b.b, s.b));
}
"#;

/// Hard light of one channel (the overlay is hard light with the colours
/// exchanged).
const HARD_LIGHT_WGSL: &str = r#"
fn ferroui_hard_light(b: f32, s: f32) -> f32 {
    if (s <= 0.5) {
        return b * 2.0 * s;
    }
    let t = 2.0 * s - 1.0;
    return b + t - b * t;
}
"#;

/// The helpers of the blend modes that are not separable.
const NON_SEPARABLE_WGSL: &str = r#"
fn ferroui_lum(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.3, 0.59, 0.11));
}

fn ferroui_clip_color(c: vec3<f32>) -> vec3<f32> {
    let l = ferroui_lum(c);
    let n = min(min(c.r, c.g), c.b);
    let x = max(max(c.r, c.g), c.b);
    var result = c;
    if (n < 0.0 && l - n > 0.0) {
        result = vec3<f32>(l) + (result - vec3<f32>(l)) * (l / (l - n));
    }
    if (x > 1.0 && x - l > 0.0) {
        result = vec3<f32>(l) + (result - vec3<f32>(l)) * ((1.0 - l) / (x - l));
    }
    return result;
}

fn ferroui_set_lum(c: vec3<f32>, l: f32) -> vec3<f32> {
    return ferroui_clip_color(c + vec3<f32>(l - ferroui_lum(c)));
}

fn ferroui_sat(c: vec3<f32>) -> f32 {
    return max(max(c.r, c.g), c.b) - min(min(c.r, c.g), c.b);
}

fn ferroui_set_sat(c: vec3<f32>, s: f32) -> vec3<f32> {
    let n = min(min(c.r, c.g), c.b);
    let x = max(max(c.r, c.g), c.b);
    if (x <= n) {
        return vec3<f32>(0.0);
    }
    return (c - vec3<f32>(n)) * (s / (x - n));
}
"#;

/// The WGSL of the composition of an overlay over the content with
/// `mode`: `fn ferroui_blend(s: vec4<f32>, d: vec4<f32>) -> vec4<f32>` of
/// two premultiplied colours, with what it calls. The Porter-Duff operators
/// and the modes Skia writes in premultiplied form are written so; the
/// others mix the colours that are not premultiplied (`ferroui_compose`).
fn blend_wgsl(mode: BitmapBlendingMode) -> String {
    let premultiplied = |body: &str| format!("\nfn ferroui_blend(s: vec4<f32>, d: vec4<f32>) -> vec4<f32> {{\n    {body}\n}}\n");
    let separable = |channel: &str| {
        format!("{BLEND_COMPOSE_WGSL}{SEPARABLE_MIX_WGSL}\nfn ferroui_channel(b: f32, s: f32) -> f32 {{\n{channel}\n}}\n")
    };
    let non_separable = |mix: &str| {
        format!(
            "{BLEND_COMPOSE_WGSL}{NON_SEPARABLE_WGSL}\nfn ferroui_mix(b: vec3<f32>, s: vec3<f32>) -> vec3<f32> {{\n    return {mix};\n}}\n"
        )
    };
    match mode {
        BitmapBlendingMode::Unspecified | BitmapBlendingMode::SourceOver => premultiplied("return s + d * (1.0 - s.a);"),
        BitmapBlendingMode::Source => premultiplied("return s;"),
        BitmapBlendingMode::Destination => premultiplied("return d;"),
        BitmapBlendingMode::DestinationOver => premultiplied("return d + s * (1.0 - d.a);"),
        BitmapBlendingMode::SourceIn => premultiplied("return s * d.a;"),
        BitmapBlendingMode::DestinationIn => premultiplied("return d * s.a;"),
        BitmapBlendingMode::SourceOut => premultiplied("return s * (1.0 - d.a);"),
        BitmapBlendingMode::DestinationOut => premultiplied("return d * (1.0 - s.a);"),
        BitmapBlendingMode::SourceAtop => premultiplied("return s * d.a + d * (1.0 - s.a);"),
        BitmapBlendingMode::DestinationAtop => premultiplied("return d * s.a + s * (1.0 - d.a);"),
        BitmapBlendingMode::Xor => premultiplied("return s * (1.0 - d.a) + d * (1.0 - s.a);"),
        BitmapBlendingMode::Plus => premultiplied("return min(s + d, vec4<f32>(1.0));"),
        BitmapBlendingMode::Screen => premultiplied("return s + d - s * d;"),
        BitmapBlendingMode::Multiply => premultiplied("return s * (1.0 - d.a) + d * (1.0 - s.a) + s * d;"),
        BitmapBlendingMode::Darken => premultiplied(
            "return vec4<f32>(s.rgb + d.rgb - max(s.rgb * d.a, d.rgb * s.a), s.a + d.a - s.a * d.a);",
        ),
        BitmapBlendingMode::Lighten => premultiplied(
            "return vec4<f32>(s.rgb + d.rgb - min(s.rgb * d.a, d.rgb * s.a), s.a + d.a - s.a * d.a);",
        ),
        BitmapBlendingMode::Difference => premultiplied(
            "return vec4<f32>(s.rgb + d.rgb - 2.0 * min(s.rgb * d.a, d.rgb * s.a), s.a + d.a - s.a * d.a);",
        ),
        BitmapBlendingMode::Exclusion => {
            premultiplied("return vec4<f32>(s.rgb + d.rgb - 2.0 * s.rgb * d.rgb, s.a + d.a - s.a * d.a);")
        }
        BitmapBlendingMode::Overlay => {
            format!("{HARD_LIGHT_WGSL}{}", separable("    return ferroui_hard_light(s, b);"))
        }
        BitmapBlendingMode::HardLight => {
            format!("{HARD_LIGHT_WGSL}{}", separable("    return ferroui_hard_light(b, s);"))
        }
        BitmapBlendingMode::ColorDodge => separable(
            "    if (b <= 0.0) {\n        return 0.0;\n    }\n    if (s >= 1.0) {\n        return 1.0;\n    }\n    return min(1.0, b / (1.0 - s));",
        ),
        BitmapBlendingMode::ColorBurn => separable(
            "    if (b >= 1.0) {\n        return 1.0;\n    }\n    if (s <= 0.0) {\n        return 0.0;\n    }\n    return 1.0 - min(1.0, (1.0 - b) / s);",
        ),
        BitmapBlendingMode::SoftLight => separable(
            "    if (s <= 0.5) {\n        return b - (1.0 - 2.0 * s) * b * (1.0 - b);\n    }\n    var e = sqrt(b);\n    if (b <= 0.25) {\n        e = ((16.0 * b - 12.0) * b + 4.0) * b;\n    }\n    return b + (2.0 * s - 1.0) * (e - b);",
        ),
        BitmapBlendingMode::Hue => non_separable("ferroui_set_lum(ferroui_set_sat(s, ferroui_sat(b)), ferroui_lum(b))"),
        BitmapBlendingMode::Saturation => {
            non_separable("ferroui_set_lum(ferroui_set_sat(b, ferroui_sat(s)), ferroui_lum(b))")
        }
        BitmapBlendingMode::Color => non_separable("ferroui_set_lum(s, ferroui_lum(b))"),
        BitmapBlendingMode::Luminosity => non_separable("ferroui_set_lum(b, ferroui_lum(s))"),
    }
}

/// The WGSL module of a shader effect: the program `wgsl`, the bindings and
/// the entry points of the pass (see
/// [`ShaderEffectDescription::wgsl_module`]).
fn shader_wgsl_module(wgsl: &str, mode: ShaderEffectMode, blend_mode: BitmapBlendingMode) -> String {
    let mut module = String::with_capacity(wgsl.len() + 4096);
    module.push_str(wgsl);
    module.push_str(SHADER_PASS_WGSL);
    match mode {
        ShaderEffectMode::ContentSampling => module.push_str(CONTENT_SAMPLING_WGSL),
        ShaderEffectMode::Overlay => {
            module.push_str(&blend_wgsl(blend_mode));
            module.push_str(OVERLAY_WGSL);
        }
    }
    module
}

/// The type of a uniform of the convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderUniformType {
    /// `float` (WGSL `f32`).
    Float,
    /// `float2` (WGSL `vec2<f32>`).
    Float2,
    /// `float4` (WGSL `vec4<f32>`).
    Float4,
    /// `int` (WGSL `i32`).
    Int,
}

impl ShaderUniformType {
    /// The name of the type in SkSL.
    pub fn sksl(self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Float2 => "float2",
            Self::Float4 => "float4",
            Self::Int => "int",
        }
    }

    /// The name of the type in WGSL.
    pub fn wgsl(self) -> &'static str {
        match self {
            Self::Float => "f32",
            Self::Float2 => "vec2<f32>",
            Self::Float4 => "vec4<f32>",
            Self::Int => "i32",
        }
    }

    /// The number of scalars of the type.
    pub fn components(self) -> usize {
        match self {
            Self::Float | Self::Int => 1,
            Self::Float2 => 2,
            Self::Float4 => 4,
        }
    }
}

/// The value of a uniform of the convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaderUniformValue {
    /// A `float`.
    Float(f32),
    /// A `float2`.
    Float2([f32; 2]),
    /// A `float4`.
    Float4([f32; 4]),
    /// An `int`.
    Int(i32),
}

impl ShaderUniformValue {
    /// The type of the value.
    pub fn ty(&self) -> ShaderUniformType {
        match self {
            Self::Float(_) => ShaderUniformType::Float,
            Self::Float2(_) => ShaderUniformType::Float2,
            Self::Float4(_) => ShaderUniformType::Float4,
            Self::Int(_) => ShaderUniformType::Int,
        }
    }

    /// The uniform of a parameter value by the convention; `None` for text,
    /// which has none.
    pub fn from_parameter(value: &EffectParameterValue) -> Option<Self> {
        Some(match value {
            EffectParameterValue::Bool(value) => Self::Int(i32::from(*value)),
            EffectParameterValue::Int(value) => Self::Int(*value),
            EffectParameterValue::Float(value) => Self::Float(*value as f32),
            EffectParameterValue::Color(color) => {
                let alpha = f32::from(color.a) / 255.0;
                let channel = |value: u8| f32::from(value) / 255.0 * alpha;
                Self::Float4([channel(color.r), channel(color.g), channel(color.b), alpha])
            }
            EffectParameterValue::Point(point) => Self::Float2([point.x as f32, point.y as f32]),
            EffectParameterValue::Vector(vector) => Self::Float2([vector.x as f32, vector.y as f32]),
            EffectParameterValue::Size(size) => Self::Float2([size.width as f32, size.height as f32]),
            EffectParameterValue::Rect(rect) => {
                Self::Float4([rect.x as f32, rect.y as f32, rect.width as f32, rect.height as f32])
            }
            EffectParameterValue::Text(_) => return None,
        })
    }

    /// The scalars of a float value, in order; empty for an `int`.
    pub fn floats(&self) -> &[f32] {
        match self {
            Self::Float(value) => std::slice::from_ref(value),
            Self::Float2(values) => values,
            Self::Float4(values) => values,
            Self::Int(_) => &[],
        }
    }
}

/// The name of the uniform of the parameter `parameter`: the name with every
/// `-` written `_`.
pub fn shader_uniform_name(parameter: &str) -> String {
    parameter.replace('-', "_")
}

/// Whether `uniform` is the name of the uniform of the parameter `parameter`
/// ([`shader_uniform_name`]), without allocating.
pub fn shader_uniform_name_matches(uniform: &str, parameter: &str) -> bool {
    uniform.len() == parameter.len()
        && uniform.bytes().zip(parameter.bytes()).all(|(u, p)| u == p || (u == b'_' && p == b'-'))
}

/// Calls `f` with the name and the value of each uniform of the block of
/// `effect` by the convention: the size and the scale of the layer, then the
/// parameters (text skipped), named as [`CustomEffect::for_each_parameter`]
/// names them.
pub(crate) fn visit_shader_uniforms(
    effect: &ImmutableCustomEffect,
    layer_size: [f32; 2],
    layer_scale: f32,
    f: &mut dyn FnMut(&'static str, ShaderUniformValue),
) {
    f(SHADER_LAYER_SIZE, ShaderUniformValue::Float2(layer_size));
    f(SHADER_LAYER_SCALE, ShaderUniformValue::Float(layer_scale));
    effect.for_each_parameter(&mut |name, value| {
        if let Some(uniform) = ShaderUniformValue::from_parameter(&value) {
            f(name, uniform);
        }
    });
}

/// What loading the shader of a type gave: `None` for a type without one
/// (or with a graph, which takes precedence).
type Loaded = Option<Arc<ShaderEffectDescription>>;

/// The shaders loaded, by type.
static SHADERS: RwLock<Vec<(TypeId, Loaded)>> = RwLock::new(Vec::new());

/// The shader of `T`, read on the first call for the type and registered for
/// the prewarm of the GPU programs; `None` when the type has no shader, or
/// has a graph ([`CustomEffect::graph`] takes precedence).
pub(crate) fn load_custom_effect_shader<T: CustomEffect>() -> Loaded {
    let type_id = TypeId::of::<T>();
    {
        let shaders = SHADERS.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((_, loaded)) = shaders.iter().find(|(id, _)| *id == type_id) {
            return loaded.clone();
        }
    }
    // Read outside the lock: `graph` and `shader` are code of the effect.
    let loaded = if T::graph().is_some() { None } else { T::shader().map(Arc::new) };
    {
        let mut shaders = SHADERS.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((_, existing)) = shaders.iter().find(|(id, _)| *id == type_id) {
            // Another thread loaded it meanwhile.
            return existing.clone();
        }
        shaders.push((type_id, loaded.clone()));
    }
    if loaded.is_some() {
        crate::rendering::composition::effects::register_custom_effect_prewarm(
            &format!("custom-effect:{}", T::NAME),
            ImmutableCustomEffect::new(T::default()),
        );
    }
    loaded
}
