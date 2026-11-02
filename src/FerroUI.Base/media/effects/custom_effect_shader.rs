//! The runtime shaders of custom effects (#455, E4; not from upstream): the
//! description of [`CustomEffect::shader`], loaded once per type, and the
//! uniform convention the backends hand the parameters of a snapshot to the
//! program with (docs/porting/custom-effects.md, section 3.4).
//!
//! # The uniform convention
//!
//! The uniform block of a shader effect is, in this order:
//!
//! 1. `layer_size` ([`SHADER_LAYER_SIZE`]), a `float2` (`vec2<f32>`): the
//!    width and the height of the area of the effect in device pixels (the
//!    content and the padding of the effect, rounded out to whole pixels);
//! 2. `layer_scale` ([`SHADER_LAYER_SCALE`]), a `float` (`f32`): the device
//!    pixels per device independent pixel of the layer;
//! 3. the parameters of the effect, in the order of
//!    [`CustomEffect::for_each_parameter`], each named as the parameter with
//!    every `-` written `_` (`cell-size` is `cell_size`): a `Float` is a
//!    `float`, a `Point`, `Vector` or `Size` a `float2`, a `Color` a
//!    premultiplied `float4` (red, green and blue times the alpha, 0 to 1), a
//!    `Rect` a `float4` (x, y, width, height), a `Bool` (0 or 1) or an `Int` an
//!    `int`. A `Text` parameter has no uniform.
//!
//! The values of the parameters are passed as they are: a length in device
//! independent pixels is turned into device pixels by the program, with
//! `layer_scale`. The program is given the coordinates of the pixel it
//! colours in device pixels of the layer: `(0, 0)` is the top left corner of
//! the area of the effect, the centre of the first pixel `(0.5, 0.5)`.
//!
//! # The programs
//!
//! The SkSL program of Skia declares the uniforms above, in that order and
//! with those types, and, when it samples the content
//! ([`ShaderEffectMode::ContentSampling`]), the child `uniform shader content;`
//! ([`CUSTOM_EFFECT_CONTENT`](super::CUSTOM_EFFECT_CONTENT)), which it samples with `content.eval(coord)`
//! (premultiplied, transparent beyond the area); its entry point is
//! `half4 main(float2 coord)`, which returns a premultiplied colour.
//!
//! The WGSL program of Vello (drawn from step E5 on) declares the uniforms as
//! the members of `struct Uniforms`, in the same order, and the function
//! `fn effect(coord: vec2<f32>) -> vec4<f32>`. The pass of the backend adds
//! the bindings: `uniforms`, the block, and `fn sample_content(coord:
//! vec2<f32>) -> vec4<f32>`, the content at `coord` (content-sampling
//! programs only), and calls `effect` at the centre of each pixel.
//!
//! Each backend checks its program against the parameters when it compiles
//! it (decision 1 of the plan): a program whose uniforms are not those of the
//! convention is reported once and the effect is drawn as an effect without
//! a factory (its content unchanged), as is a program that does not compile.

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
/// It carries two programs with one convention (the
/// [module documentation](self)): SkSL for Skia, drawn on every surface of
/// the backend, and WGSL for Vello, drawn from step E5 on (until then Vello
/// draws a shader effect as an effect without a factory: its content
/// unchanged, reported once).
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
    /// and `struct Uniforms`. Not drawn before step E5.
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
