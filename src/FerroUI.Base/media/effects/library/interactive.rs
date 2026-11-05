//! The interactive effects of the library (#455, E7; not from upstream;
//! docs/porting/custom-effects.md, section 3.5): Effector's interactive
//! shader effects (`samples/Effector.Sample.Effects`,
//! `InteractiveShaderEffects.cs`), each as two types:
//!
//! - a value of [`CustomEffect`] that is a runtime shader effect, with its
//!   SkSL for Skia, its WGSL for Vello and its CPU reference, as the other
//!   shader effects of the library: what crosses to the render thread, its
//!   pointer position and its presses as parameters, so its program, compiled
//!   once per backend, is given them as uniforms;
//! - a mutable effect class constructed with
//!   [`Effect::construct_interactive`] that implements
//!   [`IEffectInputHandler`]: set as the `Effect` of an input element, it
//!   changes its value from the pointer input of the element and raises
//!   `invalidated`, so the element takes a new snapshot.
//!
//! | Value | Class | What it draws |
//! |---|---|---|
//! | [`PointerSpotlightShaderEffect`] | [`InteractivePointerSpotlightEffect`] | a light screened over the content under the pointer, brighter while pressed |
//! | [`ReactiveGridShaderEffect`] | [`InteractiveReactiveGridEffect`] | grid lines over the content and a halo under the pointer |
//! | [`WaterRippleShaderEffect`] | [`InteractiveWaterRippleEffect`] | rings that spread from presses and drags and bend the content |
//! | [`BurningFlameShaderEffect`] | [`InteractiveBurningFlameEffect`] | a flame that burns the content from a press (the burn-away button) |
//!
//! The pointer positions are normalized to the host (0 to 1 across it), as
//! Effector's are, so they need no conversion for the scale of the layer; the
//! lengths of the programs in device independent pixels (a distortion) are
//! scaled by `layer_scale`. The water ripple and the flame animate: their
//! classes advance their values on the frames of the animation clock while
//! anything moves ([`EffectHostContext::request_animation_frame`]), and each
//! frame changes uniforms only.
//!
//! Effector's programs are followed with four changes: squares are written
//! as products (`pow` of a negative number is undefined in SkSL, GLSL and
//! WGSL); the content-sampling programs sample the moved content at the
//! centre of the texel under the moved coordinate (the same texel whatever
//! the filtering of a backend) and mix their colours in premultiplied form
//! (so transparent content stays transparent); and the phase of the flame
//! wraps at 10 pi, a period of its three waves, so that its `sin` stays
//! precise.

use super::super::{
    CustomEffect, CustomEffectClass, Effect, EffectHostContext, EffectParameterValue, IEffectInputHandler,
    ImmutableCustomEffect, ShaderEffectDescription,
};
use super::{area, clamp, unit};
use crate::input::{PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::Color;
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, ObjectType, Point, Ref, Upcast};
use std::cell::{Cell, RefCell};

// --- the arithmetic of the CPU references ----------------------------------------

/// A colour premultiplied, 0 to 1, as the uniform of the convention.
fn premultiplied(color: Color) -> [f32; 4] {
    let alpha = f32::from(color.a) / 255.0;
    let channel = |value: u8| f32::from(value) / 255.0 * alpha;
    [channel(color.r), channel(color.g), channel(color.b), alpha]
}

/// The scale of the layer as the programs read it.
fn layer_scale(scale: f64) -> f32 {
    if scale > 0.0 && scale.is_finite() {
        scale as f32
    } else {
        1.0
    }
}

/// `1` for `true`, `0` for `false`: an `int` uniform compared with 0.
fn flag(value: bool) -> f32 {
    if value {
        1.0
    } else {
        0.0
    }
}

/// A premultiplied colour of 0 to 1 as the channels of a pixel.
fn store(pixel: &mut [u8], color: [f32; 4]) {
    for channel in 0..4 {
        pixel[channel] = (color[channel] * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

/// The channels of a pixel as a premultiplied colour of 0 to 1.
fn load(pixel: &[u8]) -> [f32; 4] {
    std::array::from_fn(|channel| f32::from(pixel[channel]) / 255.0)
}

/// The texel of `source` (`width` by `height` premultiplied RGBA) under `at`,
/// transparent beyond it: what `content.eval` and `sample_content` give.
fn texel(source: &[u8], width: usize, height: usize, at: [f32; 2]) -> [f32; 4] {
    let (x, y) = (at[0].floor(), at[1].floor());
    if !(x >= 0.0 && y >= 0.0 && x < width as f32 && y < height as f32) {
        return [0.0; 4];
    }
    load(&source[(y as usize * width + x as usize) * 4..][..4])
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

/// `x` from `a` to `b`, as `mix` of the programs.
fn mix(a: f32, b: f32, x: f32) -> f32 {
    a + (b - a) * x
}

// --- the classes -------------------------------------------------------------------

/// The members of an interactive effect class that holds its value in
/// `value`: the value, its replacement and its change, and the snapshot.
macro_rules! interactive_effect_class {
    ($class:ident, $value:ty) => {
        impl $class {
            /// A new effect with the default value.
            pub fn new() -> Ref<Self> {
                Self::with_value(<$value>::default())
            }

            /// A new effect with `value`.
            pub fn with_value(value: $value) -> Ref<Self> {
                instantiate(Self::construct(value))
            }

            /// The current value of the effect: what its next snapshot is.
            pub fn value(&self) -> $value {
                self.value.borrow().clone()
            }

            /// Replaces the value of the effect; a different value
            /// invalidates the effect.
            pub fn set_value(&self, value: $value) {
                self.update(move |current| *current = value);
            }

            /// Changes the value of the effect with `f`; a change invalidates
            /// the effect, so its host takes a new snapshot. No borrow is
            /// held while the effect raises `invalidated`.
            pub fn update(&self, f: impl FnOnce(&mut $value)) {
                let changed = {
                    let mut value = self.value.borrow_mut();
                    let before = value.clone();
                    f(&mut value);
                    *value != before
                };
                if changed {
                    self.raise_invalidated();
                }
            }
        }

        impl CustomEffectClass for $class {
            fn to_custom_effect(&self) -> ImmutableCustomEffect {
                ImmutableCustomEffect::new(self.value())
            }
        }
    };
}

/// Whether the pointer of `e` is within the bounds of the host.
fn is_within_host(context: &EffectHostContext<'_>, e: &PointerEventArgs) -> bool {
    context.input_bounds().contains(context.get_position(e))
}

/// The input of the overlays that follow the pointer (the spotlight and the
/// grid), as Effector's: the normalized position on entering, moving,
/// pressing and releasing, the pointer over the host from entering to
/// leaving, pressed from the press to the release or the loss of the capture
/// (the host captures the pointer while it is pressed). Where Effector takes
/// the pointer to be over the host on every move and release, a move or a
/// release of a captured pointer outside the host is not over it: the input
/// system raises no exit for a pointer that leaves while captured.
macro_rules! follow_the_pointer {
    ($class:ident) => {
        impl $class {
            fn track(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs, pressed: Option<bool>) {
                let position = context.get_normalized_position(e, true);
                let over = is_within_host(context, e);
                self.update(|value| {
                    value.pointer_x = position.x;
                    value.pointer_y = position.y;
                    value.is_pointer_over = over;
                    if let Some(pressed) = pressed {
                        value.is_pressed = pressed;
                    }
                });
            }
        }

        impl IEffectInputHandler for $class {
            fn on_attached(&self, _context: &EffectHostContext<'_>) {
                self.update(|value| {
                    value.pointer_x = 0.5;
                    value.pointer_y = 0.5;
                    value.is_pointer_over = false;
                    value.is_pressed = false;
                });
            }

            fn on_pointer_entered(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
                self.track(context, e, None);
                self.update(|value| value.is_pointer_over = true);
            }

            fn on_pointer_exited(&self, _context: &EffectHostContext<'_>, _e: &PointerEventArgs) {
                self.update(|value| value.is_pointer_over = false);
            }

            fn on_pointer_moved(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
                self.track(context, e, None);
            }

            fn on_pointer_pressed(&self, context: &EffectHostContext<'_>, e: &PointerPressedEventArgs) {
                self.track(context, e, Some(true));
                context.capture_pointer(e);
            }

            fn on_pointer_released(&self, context: &EffectHostContext<'_>, e: &PointerReleasedEventArgs) {
                self.track(context, e, Some(false));
                context.release_pointer_capture(e);
            }

            fn on_pointer_capture_lost(&self, _context: &EffectHostContext<'_>, _e: &PointerCaptureLostEventArgs) {
                self.update(|value| value.is_pressed = false);
            }
        }
    };
}

/// The frames an animated effect class asks for while it moves: one at a
/// time, from the animation clock, stopped when the class says so or when
/// the effect is detached (a frame of an earlier run is dropped).
#[derive(Default)]
struct FrameLoop {
    running: Cell<bool>,
    generation: Cell<u64>,
    last: Cell<Option<f64>>,
}

/// An effect class that advances its value on frames.
trait Animated: ObjectType + 'static {
    fn frames(&self) -> &FrameLoop;

    /// Advances the value by `seconds`; whether it still moves.
    fn tick(&self, seconds: f64) -> bool;
}

impl FrameLoop {
    /// Starts the frames of `this`, unless they run.
    fn start<T: Animated>(this: &T, context: &EffectHostContext<'_>) {
        let frames = this.frames();
        if frames.running.replace(true) {
            return;
        }
        frames.last.set(None);
        let generation = frames.generation.get() + 1;
        frames.generation.set(generation);
        Self::request::<T>(context, generation);
    }

    fn request<T: Animated>(context: &EffectHostContext<'_>, generation: u64) {
        context.request_animation_frame(move |context, time| {
            let object: &FerroObject = context.effect().upcast();
            let Some(this) = object.downcast_ref::<T>() else { return };
            let frames = this.frames();
            if frames.generation.get() != generation {
                return;
            }
            let now = time.total_seconds();
            // The first frame advances by a frame of 60 Hz; a long pause by
            // a quarter of a second at most.
            let seconds = frames.last.replace(Some(now)).map_or(1.0 / 60.0, |last| (now - last).clamp(0.0, 0.25));
            if this.tick(seconds) {
                Self::request::<T>(context, generation);
            } else {
                frames.running.set(false);
            }
        });
    }

    /// Stops the frames: a frame already asked for does nothing.
    fn stop(&self) {
        self.running.set(false);
        self.generation.set(self.generation.get() + 1);
    }

    /// Whether frames run.
    fn is_running(&self) -> bool {
        self.running.get()
    }
}

// --- pointer spotlight ---------------------------------------------------------------

const POINTER_SPOTLIGHT_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float pointer_x;
uniform float pointer_y;
uniform int is_pointer_over;
uniform int is_pressed;
uniform float radius;
uniform float strength;
uniform float press_boost;
uniform float4 color;

half4 main(float2 coord) {
    float2 size = max(layer_size, float2(1.0));
    float2 d = coord / size - float2(clamp(pointer_x, 0.0, 1.0), clamp(pointer_y, 0.0, 1.0));
    float spot = max(0.0, 1.0 - length(d) / clamp(radius, 0.05, 0.65));
    float pressed = is_pressed != 0 ? 1.0 : 0.0;
    float engaged = max(is_pointer_over != 0 ? 1.0 : 0.0, pressed);
    float intensity = min(clamp(strength, 0.0, 1.0) * engaged + clamp(press_boost, 0.0, 1.0) * pressed, 1.0);
    return half4(color * (spot * spot * intensity));
}
"#;

const POINTER_SPOTLIGHT_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    pointer_x: f32,
    pointer_y: f32,
    is_pointer_over: i32,
    is_pressed: i32,
    radius: f32,
    strength: f32,
    press_boost: f32,
    color: vec4<f32>,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let size = max(uniforms.layer_size, vec2<f32>(1.0));
    let d = coord / size - vec2<f32>(clamp(uniforms.pointer_x, 0.0, 1.0), clamp(uniforms.pointer_y, 0.0, 1.0));
    let spot = max(0.0, 1.0 - length(d) / clamp(uniforms.radius, 0.05, 0.65));
    let pressed = select(0.0, 1.0, uniforms.is_pressed != 0);
    let engaged = max(select(0.0, 1.0, uniforms.is_pointer_over != 0), pressed);
    let intensity = min(clamp(uniforms.strength, 0.0, 1.0) * engaged + clamp(uniforms.press_boost, 0.0, 1.0) * pressed, 1.0);
    return uniforms.color * (spot * spot * intensity);
}
"#;

/// A light under the pointer, screened over the content (Effector's
/// `PointerSpotlightShaderEffect`): around (`pointer-x`, `pointer-y`) (the
/// normalized pointer, clamped to 0..1), fading with the square of the
/// distance to nothing at `radius` (a fraction of the area, clamped to
/// 0.05..0.65), at the opacity `strength` while the pointer is over the host
/// or pressed, plus `press-boost` while pressed (each clamped to 0..1, the sum
/// to 1), times the colour.
///
/// `pointer-spotlight(pointer-x=0.5, pointer-y=0.5, is-pointer-over=true,
/// radius=0.24, strength=0.28, color=#FFD26B)`. An overlay shader effect with
/// the blend mode `Screen`; without the pointer over the host (the default),
/// or with no strength and no boost (the neutral value), the content is
/// unchanged. [`InteractivePointerSpotlightEffect`] follows the pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerSpotlightShaderEffect {
    /// The horizontal position of the pointer, 0 (left) to 1 (right)
    /// (default 0.5).
    pub pointer_x: f64,
    /// The vertical position of the pointer, 0 (top) to 1 (bottom) (default
    /// 0.5).
    pub pointer_y: f64,
    /// Whether the pointer is over the host (default `false`).
    pub is_pointer_over: bool,
    /// Whether the pointer is pressed (default `false`).
    pub is_pressed: bool,
    /// Where the light ends, a fraction of the area (default 0.24).
    pub radius: f64,
    /// The opacity of the light at its centre while the pointer is over the
    /// host, 0 to 1 (default 0.28).
    pub strength: f64,
    /// The opacity added while pressed, 0 to 1 (default 0.42).
    pub press_boost: f64,
    /// The colour of the light (default #FFD26B).
    pub color: Color,
}

impl Default for PointerSpotlightShaderEffect {
    fn default() -> Self {
        Self {
            pointer_x: 0.5,
            pointer_y: 0.5,
            is_pointer_over: false,
            is_pressed: false,
            radius: 0.24,
            strength: 0.28,
            press_boost: 0.42,
            color: Color::from_rgb(0xFF, 0xD2, 0x6B),
        }
    }
}

impl CustomEffect for PointerSpotlightShaderEffect {
    const NAME: &'static str = "pointer-spotlight";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("pointer-x", self.pointer_x.into());
        f("pointer-y", self.pointer_y.into());
        f("is-pointer-over", self.is_pointer_over.into());
        f("is-pressed", self.is_pressed.into());
        f("radius", self.radius.into());
        f("strength", self.strength.into());
        f("press-boost", self.press_boost.into());
        f("color", self.color.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "pointer-x" => value.as_float().map(|x| self.pointer_x = x).is_some(),
            "pointer-y" => value.as_float().map(|y| self.pointer_y = y).is_some(),
            "is-pointer-over" => value.as_bool().map(|over| self.is_pointer_over = over).is_some(),
            "is-pressed" => value.as_bool().map(|pressed| self.is_pressed = pressed).is_some(),
            "radius" => value.as_float().map(|radius| self.radius = radius).is_some(),
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            "press-boost" => value.as_float().map(|boost| self.press_boost = boost).is_some(),
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { strength: 0.0, press_boost: 0.0, ..Self::default() })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(ShaderEffectDescription::overlay(POINTER_SPOTLIGHT_SKSL, POINTER_SPOTLIGHT_WGSL, BitmapBlendingMode::Screen))
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, _scale: f64) -> bool {
        let size = [(width as f32).max(1.0), (height as f32).max(1.0)];
        let pointer = [unit(self.pointer_x) as f32, unit(self.pointer_y) as f32];
        let radius = clamp(self.radius, 0.05, 0.65) as f32;
        let pressed = flag(self.is_pressed);
        let active = flag(self.is_pointer_over).max(pressed);
        let intensity = (unit(self.strength) as f32 * active + unit(self.press_boost) as f32 * pressed).min(1.0);
        let color = premultiplied(self.color);
        for_each_pixel(pixels, width, height, |pixel, coord| {
            let dx = coord[0] / size[0] - pointer[0];
            let dy = coord[1] / size[1] - pointer[1];
            let spot = (1.0 - (dx * dx + dy * dy).sqrt() / radius).max(0.0);
            let factor = spot * spot * intensity;
            if factor <= 0.0 {
                return;
            }
            // Screen, premultiplied: source + destination - source * destination.
            let destination = load(pixel);
            store(pixel, std::array::from_fn(|channel| {
                let source = color[channel] * factor;
                source + destination[channel] - source * destination[channel]
            }));
        });
        true
    }
}

/// The pointer spotlight as an interactive effect class (Effector's
/// `PointerSpotlightShaderEffect`): the light follows the pointer over the
/// host, and the host captures the pointer while it is pressed. Its value is
/// a [`PointerSpotlightShaderEffect`]: the radius, the strengths and the
/// colour are set on it ([`set_value`](Self::set_value),
/// [`update`](Self::update)), the pointer state is the class's.
#[repr(C)]
pub struct InteractivePointerSpotlightEffect {
    base: Effect,
    value: RefCell<PointerSpotlightShaderEffect>,
}

ferro_class!(InteractivePointerSpotlightEffect: Effect);
ferro_impl_classes!(InteractivePointerSpotlightEffect: FerroObjectImpl);

impl InteractivePointerSpotlightEffect {
    fn construct(value: PointerSpotlightShaderEffect) -> Self {
        Self { base: Effect::construct_interactive::<Self>(), value: RefCell::new(value) }
    }
}

interactive_effect_class!(InteractivePointerSpotlightEffect, PointerSpotlightShaderEffect);
follow_the_pointer!(InteractivePointerSpotlightEffect);

// --- reactive grid -------------------------------------------------------------------

const REACTIVE_GRID_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float pointer_x;
uniform float pointer_y;
uniform int is_pointer_over;
uniform int is_pressed;
uniform float cell_size;
uniform float strength;
uniform float press_boost;
uniform float4 color;

half4 main(float2 coord) {
    // The lines: the first device independent pixel of each cell.
    float span = max(clamp(cell_size, 8.0, 64.0) * layer_scale, 1.0);
    float stroke = max(layer_scale, 1.0);
    float2 cell = coord - floor(coord / span) * span;
    float grid = (cell.x < stroke || cell.y < stroke) ? 1.0 : 0.0;
    float2 size = max(layer_size, float2(1.0));
    float2 d = coord / size - float2(clamp(pointer_x, 0.0, 1.0), clamp(pointer_y, 0.0, 1.0));
    float pressed = is_pressed != 0 ? 1.0 : 0.0;
    float engaged = max(is_pointer_over != 0 ? 1.0 : 0.0, pressed);
    float halo = max(0.0, 1.0 - length(d) / (pressed > 0.5 ? 0.28 : 0.18));
    float alpha = grid * clamp(strength, 0.0, 1.0) * 0.45 + halo * engaged * (0.18 + clamp(press_boost, 0.0, 1.0) * pressed);
    return half4(color * min(alpha, 1.0));
}
"#;

const REACTIVE_GRID_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    pointer_x: f32,
    pointer_y: f32,
    is_pointer_over: i32,
    is_pressed: i32,
    cell_size: f32,
    strength: f32,
    press_boost: f32,
    color: vec4<f32>,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    // The lines: the first device independent pixel of each cell.
    let span = max(clamp(uniforms.cell_size, 8.0, 64.0) * uniforms.layer_scale, 1.0);
    let stroke = max(uniforms.layer_scale, 1.0);
    let cell = coord - floor(coord / span) * span;
    let grid = select(0.0, 1.0, cell.x < stroke || cell.y < stroke);
    let size = max(uniforms.layer_size, vec2<f32>(1.0));
    let d = coord / size - vec2<f32>(clamp(uniforms.pointer_x, 0.0, 1.0), clamp(uniforms.pointer_y, 0.0, 1.0));
    let pressed = select(0.0, 1.0, uniforms.is_pressed != 0);
    let engaged = max(select(0.0, 1.0, uniforms.is_pointer_over != 0), pressed);
    let halo = max(0.0, 1.0 - length(d) / select(0.18, 0.28, pressed > 0.5));
    let alpha = grid * clamp(uniforms.strength, 0.0, 1.0) * 0.45 + halo * engaged * (0.18 + clamp(uniforms.press_boost, 0.0, 1.0) * pressed);
    return uniforms.color * min(alpha, 1.0);
}
"#;

/// Grid lines over the content and a halo under the pointer (Effector's
/// `ReactiveGridShaderEffect`): lines of one device independent pixel along
/// the top and the left of each cell of `cell-size` (clamped to 8..64) from
/// the top left corner of the area, at the opacity `strength * 0.45`
/// (`strength` clamped to 0..1), and, while the pointer is over the host or
/// pressed, a halo around (`pointer-x`, `pointer-y`) fading to nothing at 0.18
/// of the area (0.28 while pressed), at the opacity 0.18 plus `press-boost`
/// while pressed; all times the colour, drawn source over the content.
///
/// `reactive-grid(cell-size=22, strength=0.24, color=#64D6FF)`. An overlay
/// shader effect; without strength and the pointer away (the neutral value)
/// the content is unchanged. [`InteractiveReactiveGridEffect`] follows the
/// pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct ReactiveGridShaderEffect {
    /// The horizontal position of the pointer, 0 (left) to 1 (right)
    /// (default 0.5).
    pub pointer_x: f64,
    /// The vertical position of the pointer, 0 (top) to 1 (bottom) (default
    /// 0.5).
    pub pointer_y: f64,
    /// Whether the pointer is over the host (default `false`).
    pub is_pointer_over: bool,
    /// Whether the pointer is pressed (default `false`).
    pub is_pressed: bool,
    /// The side of a cell, in device independent pixels (default 22).
    pub cell_size: f64,
    /// The strength of the lines, 0 to 1 (default 0.24).
    pub strength: f64,
    /// The opacity the halo gains while pressed, 0 to 1 (default 0.36).
    pub press_boost: f64,
    /// The colour of the lines and the halo (default #64D6FF).
    pub color: Color,
}

impl Default for ReactiveGridShaderEffect {
    fn default() -> Self {
        Self {
            pointer_x: 0.5,
            pointer_y: 0.5,
            is_pointer_over: false,
            is_pressed: false,
            cell_size: 22.0,
            strength: 0.24,
            press_boost: 0.36,
            color: Color::from_rgb(0x64, 0xD6, 0xFF),
        }
    }
}

impl CustomEffect for ReactiveGridShaderEffect {
    const NAME: &'static str = "reactive-grid";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("pointer-x", self.pointer_x.into());
        f("pointer-y", self.pointer_y.into());
        f("is-pointer-over", self.is_pointer_over.into());
        f("is-pressed", self.is_pressed.into());
        f("cell-size", self.cell_size.into());
        f("strength", self.strength.into());
        f("press-boost", self.press_boost.into());
        f("color", self.color.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "pointer-x" => value.as_float().map(|x| self.pointer_x = x).is_some(),
            "pointer-y" => value.as_float().map(|y| self.pointer_y = y).is_some(),
            "is-pointer-over" => value.as_bool().map(|over| self.is_pointer_over = over).is_some(),
            "is-pressed" => value.as_bool().map(|pressed| self.is_pressed = pressed).is_some(),
            "cell-size" => value.as_float().map(|size| self.cell_size = size).is_some(),
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            "press-boost" => value.as_float().map(|boost| self.press_boost = boost).is_some(),
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { strength: 0.0, is_pointer_over: false, is_pressed: false, ..Self::default() })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(ShaderEffectDescription::overlay(REACTIVE_GRID_SKSL, REACTIVE_GRID_WGSL, BitmapBlendingMode::SourceOver))
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        let scale = layer_scale(scale);
        let span = (clamp(self.cell_size, 8.0, 64.0) as f32 * scale).max(1.0);
        let line = scale.max(1.0);
        let size = [(width as f32).max(1.0), (height as f32).max(1.0)];
        let pointer = [unit(self.pointer_x) as f32, unit(self.pointer_y) as f32];
        let pressed = flag(self.is_pressed);
        let active = flag(self.is_pointer_over).max(pressed);
        let reach = if pressed > 0.5 { 0.28 } else { 0.18 };
        let lines = unit(self.strength) as f32 * 0.45;
        let halo_alpha = active * (0.18 + unit(self.press_boost) as f32 * pressed);
        let color = premultiplied(self.color);
        for_each_pixel(pixels, width, height, |pixel, coord| {
            let cell = [coord[0] - (coord[0] / span).floor() * span, coord[1] - (coord[1] / span).floor() * span];
            let grid = flag(cell[0] < line || cell[1] < line);
            let dx = coord[0] / size[0] - pointer[0];
            let dy = coord[1] / size[1] - pointer[1];
            let halo = (1.0 - (dx * dx + dy * dy).sqrt() / reach).max(0.0);
            let alpha = (grid * lines + halo * halo_alpha).min(1.0);
            if alpha <= 0.0 {
                return;
            }
            let destination = load(pixel);
            let source = color.map(|channel| channel * alpha);
            store(pixel, std::array::from_fn(|channel| source[channel] + destination[channel] * (1.0 - source[3])));
        });
        true
    }
}

/// The reactive grid as an interactive effect class (Effector's
/// `ReactiveGridShaderEffect`): the halo follows the pointer over the host,
/// and the host captures the pointer while it is pressed. Its value is a
/// [`ReactiveGridShaderEffect`].
#[repr(C)]
pub struct InteractiveReactiveGridEffect {
    base: Effect,
    value: RefCell<ReactiveGridShaderEffect>,
}

ferro_class!(InteractiveReactiveGridEffect: Effect);
ferro_impl_classes!(InteractiveReactiveGridEffect: FerroObjectImpl);

impl InteractiveReactiveGridEffect {
    fn construct(value: ReactiveGridShaderEffect) -> Self {
        Self { base: Effect::construct_interactive::<Self>(), value: RefCell::new(value) }
    }
}

interactive_effect_class!(InteractiveReactiveGridEffect, ReactiveGridShaderEffect);
follow_the_pointer!(InteractiveReactiveGridEffect);

// --- water ripple --------------------------------------------------------------------

const WATER_RIPPLE_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float pointer_x;
uniform float pointer_y;
uniform int is_pointer_over;
uniform int is_pressed;
uniform float2 ripple_1_center;
uniform float ripple_1_age;
uniform float ripple_1_strength;
uniform float2 ripple_2_center;
uniform float ripple_2_age;
uniform float ripple_2_strength;
uniform float2 ripple_3_center;
uniform float ripple_3_age;
uniform float ripple_3_strength;
uniform float distortion;
uniform float max_radius;
uniform float ring_width;
uniform float tint_strength;
uniform float4 color;
uniform shader content;

// One ripple at `coord`: its push (the direction from its centre times its
// wave) and its wave.
float3 ripple(float2 coord, float2 size, float2 center, float age, float strength, float travel, float band) {
    if (age < 0.0 || age > 1.25 || strength <= 0.001) {
        return float3(0.0);
    }
    float2 delta = coord - clamp(center, 0.0, 1.0) * size;
    float dist = length(delta);
    float x = (dist - travel * age) / band;
    float wave = exp(-x * x) * max(1.0 - age / 1.25, 0.0) * min(strength, 1.0);
    float2 push = dist > 1.0 ? delta / dist * wave : float2(0.0);
    return float3(push, wave);
}

half4 main(float2 coord) {
    float2 size = max(layer_size, float2(1.0));
    float travel = max(size.x, size.y) * clamp(max_radius, 0.2, 0.95);
    float band = max(min(size.x, size.y) * clamp(ring_width, 0.02, 0.18), 3.0 * layer_scale);
    float hover = (is_pointer_over != 0 || is_pressed != 0) ? 1.0 : 0.0;
    float2 hover_delta = coord - float2(clamp(pointer_x, 0.0, 1.0), clamp(pointer_y, 0.0, 1.0)) * size;
    float hover_dist = length(hover_delta);
    float hover_field = hover * max(0.0, 1.0 - hover_dist / max(min(size.x, size.y) * 0.34, 1.0));
    float2 displaced = coord;
    if (hover_dist > 1.0) {
        displaced += hover_delta / hover_dist * (hover_field * 1.25 * layer_scale);
    }
    float3 wave = ripple(coord, size, ripple_1_center, ripple_1_age, ripple_1_strength, travel, band)
        + ripple(coord, size, ripple_2_center, ripple_2_age, ripple_2_strength, travel, band)
        + ripple(coord, size, ripple_3_center, ripple_3_age, ripple_3_strength, travel, band);
    displaced += wave.xy * (clamp(distortion, 0.0, 18.0) * layer_scale);
    float highlight = hover_field * 0.18 + wave.z;
    // The texel under the moved coordinate, sampled at its centre.
    float4 base = float4(content.eval(floor(displaced) + 0.5));
    float tint = clamp(tint_strength, 0.0, 0.42);
    float amount = clamp(highlight * tint + hover_field * tint * 0.25, 0.0, 0.42);
    float3 shade = color.rgb * base.a;
    float3 rgb = mix(base.rgb, shade, amount) + shade * min(highlight * 0.06, 0.04);
    return half4(float4(clamp(rgb, float3(0.0), float3(base.a)), base.a));
}
"#;

const WATER_RIPPLE_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    pointer_x: f32,
    pointer_y: f32,
    is_pointer_over: i32,
    is_pressed: i32,
    ripple_1_center: vec2<f32>,
    ripple_1_age: f32,
    ripple_1_strength: f32,
    ripple_2_center: vec2<f32>,
    ripple_2_age: f32,
    ripple_2_strength: f32,
    ripple_3_center: vec2<f32>,
    ripple_3_age: f32,
    ripple_3_strength: f32,
    distortion: f32,
    max_radius: f32,
    ring_width: f32,
    tint_strength: f32,
    color: vec4<f32>,
}

// One ripple at `coord`: its push (the direction from its centre times its
// wave) and its wave.
fn ripple(coord: vec2<f32>, size: vec2<f32>, center: vec2<f32>, age: f32, strength: f32, travel: f32, band: f32) -> vec3<f32> {
    if (age < 0.0 || age > 1.25 || strength <= 0.001) {
        return vec3<f32>(0.0);
    }
    let delta = coord - clamp(center, vec2<f32>(0.0), vec2<f32>(1.0)) * size;
    let dist = length(delta);
    let x = (dist - travel * age) / band;
    let wave = exp(-x * x) * max(1.0 - age / 1.25, 0.0) * min(strength, 1.0);
    var push = vec2<f32>(0.0);
    if (dist > 1.0) {
        push = delta / dist * wave;
    }
    return vec3<f32>(push, wave);
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let size = max(uniforms.layer_size, vec2<f32>(1.0));
    let travel = max(size.x, size.y) * clamp(uniforms.max_radius, 0.2, 0.95);
    let band = max(min(size.x, size.y) * clamp(uniforms.ring_width, 0.02, 0.18), 3.0 * uniforms.layer_scale);
    let hover = select(0.0, 1.0, uniforms.is_pointer_over != 0 || uniforms.is_pressed != 0);
    let hover_delta = coord - vec2<f32>(clamp(uniforms.pointer_x, 0.0, 1.0), clamp(uniforms.pointer_y, 0.0, 1.0)) * size;
    let hover_dist = length(hover_delta);
    let hover_field = hover * max(0.0, 1.0 - hover_dist / max(min(size.x, size.y) * 0.34, 1.0));
    var displaced = coord;
    if (hover_dist > 1.0) {
        displaced += hover_delta / hover_dist * (hover_field * 1.25 * uniforms.layer_scale);
    }
    let wave = ripple(coord, size, uniforms.ripple_1_center, uniforms.ripple_1_age, uniforms.ripple_1_strength, travel, band)
        + ripple(coord, size, uniforms.ripple_2_center, uniforms.ripple_2_age, uniforms.ripple_2_strength, travel, band)
        + ripple(coord, size, uniforms.ripple_3_center, uniforms.ripple_3_age, uniforms.ripple_3_strength, travel, band);
    displaced += wave.xy * (clamp(uniforms.distortion, 0.0, 18.0) * uniforms.layer_scale);
    let highlight = hover_field * 0.18 + wave.z;
    // The texel under the moved coordinate, sampled at its centre.
    let base = sample_content(floor(displaced) + vec2<f32>(0.5));
    let tint = clamp(uniforms.tint_strength, 0.0, 0.42);
    let amount = clamp(highlight * tint + hover_field * tint * 0.25, 0.0, 0.42);
    let shade = uniforms.color.rgb * base.a;
    let rgb = mix(base.rgb, shade, vec3<f32>(amount)) + shade * min(highlight * 0.06, 0.04);
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(base.a)), base.a);
}
"#;

/// One ripple of the water ripple: a ring that spreads from its centre as it
/// ages and fades out (Effector's `RippleWaveState`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RippleWave {
    /// The centre, normalized to the area (0 to 1 on each axis).
    pub center: Point,
    /// The age, 0 (just made) to 1.25 (gone); the ring has spread over
    /// `max-radius` of the area at an age of 1.
    pub age: f64,
    /// The strength, 0 to 1.
    pub strength: f64,
}

impl RippleWave {
    /// The age at which a ripple is gone.
    pub const LIFETIME: f64 = 1.25;

    /// A ripple that is not there (Effector's `RippleWaveState.Inactive`).
    pub const INACTIVE: Self = Self { center: Point::new(0.5, 0.5), age: 10.0, strength: 0.0 };

    /// A new ripple at `center` (clamped to the area) of `strength` (clamped
    /// to 0..1).
    pub fn new(center: Point, strength: f64) -> Self {
        Self { center: Point::new(unit(center.x), unit(center.y)), age: 0.0, strength: unit(strength) }
    }

    /// Whether the ripple is there.
    pub fn is_active(&self) -> bool {
        self.age >= 0.0 && self.age <= Self::LIFETIME && self.strength > 0.001
    }

    /// The ripple aged by `delta` (an active ripple only).
    pub fn advance(self, delta: f64) -> Self {
        if self.is_active() {
            Self { age: self.age + delta.max(0.0), ..self }
        } else {
            self
        }
    }
}

impl Default for RippleWave {
    fn default() -> Self {
        Self::INACTIVE
    }
}

/// The names of the parameters of the three ripples: centre, age, strength.
const RIPPLE_PARAMETERS: [[&str; 3]; 3] = [
    ["ripple-1-center", "ripple-1-age", "ripple-1-strength"],
    ["ripple-2-center", "ripple-2-age", "ripple-2-strength"],
    ["ripple-3-center", "ripple-3-age", "ripple-3-strength"],
];

/// Rings that spread over the content and bend it (Effector's
/// `WaterRippleShaderEffect`): each of the three `ripples` a ring around its
/// centre whose radius grows with its age to `max-radius` of the larger side
/// (clamped to 0.2..0.95), `ring-width` of the smaller side wide (clamped to
/// 0.02..0.18, at least 3 device independent pixels), fading as it ages; the
/// content under a ring is moved outward by up to `distortion` device
/// independent pixels (clamped to 0..18) and tinted towards the colour by
/// `tint-strength` (clamped to 0..0.42). While the pointer is over the host
/// or pressed, the content around (`pointer-x`, `pointer-y`) swells a little.
///
/// `water-ripple(distortion=12, max-radius=0.72, ring-width=0.065,
/// tint-strength=0.18, color=#7FD6FF)`; a ripple is `ripple-1-center=0.5,0.5,
/// ripple-1-age=0.3, ripple-1-strength=1`. A content-sampling shader effect
/// with a sample radius of 20; without ripples and with the pointer away
/// (the default and the neutral value) the content is unchanged.
/// [`InteractiveWaterRippleEffect`] makes ripples where the pointer is pressed,
/// dragged and released, and ages them on the frames of the animation clock.
#[derive(Clone, Debug, PartialEq)]
pub struct WaterRippleShaderEffect {
    /// The horizontal position of the pointer, 0 (left) to 1 (right)
    /// (default 0.5).
    pub pointer_x: f64,
    /// The vertical position of the pointer, 0 (top) to 1 (bottom) (default
    /// 0.5).
    pub pointer_y: f64,
    /// Whether the pointer is over the host (default `false`).
    pub is_pointer_over: bool,
    /// Whether the pointer is pressed (default `false`).
    pub is_pressed: bool,
    /// The three ripples, the newest first (default none).
    pub ripples: [RippleWave; 3],
    /// How far a ring moves the content, in device independent pixels
    /// (default 12).
    pub distortion: f64,
    /// How far a ring spreads, a fraction of the larger side (default 0.72).
    pub max_radius: f64,
    /// The width of a ring, a fraction of the smaller side (default 0.065).
    pub ring_width: f64,
    /// How far a ring tints the content towards the colour (default 0.18).
    pub tint_strength: f64,
    /// The colour of the rings (default #7FD6FF).
    pub color: Color,
}

impl Default for WaterRippleShaderEffect {
    fn default() -> Self {
        Self {
            pointer_x: 0.5,
            pointer_y: 0.5,
            is_pointer_over: false,
            is_pressed: false,
            ripples: [RippleWave::INACTIVE; 3],
            distortion: 12.0,
            max_radius: 0.72,
            ring_width: 0.065,
            tint_strength: 0.18,
            color: Color::from_rgb(0x7F, 0xD6, 0xFF),
        }
    }
}

impl WaterRippleShaderEffect {
    /// The largest distance the program moves the content: the distortion
    /// and the swell under the pointer.
    pub const SAMPLE_RADIUS: f64 = 20.0;

    /// Whether a ripple is there.
    pub fn has_active_ripples(&self) -> bool {
        self.ripples.iter().any(RippleWave::is_active)
    }

    /// A new ripple at `center` of `strength`: the others move down, the
    /// oldest is dropped (Effector's `SpawnRipple`).
    pub fn spawn_ripple(&mut self, center: Point, strength: f64) {
        self.ripples = [RippleWave::new(center, strength), self.ripples[0], self.ripples[1]];
    }
}

impl CustomEffect for WaterRippleShaderEffect {
    const NAME: &'static str = "water-ripple";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("pointer-x", self.pointer_x.into());
        f("pointer-y", self.pointer_y.into());
        f("is-pointer-over", self.is_pointer_over.into());
        f("is-pressed", self.is_pressed.into());
        for (ripple, [center, age, strength]) in self.ripples.iter().zip(RIPPLE_PARAMETERS) {
            f(center, ripple.center.into());
            f(age, ripple.age.into());
            f(strength, ripple.strength.into());
        }
        f("distortion", self.distortion.into());
        f("max-radius", self.max_radius.into());
        f("ring-width", self.ring_width.into());
        f("tint-strength", self.tint_strength.into());
        f("color", self.color.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        for (ripple, [center, age, strength]) in self.ripples.iter_mut().zip(RIPPLE_PARAMETERS) {
            if name == center {
                return value.as_point().map(|point| ripple.center = point).is_some();
            }
            if name == age {
                return value.as_float().map(|value| ripple.age = value).is_some();
            }
            if name == strength {
                return value.as_float().map(|value| ripple.strength = value).is_some();
            }
        }
        match name {
            "pointer-x" => value.as_float().map(|x| self.pointer_x = x).is_some(),
            "pointer-y" => value.as_float().map(|y| self.pointer_y = y).is_some(),
            "is-pointer-over" => value.as_bool().map(|over| self.is_pointer_over = over).is_some(),
            "is-pressed" => value.as_bool().map(|pressed| self.is_pressed = pressed).is_some(),
            "distortion" => value.as_float().map(|distortion| self.distortion = distortion).is_some(),
            "max-radius" => value.as_float().map(|radius| self.max_radius = radius).is_some(),
            "ring-width" => value.as_float().map(|width| self.ring_width = width).is_some(),
            "tint-strength" => value.as_float().map(|strength| self.tint_strength = strength).is_some(),
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self::default())
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(WATER_RIPPLE_SKSL, WATER_RIPPLE_WGSL)
                .with_sample_radius(Self::SAMPLE_RADIUS),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        let scale = layer_scale(scale);
        let size = [(width as f32).max(1.0), (height as f32).max(1.0)];
        let travel = size[0].max(size[1]) * clamp(self.max_radius, 0.2, 0.95) as f32;
        let band = (size[0].min(size[1]) * clamp(self.ring_width, 0.02, 0.18) as f32).max(3.0 * scale);
        let hover = flag(self.is_pointer_over || self.is_pressed);
        let pointer = [unit(self.pointer_x) as f32 * size[0], unit(self.pointer_y) as f32 * size[1]];
        let reach = (size[0].min(size[1]) * 0.34).max(1.0);
        let distortion = clamp(self.distortion, 0.0, 18.0) as f32 * scale;
        let tint = clamp(self.tint_strength, 0.0, 0.42) as f32;
        let color = premultiplied(self.color);
        let ripples = self.ripples.map(|ripple| {
            let active = !(ripple.age < 0.0 || ripple.age > RippleWave::LIFETIME || ripple.strength <= 0.001);
            let center = [unit(ripple.center.x) as f32 * size[0], unit(ripple.center.y) as f32 * size[1]];
            (active, center, ripple.age as f32, ripple.strength as f32)
        });
        let source = area(pixels, width, height).to_vec();
        for_each_pixel(pixels, width, height, |pixel, coord| {
            let hover_delta = [coord[0] - pointer[0], coord[1] - pointer[1]];
            let hover_dist = (hover_delta[0] * hover_delta[0] + hover_delta[1] * hover_delta[1]).sqrt();
            let hover_field = hover * (1.0 - hover_dist / reach).max(0.0);
            let mut displaced = coord;
            if hover_dist > 1.0 {
                let push = hover_field * 1.25 * scale;
                displaced = [displaced[0] + hover_delta[0] / hover_dist * push, displaced[1] + hover_delta[1] / hover_dist * push];
            }
            let mut wave = [0.0f32; 3];
            for &(active, center, age, strength) in &ripples {
                if !active {
                    continue;
                }
                let delta = [coord[0] - center[0], coord[1] - center[1]];
                let dist = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
                let x = (dist - travel * age) / band;
                let w = (-x * x).exp() * (1.0 - age / 1.25).max(0.0) * strength.min(1.0);
                if dist > 1.0 {
                    wave[0] += delta[0] / dist * w;
                    wave[1] += delta[1] / dist * w;
                }
                wave[2] += w;
            }
            displaced = [displaced[0] + wave[0] * distortion, displaced[1] + wave[1] * distortion];
            let highlight = hover_field * 0.18 + wave[2];
            let base = texel(&source, width, height, displaced);
            let amount = (highlight * tint + hover_field * tint * 0.25).clamp(0.0, 0.42);
            let glint = (highlight * 0.06).min(0.04);
            store(pixel, std::array::from_fn(|channel| {
                if channel == 3 {
                    return base[3];
                }
                let shade = color[channel] * base[3];
                (mix(base[channel], shade, amount) + shade * glint).clamp(0.0, base[3])
            }));
        });
        true
    }
}

/// The water ripple as an interactive effect class (Effector's
/// `WaterRippleShaderEffect`): a press makes a strong ripple where it is and
/// captures the pointer, a drag makes weaker ones along the way (at least
/// about a tenth of the host apart), the release one more; the ripples age on
/// the frames of the animation clock until they are gone
/// ([`advance`](Self::advance)). Its value is a [`WaterRippleShaderEffect`].
#[repr(C)]
pub struct InteractiveWaterRippleEffect {
    base: Effect,
    value: RefCell<WaterRippleShaderEffect>,
    /// Where the drag made its last ripple, while pressed.
    last_drag_ripple: Cell<Option<Point>>,
    frames: FrameLoop,
}

ferro_class!(InteractiveWaterRippleEffect: Effect);
ferro_impl_classes!(InteractiveWaterRippleEffect: FerroObjectImpl);

interactive_effect_class!(InteractiveWaterRippleEffect, WaterRippleShaderEffect);

impl InteractiveWaterRippleEffect {
    /// The squared normalized distance a drag moves before it makes another
    /// ripple.
    const DRAG_SPACING: f64 = 0.008;

    /// The age a ripple gains per second.
    const AGE_PER_SECOND: f64 = 1.0 / 1.15;

    fn construct(value: WaterRippleShaderEffect) -> Self {
        Self {
            base: Effect::construct_interactive::<Self>(),
            value: RefCell::new(value),
            last_drag_ripple: Cell::new(None),
            frames: FrameLoop::default(),
        }
    }

    /// Ages the ripples by `seconds` (what each frame of the animation clock
    /// does); whether the effect still moves (a ripple is there or the
    /// pointer is pressed).
    pub fn advance(&self, seconds: f64) -> bool {
        let step = seconds.max(0.0) * Self::AGE_PER_SECOND;
        self.update(|value| value.ripples = value.ripples.map(|ripple| ripple.advance(step)));
        let value = self.value.borrow();
        value.is_pressed || value.has_active_ripples()
    }

    /// Whether the frames of the animation clock run.
    pub fn is_animating(&self) -> bool {
        self.frames.is_running()
    }

    /// The normalized position of the pointer, stored in the value with
    /// whether it is within the host.
    fn track(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) -> Point {
        let position = context.get_normalized_position(e, true);
        let over = is_within_host(context, e);
        self.update(|value| {
            value.pointer_x = position.x;
            value.pointer_y = position.y;
            value.is_pointer_over = over;
        });
        position
    }
}

impl Animated for InteractiveWaterRippleEffect {
    fn frames(&self) -> &FrameLoop {
        &self.frames
    }

    fn tick(&self, seconds: f64) -> bool {
        self.advance(seconds)
    }
}

impl IEffectInputHandler for InteractiveWaterRippleEffect {
    fn on_attached(&self, _context: &EffectHostContext<'_>) {
        self.frames.stop();
        self.last_drag_ripple.set(None);
        self.update(|value| {
            value.pointer_x = 0.5;
            value.pointer_y = 0.5;
            value.is_pointer_over = false;
            value.is_pressed = false;
            value.ripples = [RippleWave::INACTIVE; 3];
        });
    }

    fn on_detached(&self, _context: &EffectHostContext<'_>) {
        self.frames.stop();
        self.last_drag_ripple.set(None);
    }

    fn on_pointer_entered(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
        self.track(context, e);
    }

    fn on_pointer_exited(&self, _context: &EffectHostContext<'_>, _e: &PointerEventArgs) {
        self.update(|value| {
            if !value.is_pressed {
                value.is_pointer_over = false;
            }
        });
    }

    fn on_pointer_moved(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
        let position = self.track(context, e);
        if !self.value.borrow().is_pressed {
            return;
        }
        let far_enough = self.last_drag_ripple.get().is_none_or(|last| {
            let (dx, dy) = (position.x - last.x, position.y - last.y);
            dx * dx + dy * dy >= Self::DRAG_SPACING
        });
        if far_enough {
            self.update(|value| value.spawn_ripple(position, 0.45));
            self.last_drag_ripple.set(Some(position));
            FrameLoop::start(self, context);
        }
    }

    fn on_pointer_pressed(&self, context: &EffectHostContext<'_>, e: &PointerPressedEventArgs) {
        let position = self.track(context, e);
        self.update(|value| {
            value.is_pressed = true;
            value.spawn_ripple(position, 1.0);
        });
        self.last_drag_ripple.set(Some(position));
        context.capture_pointer(e);
        FrameLoop::start(self, context);
    }

    fn on_pointer_released(&self, context: &EffectHostContext<'_>, e: &PointerReleasedEventArgs) {
        let position = self.track(context, e);
        self.update(|value| {
            value.is_pressed = false;
            value.spawn_ripple(position, 0.55);
        });
        self.last_drag_ripple.set(None);
        context.release_pointer_capture(e);
        FrameLoop::start(self, context);
    }

    fn on_pointer_capture_lost(&self, _context: &EffectHostContext<'_>, _e: &PointerCaptureLostEventArgs) {
        self.update(|value| value.is_pressed = false);
        self.last_drag_ripple.set(None);
    }
}

// --- burning flame -------------------------------------------------------------------

const BURNING_FLAME_SKSL: &str = r#"
uniform float2 layer_size;
uniform float layer_scale;
uniform float ignition_x;
uniform float ignition_y;
uniform float burn_amount;
uniform float flame_phase;
uniform int is_pressed;
uniform float flame_height;
uniform float distortion;
uniform float glow_strength;
uniform float smoke_strength;
uniform float4 core_color;
uniform float4 ember_color;
uniform shader content;

half4 main(float2 coord) {
    float pressed = is_pressed != 0 ? 1.0 : 0.0;
    float engaged = clamp(clamp(burn_amount, 0.0, 1.0) + pressed * 0.18, 0.0, 1.0);
    if (engaged <= 0.0) {
        return content.eval(coord);
    }
    float2 uv = coord / max(layer_size, float2(1.0));
    float ix = clamp(ignition_x, 0.0, 1.0);
    float iy = clamp(ignition_y, 0.0, 1.0);
    float upward = 1.0 - uv.y;
    float gx = (uv.x - ix) * 5.5;
    float gy = (uv.y - iy) * 8.0;
    float ignite = exp(-gx * gx - gy * gy);
    float wave_a = sin(uv.x * 15.0 + flame_phase * 5.8 - uv.y * 12.0);
    float wave_b = sin(uv.x * 31.0 - flame_phase * 7.4 + uv.y * 17.0);
    float wave_c = sin((uv.x - ix) * 26.0 + flame_phase * 9.6);
    float turbulence = (wave_a * 0.5 + wave_b * 0.35 + wave_c * 0.25) * 0.5 + 0.5;
    float t = clamp(upward / clamp(flame_height, 0.18, 0.95), 0.0, 1.0);
    float height_mask = t * t * (3.0 - 2.0 * t);
    float plume_width = mix(0.12, 0.34, engaged);
    float drift = (wave_a * 0.025 + wave_b * 0.018) * upward;
    float plume = max(0.0, 1.0 - abs(uv.x - ix - drift) / max(plume_width * (0.28 + upward), 0.02));
    float flame = clamp(engaged * max(ignite * 0.95, height_mask * plume * turbulence), 0.0, 1.0);
    float smoke = clamp(smoke_strength, 0.0, 0.8) * engaged * upward * upward * plume * (0.4 + 0.6 * (wave_b * 0.5 + 0.5));
    float amount = clamp(distortion, 0.0, 18.0) * layer_scale;
    float2 displaced = coord + float2((wave_a + wave_b) * 0.5 * amount * flame,
        -flame * (2.5 + 5.0 * engaged) * (0.25 + upward) * layer_scale);
    // The texel under the moved coordinate, sampled at its centre.
    float4 base = float4(content.eval(floor(displaced) + 0.5));
    float hot = clamp(flame * 1.75 - upward * 0.55, 0.0, 1.0);
    float3 fire = mix(ember_color.rgb, core_color.rgb, hot) * base.a;
    float3 rgb = mix(base.rgb, fire, clamp(flame * 0.58, 0.0, 0.82));
    rgb += fire * (flame * clamp(glow_strength, 0.0, 1.0) * 0.42);
    rgb = mix(rgb, float3(0.13, 0.08, 0.05) * base.a, clamp(smoke * 0.28, 0.0, 0.24));
    return half4(float4(clamp(rgb, float3(0.0), float3(base.a)), base.a));
}
"#;

const BURNING_FLAME_WGSL: &str = r#"
struct Uniforms {
    layer_size: vec2<f32>,
    layer_scale: f32,
    ignition_x: f32,
    ignition_y: f32,
    burn_amount: f32,
    flame_phase: f32,
    is_pressed: i32,
    flame_height: f32,
    distortion: f32,
    glow_strength: f32,
    smoke_strength: f32,
    core_color: vec4<f32>,
    ember_color: vec4<f32>,
}

fn effect(coord: vec2<f32>) -> vec4<f32> {
    let pressed = select(0.0, 1.0, uniforms.is_pressed != 0);
    let engaged = clamp(clamp(uniforms.burn_amount, 0.0, 1.0) + pressed * 0.18, 0.0, 1.0);
    if (engaged <= 0.0) {
        return sample_content(coord);
    }
    let uv = coord / max(uniforms.layer_size, vec2<f32>(1.0));
    let ix = clamp(uniforms.ignition_x, 0.0, 1.0);
    let iy = clamp(uniforms.ignition_y, 0.0, 1.0);
    let upward = 1.0 - uv.y;
    let gx = (uv.x - ix) * 5.5;
    let gy = (uv.y - iy) * 8.0;
    let ignite = exp(-gx * gx - gy * gy);
    let phase = uniforms.flame_phase;
    let wave_a = sin(uv.x * 15.0 + phase * 5.8 - uv.y * 12.0);
    let wave_b = sin(uv.x * 31.0 - phase * 7.4 + uv.y * 17.0);
    let wave_c = sin((uv.x - ix) * 26.0 + phase * 9.6);
    let turbulence = (wave_a * 0.5 + wave_b * 0.35 + wave_c * 0.25) * 0.5 + 0.5;
    let t = clamp(upward / clamp(uniforms.flame_height, 0.18, 0.95), 0.0, 1.0);
    let height_mask = t * t * (3.0 - 2.0 * t);
    let plume_width = mix(0.12, 0.34, engaged);
    let drift = (wave_a * 0.025 + wave_b * 0.018) * upward;
    let plume = max(0.0, 1.0 - abs(uv.x - ix - drift) / max(plume_width * (0.28 + upward), 0.02));
    let flame = clamp(engaged * max(ignite * 0.95, height_mask * plume * turbulence), 0.0, 1.0);
    let smoke = clamp(uniforms.smoke_strength, 0.0, 0.8) * engaged * upward * upward * plume * (0.4 + 0.6 * (wave_b * 0.5 + 0.5));
    let amount = clamp(uniforms.distortion, 0.0, 18.0) * uniforms.layer_scale;
    let displaced = coord + vec2<f32>((wave_a + wave_b) * 0.5 * amount * flame,
        -flame * (2.5 + 5.0 * engaged) * (0.25 + upward) * uniforms.layer_scale);
    // The texel under the moved coordinate, sampled at its centre.
    let base = sample_content(floor(displaced) + vec2<f32>(0.5));
    let hot = clamp(flame * 1.75 - upward * 0.55, 0.0, 1.0);
    let fire = mix(uniforms.ember_color.rgb, uniforms.core_color.rgb, vec3<f32>(hot)) * base.a;
    var rgb = mix(base.rgb, fire, vec3<f32>(clamp(flame * 0.58, 0.0, 0.82)));
    rgb += fire * (flame * clamp(uniforms.glow_strength, 0.0, 1.0) * 0.42);
    rgb = mix(rgb, vec3<f32>(0.13, 0.08, 0.05) * base.a, vec3<f32>(clamp(smoke * 0.28, 0.0, 0.24)));
    return vec4<f32>(clamp(rgb, vec3<f32>(0.0), vec3<f32>(base.a)), base.a);
}
"#;

/// A flame that burns the content (Effector's `BurningFlameShaderEffect`,
/// the burn-away button): from the ignition point (`ignition-x`,
/// `ignition-y`, normalized, clamped to 0..1) a plume rises to `flame-height`
/// of the area (clamped to 0.18..0.95), its waves moved by `flame-phase`
/// (radians, a period of 10 pi), as strong as `burn-amount` (0 to 1) plus a
/// little while pressed; the content under it is moved by up to `distortion`
/// device independent pixels (clamped to 0..18), coloured from `ember-color`
/// to `core-color` where it is hot, lit by `glow-strength` (0..1) and darkened
/// by smoke (`smoke-strength`, 0..0.8).
///
/// `burning-flame(burn-amount=0.8, flame-phase=1.2, ignition-x=0.5,
/// ignition-y=0.68)`. A content-sampling shader effect with a sample radius
/// of 20; without burning and not pressed (the default and the neutral
/// value) the content is unchanged. [`InteractiveBurningFlameEffect`] ignites
/// it where the pointer is pressed and lets it burn down after the release.
#[derive(Clone, Debug, PartialEq)]
pub struct BurningFlameShaderEffect {
    /// The horizontal position of the ignition, 0 (left) to 1 (right)
    /// (default 0.5).
    pub ignition_x: f64,
    /// The vertical position of the ignition, 0 (top) to 1 (bottom)
    /// (default 0.68).
    pub ignition_y: f64,
    /// How strongly it burns, 0 to 1 (default 0).
    pub burn_amount: f64,
    /// The phase of the waves of the flame, in radians (default 0).
    pub flame_phase: f64,
    /// Whether the pointer is pressed (default `false`).
    pub is_pressed: bool,
    /// How far up the flame reaches, a fraction of the height (default 0.72).
    pub flame_height: f64,
    /// How far the flame moves the content, in device independent pixels
    /// (default 8).
    pub distortion: f64,
    /// How brightly the flame lights the content, 0 to 1 (default 0.58).
    pub glow_strength: f64,
    /// How much smoke darkens the content, 0 to 0.8 (default 0.24).
    pub smoke_strength: f64,
    /// The colour of the hot core (default #FFD36B).
    pub core_color: Color,
    /// The colour of the embers (default #FF5B1F).
    pub ember_color: Color,
}

impl Default for BurningFlameShaderEffect {
    fn default() -> Self {
        Self {
            ignition_x: 0.5,
            ignition_y: 0.68,
            burn_amount: 0.0,
            flame_phase: 0.0,
            is_pressed: false,
            flame_height: 0.72,
            distortion: 8.0,
            glow_strength: 0.58,
            smoke_strength: 0.24,
            core_color: Color::from_rgb(0xFF, 0xD3, 0x6B),
            ember_color: Color::from_rgb(0xFF, 0x5B, 0x1F),
        }
    }
}

impl BurningFlameShaderEffect {
    /// The largest distance the program moves the content.
    pub const SAMPLE_RADIUS: f64 = 20.0;

    /// The period of the phase: 10 pi, a whole number of periods of each
    /// wave of the flame (5.8, 7.4 and 9.6 are 29, 37 and 48 fifths).
    pub const PHASE_PERIOD: f64 = 10.0 * std::f64::consts::PI;
}

impl CustomEffect for BurningFlameShaderEffect {
    const NAME: &'static str = "burning-flame";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("ignition-x", self.ignition_x.into());
        f("ignition-y", self.ignition_y.into());
        f("burn-amount", self.burn_amount.into());
        f("flame-phase", self.flame_phase.into());
        f("is-pressed", self.is_pressed.into());
        f("flame-height", self.flame_height.into());
        f("distortion", self.distortion.into());
        f("glow-strength", self.glow_strength.into());
        f("smoke-strength", self.smoke_strength.into());
        f("core-color", self.core_color.into());
        f("ember-color", self.ember_color.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "ignition-x" => value.as_float().map(|x| self.ignition_x = x).is_some(),
            "ignition-y" => value.as_float().map(|y| self.ignition_y = y).is_some(),
            "burn-amount" => value.as_float().map(|amount| self.burn_amount = amount).is_some(),
            "flame-phase" => value.as_float().map(|phase| self.flame_phase = phase).is_some(),
            "is-pressed" => value.as_bool().map(|pressed| self.is_pressed = pressed).is_some(),
            "flame-height" => value.as_float().map(|height| self.flame_height = height).is_some(),
            "distortion" => value.as_float().map(|distortion| self.distortion = distortion).is_some(),
            "glow-strength" => value.as_float().map(|strength| self.glow_strength = strength).is_some(),
            "smoke-strength" => value.as_float().map(|strength| self.smoke_strength = strength).is_some(),
            "core-color" => value.as_color().map(|color| self.core_color = color).is_some(),
            "ember-color" => value.as_color().map(|color| self.ember_color = color).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { burn_amount: 0.0, is_pressed: false, ..Self::default() })
    }

    fn shader() -> Option<ShaderEffectDescription> {
        Some(
            ShaderEffectDescription::content_sampling(BURNING_FLAME_SKSL, BURNING_FLAME_WGSL)
                .with_sample_radius(Self::SAMPLE_RADIUS),
        )
    }

    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        let scale = layer_scale(scale);
        let pressed = flag(self.is_pressed);
        let active = (unit(self.burn_amount) as f32 + pressed * 0.18).clamp(0.0, 1.0);
        if active <= 0.0 {
            return true;
        }
        let size = [(width as f32).max(1.0), (height as f32).max(1.0)];
        let (ix, iy) = (unit(self.ignition_x) as f32, unit(self.ignition_y) as f32);
        let phase = self.flame_phase as f32;
        let flame_height = clamp(self.flame_height, 0.18, 0.95) as f32;
        let plume_width = mix(0.12, 0.34, active);
        let smoke_strength = clamp(self.smoke_strength, 0.0, 0.8) as f32;
        let glow = unit(self.glow_strength) as f32;
        let amount = clamp(self.distortion, 0.0, 18.0) as f32 * scale;
        let (core, ember) = (premultiplied(self.core_color), premultiplied(self.ember_color));
        let smoke_color = [0.13f32, 0.08, 0.05];
        let source = area(pixels, width, height).to_vec();
        for_each_pixel(pixels, width, height, |pixel, coord| {
            let uv = [coord[0] / size[0], coord[1] / size[1]];
            let upward = 1.0 - uv[1];
            let gx = (uv[0] - ix) * 5.5;
            let gy = (uv[1] - iy) * 8.0;
            let ignite = (-gx * gx - gy * gy).exp();
            let wave_a = (uv[0] * 15.0 + phase * 5.8 - uv[1] * 12.0).sin();
            let wave_b = (uv[0] * 31.0 - phase * 7.4 + uv[1] * 17.0).sin();
            let wave_c = ((uv[0] - ix) * 26.0 + phase * 9.6).sin();
            let turbulence = (wave_a * 0.5 + wave_b * 0.35 + wave_c * 0.25) * 0.5 + 0.5;
            let t = (upward / flame_height).clamp(0.0, 1.0);
            let height_mask = t * t * (3.0 - 2.0 * t);
            let drift = (wave_a * 0.025 + wave_b * 0.018) * upward;
            let plume = (1.0 - (uv[0] - ix - drift).abs() / (plume_width * (0.28 + upward)).max(0.02)).max(0.0);
            let flame = (active * (ignite * 0.95).max(height_mask * plume * turbulence)).clamp(0.0, 1.0);
            let smoke = smoke_strength * active * upward * upward * plume * (0.4 + 0.6 * (wave_b * 0.5 + 0.5));
            let displaced = [
                coord[0] + (wave_a + wave_b) * 0.5 * amount * flame,
                coord[1] - flame * (2.5 + 5.0 * active) * (0.25 + upward) * scale,
            ];
            let base = texel(&source, width, height, displaced);
            let hot = (flame * 1.75 - upward * 0.55).clamp(0.0, 1.0);
            let lit = (flame * 0.58).clamp(0.0, 0.82);
            let shine = flame * glow * 0.42;
            let dark = (smoke * 0.28).clamp(0.0, 0.24);
            store(pixel, std::array::from_fn(|channel| {
                if channel == 3 {
                    return base[3];
                }
                let fire = mix(ember[channel], core[channel], hot) * base[3];
                let rgb = mix(base[channel], fire, lit) + fire * shine;
                mix(rgb, smoke_color[channel] * base[3], dark).clamp(0.0, base[3])
            }));
        });
        true
    }
}

/// The burning flame as an interactive effect class (Effector's
/// `BurningFlameShaderEffect`, the burn-away button): a press ignites the
/// flame where it is (the height clamped to 0.18..0.92 of the host) at full
/// strength, a drag while pressed moves the ignition, and after the release
/// the flame burns down over 2.6 seconds; the phase of its waves and its
/// strength advance on the frames of the animation clock
/// ([`advance`](Self::advance)). It takes handled events: the host of the
/// burn-away button is a button, whose own handler handles the press. Its
/// value is a [`BurningFlameShaderEffect`].
#[repr(C)]
pub struct InteractiveBurningFlameEffect {
    base: Effect,
    value: RefCell<BurningFlameShaderEffect>,
    frames: FrameLoop,
}

ferro_class!(InteractiveBurningFlameEffect: Effect);
ferro_impl_classes!(InteractiveBurningFlameEffect: FerroObjectImpl);

interactive_effect_class!(InteractiveBurningFlameEffect, BurningFlameShaderEffect);

impl InteractiveBurningFlameEffect {
    /// The phase the waves gain per second.
    const PHASE_PER_SECOND: f64 = 2.8;

    /// The strength the flame gains per second while pressed.
    const BURN_PER_SECOND: f64 = 0.8;

    /// The seconds the flame takes to burn down after the release.
    const BURN_DOWN_SECONDS: f64 = 2.6;

    fn construct(value: BurningFlameShaderEffect) -> Self {
        Self { base: Effect::construct_interactive::<Self>(), value: RefCell::new(value), frames: FrameLoop::default() }
    }

    /// Advances the flame by `seconds` (what each frame of the animation
    /// clock does): its phase, and its strength up while pressed and down
    /// after; whether it still burns.
    pub fn advance(&self, seconds: f64) -> bool {
        let seconds = seconds.max(0.0);
        let mut burning = true;
        self.update(|value| {
            value.flame_phase = (value.flame_phase + seconds * Self::PHASE_PER_SECOND)
                .rem_euclid(BurningFlameShaderEffect::PHASE_PERIOD);
            if value.is_pressed {
                value.burn_amount = (value.burn_amount + seconds * Self::BURN_PER_SECOND).min(1.0);
            } else {
                value.burn_amount = (value.burn_amount - seconds / Self::BURN_DOWN_SECONDS).max(0.0);
                if value.burn_amount <= 0.001 {
                    value.burn_amount = 0.0;
                    burning = false;
                }
            }
        });
        burning
    }

    /// Whether the frames of the animation clock run.
    pub fn is_animating(&self) -> bool {
        self.frames.is_running()
    }

    /// Moves the ignition to the pointer.
    fn ignite_at(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
        let position = context.get_normalized_position(e, true);
        self.update(|value| {
            value.ignition_x = position.x;
            value.ignition_y = position.y.clamp(0.18, 0.92);
        });
    }
}

impl Animated for InteractiveBurningFlameEffect {
    fn frames(&self) -> &FrameLoop {
        &self.frames
    }

    fn tick(&self, seconds: f64) -> bool {
        self.advance(seconds)
    }
}

impl IEffectInputHandler for InteractiveBurningFlameEffect {
    fn handled_events_too(&self) -> bool {
        true
    }

    fn on_attached(&self, _context: &EffectHostContext<'_>) {
        self.frames.stop();
        self.update(|value| {
            value.ignition_x = 0.5;
            value.ignition_y = 0.68;
            value.burn_amount = 0.0;
            value.flame_phase = 0.0;
            value.is_pressed = false;
        });
    }

    fn on_detached(&self, _context: &EffectHostContext<'_>) {
        self.frames.stop();
    }

    fn on_pointer_moved(&self, context: &EffectHostContext<'_>, e: &PointerEventArgs) {
        if self.value.borrow().is_pressed {
            self.ignite_at(context, e);
        }
    }

    fn on_pointer_pressed(&self, context: &EffectHostContext<'_>, e: &PointerPressedEventArgs) {
        self.ignite_at(context, e);
        self.update(|value| {
            value.is_pressed = true;
            value.burn_amount = 1.0;
        });
        FrameLoop::start(self, context);
    }

    fn on_pointer_released(&self, context: &EffectHostContext<'_>, e: &PointerReleasedEventArgs) {
        self.ignite_at(context, e);
        self.update(|value| value.is_pressed = false);
        FrameLoop::start(self, context);
    }

    fn on_pointer_capture_lost(&self, _context: &EffectHostContext<'_>, _e: &PointerCaptureLostEventArgs) {
        self.update(|value| value.is_pressed = false);
    }
}
