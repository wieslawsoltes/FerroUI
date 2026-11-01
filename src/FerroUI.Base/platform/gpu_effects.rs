//! What a drawing context that evaluates the lights and the pixel
//! operations of WinUI's composition itself is given: plain data, the same
//! for every backend (docs/porting/winui-composition.md, "How the effects
//! are drawn").
//!
//! The compositor describes the lights in their own spaces, as the light
//! map of the CPU path does ([`LightMap`]), so a backend that evaluates them
//! per pixel (an SkSL shader in Skia) and the CPU path agree: both evaluate
//! [`LightSource::lit_at`] at the point of the lit visual mapped by
//! `to_light_space`.

use crate::rendering::composition::effects::BlendEffectMode;
use crate::rendering::composition::server::{LightColor, LightMap, LightSource, LightingMaterial};
use crate::{Matrix, PixelSize, Point, Rect};

/// A point or spot light with the transform from the coordinates of the
/// drawing context (those of the lit visual) to those of its coordinate
/// space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedLight {
    /// The light in its coordinate space ([`LightSource::Point`] or
    /// [`LightSource::Spot`]).
    pub source: LightSource,
    /// Maps a point of the lit visual to the coordinate space of the light.
    pub to_light_space: Matrix,
}

/// The lights that reach a lit area and how its surface reflects them.
///
/// The ambient and distant lights light every point alike: they are summed
/// into [`constant`](Self::constant) once, in double precision, with the
/// material applied. The point and spot lights depend on the point and are
/// listed in [`lights`](Self::lights).
#[derive(Clone, Debug, PartialEq)]
pub struct LightingParameters {
    /// The light of the ambient and distant lights reflected by the
    /// material, the same at every point (not clamped).
    pub constant: LightColor,
    /// The point and spot lights.
    pub lights: Vec<PositionedLight>,
    /// How the surface reflects the lights.
    pub material: LightingMaterial,
}

impl LightingParameters {
    /// The most point and spot lights a drawing context is asked to
    /// evaluate: with more, the compositor draws the lighting on the CPU.
    pub const MAX_LIGHTS: usize = 8;

    /// The parameters of the lights of `map` reflected by `material`.
    pub fn new(map: &LightMap, material: LightingMaterial) -> LightingParameters {
        let mut constant = LightColor::BLACK;
        let mut lights = Vec::new();
        for (source, to_light_space) in map.lights() {
            match source {
                // The same at every point: the point does not matter.
                LightSource::Ambient { .. } | LightSource::Distant { .. } => {
                    constant = constant.add(source.lit_at(Point::default(), &material));
                }
                LightSource::Point { .. } | LightSource::Spot { .. } => {
                    lights.push(PositionedLight { source: *source, to_light_space: *to_light_space });
                }
            }
        }
        LightingParameters { constant, lights, material }
    }

    /// Whether the light is the same at every point (no point or spot
    /// light): it is [`constant`](Self::constant).
    pub fn is_uniform(&self) -> bool {
        self.lights.is_empty()
    }

    /// Whether a drawing context is asked to evaluate the lights: at most
    /// [`MAX_LIGHTS`](Self::MAX_LIGHTS) point and spot lights.
    pub fn fits_a_drawing_context(&self) -> bool {
        self.lights.len() <= Self::MAX_LIGHTS
    }

    /// The light reflected at `point` of the lit visual (not clamped): the
    /// reference the drawing contexts evaluate per pixel, equal to
    /// [`LightMap::lit_at`] of the map the parameters were made of.
    pub fn lit_at(&self, point: Point) -> LightColor {
        let mut light = self.constant;
        for light_source in &self.lights {
            light = light.add(light_source.source.lit_at(light_source.to_light_space.transform(point), &self.material));
        }
        light
    }
}

/// An operation on the pixels of the sources of an effect that the blend
/// layers do not have, drawn by
/// [`push_pixel_operation`](super::IDrawingContextImplWithLayerEffects::push_pixel_operation).
///
/// The first source is the destination `d`, each next source `s` is
/// combined with what the sources before it made; colours are
/// premultiplied, 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PixelOperation {
    /// A mode of Direct2D's blend effect the blend layers lack (`Dissolve`,
    /// `LinearBurn`, `DarkerColor`, `LighterColor`, `VividLight`,
    /// `LinearLight`, `PinLight`, `HardMix`, `Subtract`, `Division`), of the
    /// second source (the foreground) over the first (the background).
    Blend {
        /// The mode.
        mode: BlendEffectMode,
        /// The area of the grid `Dissolve` numbers its pixels on: the
        /// random choice of a pixel is that of the cell it is in.
        noise_grid: Rect,
        /// The number of cells of the grid along each side.
        noise_size: PixelSize,
    },
    /// `k[0] d s + k[1] d + k[2] s + k[3]` per channel, clamped to 0..1 and
    /// the colour to the alpha (Direct2D's arithmetic composite of two
    /// images).
    Arithmetic {
        /// The multiply, first source, second source and offset amounts.
        k: [f32; 4],
    },
    /// The `MaskInvert` composite of each next source onto what the sources
    /// before it made: `as (ad - Cd) + (1 - as) Cd`, the alpha of the
    /// destination kept.
    MaskInvert,
}

impl PixelOperation {
    /// Combines the premultiplied RGBA pixels of a next source `source`
    /// into those of `destination` (what the sources before it made), row
    /// by row, `width` pixels a row; the `Dissolve` noise numbers the pixels
    /// row by row. The CPU reference of the drawing contexts, and what the
    /// compositor does without them.
    pub fn apply_to_premultiplied_rgba8(&self, destination: &mut [u8], source: &[u8], width: usize) {
        use crate::rendering::composition::brushes::{arithmetic_pixels, blend_pixels, mask_invert_pixels};
        match *self {
            PixelOperation::Blend { mode, .. } => blend_pixels(mode, destination, source, width),
            PixelOperation::Arithmetic { k } => arithmetic_pixels(k, destination, source),
            PixelOperation::MaskInvert => mask_invert_pixels(destination, source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::composition::server::LightAttenuation;
    use crate::Vector3D;

    fn point_light(x: f64) -> LightSource {
        LightSource::Point {
            color: LightColor::new(0.5, 0.25, 1.0),
            position: Vector3D::new(x, 10.0, 40.0),
            attenuation: LightAttenuation { constant: 1.0, linear: 0.01, quadratic: 0.0, min_cutoff: 5.0, max_cutoff: 200.0 },
        }
    }

    #[test]
    fn ambient_and_distant_lights_are_summed_into_the_constant_light() {
        let mut map = LightMap::new();
        map.add(LightSource::Ambient { color: LightColor::new(0.1, 0.2, 0.3) }, Matrix::IDENTITY);
        map.add(
            LightSource::Distant { color: LightColor::WHITE, direction: Vector3D::new(0.0, 0.0, -1.0) },
            Matrix::create_translation(50.0, 0.0),
        );
        let material = LightingMaterial { ambient: 0.5, diffuse: 0.5, specular: 0.25, shine: 8.0 };
        let parameters = LightingParameters::new(&map, material);
        assert!(parameters.is_uniform());
        assert!(parameters.lights.is_empty());
        // Ambient times its amount; the distant light straight down: full
        // diffuse incidence and a full specular highlight.
        let expected = LightColor::new(0.05 + 0.5 + 0.25, 0.1 + 0.5 + 0.25, 0.15 + 0.5 + 0.25);
        for (actual, expected) in [
            (parameters.constant.r, expected.r),
            (parameters.constant.g, expected.g),
            (parameters.constant.b, expected.b),
        ] {
            assert!((actual - expected).abs() < 1e-12, "{actual} {expected}");
        }
        let at = Point::new(13.0, 77.0);
        assert_eq!(map.lit_at(at, &material), parameters.lit_at(at));
    }

    #[test]
    fn point_and_spot_lights_are_listed_with_their_transforms() {
        let mut map = LightMap::new();
        let to_light_space = Matrix::create_scale(2.0, 2.0) * Matrix::create_translation(5.0, -3.0);
        map.add(LightSource::Ambient { color: LightColor::new(0.1, 0.1, 0.1) }, Matrix::IDENTITY);
        map.add(point_light(20.0), to_light_space);
        let parameters = LightingParameters::new(&map, LightingMaterial::VISUAL);
        assert!(!parameters.is_uniform());
        assert_eq!(vec![PositionedLight { source: point_light(20.0), to_light_space }], parameters.lights);
        for point in [Point::new(0.0, 0.0), Point::new(12.5, 4.0), Point::new(90.0, 60.0)] {
            let (expected, actual) = (map.lit_at(point, &LightingMaterial::VISUAL), parameters.lit_at(point));
            assert!((expected.r - actual.r).abs() < 1e-12 && (expected.b - actual.b).abs() < 1e-12, "{expected:?} {actual:?}");
        }
    }

    #[test]
    fn more_than_the_most_lights_are_not_given_to_a_drawing_context() {
        let mut map = LightMap::new();
        for index in 0..LightingParameters::MAX_LIGHTS {
            map.add(point_light(index as f64), Matrix::IDENTITY);
        }
        // Ambient and distant lights do not count.
        map.add(LightSource::Ambient { color: LightColor::WHITE }, Matrix::IDENTITY);
        assert!(LightingParameters::new(&map, LightingMaterial::VISUAL).fits_a_drawing_context());
        map.add(point_light(100.0), Matrix::IDENTITY);
        assert!(!LightingParameters::new(&map, LightingMaterial::VISUAL).fits_a_drawing_context());
    }
}
