//! The brushes of material layers as the renderers sample them (`docs/porting/media3d.md`,
//! sections 3.4 and 3.16, step D6): a [`MaterialBrush`] is what a draw of a layer needs to paint
//! its brush over the texture coordinates of a mesh, the same description for the CPU reference
//! and the GPU executors.
//!
//! - **Solid colour brushes** are in the colour of the draw uniforms
//!   ([`DrawUniforms::color`](crate::uniforms::DrawUniforms::color)), as before.
//! - **Linear and radial gradients** are computed per pixel from [`BrushUniforms`]: up to
//!   [`MAX_INLINE_STOPS`] stops inline in the uniforms; more stops through a lookup texture of
//!   [`GRADIENT_TEXTURE_WIDTH`] by 1 texels ([`BrushShader::GradientTexture`]), made once per set
//!   of stops ([`BrushTextureKey::Gradient`]).
//! - **Image brushes** sample the pixels of their bitmap ([`BrushShader::Image`]), uploaded once per
//!   `(identity, version)` of the bitmap ([`BrushTextureKey::Image`]), with the tile, stretch,
//!   alignment, viewbox (`SourceRect`) and viewport (`DestinationRect`) of the brush.
//! - Any other brush (a conic gradient, an image whose pixels cannot be read, a visual or drawing
//!   brush without a shared form) is drawn with the layer's `Color` and counted
//!   ([`crate::layers_with_unsampled_brushes`]).
//!
//! **The mapping** of a mesh to a brush is WPF's: the texture coordinate of a point is a point of
//! the brush's 2D space, and the bounding box of the mesh's texture coordinates is the box a
//! brush relative to its bounding box (`RelativeUnit::Relative` points, rectangles and radii, the
//! default) is mapped onto; an absolute brush takes the texture coordinates as they are. The
//! brush's `Transform` (about its `TransformOrigin`) and `RelativeTransform` apply in that space,
//! as they do over the bounds of a 2D element. The uniforms hold the inverse: the texture
//! coordinate to the brush's space.
//!
//! **Colours.** Gradient stops are interpolated as straight (not premultiplied) colours in WPF's
//! space (the gamma-encoded components), after the spread method (`Pad`, `Reflect`, `Repeat`);
//! an image is sampled bilinearly in premultiplied colour and divided by its alpha. A sample is a
//! straight colour that multiplies the colour of the draw (the material's `Color` and the brush's
//! opacity), which [`crate::lighting::shade_color`] lights.
//!
//! The functions of this module are those of `scene3d.wgsl` (`brush_point`, `gradient_t`,
//! `inline_gradient`, `gradient_texture_coordinate`, `image_coordinate`), in the same order of
//! operations, so the comparison tests check math, not translations.

use crate::uniforms::color_uniform;
use ferroui_base::media::{
    GradientSpreadMethod, IBrush, IGradientBrush, ITileBrush, MediaExtensions, SharedBrush, StretchDirection, TileMode,
};
use ferroui_base::platform::{AlphaFormat, PixelFormat, SharedBitmapImpl};
use ferroui_base::rendering::scene3d::{MeshData, Scene3DSnapshot};
use ferroui_base::rendering::utilities::TileBrushCalculator;
use ferroui_base::{Matrix, Rect, Size};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

/// The gradient stops a draw takes inline in its uniforms; a gradient with more stops is sampled
/// from a lookup texture ([`BrushShader::GradientTexture`]).
pub const MAX_INLINE_STOPS: usize = 8;

/// The texels of the lookup texture of a gradient: the colour at `t = i / (width - 1)` in texel
/// `i`, sampled with linear filtering.
pub const GRADIENT_TEXTURE_WIDTH: u32 = 256;

/// The kind of a brush in [`BrushUniforms::metadata`] (its `x`; 0 for a solid colour).
mod kind {
    pub(super) const LINEAR: f32 = 1.0;
    pub(super) const RADIAL: f32 = 2.0;
    pub(super) const IMAGE: f32 = 3.0;
}

/// How the fragment stage of a draw samples its brush: the variant of the shader, one bit of the
/// pipeline key of the GPU executors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BrushShader {
    /// The colour of the draw uniforms alone: a solid colour brush, or a brush that is not
    /// sampled (drawn with the layer's `Color`).
    Solid,
    /// A linear or radial gradient with its stops inline in the [`BrushUniforms`].
    Gradient,
    /// A linear or radial gradient whose colours are read from a lookup texture.
    GradientTexture,
    /// An image brush: the pixels of its bitmap.
    Image,
}

/// What a brush adds to the uniforms of a draw: 256 bytes (`#[repr(C)]`, std140-compatible, every
/// member a `vec4`), placed at multiples of [`BRUSH_UNIFORMS_STRIDE`] in a buffer of many draws.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BrushUniforms {
    /// `x`: the kind (0 solid, 1 linear gradient, 2 radial gradient, 3 image); `y`: the spread
    /// method of a gradient (0 `Pad`, 1 `Reflect`, 2 `Repeat`) or the `TileMode` of an image (0
    /// `None`, 1 `FlipX`, 2 `FlipY`, 3 `FlipXY`, 4 `Tile`); `z`: the number of inline stops (0
    /// when they are in the lookup texture); `w`: 1 when the brush paints nothing (an image brush
    /// without an image).
    pub metadata: [f32; 4],
    /// The texture coordinate to the brush's space: `x = dot(vec3(uv, 1), to_brush_x.xyz)`.
    pub to_brush_x: [f32; 4],
    /// ... and `y = dot(vec3(uv, 1), to_brush_y.xyz)`.
    pub to_brush_y: [f32; 4],
    /// A linear gradient: the start point (`xy`) and the end point (`zw`). A radial gradient: the
    /// centre (`xy`) and one over the radii (`zw`, 0 for a radius of 0). An image: the destination
    /// rectangle (the tile): `x`, `y`, width, height.
    pub geometry: [f32; 4],
    /// A radial gradient: the gradient origin in the coordinates of the unit circle of the
    /// ellipse (`xy`), moved inside the circle. An image: the point of a tile to the texture
    /// coordinate of the bitmap, `uv = tile * xy + zw`.
    pub geometry2: [f32; 4],
    /// An image: the viewbox in texture coordinates of the bitmap (`xy` its top left, `zw` its
    /// bottom right), outside which the tile is transparent.
    pub clip: [f32; 4],
    /// The offsets of the inline stops, four to a `vec4`, sorted.
    pub stop_offsets: [[f32; 4]; 2],
    /// The colours of the inline stops, straight, 0 to 1.
    pub stop_colors: [[f32; 4]; MAX_INLINE_STOPS],
}

/// The stride of [`BrushUniforms`] in a buffer bound with dynamic offsets: their size, 256 bytes,
/// the offset alignment every GPU API accepts.
pub const BRUSH_UNIFORMS_STRIDE: usize = 256;

/// [`BrushUniforms`] in WGSL, member for member, for `scene3d.wgsl`.
pub const BRUSH_WGSL_DECLARATIONS: &str = "\
struct BrushUniforms {
    metadata: vec4<f32>,
    to_brush_x: vec4<f32>,
    to_brush_y: vec4<f32>,
    geometry: vec4<f32>,
    geometry2: vec4<f32>,
    clip: vec4<f32>,
    stop_offsets: array<vec4<f32>, 2>,
    stop_colors: array<vec4<f32>, 8>,
};
";

impl crate::uniforms::Std140 for BrushUniforms {
    fn write_std140(&self, out: &mut Vec<u8>) {
        let mut write = |values: &[f32; 4]| {
            for value in values {
                out.extend_from_slice(&value.to_ne_bytes());
            }
        };
        write(&self.metadata);
        write(&self.to_brush_x);
        write(&self.to_brush_y);
        write(&self.geometry);
        write(&self.geometry2);
        write(&self.clip);
        for offsets in &self.stop_offsets {
            write(offsets);
        }
        for color in &self.stop_colors {
            write(color);
        }
    }
}

/// What names the texture of a brush on a device: an executor uploads it once per key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BrushTextureKey {
    /// The lookup texture of a gradient: a hash of its sorted stops (offsets and colours).
    Gradient(u64),
    /// The pixels of a bitmap: its identity (the address of its allocation, kept from reuse by
    /// [`BrushTextureSource::identity`]) and the version of its pixels.
    Image {
        /// The address of the bitmap.
        identity: usize,
        /// [`IBitmapImpl::version`](ferroui_base::platform::IBitmapImpl::version).
        version: i32,
    },
}

/// Where the texels of a brush come from, read only when its texture is not on the device yet.
#[derive(Clone)]
pub struct BrushTextureSource {
    key: BrushTextureKey,
    origin: TextureOrigin,
}

#[derive(Clone)]
enum TextureOrigin {
    Gradient(Arc<[(f32, [f32; 4])]>),
    Image(Arc<SharedBitmapImpl>),
}

/// A handle that keeps the address of a bitmap from being reused while a texture made of it is
/// kept (a weak reference to its allocation), so that a [`BrushTextureKey::Image`] names one
/// bitmap for as long as the texture lives.
#[derive(Clone)]
pub struct BrushTextureIdentity(Option<Weak<SharedBitmapImpl>>);

impl BrushTextureIdentity {
    /// Whether the bitmap is alive (always for a gradient): a texture of a bitmap that is gone is
    /// dropped.
    pub fn is_alive(&self) -> bool {
        self.0.as_ref().is_none_or(|bitmap| bitmap.strong_count() > 0)
    }
}

impl BrushTextureSource {
    /// The key of the texture.
    pub fn key(&self) -> BrushTextureKey {
        self.key
    }

    /// What keeps the key unique while a texture of it is kept.
    pub fn identity(&self) -> BrushTextureIdentity {
        match &self.origin {
            TextureOrigin::Gradient(_) => BrushTextureIdentity(None),
            TextureOrigin::Image(bitmap) => BrushTextureIdentity(Some(Arc::downgrade(bitmap))),
        }
    }

    /// The texels: the lookup texture of a gradient (straight colours), or the pixels of a bitmap
    /// (premultiplied). `None` for a bitmap whose pixels cannot be read (no CPU pixels, or a
    /// format other than 8-bit RGBA or BGRA).
    pub fn texels(&self) -> Option<BrushTexels> {
        match &self.origin {
            TextureOrigin::Gradient(stops) => Some(gradient_texels(stops)),
            TextureOrigin::Image(bitmap) => bitmap_texels(&**bitmap),
        }
    }
}

/// The texels of a brush's texture: RGBA8, rows from the top without padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrushTexels {
    /// The width in texels.
    pub width: u32,
    /// The height in texels.
    pub height: u32,
    /// The texels.
    pub rgba: Vec<u8>,
    /// Whether the colours are premultiplied (an image) or straight (a gradient's lookup).
    pub premultiplied: bool,
}

impl BrushTexels {
    /// The texel at `x`, `y`, clamped to the edges, as four components from 0 to 1.
    fn texel(&self, x: i64, y: i64) -> [f32; 4] {
        let x = x.clamp(0, i64::from(self.width) - 1) as usize;
        let y = y.clamp(0, i64::from(self.height) - 1) as usize;
        let index = (y * self.width as usize + x) * 4;
        let scale = 1.0 / 255.0;
        [
            f32::from(self.rgba[index]) * scale,
            f32::from(self.rgba[index + 1]) * scale,
            f32::from(self.rgba[index + 2]) * scale,
            f32::from(self.rgba[index + 3]) * scale,
        ]
    }

    /// The texture at the normalized coordinate `uv` with linear filtering and the edges clamped:
    /// what a GPU sampler with `FilterMode::Linear` and `AddressMode::ClampToEdge` returns.
    pub fn sample_bilinear(&self, uv: [f32; 2]) -> [f32; 4] {
        if self.width == 0 || self.height == 0 {
            return [0.0; 4];
        }
        let x = uv[0] * self.width as f32 - 0.5;
        let y = uv[1] * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let a = self.texel(x0, y0);
        let b = self.texel(x0 + 1, y0);
        let c = self.texel(x0, y0 + 1);
        let d = self.texel(x0 + 1, y0 + 1);
        let mut result = [0.0f32; 4];
        for i in 0..4 {
            let top = a[i] + (b[i] - a[i]) * fx;
            let bottom = c[i] + (d[i] - c[i]) * fx;
            result[i] = top + (bottom - top) * fy;
        }
        result
    }
}

/// The brush of a material layer as a draw samples it.
#[derive(Clone)]
pub struct MaterialBrush {
    shader: BrushShader,
    uniforms: BrushUniforms,
    texture: Option<BrushTextureSource>,
    unsampled: bool,
}

impl MaterialBrush {
    /// The brush of a layer (`None`: a brush without a shared form) over a mesh whose texture
    /// coordinates have the bounding box `texture_bounds` ([`texture_coordinate_bounds`]).
    pub fn of(brush: Option<&SharedBrush>, texture_bounds: Rect) -> MaterialBrush {
        let unsampled = MaterialBrush {
            shader: BrushShader::Solid,
            uniforms: BrushUniforms::default(),
            texture: None,
            unsampled: true,
        };
        let Some(brush) = brush else { return unsampled };
        if brush.as_solid_color_brush().is_some() {
            return MaterialBrush { unsampled: false, ..unsampled };
        }
        let mut uniforms = BrushUniforms::default();
        let (to_brush_x, to_brush_y) = inverse_brush_rows(brush, texture_bounds);
        uniforms.to_brush_x = to_brush_x;
        uniforms.to_brush_y = to_brush_y;

        if let Some(gradient) = brush.as_gradient_brush() {
            let gradient_kind = if let Some(linear) = brush.as_linear_gradient_brush() {
                let start = linear.start_point().to_pixels_rect(texture_bounds);
                let end = linear.end_point().to_pixels_rect(texture_bounds);
                uniforms.geometry = [start.x as f32, start.y as f32, end.x as f32, end.y as f32];
                kind::LINEAR
            } else if let Some(radial) = brush.as_radial_gradient_brush() {
                let center = radial.center().to_pixels_rect(texture_bounds);
                let origin = radial.gradient_origin().to_pixels_rect(texture_bounds);
                let radius_x = radial.radius_x().to_value(texture_bounds.width);
                let radius_y = radial.radius_y().to_value(texture_bounds.height);
                let inverse = |radius: f64| if radius > 0.0 && radius.is_finite() { 1.0 / radius } else { 0.0 };
                let (inverse_x, inverse_y) = (inverse(radius_x), inverse(radius_y));
                uniforms.geometry = [center.x as f32, center.y as f32, inverse_x as f32, inverse_y as f32];
                // The focus in the unit circle of the ellipse, kept inside it.
                let (mut fx, mut fy) = ((origin.x - center.x) * inverse_x, (origin.y - center.y) * inverse_y);
                let length = fx.hypot(fy);
                const INSIDE: f64 = 0.999;
                if length > INSIDE {
                    fx *= INSIDE / length;
                    fy *= INSIDE / length;
                }
                uniforms.geometry2 = [fx as f32, fy as f32, 0.0, 0.0];
                kind::RADIAL
            } else {
                // A conic gradient (not a WPF brush) is not sampled.
                return unsampled;
            };
            let stops = sorted_stops(gradient);
            if stops.is_empty() {
                // No stops: a gradient paints nothing (as Skia's and WPF's).
                uniforms.metadata = [gradient_kind, spread_value(gradient.spread_method()), 0.0, 1.0];
                return MaterialBrush { shader: BrushShader::Gradient, uniforms, texture: None, unsampled: false };
            }
            let spread = spread_value(gradient.spread_method());
            if stops.len() <= MAX_INLINE_STOPS {
                uniforms.metadata = [gradient_kind, spread, stops.len() as f32, 0.0];
                for (index, (offset, color)) in stops.iter().enumerate() {
                    uniforms.stop_offsets[index / 4][index % 4] = *offset;
                    uniforms.stop_colors[index] = *color;
                }
                return MaterialBrush { shader: BrushShader::Gradient, uniforms, texture: None, unsampled: false };
            }
            uniforms.metadata = [gradient_kind, spread, 0.0, 0.0];
            let key = BrushTextureKey::Gradient(gradient_hash(&stops));
            let texture = BrushTextureSource { key, origin: TextureOrigin::Gradient(stops.into()) };
            return MaterialBrush {
                shader: BrushShader::GradientTexture,
                uniforms,
                texture: Some(texture),
                unsampled: false,
            };
        }

        if let (Some(image), Some(tile)) = (brush.as_image_brush(), brush.as_tile_brush()) {
            let bitmap = image.source().and_then(|source| source.get_bitmap());
            let Some(bitmap) = bitmap else {
                // An image brush without an image paints nothing.
                uniforms.metadata = [kind::IMAGE, 0.0, 0.0, 1.0];
                return MaterialBrush { shader: BrushShader::Image, uniforms, texture: None, unsampled: false };
            };
            let pixels = bitmap.pixel_size();
            if pixels.width <= 0 || pixels.height <= 0 || bitmap.as_readable_bitmap().is_none() {
                return unsampled;
            }
            image_uniforms(&mut uniforms, tile, image_size(&*bitmap), texture_bounds);
            let key = BrushTextureKey::Image {
                identity: Arc::as_ptr(&bitmap) as *const () as usize,
                version: bitmap.version(),
            };
            let texture = BrushTextureSource { key, origin: TextureOrigin::Image(bitmap) };
            return MaterialBrush { shader: BrushShader::Image, uniforms, texture: Some(texture), unsampled: false };
        }
        unsampled
    }

    /// The variant of the shader that samples the brush.
    pub fn shader(&self) -> BrushShader {
        self.shader
    }

    /// The uniforms of the brush (zero for a solid colour brush).
    pub fn uniforms(&self) -> &BrushUniforms {
        &self.uniforms
    }

    /// The texture the brush samples: the lookup texture of a gradient with many stops, the
    /// pixels of an image.
    pub fn texture(&self) -> Option<&BrushTextureSource> {
        self.texture.as_ref()
    }

    /// Whether the brush is not sampled: the layer is drawn with its `Color` and the brush's
    /// opacity, and counted ([`crate::layers_with_unsampled_brushes`]).
    pub fn is_unsampled(&self) -> bool {
        self.unsampled
    }

    /// The brush drawn with the layer's `Color` alone: what a renderer draws when the texels of
    /// its texture cannot be read.
    pub fn to_unsampled(&self) -> MaterialBrush {
        MaterialBrush { shader: BrushShader::Solid, uniforms: BrushUniforms::default(), texture: None, unsampled: true }
    }
}

/// The bounding box of the texture coordinates of `mesh`: the box a relative brush is mapped onto
/// (WPF). A mesh without texture coordinates has the empty box at the origin, where every point
/// takes the brush's colour at its origin.
pub fn texture_coordinate_bounds(mesh: &MeshData) -> Rect {
    let mut points = mesh.texture_coordinates.iter();
    let Some(first) = points.next() else { return Rect::default() };
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Rect::new(f64::from(min_x), f64::from(min_y), f64::from(max_x - min_x), f64::from(max_y - min_y))
}

/// A key of the textures of the image brushes of `scene` (the identities and versions of their
/// bitmaps, in order): an output kept for a snapshot is drawn again only while it is the same,
/// since new pixels of a bitmap change no generation of the snapshot. 0 without image brushes.
pub fn scene_texture_inputs(scene: &Scene3DSnapshot) -> u64 {
    let mut hasher: Option<DefaultHasher> = None;
    for model in &scene.models {
        for layer in model.front.iter().chain(&model.back) {
            let Some(brush) = &layer.brush else { continue };
            let Some(image) = brush.as_image_brush() else { continue };
            let Some(bitmap) = image.source().and_then(|source| source.get_bitmap()) else { continue };
            let hasher = hasher.get_or_insert_with(DefaultHasher::new);
            (Arc::as_ptr(&bitmap) as *const () as usize).hash(hasher);
            bitmap.version().hash(hasher);
        }
    }
    hasher.map_or(0, |hasher| hasher.finish() | 1)
}

/// The value of a spread method in the uniforms.
fn spread_value(spread: GradientSpreadMethod) -> f32 {
    match spread {
        GradientSpreadMethod::Pad => 0.0,
        GradientSpreadMethod::Reflect => 1.0,
        GradientSpreadMethod::Repeat => 2.0,
    }
}

/// The stops of a gradient sorted by their offsets (stable: stops at one offset keep their
/// order), with straight colours from 0 to 1.
fn sorted_stops(gradient: &dyn IGradientBrush) -> Vec<(f32, [f32; 4])> {
    let mut stops: Vec<(f32, [f32; 4])> =
        gradient.gradient_stops().iter().map(|stop| (stop.offset() as f32, color_uniform(stop.color()))).collect();
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    stops
}

fn gradient_hash(stops: &[(f32, [f32; 4])]) -> u64 {
    let mut hasher = DefaultHasher::new();
    stops.len().hash(&mut hasher);
    for (offset, color) in stops {
        offset.to_bits().hash(&mut hasher);
        for channel in color {
            channel.to_bits().hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// The transform of a brush over `bounds`, as a 2D backend applies it: the `RelativeTransform`
/// conjugated into the box (the unit square onto `bounds`), then the `Transform` about the
/// `TransformOrigin` (Skia's `get_relative_transform` and `get_absolute_transform`).
fn brush_transform(brush: &dyn IBrush, bounds: Rect) -> Option<Matrix> {
    let relative = brush.relative_transform().and_then(|transform| {
        let matrix = transform.value();
        if bounds.width <= 0.0 || bounds.height <= 0.0 || matrix.is_identity() {
            return None;
        }
        Some(
            Matrix::create_translation(-bounds.x, -bounds.y)
                * Matrix::create_scale(1.0 / bounds.width, 1.0 / bounds.height)
                * matrix
                * Matrix::create_scale(bounds.width, bounds.height)
                * Matrix::create_translation(bounds.x, bounds.y),
        )
    });
    let absolute = brush.transform().map(|transform| {
        let origin = brush.transform_origin().to_pixels_rect(bounds);
        let offset = Matrix::create_translation(origin.x, origin.y);
        (-offset) * transform.value() * offset
    });
    match (relative, absolute) {
        (Some(relative), Some(absolute)) => Some(relative * absolute),
        (relative, absolute) => relative.or(absolute),
    }
}

/// The rows of the inverse of the brush's transform (the texture coordinate to the brush's
/// space); a transform without an inverse maps every point to the origin.
fn inverse_brush_rows(brush: &dyn IBrush, bounds: Rect) -> ([f32; 4], [f32; 4]) {
    let inverse = match brush_transform(brush, bounds) {
        None => Some(Matrix::IDENTITY),
        Some(transform) => transform.try_invert(),
    };
    match inverse {
        Some(m) => ([m.m11 as f32, m.m21 as f32, m.m31 as f32, 0.0], [m.m12 as f32, m.m22 as f32, m.m32 as f32, 0.0]),
        None => ([0.0; 4], [0.0; 4]),
    }
}

/// The size of a bitmap in the units its viewbox is given in (device-independent pixels: its
/// pixels at 96 DPI), as a 2D tile brush takes it.
fn image_size(bitmap: &SharedBitmapImpl) -> Size {
    let pixels = bitmap.pixel_size();
    let dpi = bitmap.dpi();
    let scale = |dpi: f64| if dpi > 0.0 && dpi.is_finite() { 96.0 / dpi } else { 1.0 };
    Size::new(f64::from(pixels.width) * scale(dpi.x), f64::from(pixels.height) * scale(dpi.y))
}

/// The uniforms of an image brush: the tile (the destination rectangle in the brush's space), the
/// tile to the texture coordinate of the bitmap (the viewbox stretched and aligned into the tile,
/// as `TileBrushCalculator` places it), the viewbox as the clip, and the tile mode.
fn image_uniforms(uniforms: &mut BrushUniforms, tile: &dyn ITileBrush, image: Size, bounds: Rect) {
    let source = tile.source_rect().to_pixels(image);
    let destination = tile.destination_rect().to_pixels_rect(bounds);
    let scale =
        MediaExtensions::calculate_scaling(tile.stretch(), destination.size(), source.size(), StretchDirection::Both);
    let translate =
        TileBrushCalculator::calculate_translate(tile.alignment_x(), tile.alignment_y(), source, destination, scale);
    let tile_mode = match tile.tile_mode() {
        TileMode::None => 0.0,
        TileMode::FlipX => 1.0,
        TileMode::FlipY => 2.0,
        TileMode::FlipXY => 3.0,
        TileMode::Tile => 4.0,
    };
    let degenerate = scale.x <= 0.0
        || scale.y <= 0.0
        || destination.width <= 0.0
        || destination.height <= 0.0
        || image.width <= 0.0
        || image.height <= 0.0;
    uniforms.metadata = [kind::IMAGE, tile_mode, 0.0, if degenerate { 1.0 } else { 0.0 }];
    if degenerate {
        return;
    }
    uniforms.geometry =
        [destination.x as f32, destination.y as f32, destination.width as f32, destination.height as f32];
    // A point of the tile, less the translation of the alignment, over the scale, is a point of
    // the viewbox; over the size of the image, a texture coordinate.
    uniforms.geometry2 = [
        (1.0 / (scale.x * image.width)) as f32,
        (1.0 / (scale.y * image.height)) as f32,
        ((source.x - translate.x / scale.x) / image.width) as f32,
        ((source.y - translate.y / scale.y) / image.height) as f32,
    ];
    let clamp = |value: f64| value.clamp(0.0, 1.0) as f32;
    uniforms.clip = [
        clamp(source.x / image.width),
        clamp(source.y / image.height),
        clamp((source.x + source.width) / image.width),
        clamp((source.y + source.height) / image.height),
    ];
}

/// The tolerance of the tests of a point against the edges of a tile or a viewbox, so that the
/// edge itself is inside.
const EDGE_EPSILON: f32 = 1e-5;

/// The point of the brush's space at the texture coordinate `uv` (`brush_point` of the shader).
#[inline]
pub fn brush_point(uniforms: &BrushUniforms, uv: [f32; 2]) -> [f32; 2] {
    let (x, y) = (uniforms.to_brush_x, uniforms.to_brush_y);
    [uv[0] * x[0] + uv[1] * x[1] + x[2], uv[0] * y[0] + uv[1] * y[1] + y[2]]
}

/// The parameter of a gradient at the point `p` of the brush's space, after the spread method
/// (`gradient_t` of the shader): from 0 to 1.
pub fn gradient_t(uniforms: &BrushUniforms, p: [f32; 2]) -> f32 {
    let g = uniforms.geometry;
    let t = if uniforms.metadata[0] < 1.5 {
        // Linear: the projection onto the line from the start to the end point.
        let d = [g[2] - g[0], g[3] - g[1]];
        let length_squared = d[0] * d[0] + d[1] * d[1];
        if length_squared > 0.0 { ((p[0] - g[0]) * d[0] + (p[1] - g[1]) * d[1]) / length_squared } else { 0.0 }
    } else {
        // Radial: in the unit circle of the ellipse, the point's distance from the focus over the
        // distance from the focus to the circle along the same ray.
        let q = [(p[0] - g[0]) * g[2], (p[1] - g[1]) * g[3]];
        let f = [uniforms.geometry2[0], uniforms.geometry2[1]];
        let d = [q[0] - f[0], q[1] - f[1]];
        let a = d[0] * d[0] + d[1] * d[1];
        if a > 1e-12 {
            let b = f[0] * d[0] + f[1] * d[1];
            let c = f[0] * f[0] + f[1] * f[1] - 1.0;
            let s = (-b + (b * b - a * c).max(0.0).sqrt()) / a;
            if s > 0.0 { 1.0 / s } else { 0.0 }
        } else {
            0.0
        }
    };
    spread(t, uniforms.metadata[1])
}

/// `t` after the spread method of its value (0 `Pad`, 1 `Reflect`, 2 `Repeat`).
#[inline]
fn spread(t: f32, method: f32) -> f32 {
    if method < 0.5 {
        t.clamp(0.0, 1.0)
    } else if method < 1.5 {
        let m = t - 2.0 * (t * 0.5).floor();
        if m > 1.0 { 2.0 - m } else { m }
    } else {
        t - t.floor()
    }
}

/// The colour of sorted stops at `t`: the first stop's before it, the last stop's after the
/// last, linear between two (a step where two stops share an offset).
fn stops_at(count: usize, offset: impl Fn(usize) -> f32, color: impl Fn(usize) -> [f32; 4], t: f32) -> [f32; 4] {
    if count == 0 {
        return [0.0; 4];
    }
    if t <= offset(0) {
        return color(0);
    }
    for index in 1..count {
        let end = offset(index);
        if t <= end {
            let start = offset(index - 1);
            let span = end - start;
            let f = if span > 0.0 { (t - start) / span } else { 1.0 };
            let (a, b) = (color(index - 1), color(index));
            return [
                a[0] + (b[0] - a[0]) * f,
                a[1] + (b[1] - a[1]) * f,
                a[2] + (b[2] - a[2]) * f,
                a[3] + (b[3] - a[3]) * f,
            ];
        }
    }
    color(count - 1)
}

/// The colour of the inline stops at `t` (`inline_gradient` of the shader), straight.
pub fn inline_gradient(uniforms: &BrushUniforms, t: f32) -> [f32; 4] {
    let count = (uniforms.metadata[2] + 0.5) as usize;
    stops_at(
        count.min(MAX_INLINE_STOPS),
        |index| uniforms.stop_offsets[index / 4][index % 4],
        |index| uniforms.stop_colors[index],
        t,
    )
}

/// The horizontal texture coordinate of `t` in a gradient's lookup texture
/// (`gradient_texture_coordinate` of the shader): the centre of texel `t * (width - 1)`.
#[inline]
pub fn gradient_texture_coordinate(t: f32) -> f32 {
    let width = GRADIENT_TEXTURE_WIDTH as f32;
    (t * (width - 1.0) + 0.5) / width
}

/// The texture coordinate of the bitmap of an image brush at the point `p` of the brush's space
/// (`image_coordinate` of the shader), or `None` where the brush is transparent: outside the
/// tile of `TileMode::None`, outside the viewbox, or a brush that paints nothing.
pub fn image_coordinate(uniforms: &BrushUniforms, p: [f32; 2]) -> Option<[f32; 2]> {
    if uniforms.metadata[3] > 0.5 {
        return None;
    }
    let g = uniforms.geometry;
    let mut local = [p[0] - g[0], p[1] - g[1]];
    let mode = (uniforms.metadata[1] + 0.5) as u32;
    if mode == 0 {
        if local[0] < -EDGE_EPSILON
            || local[1] < -EDGE_EPSILON
            || local[0] > g[2] + EDGE_EPSILON
            || local[1] > g[3] + EDGE_EPSILON
        {
            return None;
        }
    } else {
        let size = [g[2], g[3]];
        for axis in 0..2 {
            let tile = (local[axis] / size[axis]).floor();
            let mut inside = local[axis] - tile * size[axis];
            // FlipX (1) and FlipXY (3) mirror the odd columns, FlipY (2) and FlipXY the odd rows.
            let flips = if axis == 0 { mode == 1 || mode == 3 } else { mode == 2 || mode == 3 };
            if flips && (tile - 2.0 * (tile * 0.5).floor()) > 0.5 {
                inside = size[axis] - inside;
            }
            local[axis] = inside;
        }
    }
    let g2 = uniforms.geometry2;
    let uv = [local[0] * g2[0] + g2[2], local[1] * g2[1] + g2[3]];
    let clip = uniforms.clip;
    if uv[0] < clip[0] - EDGE_EPSILON
        || uv[1] < clip[1] - EDGE_EPSILON
        || uv[0] > clip[2] + EDGE_EPSILON
        || uv[1] > clip[3] + EDGE_EPSILON
    {
        return None;
    }
    Some(uv)
}

/// The straight colour of a premultiplied sample.
#[inline]
fn unpremultiply(sample: [f32; 4]) -> [f32; 4] {
    if sample[3] > 0.0 {
        let inverse = 1.0 / sample[3];
        [sample[0] * inverse, sample[1] * inverse, sample[2] * inverse, sample[3]]
    } else {
        [0.0; 4]
    }
}

/// The straight colour of a brush at the texture coordinate `uv`, which multiplies the colour of
/// the draw: white for [`BrushShader::Solid`]; `texels` is the brush's texture (`None` for a
/// texture that is missing: transparent).
pub fn sample_brush(
    shader: BrushShader,
    uniforms: &BrushUniforms,
    texels: Option<&BrushTexels>,
    uv: [f32; 2],
) -> [f32; 4] {
    match shader {
        BrushShader::Solid => [1.0; 4],
        BrushShader::Gradient => {
            if uniforms.metadata[3] > 0.5 {
                return [0.0; 4];
            }
            inline_gradient(uniforms, gradient_t(uniforms, brush_point(uniforms, uv)))
        }
        BrushShader::GradientTexture => {
            let Some(texels) = texels else { return [0.0; 4] };
            let t = gradient_t(uniforms, brush_point(uniforms, uv));
            texels.sample_bilinear([gradient_texture_coordinate(t), 0.5])
        }
        BrushShader::Image => {
            let Some(texels) = texels else { return [0.0; 4] };
            match image_coordinate(uniforms, brush_point(uniforms, uv)) {
                Some(coordinate) => unpremultiply(texels.sample_bilinear(coordinate)),
                None => [0.0; 4],
            }
        }
    }
}

/// The lookup texture of sorted stops: texel `i` is the colour at `t = i / (width - 1)`,
/// straight, rounded to 8 bits.
fn gradient_texels(stops: &[(f32, [f32; 4])]) -> BrushTexels {
    let width = GRADIENT_TEXTURE_WIDTH as usize;
    let mut rgba = Vec::with_capacity(width * 4);
    for index in 0..width {
        let t = index as f32 / (width - 1) as f32;
        let color = stops_at(stops.len(), |i| stops[i].0, |i| stops[i].1, t);
        for channel in color {
            rgba.push((channel.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
        }
    }
    BrushTexels { width: GRADIENT_TEXTURE_WIDTH, height: 1, rgba, premultiplied: false }
}

/// The pixels of a bitmap as premultiplied RGBA8, read through its framebuffer: 8-bit RGBA or
/// BGRA, premultiplied, straight or opaque.
fn bitmap_texels(bitmap: &SharedBitmapImpl) -> Option<BrushTexels> {
    let readable = bitmap.as_readable_bitmap()?;
    let framebuffer = readable.lock();
    let size = framebuffer.size();
    let row_bytes = framebuffer.row_bytes();
    let format = framebuffer.format();
    let alpha = framebuffer.alpha_format();
    let bgra = if format == PixelFormat::BGRA8888 {
        true
    } else if format == PixelFormat::RGBA8888 {
        false
    } else {
        framebuffer.dispose();
        return None;
    };
    if size.width <= 0 || size.height <= 0 || row_bytes < size.width * 4 {
        framebuffer.dispose();
        return None;
    }
    let (width, height, row_bytes) = (size.width as usize, size.height as usize, row_bytes as usize);
    let mut rgba = vec![0u8; width * height * 4];
    let mut complete = false;
    framebuffer.with_data(&mut |data| {
        if data.len() < row_bytes * (height - 1) + width * 4 {
            return;
        }
        for y in 0..height {
            let source = &data[y * row_bytes..y * row_bytes + width * 4];
            let target = &mut rgba[y * width * 4..(y + 1) * width * 4];
            for (pixel, out) in source.chunks_exact(4).zip(target.chunks_exact_mut(4)) {
                let (r, g, b, a) = if bgra {
                    (pixel[2], pixel[1], pixel[0], pixel[3])
                } else {
                    (pixel[0], pixel[1], pixel[2], pixel[3])
                };
                let (r, g, b, a) = match alpha {
                    AlphaFormat::Premul => (r, g, b, a),
                    AlphaFormat::Opaque => (r, g, b, 255),
                    AlphaFormat::Unpremul => {
                        let scale = |channel: u8| ((u32::from(channel) * u32::from(a) + 127) / 255) as u8;
                        (scale(r), scale(g), scale(b), a)
                    }
                };
                out.copy_from_slice(&[r, g, b, a]);
            }
        }
        complete = true;
    });
    framebuffer.dispose();
    complete.then_some(BrushTexels { width: width as u32, height: height as u32, rgba, premultiplied: true })
}

#[cfg(test)]
#[path = "brush_tests.rs"]
mod tests;
