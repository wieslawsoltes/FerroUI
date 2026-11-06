//! The scenes of the custom effects FerroUI ships (#455, E9): every effect
//! of the library (`ferroui_base::media::effects::library`) around the same
//! content, drawn through the contracts only, as the render walk draws it (a
//! graph effect as its graph with `push_custom_effect_graph`, a runtime
//! shader as an effect of the drawing context), at 96 and 192 DPI.
//!
//! Each effect has a CPU reference (decision 3 of custom-effects.md): the
//! tests apply it to the content drawn plain by the same backend and mode,
//! and hold each backend and mode to it with the tolerance of the effect;
//! then they compare the backends with each other, Skia raster against each
//! mode of the Vello backend.

use crate::{read_pixels, Backend, Pixels};
use ferroui_base::media::effects::library::{
    BrightnessContrastEffect, BurningFlameShaderEffect, DissolveTransitionEffect, EdgeDetectEffect, GlowEffect,
    GrayscaleEffect, InvertEffect, MaskTransitionEffect, PixelateEffect, PointerSpotlightShaderEffect,
    ReactiveGridShaderEffect, SaturationEffect, ScanlineShaderEffect, SepiaEffect, SharpenEffect,
    SpotlightShaderEffect, TintEffect, WaterRippleShaderEffect,
};
use ferroui_base::media::effects::{EffectImage, ImmutableCustomEffect};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Color, Colors, EdgeMode, RenderOptions};
use ferroui_base::platform::{AlphaFormat, IDrawingContextImpl, PixelFormat};
use ferroui_base::rendering::composition::effects::{pop_custom_effect_graph, push_custom_effect_graph};
use ferroui_base::{PixelSize, Point, Rect, RoundedRect, Vector};

/// The logical size of every scene.
pub const SIDE: i32 = 100;

/// How a backend is held to the CPU reference of an effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tolerance {
    /// No channel further than this many levels, and a mean difference
    /// within half a level.
    Largest(u8),
    /// A program that moves the content (the water ripple, the flame): it
    /// picks the texel under a coordinate it computes, and where that falls
    /// on the edge of a texel at the edge of a square the last bit of a
    /// `sin` or an `exp` of the processor picks the neighbour. At most
    /// [`MOVED_SHARE`] of the pixels further than [`MOVED_LEVELS`] levels, a
    /// mean within [`MOVED_MEAN`].
    Moves,
}

/// The share of the pixels of a program that moves the content that may be
/// outliers.
pub const MOVED_SHARE: f64 = 0.01;
/// The levels beyond which a pixel of such a program is an outlier.
pub const MOVED_LEVELS: u8 = 3;
/// The mean difference such a program may have.
pub const MOVED_MEAN: f64 = 1.5;

impl Tolerance {
    /// The mean difference the tolerance allows.
    pub fn mean(self) -> f64 {
        match self {
            Tolerance::Largest(_) => 0.5,
            Tolerance::Moves => MOVED_MEAN,
        }
    }

    /// The largest difference of a channel the tolerance allows in the
    /// pixels that are not outliers.
    pub fn largest(self) -> u8 {
        match self {
            Tolerance::Largest(largest) => largest,
            Tolerance::Moves => MOVED_LEVELS,
        }
    }

    /// The share of the pixels that may be further than [`largest`](Self::largest).
    pub fn outliers(self) -> f64 {
        match self {
            Tolerance::Largest(_) => 0.0,
            Tolerance::Moves => MOVED_SHARE,
        }
    }
}

/// An effect of the library with the tolerance it is held to: the largest
/// of the tolerances its tests in `ferroui-skia` and `ferroui-vello` hold it
/// to in any backend and mode.
pub struct CustomEffectScene {
    pub name: &'static str,
    /// The effect, made with images of the backend (the transitions sample
    /// two pages and a pattern).
    pub effect: fn(&Backend) -> ImmutableCustomEffect,
    pub tolerance: Tolerance,
}

/// Opaque red and steel blue squares and a translucent orange band, with
/// aliased edges on whole pixels: the scene of the tests of both backends.
pub fn content(context: &mut dyn IDrawingContextImpl) {
    let solid = |color: Color| ImmutableSolidColorBrush::new(color);
    let square = |x: f64, y: f64, w: f64, h: f64| RoundedRect::from_rect(Rect::new(x, y, w, h));
    context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
    context.draw_rectangle(Some(&solid(Colors::RED)), None, square(30.0, 30.0, 20.0, 20.0), &BoxShadows::default());
    let steel_blue = Color::from_rgb(48, 128, 192);
    context.draw_rectangle(Some(&solid(steel_blue)), None, square(52.0, 30.0, 20.0, 20.0), &BoxShadows::default());
    let orange = Color::from_argb(128, 255, 128, 0);
    context.draw_rectangle(Some(&solid(orange)), None, square(30.0, 52.0, 42.0, 18.0), &BoxShadows::default());
    context.pop_render_options();
}

/// Draws `effect` around [`content`] over the whole scene as the render walk
/// draws it: a graph effect as its graph, any other as an effect of the
/// drawing context.
pub fn draw_with_effect(context: &mut dyn IDrawingContextImpl, effect: &ImmutableCustomEffect) {
    let bounds = Rect::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE));
    if effect.has_graph() {
        assert!(push_custom_effect_graph(context, effect, bounds), "{effect}");
        content(context);
        assert!(pop_custom_effect_graph(context, effect, bounds), "{effect}");
    } else {
        let effects = context.as_drawing_context_impl_with_effects().expect("a drawing context with effects");
        effects.push_effect(Some(bounds), effect);
        content(effects);
        effects.pop_effect();
    }
}

/// Draws `scene` with `backend` into a render target bitmap of [`SIDE`]
/// logical pixels at `scale` device pixels each (`96 * scale` DPI) and
/// reads the pixels back.
pub fn render_at(backend: &Backend, scale: i32, scene: &dyn Fn(&mut dyn IDrawingContextImpl)) -> Pixels {
    let size = PixelSize::new(SIDE * scale, SIDE * scale);
    let dpi = Vector::new(96.0 * f64::from(scale), 96.0 * f64::from(scale));
    let bitmap = backend.interface.create_render_target_bitmap(size, dpi);
    let mut context = bitmap.create_drawing_context();
    scene(&mut *context);
    context.dispose();
    let pixels = read_pixels(&*bitmap);
    bitmap.dispose();
    pixels
}

/// An image of [`SIDE`] by [`SIDE`] premultiplied RGBA pixels made by
/// `pixel`: a bitmap of `backend`, owned by the handle.
fn image(backend: &Backend, pixel: impl Fn(i32, i32) -> [u8; 4]) -> EffectImage {
    let mut data = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            data.extend_from_slice(&pixel(x, y));
        }
    }
    let bitmap = backend.interface.load_bitmap_from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
        &data,
        PixelSize::new(SIDE, SIDE),
        Vector::new(96.0, 96.0),
        SIDE * 4,
    );
    EffectImage::from_impl(bitmap)
}

fn from_page(backend: &Backend) -> EffectImage {
    image(backend, |x, y| [(x * 2) as u8, (y * 2) as u8, 200, 255])
}

fn to_page(backend: &Backend) -> EffectImage {
    image(backend, |x, y| [40, (255 - x * 2) as u8, (y * 2) as u8, 255])
}

fn pattern(backend: &Backend) -> EffectImage {
    image(backend, |x, y| {
        let level = ((x * 37 + y * 91 + (x * y) % 17) % 256) as u8;
        [level, level, level, 255]
    })
}

/// A scene of `effect`, held to `tolerance`.
fn scene(name: &'static str, tolerance: Tolerance, effect: fn(&Backend) -> ImmutableCustomEffect) -> CustomEffectScene {
    CustomEffectScene { name, effect, tolerance }
}

/// The effects of the library at values that draw something.
pub fn scenes() -> Vec<CustomEffectScene> {
    use ferroui_base::media::effects::ImmutableCustomEffect as Effect;
    vec![
        // Graph effects (E2, E3).
        scene("tint", Tolerance::Largest(2), |_| Effect::new(TintEffect { color: Colors::TEAL, strength: 0.6 })),
        scene("grayscale", Tolerance::Largest(2), |_| Effect::new(GrayscaleEffect { amount: 0.8 })),
        scene("sepia", Tolerance::Largest(2), |_| Effect::new(SepiaEffect { amount: 0.9 })),
        scene("saturation", Tolerance::Largest(2), |_| Effect::new(SaturationEffect { saturation: 1.7 })),
        scene("brightness-contrast", Tolerance::Largest(2), |_| {
            Effect::new(BrightnessContrastEffect { brightness: 0.1, contrast: 1.4 })
        }),
        scene("invert", Tolerance::Largest(2), |_| Effect::new(InvertEffect { amount: 1.0 })),
        scene("glow", Tolerance::Largest(12), |_| {
            Effect::new(GlowEffect { color: Colors::BLUE, blur_radius: 4.0, intensity: 0.8 })
        }),
        scene("sharpen", Tolerance::Largest(2), |_| Effect::new(SharpenEffect { strength: 0.8 })),
        scene("edge-detect", Tolerance::Largest(2), |_| Effect::new(EdgeDetectEffect { strength: 1.0 })),
        // Runtime shaders (E4, E5).
        scene("pixelate", Tolerance::Largest(2), |_| Effect::new(PixelateEffect { cell_size: 6.0 })),
        scene("scanline", Tolerance::Largest(2), |_| Effect::new(ScanlineShaderEffect { spacing: 4.0, strength: 0.5 })),
        scene("spotlight", Tolerance::Largest(3), |_| {
            Effect::new(SpotlightShaderEffect { center_x: 0.4, center_y: 0.5, radius: 0.5, strength: 0.8, color: Colors::ORANGE })
        }),
        // Page transitions with images (E6).
        scene("dissolve-transition", Tolerance::Largest(3), |backend| {
            Effect::new(DissolveTransitionEffect {
                from_page: from_page(backend),
                to_page: to_page(backend),
                pattern: pattern(backend),
                progress: 0.45,
                center_x: 0.3,
                center_y: 0.6,
                ..DissolveTransitionEffect::default()
            })
        }),
        scene("mask-transition", Tolerance::Largest(3), |backend| {
            Effect::new(MaskTransitionEffect { to_page: to_page(backend), mask: pattern(backend), progress: 0.45, softness: 0.2 })
        }),
        // Interactive effects (E7), at a frame of an interaction.
        scene("pointer-spotlight", Tolerance::Largest(3), |_| {
            Effect::new(PointerSpotlightShaderEffect {
                pointer_x: 0.45,
                is_pointer_over: true,
                is_pressed: true,
                radius: 0.4,
                ..PointerSpotlightShaderEffect::default()
            })
        }),
        scene("reactive-grid", Tolerance::Largest(3), |_| {
            Effect::new(ReactiveGridShaderEffect {
                is_pointer_over: true,
                cell_size: 12.0,
                strength: 0.8,
                ..ReactiveGridShaderEffect::default()
            })
        }),
        scene("water-ripple", Tolerance::Moves, |_| {
            let mut ripple = WaterRippleShaderEffect {
                pointer_x: 0.5,
                pointer_y: 0.4,
                is_pointer_over: true,
                ..WaterRippleShaderEffect::default()
            };
            ripple.spawn_ripple(Point::new(0.45, 0.55), 1.0);
            ripple.ripples[0].age = 0.35;
            Effect::new(ripple)
        }),
        scene("burning-flame", Tolerance::Moves, |_| {
            Effect::new(BurningFlameShaderEffect {
                burn_amount: 0.9,
                flame_phase: 1.4,
                ignition_y: 0.6,
                ..BurningFlameShaderEffect::default()
            })
        }),
    ]
}

/// How far a rendering is from the CPU reference of its effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceDifference {
    /// The largest difference of a channel, of 255.
    pub largest: u8,
    /// The mean difference of the channels of all pixels, of 255.
    pub mean: f64,
    /// The share of the pixels (0 to 1) of which a channel differs by more
    /// than `levels`.
    pub outliers: f64,
}

/// Compares `actual` with `expected` (both premultiplied RGBA of the same
/// size), counting the pixels further than `levels`.
pub fn reference_difference(expected: &[u8], actual: &[u8], levels: u8) -> ReferenceDifference {
    assert_eq!(expected.len(), actual.len(), "the renderings have the same size");
    let largest = expected.iter().zip(actual).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    let total: u64 = expected.iter().zip(actual).map(|(a, b)| u64::from(a.abs_diff(*b))).sum();
    let far = expected
        .chunks_exact(4)
        .zip(actual.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(a, b)| a.abs_diff(*b) > levels))
        .count();
    let pixels = (expected.len() / 4).max(1);
    ReferenceDifference {
        largest,
        mean: total as f64 / expected.len().max(1) as f64,
        outliers: far as f64 / pixels as f64,
    }
}

impl ReferenceDifference {
    /// Whether the difference is within `tolerance`.
    pub fn is_within(&self, tolerance: Tolerance) -> bool {
        match tolerance {
            Tolerance::Largest(largest) => self.largest <= largest && self.mean <= tolerance.mean(),
            Tolerance::Moves => self.outliers <= tolerance.outliers() && self.mean <= tolerance.mean(),
        }
    }
}
