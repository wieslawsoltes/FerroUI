//! The page transitions of the library (#455, E6; not from upstream):
//! runtime shader effects that sample images ([`EffectImage`], the children
//! of the programs, docs/porting/custom-effects.md, section 3.6) and draw
//! them over the content, each with its SkSL for Skia, its WGSL for Vello and
//! its CPU reference.
//!
//! - [`DissolveTransitionEffect`] is Effector's dissolve of the compiz-style
//!   transitions (`samples/Effector.Compiz.Sample.Effects`,
//!   `CompizTransitionEffect.cs`, `DissolveShaderSource`): from one page to
//!   the other along a frontier that spreads from a point, through a pattern,
//!   with a glow at the frontier. Effector computes the pattern from a hash
//!   of the position in the program; here it is the luminance of an image,
//!   so that the program draws the same on every backend and processor (a
//!   hash of large numbers through `sin` does not).
//! - [`MaskTransitionEffect`] reveals a page over the content where the
//!   luminance of a mask is below the progress (the reveal of the dissolve,
//!   with the content as the page that leaves): a wipe with a gradient mask,
//!   a dissolve with a noise, a fade without one.
//!
//! Both are overlays drawn source over the content (an opaque page replaces
//! it), as Effector's transitions are drawn over the content of their
//! presenter; at a progress of 0 and without images (the neutral values)
//! they draw nothing.

use super::super::{sample_image, CustomEffect, EffectImage, EffectParameterValue, ShaderEffectDescription};
use super::{area, unit};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::Color;

/// The weights of the luminance of the patterns and masks (Rec. 709), applied
/// to the premultiplied colour.
const LUMINANCE: [f32; 3] = [0.2126, 0.7152, 0.0722];

/// The luminance of a premultiplied colour of the patterns and masks.
fn luminance(texel: [u8; 4]) -> f32 {
    let channel = |index: usize| f32::from(texel[index]) / 255.0;
    channel(0) * LUMINANCE[0] + channel(1) * LUMINANCE[1] + channel(2) * LUMINANCE[2]
}

/// `smoothstep` of SkSL and WGSL.
fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A texel as premultiplied channels of 0 to 1.
fn unit_texel(texel: [u8; 4]) -> [f32; 4] {
    texel.map(|channel| f32::from(channel) / 255.0)
}

/// `source` (premultiplied, 0 to 1) drawn source over `pixel`.
fn source_over(pixel: &mut [u8], source: [f32; 4]) {
    let source = source.map(|channel| channel.clamp(0.0, 1.0));
    for channel in 0..4 {
        let destination = f32::from(pixel[channel]) / 255.0;
        let value = source[channel] + destination * (1.0 - source[3]);
        pixel[channel] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

/// Calls `f` with each pixel of the area and the coordinates of its centre
/// in the convention (device pixels of the area).
fn for_each_pixel(pixels: &mut [u8], width: usize, height: usize, mut f: impl FnMut(&mut [u8], [f32; 2])) {
    if width == 0 || height == 0 {
        return;
    }
    for (index, pixel) in area(pixels, width, height).chunks_exact_mut(4).enumerate() {
        let (x, y) = (index % width, index / width);
        f(pixel, [x as f32 + 0.5, y as f32 + 0.5]);
    }
}

// --- dissolve ------------------------------------------------------------------

const DISSOLVE_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float progress;
uniform float center_x;
uniform float center_y;
uniform float4 glow_color;
uniform shader from_page;
uniform shader to_page;
uniform shader pattern;

half4 main(float2 coord) {
    float p = clamp(progress, 0.0, 1.0);
    float2 uv = coord / max(layer_size, float2(1.0));
    float2 center = float2(clamp(center_x, 0.0, 1.0), clamp(center_y, 0.0, 1.0));
    float d = min(distance(uv, center), 1.0);
    // The frontier: behind it (the pattern below it) the page that comes.
    float front = p * (1.0 + 2.0 * 0.09 + 0.42) - 0.09 - 0.42 * d;
    float n = dot(float3(pattern.eval(coord).rgb), float3(0.2126, 0.7152, 0.0722));
    float reveal = 1.0 - smoothstep(front - 0.09, front + 0.09, n);
    float glow = (smoothstep(front - 0.02, front + 0.02, n) - smoothstep(front + 0.02, front + 0.1, n)) * 4.0 * p * (1.0 - p);
    float4 base = mix(float4(from_page.eval(coord)), float4(to_page.eval(coord)), reveal);
    float3 rgb = min(base.rgb + glow_color.rgb * (glow * 0.3 * base.a), float3(base.a));
    return half4(float4(rgb, base.a));
}
"#;

const DISSOLVE_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    progress: f32,
    center_x: f32,
    center_y: f32,
    glow_color: vec4<f32>,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let p = clamp(uniforms.progress, 0.0, 1.0);
    let uv = coord / max(uniforms.layer_size, vec2<f32>(1.0));
    let center = vec2<f32>(clamp(uniforms.center_x, 0.0, 1.0), clamp(uniforms.center_y, 0.0, 1.0));
    let d = min(distance(uv, center), 1.0);
    // The frontier: behind it (the pattern below it) the page that comes.
    let front = p * (1.0 + 2.0 * 0.09 + 0.42) - 0.09 - 0.42 * d;
    let n = dot(sample_pattern(coord).rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let reveal = 1.0 - smoothstep(front - 0.09, front + 0.09, n);
    let glow = (smoothstep(front - 0.02, front + 0.02, n) - smoothstep(front + 0.02, front + 0.1, n)) * 4.0 * p * (1.0 - p);
    let base = mix(sample_from_page(coord), sample_to_page(coord), vec4<f32>(reveal));
    let rgb = min(base.rgb + uniforms.glow_color.rgb * (glow * 0.3 * base.a), vec3<f32>(base.a));
    return vec4<f32>(rgb, base.a);
}
"#;

/// Dissolves from one page to another (Effector's compiz-style dissolve):
/// a frontier spreads from the point (`center-x`, `center-y`) of the area
/// (fractions of its width and height, clamped to 0..1) as `progress` goes
/// from 0 (`from-page`) to 1 (`to-page`), and crosses each pixel when it
/// reaches the luminance of `pattern` there, with a soft edge and a glow of
/// `glow-color` along it. The pages and the pattern are stretched over the
/// area; without a pattern the dissolve is a round wipe from the point.
///
/// `dissolve-transition(progress=0.5, center-x=0.5, center-y=0.5,
/// glow-color=#FF942E)`, the images set from code. An overlay shader effect
/// with three children, drawn source over the content; a progress of 0
/// without images (the neutral value) leaves the content unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct DissolveTransitionEffect {
    /// The page the transition leaves.
    pub from_page: EffectImage,
    /// The page the transition comes to.
    pub to_page: EffectImage,
    /// The pattern of the dissolve: the frontier crosses a pixel when it
    /// reaches the luminance of the pattern there.
    pub pattern: EffectImage,
    /// How far the transition is, 0 to 1 (default 0.5).
    pub progress: f64,
    /// The horizontal position of the point the frontier spreads from, 0
    /// (left) to 1 (right) (default 0.5).
    pub center_x: f64,
    /// The vertical position of that point, 0 (top) to 1 (bottom) (default
    /// 0.5).
    pub center_y: f64,
    /// The colour of the glow along the frontier (default #FF942E, Effector's
    /// orange).
    pub glow_color: Color,
}

impl Default for DissolveTransitionEffect {
    fn default() -> Self {
        Self {
            from_page: EffectImage::empty(),
            to_page: EffectImage::empty(),
            pattern: EffectImage::empty(),
            progress: 0.5,
            center_x: 0.5,
            center_y: 0.5,
            glow_color: Color::from_rgb(0xFF, 0x94, 0x2E),
        }
    }
}

impl CustomEffect for DissolveTransitionEffect {
    const NAME: &'static str = "dissolve-transition";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("from-page", (&self.from_page).into());
        f("to-page", (&self.to_page).into());
        f("pattern", (&self.pattern).into());
        f("progress", self.progress.into());
        f("center-x", self.center_x.into());
        f("center-y", self.center_y.into());
        f("glow-color", self.glow_color.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "from-page" => value.as_image().map(|image| self.from_page = image.clone()).is_some(),
            "to-page" => value.as_image().map(|image| self.to_page = image.clone()).is_some(),
            "pattern" => value.as_image().map(|image| self.pattern = image.clone()).is_some(),
            "progress" => value.as_float().map(|progress| self.progress = progress).is_some(),
            "center-x" => value.as_float().map(|x| self.center_x = x).is_some(),
            "center-y" => value.as_float().map(|y| self.center_y = y).is_some(),
            "glow-color" => value.as_color().map(|color| self.glow_color = color).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { progress: 0.0, ..Self::default() })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::overlay(DISSOLVE_SKSL, DISSOLVE_WGSL, BitmapBlendingMode::SourceOver)
                .with_children(["from-page", "to-page", "pattern"]),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let layer = [width as f32, height as f32];
        let (from, to, pattern) = (self.from_page.pixels(), self.to_page.pixels(), self.pattern.pixels());
        let p = unit(self.progress) as f32;
        let center = [unit(self.center_x) as f32, unit(self.center_y) as f32];
        // The colour premultiplied, as the uniform of the convention.
        let alpha = f32::from(self.glow_color.a) / 255.0;
        let glow_color =
            [self.glow_color.r, self.glow_color.g, self.glow_color.b].map(|channel| f32::from(channel) / 255.0 * alpha);
        let size = [layer[0].max(1.0), layer[1].max(1.0)];
        for_each_pixel(pixels, width, height, |pixel, coord| {
            let uv = [coord[0] / size[0], coord[1] / size[1]];
            let (dx, dy) = (uv[0] - center[0], uv[1] - center[1]);
            let d = (dx * dx + dy * dy).sqrt().min(1.0);
            let front = p * (1.0 + 2.0 * 0.09 + 0.42) - 0.09 - 0.42 * d;
            let n = luminance(sample_image(pattern.as_ref(), coord, layer));
            let reveal = 1.0 - smoothstep(front - 0.09, front + 0.09, n);
            let glow =
                (smoothstep(front - 0.02, front + 0.02, n) - smoothstep(front + 0.02, front + 0.1, n)) * 4.0 * p * (1.0 - p);
            let from = unit_texel(sample_image(from.as_ref(), coord, layer));
            let to = unit_texel(sample_image(to.as_ref(), coord, layer));
            let base: [f32; 4] = std::array::from_fn(|channel| from[channel] + (to[channel] - from[channel]) * reveal);
            let lit = |channel: usize| (base[channel] + glow_color[channel] * (glow * 0.3 * base[3])).min(base[3]);
            source_over(pixel, [lit(0), lit(1), lit(2), base[3]]);
        });
        true
    }
}

// --- mask ----------------------------------------------------------------------

const MASK_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float progress;
uniform float softness;
uniform shader to_page;
uniform shader mask;

half4 main(float2 coord) {
    float s = clamp(softness, 0.001, 1.0);
    float threshold = clamp(progress, 0.0, 1.0) * (1.0 + s);
    float m = dot(float3(mask.eval(coord).rgb), float3(0.2126, 0.7152, 0.0722));
    float reveal = clamp((threshold - m) / s, 0.0, 1.0);
    return half4(float4(to_page.eval(coord)) * reveal);
}
"#;

const MASK_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    progress: f32,
    softness: f32,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let s = clamp(uniforms.softness, 0.001, 1.0);
    let threshold = clamp(uniforms.progress, 0.0, 1.0) * (1.0 + s);
    let m = dot(sample_mask(coord).rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let reveal = clamp((threshold - m) / s, 0.0, 1.0);
    return sample_to_page(coord) * reveal;
}
"#;

/// Reveals a page over the content through a mask (the reveal of Effector's
/// dissolve, with the content as the page that leaves): `to-page` is drawn
/// where the luminance of `mask` (premultiplied) is below the threshold
/// `progress * (1 + softness)`, fading in over `softness` of luminance
/// (clamped to 0.001..1); `progress` is clamped to 0..1, so 0 shows the
/// content and 1 the page. The page and the mask are stretched over the
/// area. A horizontal gradient as the mask is a wipe, a noise a dissolve;
/// without a mask the page fades in (fully at a progress of
/// `softness / (1 + softness)`).
///
/// `mask-transition(progress=0.5, softness=0.1)`, the images set from code.
/// An overlay shader effect with two children, drawn source over the
/// content; a progress of 0 (the neutral value) leaves the content
/// unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct MaskTransitionEffect {
    /// The page the transition reveals.
    pub to_page: EffectImage,
    /// The mask: dark parts are revealed first.
    pub mask: EffectImage,
    /// How far the transition is, 0 to 1 (default 0.5).
    pub progress: f64,
    /// The width of the soft edge, in luminance, 0.001 to 1 (default 0.1).
    pub softness: f64,
}

impl Default for MaskTransitionEffect {
    fn default() -> Self {
        Self { to_page: EffectImage::empty(), mask: EffectImage::empty(), progress: 0.5, softness: 0.1 }
    }
}

impl CustomEffect for MaskTransitionEffect {
    const NAME: &'static str = "mask-transition";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("to-page", (&self.to_page).into());
        f("mask", (&self.mask).into());
        f("progress", self.progress.into());
        f("softness", self.softness.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "to-page" => value.as_image().map(|image| self.to_page = image.clone()).is_some(),
            "mask" => value.as_image().map(|image| self.mask = image.clone()).is_some(),
            "progress" => value.as_float().map(|progress| self.progress = progress).is_some(),
            "softness" => value.as_float().map(|softness| self.softness = softness).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { progress: 0.0, ..Self::default() })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::overlay(MASK_SKSL, MASK_WGSL, BitmapBlendingMode::SourceOver)
                .with_children(["to-page", "mask"]),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let layer = [width as f32, height as f32];
        let (to, mask) = (self.to_page.pixels(), self.mask.pixels());
        let s = (if self.softness.is_nan() { 0.001 } else { self.softness.clamp(0.001, 1.0) }) as f32;
        let threshold = unit(self.progress) as f32 * (1.0 + s);
        for_each_pixel(pixels, width, height, |pixel, coord| {
            let m = luminance(sample_image(mask.as_ref(), coord, layer));
            let reveal = ((threshold - m) / s).clamp(0.0, 1.0);
            let to = unit_texel(sample_image(to.as_ref(), coord, layer));
            source_over(pixel, to.map(|channel| channel * reveal));
        });
        true
    }
}
