//! The runtime shader effects of the library (#455, E4; not from upstream):
//! effects of Effector's sample set as programs with the uniform convention
//! of [`ShaderEffectDescription`] (docs/porting/custom-effects.md, section
//! 3.4), each with its SkSL for Skia, its WGSL for Vello (drawn by a pass of
//! the device, E5) and its CPU reference (Vello's CPU mode).
//!
//! - [`PixelateEffect`] samples the content: each cell of the grid shows the
//!   pixel at its centre (Effector's pixelation, a nearest scale down and up).
//! - [`ScanlineShaderEffect`] is an overlay drawn over the content: dark bands
//!   across it.
//! - [`SpotlightShaderEffect`] is an overlay screened over the content: a
//!   coloured light fading from a point of the area.
//!
//! The arithmetic of the programs follows Effector's shaders and filters
//! (`samples/Effector.Sample.Effects`, `ShaderEffects.cs` and
//! `ImageEffects.cs`, with the clamps of its factories), in the coordinates
//! of the convention: device pixels of the layer, lengths in device
//! independent pixels scaled by `layer_scale`.

use super::super::{CustomEffect, EffectParameterValue, ShaderEffectDescription};
use super::{area, clamp, unit};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::Color;

// --- pixelate ------------------------------------------------------------------

const PIXELATE_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float cell_size;
uniform shader content;

half4 main(float2 coord) {
    // The cell in device pixels: at least one pixel, so that 0 leaves the
    // content unchanged.
    float cell = max(clamp(cell_size, 0.0, 64.0) * layer_scale, 1.0);
    float2 center = (floor(coord / cell) + 0.5) * cell;
    // The centre of the pixel under the centre of the cell, inside the layer.
    float2 pixel = min(floor(center), max(layer_size - 1.0, float2(0.0))) + 0.5;
    return content.eval(pixel);
}
"#;

const PIXELATE_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    cell_size: f32,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    // The cell in device pixels: at least one pixel, so that 0 leaves the
    // content unchanged.
    let cell = max(clamp(uniforms.cell_size, 0.0, 64.0) * uniforms.layer_scale, 1.0);
    let center = (floor(coord / cell) + 0.5) * cell;
    // The centre of the pixel under the centre of the cell, inside the layer.
    let pixel = min(floor(center), max(uniforms.layer_size - 1.0, vec2<f32>(0.0))) + 0.5;
    return sample_content(pixel);
}
"#;

/// Pixelates the content (Effector's `PixelateEffect`): a grid of square
/// cells of `cell-size` device independent pixels from the top left corner of
/// the area, each the colour of the pixel at its centre. `cell-size` is
/// clamped to 0..64; a cell is at least one device pixel, so 0 (the neutral
/// value) leaves the content unchanged.
///
/// `pixelate(cell-size=10)`. A content-sampling shader effect with a sample
/// radius of 64 (the largest cell).
#[derive(Clone, Debug, PartialEq)]
pub struct PixelateEffect {
    /// The side of a cell, in device independent pixels (default 10).
    pub cell_size: f64,
}

impl Default for PixelateEffect {
    fn default() -> Self {
        Self { cell_size: 10.0 }
    }
}

impl PixelateEffect {
    /// The largest side of a cell.
    pub const MAXIMUM_CELL_SIZE: f64 = 64.0;

    /// The side of a cell in device pixels at `scale`, as the program
    /// computes it.
    fn cell(&self, scale: f64) -> f32 {
        let scale = if scale > 0.0 && scale.is_finite() { scale as f32 } else { 1.0 };
        (clamp(self.cell_size, 0.0, Self::MAXIMUM_CELL_SIZE) as f32 * scale).max(1.0)
    }
}

impl CustomEffect for PixelateEffect {
    const NAME: &'static str = "pixelate";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("cell-size", self.cell_size.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "cell-size" => value.as_float().map(|size| self.cell_size = size).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { cell_size: 0.0 })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(PIXELATE_SKSL, PIXELATE_WGSL)
                .with_sample_radius(Self::MAXIMUM_CELL_SIZE),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        let pixels = area(pixels, width, height);
        if width == 0 || height == 0 {
            return true;
        }
        let cell = self.cell(scale);
        // The pixel under the centre of the cell of a coordinate, as the
        // program computes it.
        let source_of = |coordinate: usize, extent: usize| -> usize {
            let center = (((coordinate as f32 + 0.5) / cell).floor() + 0.5) * cell;
            center.floor().clamp(0.0, (extent - 1) as f32) as usize
        };
        let columns: Vec<usize> = (0..width).map(|x| source_of(x, width)).collect();
        let source = pixels.to_vec();
        for y in 0..height {
            let row = source_of(y, height);
            for (x, column) in columns.iter().enumerate() {
                let from = (row * width + column) * 4;
                pixels[(y * width + x) * 4..][..4].copy_from_slice(&source[from..from + 4]);
            }
        }
        true
    }
}

// --- scan lines ----------------------------------------------------------------

const SCANLINE_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float spacing;
uniform float strength;

half4 main(float2 coord) {
    float span = max(clamp(spacing, 2.0, 32.0) * layer_scale, 1.0);
    // The lower half of each band is dark.
    float alpha = fract(coord.y / span) >= 0.5 ? clamp(strength, 0.0, 1.0) : 0.0;
    return half4(0.0, 0.0, 0.0, alpha);
}
"#;

const SCANLINE_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    spacing: f32,
    strength: f32,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let span = max(clamp(uniforms.spacing, 2.0, 32.0) * uniforms.layer_scale, 1.0);
    // The lower half of each band is dark.
    let alpha = select(0.0, clamp(uniforms.strength, 0.0, 1.0), fract(coord.y / span) >= 0.5);
    return vec4<f32>(0.0, 0.0, 0.0, alpha);
}
"#;

/// Dark scan lines over the content (Effector's `ScanlineShaderEffect`):
/// bands of `spacing` device independent pixels (clamped to 2..32) from the
/// top of the area, the lower half of each black at the opacity `strength`
/// (clamped to 0..1), drawn source over the content.
///
/// `scanline(spacing=8, strength=0.24)`. An overlay shader effect; a
/// strength of 0 (the neutral value) leaves the content unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct ScanlineShaderEffect {
    /// The height of a band, in device independent pixels (default 8).
    pub spacing: f64,
    /// The opacity of the dark half of a band, 0 to 1 (default 0.24).
    pub strength: f64,
}

impl Default for ScanlineShaderEffect {
    fn default() -> Self {
        Self { spacing: 8.0, strength: 0.24 }
    }
}

impl CustomEffect for ScanlineShaderEffect {
    const NAME: &'static str = "scanline";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("spacing", self.spacing.into());
        f("strength", self.strength.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "spacing" => value.as_float().map(|spacing| self.spacing = spacing).is_some(),
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { strength: 0.0, ..Self::default() })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(ShaderEffectDescription::overlay(SCANLINE_SKSL, SCANLINE_WGSL, BitmapBlendingMode::SourceOver))
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        let pixels = area(pixels, width, height);
        let scale = if scale > 0.0 && scale.is_finite() { scale as f32 } else { 1.0 };
        let span = (clamp(self.spacing, 2.0, 32.0) as f32 * scale).max(1.0);
        let alpha = unit(self.strength) as f32;
        for (y, row) in pixels.chunks_exact_mut(width.max(1) * 4).enumerate() {
            if ((y as f32 + 0.5) / span).fract() < 0.5 {
                continue;
            }
            // Black at `alpha` over the pixel: each premultiplied channel
            // times 1 - alpha, the alpha plus alpha times what it lacks.
            for pixel in row.chunks_exact_mut(4) {
                let over = |value: u8, source: f32| {
                    ((source + f32::from(value) / 255.0 * (1.0 - alpha)) * 255.0).round().clamp(0.0, 255.0) as u8
                };
                for channel in 0..3 {
                    pixel[channel] = over(pixel[channel], 0.0);
                }
                pixel[3] = over(pixel[3], alpha);
            }
        }
        true
    }
}

// --- spotlight -----------------------------------------------------------------

const SPOTLIGHT_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float center_x;
uniform float center_y;
uniform float radius;
uniform float strength;
uniform float4 color;

half4 main(float2 coord) {
    float2 size = max(layer_size, float2(1.0));
    float2 d = coord / size - float2(clamp(center_x, 0.0, 1.0), clamp(center_y, 0.0, 1.0));
    float r = clamp(radius, 0.05, 1.0);
    float dist = length(d);
    float fade = dist >= r ? 0.0 : 1.0 - dist / r;
    return half4(color * (fade * clamp(strength, 0.0, 1.0)));
}
"#;

const SPOTLIGHT_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    center_x: f32,
    center_y: f32,
    radius: f32,
    strength: f32,
    color: vec4<f32>,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let size = max(uniforms.layer_size, vec2<f32>(1.0));
    let d = coord / size - vec2<f32>(clamp(uniforms.center_x, 0.0, 1.0), clamp(uniforms.center_y, 0.0, 1.0));
    let r = clamp(uniforms.radius, 0.05, 1.0);
    let dist = length(d);
    let fade = select(1.0 - dist / r, 0.0, dist >= r);
    return uniforms.color * (fade * clamp(uniforms.strength, 0.0, 1.0));
}
"#;

/// A coloured light screened over the content (Effector's
/// `SpotlightShaderEffect`): around the point (`center-x`, `center-y`) of the
/// area (fractions of its width and height, clamped to 0..1), fading linearly
/// to nothing at the distance `radius` (a fraction of the area, measured in
/// the fractions of each axis, clamped to 0.05..1), the colour at the opacity
/// `strength` (clamped to 0..1) times its own at the centre.
///
/// `spotlight(center-x=0.55, center-y=0.4, radius=0.42, strength=0.35,
/// color=#FFD26B)`. An overlay shader effect with the blend mode `Screen`; a
/// strength of 0 (the neutral value) leaves the content unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct SpotlightShaderEffect {
    /// The horizontal position of the centre, 0 (left) to 1 (right) (default
    /// 0.55).
    pub center_x: f64,
    /// The vertical position of the centre, 0 (top) to 1 (bottom) (default
    /// 0.4).
    pub center_y: f64,
    /// Where the light ends, a fraction of the area (default 0.42).
    pub radius: f64,
    /// The opacity of the light at its centre, 0 to 1 (default 0.35).
    pub strength: f64,
    /// The colour of the light (default #FFD26B).
    pub color: Color,
}

impl Default for SpotlightShaderEffect {
    fn default() -> Self {
        Self { center_x: 0.55, center_y: 0.4, radius: 0.42, strength: 0.35, color: Color::from_rgb(0xFF, 0xD2, 0x6B) }
    }
}

impl CustomEffect for SpotlightShaderEffect {
    const NAME: &'static str = "spotlight";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("center-x", self.center_x.into());
        f("center-y", self.center_y.into());
        f("radius", self.radius.into());
        f("strength", self.strength.into());
        f("color", self.color.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "center-x" => value.as_float().map(|x| self.center_x = x).is_some(),
            "center-y" => value.as_float().map(|y| self.center_y = y).is_some(),
            "radius" => value.as_float().map(|radius| self.radius = radius).is_some(),
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { strength: 0.0, ..Self::default() })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(ShaderEffectDescription::overlay(SPOTLIGHT_SKSL, SPOTLIGHT_WGSL, BitmapBlendingMode::Screen))
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let pixels = area(pixels, width, height);
        let size = [(width as f32).max(1.0), (height as f32).max(1.0)];
        let center = [unit(self.center_x) as f32, unit(self.center_y) as f32];
        let radius = clamp(self.radius, 0.05, 1.0) as f32;
        let strength = unit(self.strength) as f32;
        // The colour premultiplied, as the uniform of the convention.
        let alpha = f32::from(self.color.a) / 255.0;
        let color = [self.color.r, self.color.g, self.color.b].map(|channel| f32::from(channel) / 255.0 * alpha);
        let color = [color[0], color[1], color[2], alpha];
        for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
            let (x, y) = (index % width, index / width);
            let dx = (x as f32 + 0.5) / size[0] - center[0];
            let dy = (y as f32 + 0.5) / size[1] - center[1];
            let distance = (dx * dx + dy * dy).sqrt();
            let fade = if distance >= radius { 0.0 } else { 1.0 - distance / radius };
            let factor = fade * strength;
            if factor <= 0.0 {
                continue;
            }
            // Screen, premultiplied: source + destination - source * destination.
            for channel in 0..4 {
                let source = color[channel] * factor;
                let destination = f32::from(pixel[channel]) / 255.0;
                let value = source + destination - source * destination;
                pixel[channel] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
        true
    }
}
