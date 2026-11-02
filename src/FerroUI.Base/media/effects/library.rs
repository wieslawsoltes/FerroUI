//! The custom effects FerroUI ships (#455, E2 and E3; not from upstream):
//! the colour effects of Effector's sample set (tint, grayscale, sepia,
//! saturation, brightness and contrast, invert), its glow, and the image
//! effects sharpen and edge detect (convolutions, E3), as values of
//! [`CustomEffect`] that describe themselves as effect graphs
//! ([`CustomEffect::graph`]), so that every backend draws them on the GPU
//! with the painter of the composition effects
//! (docs/porting/custom-effects.md).
//!
//! They are registered with [`CustomEffectRegistry`] the first time the
//! registry is used ([`register_library_effects`]), so that effect strings
//! and markup name them: `Effect="tint(color=#0F9D8E, strength=0.55)"`.
//! Each has a CPU reference ([`CustomEffect::cpu_reference`]), the function
//! the render tests compare the backends with.
//!
//! The names of the types are those of Effector's classes; the composition
//! effect descriptions with the names of Win2D's classes (`TintEffect`,
//! `SepiaEffect`, ...) are other types, in
//! `rendering::composition::effects`.

use super::{
    CustomEffect, CustomEffectRegistry, EffectExtensions, EffectParameterValue, IEffect, ImmutableDropShadowEffect,
    CUSTOM_EFFECT_CONTENT,
};
use crate::media::{Color, ColorMatrix, ColorMatrixAlphaMode, Colors, ConvolveMatrix};
use crate::rendering::composition::effects as graph;
use crate::rendering::composition::effects::{
    CanvasComposite, CompositionEffectSourceParameter, GraphicsEffect, GraphicsEffectSource,
};
use crate::Thickness;
use std::sync::Once;

/// Registers the effects of this module with [`CustomEffectRegistry`], once
/// per process. The registry calls it the first time it is used; a name an
/// application registered before keeps the application's effect.
pub fn register_library_effects() {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| {
        let _ = CustomEffectRegistry::register::<TintEffect>();
        let _ = CustomEffectRegistry::register::<GrayscaleEffect>();
        let _ = CustomEffectRegistry::register::<SepiaEffect>();
        let _ = CustomEffectRegistry::register::<SaturationEffect>();
        let _ = CustomEffectRegistry::register::<BrightnessContrastEffect>();
        let _ = CustomEffectRegistry::register::<InvertEffect>();
        let _ = CustomEffectRegistry::register::<GlowEffect>();
        let _ = CustomEffectRegistry::register::<SharpenEffect>();
        let _ = CustomEffectRegistry::register::<EdgeDetectEffect>();
    });
}

/// The source parameter of the content.
fn content() -> Option<GraphicsEffectSource> {
    Some(CompositionEffectSourceParameter::new(CUSTOM_EFFECT_CONTENT).into())
}

/// `value` clamped to `min..=max`; not a number is `min`.
fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value.is_nan() {
        min
    } else {
        value.clamp(min, max)
    }
}

/// `value` clamped to `0..=1`.
fn unit(value: f64) -> f64 {
    clamp(value, 0.0, 1.0)
}

/// The pixels of `width` by `height` in `pixels`.
fn area(pixels: &mut [u8], width: usize, height: usize) -> &mut [u8] {
    let length = width.saturating_mul(height).saturating_mul(4).min(pixels.len());
    &mut pixels[..length]
}

/// The CPU reference of a colour matrix node: the matrix applied to the
/// colours un-premultiplied (the alpha mode of the nodes of these effects).
fn apply_matrix(matrix: &ColorMatrix, pixels: &mut [u8], width: usize, height: usize) -> bool {
    if !matrix.is_identity() {
        matrix.apply_to_rgba8(area(pixels, width, height), ColorMatrixAlphaMode::Premultiplied);
    }
    true
}

// --- tint ----------------------------------------------------------------------

/// Tints the content towards a colour (Effector's `TintEffect`): each colour
/// channel multiplied by `1 - strength + strength * color`, the alpha kept;
/// `strength` is clamped to 0..1 and the alpha of `color` is not used.
///
/// `tint(color=#0F9D8E, strength=0.55)`. The graph is a tint effect
/// (`Tint.Color`, the colour mixed with white by the strength).
#[derive(Clone, Debug, PartialEq)]
pub struct TintEffect {
    /// The colour (default deep sky blue).
    pub color: Color,
    /// How far towards the colour, 0 to 1 (default 0.55).
    pub strength: f64,
}

impl Default for TintEffect {
    fn default() -> Self {
        Self { color: Colors::DEEP_SKY_BLUE, strength: 0.55 }
    }
}

impl TintEffect {
    /// The colour of the tint node: white mixed with the colour by the
    /// strength, opaque.
    pub fn tint_color(&self) -> Color {
        let strength = unit(self.strength);
        let channel = |value: u8| (255.0 + (f64::from(value) - 255.0) * strength).round().clamp(0.0, 255.0) as u8;
        Color::from_argb(255, channel(self.color.r), channel(self.color.g), channel(self.color.b))
    }

    /// The colour matrix of the effect.
    pub fn matrix(&self) -> ColorMatrix {
        let color = self.tint_color();
        ColorMatrix::tint([color.r, color.g, color.b, color.a].map(|channel| f32::from(channel) / 255.0))
    }
}

impl CustomEffect for TintEffect {
    const NAME: &'static str = "tint";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("color", self.color.into());
        f("strength", self.strength.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { color: Colors::WHITE, strength: 0.0 })
    }

    fn graph() -> Option<GraphicsEffect> {
        Some(graph::TintEffect { name: "Tint".into(), source: content(), ..Default::default() }.into())
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("Tint.Color", self.tint_color().into());
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        apply_matrix(&self.matrix(), pixels, width, height)
    }
}

// --- grayscale -----------------------------------------------------------------

/// Turns the content grey (Effector's `GrayscaleEffect`): the saturation
/// matrix at `1 - amount`, `amount` clamped to 0..1.
///
/// `grayscale(amount=1)`. The graph is a saturation effect
/// (`Grayscale.Saturation`).
#[derive(Clone, Debug, PartialEq)]
pub struct GrayscaleEffect {
    /// How grey, 0 (unchanged) to 1 (grey) (default 1).
    pub amount: f64,
}

impl Default for GrayscaleEffect {
    fn default() -> Self {
        Self { amount: 1.0 }
    }
}

impl GrayscaleEffect {
    /// The colour matrix of the effect.
    pub fn matrix(&self) -> ColorMatrix {
        ColorMatrix::saturation((1.0 - unit(self.amount)) as f32)
    }
}

impl CustomEffect for GrayscaleEffect {
    const NAME: &'static str = "grayscale";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("amount", self.amount.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "amount" => value.as_float().map(|amount| self.amount = amount).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { amount: 0.0 })
    }

    fn graph() -> Option<GraphicsEffect> {
        Some(graph::SaturationEffect { name: "Grayscale".into(), saturation: 0.0, source: content() }.into())
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("Grayscale.Saturation", (1.0 - unit(self.amount)).into());
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        apply_matrix(&self.matrix(), pixels, width, height)
    }
}

// --- sepia ---------------------------------------------------------------------

/// Tones the content in sepia (Effector's `SepiaEffect`): the identity
/// mixed with the sepia matrix by `amount`, clamped to 0..1.
///
/// `sepia(amount=1)`. The graph is a sepia effect named after the
/// parameter, which sets its intensity (`amount.Intensity`): the default
/// binding of parameters to members.
#[derive(Clone, Debug, PartialEq)]
pub struct SepiaEffect {
    /// How much sepia, 0 (unchanged) to 1 (default 1).
    pub amount: f64,
}

impl Default for SepiaEffect {
    fn default() -> Self {
        Self { amount: 1.0 }
    }
}

impl SepiaEffect {
    /// The colour matrix of the effect.
    pub fn matrix(&self) -> ColorMatrix {
        ColorMatrix::sepia(self.amount as f32)
    }
}

impl CustomEffect for SepiaEffect {
    const NAME: &'static str = "sepia";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("amount", self.amount.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "amount" => value.as_float().map(|amount| self.amount = amount).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { amount: 0.0 })
    }

    fn graph() -> Option<GraphicsEffect> {
        let sepia = graph::SepiaEffect { name: "amount".into(), intensity: 1.0, source: content(), ..Default::default() };
        Some(sepia.into())
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        apply_matrix(&self.matrix(), pixels, width, height)
    }
}

// --- saturation ----------------------------------------------------------------

/// Changes the saturation of the content (Effector's `SaturationEffect`):
/// 0 is grey, 1 unchanged, up to 2.5 more saturated (clamped).
///
/// `saturation(saturation=1.5)`. The graph is a saturation effect
/// (`Saturation.Saturation`).
#[derive(Clone, Debug, PartialEq)]
pub struct SaturationEffect {
    /// The saturation, 0 to 2.5 (default 1).
    pub saturation: f64,
}

impl Default for SaturationEffect {
    fn default() -> Self {
        Self { saturation: 1.0 }
    }
}

impl SaturationEffect {
    /// The largest saturation.
    pub const MAXIMUM: f64 = 2.5;

    /// The colour matrix of the effect.
    pub fn matrix(&self) -> ColorMatrix {
        ColorMatrix::saturation(clamp(self.saturation, 0.0, Self::MAXIMUM) as f32)
    }
}

impl CustomEffect for SaturationEffect {
    const NAME: &'static str = "saturation";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("saturation", self.saturation.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "saturation" => value.as_float().map(|saturation| self.saturation = saturation).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { saturation: 1.0 })
    }

    fn graph() -> Option<GraphicsEffect> {
        Some(graph::SaturationEffect { name: "Saturation".into(), saturation: 1.0, source: content() }.into())
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("Saturation.Saturation", clamp(self.saturation, 0.0, Self::MAXIMUM).into());
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        apply_matrix(&self.matrix(), pixels, width, height)
    }
}

// --- brightness and contrast ---------------------------------------------------

/// The graph of a linear transfer of the colour channels, the alpha kept:
/// `Transfer.RedSlope`, `Transfer.RedOffset` and the same of green and blue.
fn transfer_graph() -> GraphicsEffect {
    graph::LinearTransferEffect { name: "Transfer".into(), alpha_disable: true, source: content(), ..Default::default() }
        .into()
}

/// The values of the members of [`transfer_graph`]: the same slope and
/// offset for red, green and blue.
fn transfer_values(slope: f64, offset: f64, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
    f("Transfer.RedSlope", slope.into());
    f("Transfer.GreenSlope", slope.into());
    f("Transfer.BlueSlope", slope.into());
    f("Transfer.RedOffset", offset.into());
    f("Transfer.GreenOffset", offset.into());
    f("Transfer.BlueOffset", offset.into());
}

/// The colour matrix of [`transfer_graph`] with a slope and an offset.
fn transfer_matrix(slope: f64, offset: f64) -> ColorMatrix {
    let (slope, offset) = (slope as f32, offset as f32);
    ColorMatrix::scale_and_offset([slope, slope, slope, 1.0], [offset, offset, offset, 0.0])
}

/// Changes the brightness and the contrast of the content (Effector's
/// `BrightnessContrastEffect`): each colour channel `c` becomes
/// `contrast * c + (1 - contrast) / 2 + brightness`, the brightness clamped
/// to -1..1 and the contrast to 0..2.5; the alpha is kept.
///
/// `brightness-contrast(brightness=0.1, contrast=1.3)`. The graph is a
/// linear transfer (`Transfer.RedSlope`, `Transfer.RedOffset`, ...).
#[derive(Clone, Debug, PartialEq)]
pub struct BrightnessContrastEffect {
    /// The brightness added, -1 to 1 (default 0).
    pub brightness: f64,
    /// The contrast, 0 (grey) to 2.5, 1 unchanged (default 1).
    pub contrast: f64,
}

impl Default for BrightnessContrastEffect {
    fn default() -> Self {
        Self { brightness: 0.0, contrast: 1.0 }
    }
}

impl BrightnessContrastEffect {
    /// The largest contrast.
    pub const MAXIMUM_CONTRAST: f64 = 2.5;

    /// The slope and the offset of each colour channel.
    fn transfer(&self) -> (f64, f64) {
        let contrast = clamp(self.contrast, 0.0, Self::MAXIMUM_CONTRAST);
        let brightness = clamp(self.brightness, -1.0, 1.0);
        (contrast, (1.0 - contrast) * 0.5 + brightness)
    }

    /// The colour matrix of the effect.
    pub fn matrix(&self) -> ColorMatrix {
        let (slope, offset) = self.transfer();
        transfer_matrix(slope, offset)
    }
}

impl CustomEffect for BrightnessContrastEffect {
    const NAME: &'static str = "brightness-contrast";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("brightness", self.brightness.into());
        f("contrast", self.contrast.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "brightness" => value.as_float().map(|brightness| self.brightness = brightness).is_some(),
            "contrast" => value.as_float().map(|contrast| self.contrast = contrast).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self::default())
    }

    fn graph() -> Option<GraphicsEffect> {
        Some(transfer_graph())
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        let (slope, offset) = self.transfer();
        transfer_values(slope, offset, f);
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        apply_matrix(&self.matrix(), pixels, width, height)
    }
}

// --- invert --------------------------------------------------------------------

/// Inverts the colours of the content (Effector's `InvertEffect`): each
/// colour channel `c` becomes `c + amount * (1 - 2 c)`, `amount` clamped to
/// 0..1; the alpha is kept.
///
/// `invert(amount=1)`. The graph is a linear transfer
/// (`Transfer.RedSlope`, `Transfer.RedOffset`, ...).
#[derive(Clone, Debug, PartialEq)]
pub struct InvertEffect {
    /// How far inverted, 0 (unchanged) to 1 (default 1).
    pub amount: f64,
}

impl Default for InvertEffect {
    fn default() -> Self {
        Self { amount: 1.0 }
    }
}

impl InvertEffect {
    /// The slope and the offset of each colour channel.
    fn transfer(&self) -> (f64, f64) {
        let amount = unit(self.amount);
        (1.0 - 2.0 * amount, amount)
    }

    /// The colour matrix of the effect.
    pub fn matrix(&self) -> ColorMatrix {
        let (slope, offset) = self.transfer();
        transfer_matrix(slope, offset)
    }
}

impl CustomEffect for InvertEffect {
    const NAME: &'static str = "invert";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("amount", self.amount.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "amount" => value.as_float().map(|amount| self.amount = amount).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { amount: 0.0 })
    }

    fn graph() -> Option<GraphicsEffect> {
        Some(transfer_graph())
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        let (slope, offset) = self.transfer();
        transfer_values(slope, offset, f);
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        apply_matrix(&self.matrix(), pixels, width, height)
    }
}

// --- glow ----------------------------------------------------------------------

/// A blurred halo of a colour around the content, under it (Effector's
/// `GlowEffect`): the alpha of the content in the colour, its alpha times
/// `intensity` (clamped to 0..1), blurred by `blur-radius` (the blur radius
/// of the drop shadow effect), and the content over it.
///
/// `glow(color=Gold, blur-radius=12, intensity=0.9)`. The graph is a
/// source-over composite of a shadow of the content (`Glow.BlurAmount`,
/// `Glow.ShadowColor`) and the content, drawn as the drop shadow effect of
/// the drawing contexts with no offset.
#[derive(Clone, Debug, PartialEq)]
pub struct GlowEffect {
    /// The colour of the halo (default gold).
    pub color: Color,
    /// The blur radius of the halo, in device independent pixels (default
    /// 12).
    pub blur_radius: f64,
    /// The opacity of the halo, 0 to 1 (default 0.9).
    pub intensity: f64,
}

impl Default for GlowEffect {
    fn default() -> Self {
        Self { color: Colors::GOLD, blur_radius: 12.0, intensity: 0.9 }
    }
}

impl GlowEffect {
    /// The blur radius, at least 0.
    fn radius(&self) -> f64 {
        if self.blur_radius.is_nan() {
            0.0
        } else {
            self.blur_radius.max(0.0)
        }
    }

    /// The colour of the halo: the colour with its alpha times the
    /// intensity.
    pub fn halo_color(&self) -> Color {
        let alpha = (f64::from(self.color.a) * unit(self.intensity)).round().clamp(0.0, 255.0) as u8;
        Color::from_argb(alpha, self.color.r, self.color.g, self.color.b)
    }

    /// The drop shadow effect the glow is drawn as.
    pub fn drop_shadow(&self) -> ImmutableDropShadowEffect {
        ImmutableDropShadowEffect::new(0.0, 0.0, self.radius(), self.halo_color(), 1.0)
    }
}

impl CustomEffect for GlowEffect {
    const NAME: &'static str = "glow";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("color", self.color.into());
        f("blur-radius", self.blur_radius.into());
        f("intensity", self.intensity.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            "blur-radius" => value.as_float().map(|radius| self.blur_radius = radius).is_some(),
            "intensity" => value.as_float().map(|intensity| self.intensity = intensity).is_some(),
            _ => false,
        }
    }

    /// The padding of the drop shadow effect of the same radius.
    fn padding(&self) -> Thickness {
        let shadow = self.drop_shadow();
        EffectExtensions::get_effect_output_padding(Some(&shadow as &dyn IEffect))
    }

    fn neutral() -> Option<Self> {
        Some(Self { blur_radius: 0.0, intensity: 0.0, ..Self::default() })
    }

    fn graph() -> Option<GraphicsEffect> {
        let shadow = graph::ShadowEffect { name: "Glow".into(), source: content(), ..Default::default() };
        let content = content().expect("the content is a source");
        let sources = vec![shadow.into(), content];
        Some(graph::CompositeEffect { name: String::new(), mode: CanvasComposite::SourceOver, sources }.into())
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        // The standard deviation of the blur of the drop shadow effect, which
        // the painter turns back into its radius.
        let radius = self.radius();
        let amount = if radius > 0.0 { 0.288675 * radius + 0.5 } else { 0.0 };
        f("Glow.BlurAmount", amount.into());
        f("Glow.ShadowColor", self.halo_color().into());
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        let pixels = area(pixels, width, height);
        let color = self.halo_color();
        let color_alpha = f32::from(color.a) / 255.0;
        // The halo: the alpha of the content times the alpha of the colour,
        // blurred.
        let mut halo: Vec<f32> =
            pixels.chunks_exact(4).map(|pixel| f32::from(pixel[3]) / 255.0 * color_alpha).collect();
        let radius = self.radius();
        if radius > 0.0 {
            let sigma = (0.288675 * radius + 0.5) * if scale > 0.0 { scale } else { 1.0 };
            gaussian_blur(&mut halo, width, height, sigma as f32);
        }
        // The content over the halo.
        let rgb = [color.r, color.g, color.b].map(|channel| f32::from(channel) / 255.0);
        for (pixel, halo) in pixels.chunks_exact_mut(4).zip(halo) {
            let under = 1.0 - f32::from(pixel[3]) / 255.0;
            for channel in 0..3 {
                let value = f32::from(pixel[channel]) / 255.0 + rgb[channel] * halo * under;
                pixel[channel] = (value * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            let alpha = f32::from(pixel[3]) / 255.0 + halo * under;
            pixel[3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        true
    }
}

// --- sharpen and edge detect ---------------------------------------------------

/// The names of the elements of a 3 by 3 kernel as members of the effect
/// named `Sharpen` and of the effect named `Edges`.
const SHARPEN_MEMBERS: [&str; 9] = [
    "Sharpen.KernelMatrix0",
    "Sharpen.KernelMatrix1",
    "Sharpen.KernelMatrix2",
    "Sharpen.KernelMatrix3",
    "Sharpen.KernelMatrix4",
    "Sharpen.KernelMatrix5",
    "Sharpen.KernelMatrix6",
    "Sharpen.KernelMatrix7",
    "Sharpen.KernelMatrix8",
];
const EDGE_MEMBERS: [&str; 9] = [
    "Edges.KernelMatrix0",
    "Edges.KernelMatrix1",
    "Edges.KernelMatrix2",
    "Edges.KernelMatrix3",
    "Edges.KernelMatrix4",
    "Edges.KernelMatrix5",
    "Edges.KernelMatrix6",
    "Edges.KernelMatrix7",
    "Edges.KernelMatrix8",
];

/// Sharpens the content (Effector's `SharpenEffect`): the 3 by 3 kernel
/// `[0, -s, 0; -s, 1 + 4 s, -s; 0, -s, 0]` with `s` the strength clamped
/// to 0..2, the four premultiplied channels convolved, transparent beyond
/// the edges.
///
/// `sharpen(strength=1)`. The graph is a convolution (Win2D's
/// `ConvolveMatrixEffect`, `Sharpen.KernelMatrix0` to
/// `Sharpen.KernelMatrix8`), whose kernel steps over device pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct SharpenEffect {
    /// How much, 0 (unchanged) to 2 (default 1).
    pub strength: f64,
}

impl Default for SharpenEffect {
    fn default() -> Self {
        Self { strength: 1.0 }
    }
}

impl SharpenEffect {
    /// The kernel, row by row.
    pub fn kernel(&self) -> [f32; 9] {
        let s = clamp(self.strength, 0.0, 2.0) as f32;
        [0.0, -s, 0.0, -s, 1.0 + 4.0 * s, -s, 0.0, -s, 0.0]
    }

    /// The convolution of the effect.
    pub fn convolution(&self) -> ConvolveMatrix {
        ConvolveMatrix::new(3, 3, &self.kernel()).unwrap_or_default()
    }
}

impl CustomEffect for SharpenEffect {
    const NAME: &'static str = "sharpen";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("strength", self.strength.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { strength: 0.0 })
    }

    fn graph() -> Option<GraphicsEffect> {
        Some(graph::ConvolveMatrixEffect { name: "Sharpen".into(), source: content(), ..Default::default() }.into())
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        for (member, element) in SHARPEN_MEMBERS.into_iter().zip(self.kernel()) {
            f(member, f64::from(element).into());
        }
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        self.convolution().apply_to_premultiplied_rgba8(area(pixels, width, height), width, height);
        true
    }
}

/// Finds the edges of the content: the 3 by 3 Laplacian kernel of
/// Effector's `EdgeDetectEffect`, `[-s, -s, -s; -s, 8 s, -s; -s, -s, -s]`
/// with `s` the strength clamped to 0..2, applied to the colours
/// un-premultiplied with the alpha kept: flat areas turn black, edges keep
/// the change of colour across them.
///
/// Deviation from Effector, which lays a black line of the response of a
/// grey copy over the content: that uses the content twice, which a graph
/// around the content of a visual does not (custom-effects.md, section
/// 3.3); here the response is the output. There is no neutral value (a
/// transition from no effect switches at half way).
///
/// `edge-detect(strength=1)`. The graph is a convolution
/// (`Edges.KernelMatrix0` to `Edges.KernelMatrix8`).
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeDetectEffect {
    /// How strong the response is, 0 to 2 (default 1).
    pub strength: f64,
}

impl Default for EdgeDetectEffect {
    fn default() -> Self {
        Self { strength: 1.0 }
    }
}

impl EdgeDetectEffect {
    /// The kernel, row by row.
    pub fn kernel(&self) -> [f32; 9] {
        let s = clamp(self.strength, 0.0, 2.0) as f32;
        [-s, -s, -s, -s, 8.0 * s, -s, -s, -s, -s]
    }

    /// The convolution of the effect.
    pub fn convolution(&self) -> ConvolveMatrix {
        let mut convolution = ConvolveMatrix::new(3, 3, &self.kernel()).unwrap_or_default();
        convolution.preserve_alpha = true;
        convolution
    }
}

impl CustomEffect for EdgeDetectEffect {
    const NAME: &'static str = "edge-detect";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("strength", self.strength.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            _ => false,
        }
    }

    fn graph() -> Option<GraphicsEffect> {
        Some(
            graph::ConvolveMatrixEffect {
                name: "Edges".into(),
                preserve_alpha: true,
                source: content(),
                ..Default::default()
            }
            .into(),
        )
    }

    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        for (member, element) in EDGE_MEMBERS.into_iter().zip(self.kernel()) {
            f(member, f64::from(element).into());
        }
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        self.convolution().apply_to_premultiplied_rgba8(area(pixels, width, height), width, height);
        true
    }
}

/// Blurs `values` (`width` by `height`) with a Gaussian of standard
/// deviation `sigma` pixels, separably, reading zero beyond the edges.
fn gaussian_blur(values: &mut [f32], width: usize, height: usize, sigma: f32) {
    if sigma <= 0.0 || width == 0 || height == 0 {
        return;
    }
    let reach = (3.0 * sigma).ceil() as isize;
    let mut kernel: Vec<f32> =
        (-reach..=reach).map(|offset| (-((offset * offset) as f32) / (2.0 * sigma * sigma)).exp()).collect();
    let total: f32 = kernel.iter().sum();
    kernel.iter_mut().for_each(|weight| *weight /= total);
    let mut pass = vec![0.0f32; values.len()];
    // Rows.
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            for (index, weight) in kernel.iter().enumerate() {
                let source = x as isize + index as isize - reach;
                if source >= 0 && (source as usize) < width {
                    sum += values[y * width + source as usize] * weight;
                }
            }
            pass[y * width + x] = sum;
        }
    }
    // Columns.
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            for (index, weight) in kernel.iter().enumerate() {
                let source = y as isize + index as isize - reach;
                if source >= 0 && (source as usize) < height {
                    sum += pass[source as usize * width + x] * weight;
                }
            }
            values[y * width + x] = sum;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gaussian_blur_keeps_the_total_away_from_the_edges() {
        let (width, height) = (21, 21);
        let mut values = vec![0.0f32; width * height];
        values[10 * width + 10] = 1.0;
        gaussian_blur(&mut values, width, height, 2.0);
        let total: f32 = values.iter().sum();
        assert!((total - 1.0).abs() < 1e-3, "{total}");
        assert!(values[10 * width + 10] > values[10 * width + 12]);
        assert!((values[10 * width + 12] - values[12 * width + 10]).abs() < 1e-6);
    }
}
