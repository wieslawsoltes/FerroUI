use super::*;
use crate::test_scenes::{PixelBitmap, image_brush};
use crate::uniforms::Std140;
use ferroui_base::media::immutable::{
    ImmutableConicGradientBrush, ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableRadialGradientBrush,
    ImmutableSolidColorBrush, ImmutableTransform,
};
use ferroui_base::media::{Color, Stretch};
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::{RelativePoint, RelativeRect, RelativeScalar, RelativeUnit};
use std::mem::{offset_of, size_of};
use std::rc::Rc;

const RED: Color = Color::from_argb(255, 255, 0, 0);
const BLUE: Color = Color::from_argb(255, 0, 0, 255);
const UNIT: Rect = Rect::new(0.0, 0.0, 1.0, 1.0);

fn shared(brush: &dyn IBrush) -> SharedBrush {
    SharedBrush::from_brush(brush).expect("a brush with a shared form")
}

fn red_to_blue(start: RelativePoint, end: RelativePoint, spread: GradientSpreadMethod) -> SharedBrush {
    let stops = [ImmutableGradientStop::new(0.0, RED), ImmutableGradientStop::new(1.0, BLUE)];
    shared(&ImmutableLinearGradientBrush::new(&stops, 1.0, None, None, spread, Some(start), Some(end), None))
}

fn sample(brush: &MaterialBrush, uv: [f32; 2]) -> [f32; 4] {
    let texels = brush.texture().and_then(BrushTextureSource::texels);
    sample_brush(brush.shader(), brush.uniforms(), texels.as_ref(), uv)
}

fn close(expected: [f32; 4], actual: [f32; 4], tolerance: f32) -> bool {
    expected.iter().zip(actual).all(|(e, a)| (e - a).abs() <= tolerance)
}

#[track_caller]
fn assert_close(expected: [f32; 4], actual: [f32; 4], tolerance: f32) {
    assert!(close(expected, actual, tolerance), "expected {expected:?}, got {actual:?}");
}

/// Red to blue at `f`, straight.
fn mix(f: f32) -> [f32; 4] {
    [1.0 - f, 0.0, f, 1.0]
}

#[test]
fn the_brush_uniforms_are_256_bytes_of_vec4s_in_their_declared_order() {
    assert_eq!(256, size_of::<BrushUniforms>());
    assert_eq!(BRUSH_UNIFORMS_STRIDE, size_of::<BrushUniforms>());
    let offsets = [
        offset_of!(BrushUniforms, metadata),
        offset_of!(BrushUniforms, to_brush_x),
        offset_of!(BrushUniforms, to_brush_y),
        offset_of!(BrushUniforms, geometry),
        offset_of!(BrushUniforms, geometry2),
        offset_of!(BrushUniforms, clip),
        offset_of!(BrushUniforms, stop_offsets),
        offset_of!(BrushUniforms, stop_colors),
    ];
    assert_eq!([0, 16, 32, 48, 64, 80, 96, 128], offsets);
    let mut bytes = Vec::new();
    let uniforms = BrushUniforms { metadata: [1.0, 2.0, 3.0, 4.0], ..BrushUniforms::default() };
    uniforms.write_std140(&mut bytes);
    assert_eq!(256, bytes.len());
    assert_eq!(2.0f32.to_ne_bytes(), bytes[4..8]);
    // The members of the WGSL declaration are those of the structure, in order.
    let members: Vec<&str> =
        BRUSH_WGSL_DECLARATIONS.lines().filter_map(|line| line.trim().split_once(':').map(|(name, _)| name)).collect();
    assert_eq!(
        vec!["metadata", "to_brush_x", "to_brush_y", "geometry", "geometry2", "clip", "stop_offsets", "stop_colors"],
        members
    );
}

#[test]
fn solid_brushes_are_their_colour_and_other_brushes_without_a_form_are_unsampled() {
    let solid = MaterialBrush::of(Some(&shared(&ImmutableSolidColorBrush::new(RED))), UNIT);
    assert_eq!(BrushShader::Solid, solid.shader());
    assert!(!solid.is_unsampled());
    assert_eq!([1.0; 4], sample(&solid, [0.3, 0.3]));

    let none = MaterialBrush::of(None, UNIT);
    assert!(none.is_unsampled());
    assert_eq!(BrushShader::Solid, none.shader());

    let stops = [ImmutableGradientStop::new(0.0, RED), ImmutableGradientStop::new(1.0, BLUE)];
    let conic = shared(&ImmutableConicGradientBrush::from_stops(&stops));
    assert!(MaterialBrush::of(Some(&conic), UNIT).is_unsampled());
}

#[test]
fn a_relative_linear_gradient_spans_the_box_of_the_texture_coordinates() {
    let brush = red_to_blue(
        RelativePoint::TOP_LEFT,
        RelativePoint::new(1.0, 0.0, RelativeUnit::Relative),
        GradientSpreadMethod::Pad,
    );
    // The texture coordinates of the mesh from (0.2, 0.0) to (0.6, 1.0): the gradient runs across that.
    let bounds = Rect::new(0.2, 0.0, 0.4, 1.0);
    let material = MaterialBrush::of(Some(&brush), bounds);
    assert_eq!(BrushShader::Gradient, material.shader());
    assert!(material.texture().is_none());
    assert_close(mix(0.0), sample(&material, [0.2, 0.5]), 1e-6);
    assert_close(mix(0.5), sample(&material, [0.4, 0.1]), 1e-6);
    assert_close(mix(1.0), sample(&material, [0.6, 0.9]), 1e-6);
    // Padded beyond the ends.
    assert_close(mix(0.0), sample(&material, [0.0, 0.5]), 1e-6);
    assert_close(mix(1.0), sample(&material, [0.9, 0.5]), 1e-6);
}

#[test]
fn an_absolute_linear_gradient_takes_the_texture_coordinates_as_they_are() {
    let start = RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute);
    let end = RelativePoint::new(0.0, 2.0, RelativeUnit::Absolute);
    let material =
        MaterialBrush::of(Some(&red_to_blue(start, end, GradientSpreadMethod::Pad)), Rect::new(0.5, 0.5, 0.1, 0.1));
    assert_close(mix(0.25), sample(&material, [0.55, 0.5]), 1e-6);
    assert_close(mix(0.5), sample(&material, [9.0, 1.0]), 1e-6);
}

#[test]
fn the_spread_methods_pad_reflect_and_repeat_the_gradient() {
    let end = RelativePoint::new(1.0, 0.0, RelativeUnit::Relative);
    for (spread, at_1_25, at_minus_0_25) in [
        (GradientSpreadMethod::Pad, 1.0, 0.0),
        (GradientSpreadMethod::Reflect, 0.75, 0.25),
        (GradientSpreadMethod::Repeat, 0.25, 0.75),
    ] {
        let material = MaterialBrush::of(Some(&red_to_blue(RelativePoint::TOP_LEFT, end, spread)), UNIT);
        assert_close(mix(at_1_25), sample(&material, [1.25, 0.0]), 1e-5);
        assert_close(mix(at_minus_0_25), sample(&material, [-0.25, 0.0]), 1e-5);
    }
}

#[test]
fn a_radial_gradient_runs_from_its_origin_to_its_ellipse() {
    let stops = [ImmutableGradientStop::new(0.0, RED), ImmutableGradientStop::new(1.0, BLUE)];
    // Centre (0.5, 0.5), radii 0.5 and 0.25 of the box.
    let brush = ImmutableRadialGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        None,
        Some(RelativeScalar::new(0.5, RelativeUnit::Relative)),
        Some(RelativeScalar::new(0.25, RelativeUnit::Relative)),
        None,
    );
    let material = MaterialBrush::of(Some(&shared(&brush)), UNIT);
    assert_close(mix(0.0), sample(&material, [0.5, 0.5]), 1e-6);
    assert_close(mix(0.5), sample(&material, [0.75, 0.5]), 1e-5);
    assert_close(mix(0.5), sample(&material, [0.5, 0.625]), 1e-5);
    assert_close(mix(1.0), sample(&material, [1.0, 0.5]), 1e-5);
    assert_close(mix(1.0), sample(&material, [0.5, 0.0]), 1e-5);

    // A focus moved to the left: t is 0 there, and 1 on the circle on either side.
    let origin = RelativePoint::new(0.25, 0.5, RelativeUnit::Relative);
    let focal = ImmutableRadialGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        Some(origin),
        None,
        None,
        None,
    );
    let material = MaterialBrush::of(Some(&shared(&focal)), UNIT);
    assert_close(mix(0.0), sample(&material, [0.25, 0.5]), 1e-5);
    assert_close(mix(1.0), sample(&material, [0.0, 0.5]), 1e-4);
    assert_close(mix(1.0), sample(&material, [1.0, 0.5]), 1e-4);
    // Halfway from the focus to the circle on the right.
    assert_close(mix(0.5), sample(&material, [0.625, 0.5]), 1e-4);
}

#[test]
fn stops_are_sorted_and_interpolated_as_straight_colours() {
    let translucent = Color::from_argb(0, 0, 255, 0);
    let stops = [
        ImmutableGradientStop::new(1.0, BLUE),
        ImmutableGradientStop::new(0.0, RED),
        ImmutableGradientStop::new(0.5, translucent),
    ];
    let brush = shared(&ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::TOP_LEFT),
        Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative)),
        None,
    ));
    let material = MaterialBrush::of(Some(&brush), UNIT);
    assert_close([0.5, 0.5, 0.0, 0.5], sample(&material, [0.25, 0.0]), 1e-5);
    assert_close([0.0, 1.0, 0.0, 0.0], sample(&material, [0.5, 0.0]), 1e-5);
    assert_close([0.0, 0.5, 0.5, 0.5], sample(&material, [0.75, 0.0]), 1e-5);
}

/// Ten stops: red and blue alternating every ninth.
fn ten_stops(shift: f64) -> SharedBrush {
    let stops: Vec<ImmutableGradientStop> = (0..10)
        .map(|index| {
            ImmutableGradientStop::new(f64::from(index) / 9.0 * (1.0 - shift), if index % 2 == 0 { RED } else { BLUE })
        })
        .collect();
    shared(&ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::TOP_LEFT),
        Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative)),
        None,
    ))
}

#[test]
fn more_than_eight_stops_are_sampled_from_a_lookup_texture_made_once_per_set_of_stops() {
    let material = MaterialBrush::of(Some(&ten_stops(0.0)), UNIT);
    assert_eq!(BrushShader::GradientTexture, material.shader());
    let source = material.texture().expect("a lookup texture");
    let texels = source.texels().expect("its texels");
    assert_eq!((GRADIENT_TEXTURE_WIDTH, 1, false), (texels.width, texels.height, texels.premultiplied));
    // The ends are the first and last stop.
    assert_eq!([255, 0, 0, 255], texels.rgba[0..4]);
    assert_eq!([0, 0, 255, 255], texels.rgba[texels.rgba.len() - 4..]);
    // Sampled within the rounding of the texels of the stops computed exactly (no kink of the
    // stops between the two texels of these points).
    for t in [0.0f32, 0.1, 0.37, 0.5, 0.83, 1.0] {
        let from_texture = sample(&material, [t, 0.5]);
        let stops_at_t = stops_at(
            10,
            |i| i as f32 / 9.0,
            |i| if i % 2 == 0 { [1.0, 0.0, 0.0, 1.0] } else { [0.0, 0.0, 1.0, 1.0] },
            t,
        );
        assert_close(stops_at_t, from_texture, 2.5 / 255.0);
    }
    // The same stops are the same texture; other stops another.
    let again = MaterialBrush::of(Some(&ten_stops(0.0)), Rect::new(0.0, 0.0, 2.0, 2.0));
    assert_eq!(source.key(), again.texture().unwrap().key());
    let other = MaterialBrush::of(Some(&ten_stops(0.1)), UNIT);
    assert_ne!(source.key(), other.texture().unwrap().key());
    assert!(source.identity().is_alive());
}

#[test]
fn the_relative_transform_and_the_transform_of_a_brush_apply_in_its_box() {
    let stops = [ImmutableGradientStop::new(0.0, RED), ImmutableGradientStop::new(1.0, BLUE)];
    let end = Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative));
    // A relative rotation by 90 degrees about the centre of the box turns the horizontal
    // gradient into a vertical one, from the top.
    let rotation = Rc::new(ImmutableTransform::new(Matrix::create_rotation_at(
        90f64.to_radians(),
        ferroui_base::Point::new(0.5, 0.5),
    )));
    let rotated = ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        end,
        Some(rotation),
    );
    let bounds = Rect::new(0.0, 0.0, 2.0, 2.0);
    let material = MaterialBrush::of(Some(&shared(&rotated)), bounds);
    assert_close(mix(0.0), sample(&material, [1.0, 0.0]), 1e-5);
    assert_close(mix(0.25), sample(&material, [0.3, 0.5]), 1e-5);
    assert_close(mix(1.0), sample(&material, [1.7, 2.0]), 1e-5);

    // A translation of the brush by half the box moves the start of the gradient there.
    let translation = Rc::new(ImmutableTransform::new(Matrix::create_translation(1.0, 0.0)));
    let moved = ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        Some(translation),
        None,
        GradientSpreadMethod::Pad,
        None,
        end,
        None,
    );
    let material = MaterialBrush::of(Some(&shared(&moved)), bounds);
    assert_close(mix(0.0), sample(&material, [1.0, 0.0]), 1e-5);
    assert_close(mix(0.5), sample(&material, [2.0, 0.0]), 1e-5);
}

/// A bitmap of 4 by 4 opaque pixels in quadrants of 2 by 2: red, green / blue, white.
fn quadrants() -> Arc<PixelBitmap> {
    let mut pixels = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            pixels.extend_from_slice(match (x < 2, y < 2) {
                (true, true) => &[255, 0, 0, 255],
                (false, true) => &[0, 255, 0, 255],
                (true, false) => &[0, 0, 255, 255],
                (false, false) => &[255, 255, 255, 255],
            });
        }
    }
    PixelBitmap::rgba(4, 4, pixels)
}

fn image(
    bitmap: &Arc<PixelBitmap>,
    stretch: Stretch,
    tile_mode: TileMode,
    destination: RelativeRect,
    source: RelativeRect,
) -> SharedBrush {
    shared(&image_brush(bitmap, stretch, tile_mode, destination, source))
}

#[test]
fn an_image_brush_fills_the_box_of_the_texture_coordinates() {
    let bitmap = quadrants();
    let material = MaterialBrush::of(
        Some(&image(&bitmap, Stretch::Fill, TileMode::None, RelativeRect::FILL, RelativeRect::FILL)),
        UNIT,
    );
    assert_eq!(BrushShader::Image, material.shader());
    let red = [1.0, 0.0, 0.0, 1.0];
    let white = [1.0; 4];
    // Within a quadrant, its colour.
    assert_close(red, sample(&material, [0.25, 0.25]), 1e-6);
    assert_close([0.0, 1.0, 0.0, 1.0], sample(&material, [0.75, 0.25]), 1e-6);
    assert_close(white, sample(&material, [0.75, 0.75]), 1e-6);
    // Between two quadrants, bilinear; at the edges, clamped.
    assert_close([0.5, 0.5, 0.0, 1.0], sample(&material, [0.5, 0.25]), 1e-6);
    assert_close(red, sample(&material, [0.0, 0.0]), 1e-6);
    // Outside the tile of TileMode::None: transparent.
    assert_close([0.0; 4], sample(&material, [1.2, 0.5]), 0.0);
}

#[test]
fn the_viewbox_the_stretch_and_the_tiles_of_an_image_brush_are_those_of_a_2d_tile_brush() {
    let bitmap = quadrants();
    // The viewbox the top right quadrant (green), stretched over the box.
    let green = image(
        &bitmap,
        Stretch::Fill,
        TileMode::None,
        RelativeRect::FILL,
        RelativeRect::new(0.5, 0.0, 0.5, 0.5, RelativeUnit::Relative),
    );
    let material = MaterialBrush::of(Some(&green), UNIT);
    // (Nearer its left or bottom edge, the filter would also read the texels beside the viewbox,
    // as a GPU sampler does.)
    for uv in [[0.3, 0.3], [0.5, 0.5], [0.6, 0.6]] {
        assert_close([0.0, 1.0, 0.0, 1.0], sample(&material, uv), 1e-6);
    }

    // Uniform into a box twice as wide as high: centred, transparent on the sides.
    let uniform = image(&bitmap, Stretch::Uniform, TileMode::None, RelativeRect::FILL, RelativeRect::FILL);
    let material = MaterialBrush::of(Some(&uniform), Rect::new(0.0, 0.0, 2.0, 1.0));
    assert_close([0.0; 4], sample(&material, [0.25, 0.5]), 0.0);
    assert_close([1.0, 0.0, 0.0, 1.0], sample(&material, [0.75, 0.25]), 1e-6);
    assert_close([1.0; 4], sample(&material, [1.25, 0.75]), 1e-6);
    assert_close([0.0; 4], sample(&material, [1.75, 0.5]), 0.0);

    // Tiles of half the box: the image repeats; FlipX mirrors the odd columns.
    let half = RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative);
    let tiled = MaterialBrush::of(Some(&image(&bitmap, Stretch::Fill, TileMode::Tile, half, RelativeRect::FILL)), UNIT);
    let flipped =
        MaterialBrush::of(Some(&image(&bitmap, Stretch::Fill, TileMode::FlipX, half, RelativeRect::FILL)), UNIT);
    // The top left texel of the second column of tiles.
    assert_close([1.0, 0.0, 0.0, 1.0], sample(&tiled, [0.625, 0.125]), 1e-6);
    assert_close([0.0, 1.0, 0.0, 1.0], sample(&flipped, [0.625, 0.125]), 1e-6);
    // The rows are not flipped by FlipX.
    assert_close([0.0, 0.0, 1.0, 1.0], sample(&flipped, [0.125, 0.875]), 1e-6);
}

#[test]
fn an_image_brush_without_an_image_paints_nothing_and_one_without_pixels_is_unsampled() {
    let empty = shared(&ferroui_base::media::immutable::ImmutableImageBrush::from_bitmap(None));
    let material = MaterialBrush::of(Some(&empty), UNIT);
    assert!(!material.is_unsampled());
    assert_close([0.0; 4], sample(&material, [0.5, 0.5]), 0.0);
}

#[test]
fn the_pixels_of_a_bitmap_are_read_as_premultiplied_rgba() {
    // One BGRA pixel, straight: blue 255, green 0, red 128, alpha 128.
    let bitmap = PixelBitmap::with_format(1, 1, vec![255, 0, 128, 128], PixelFormat::BGRA8888, AlphaFormat::Unpremul);
    let material = MaterialBrush::of(
        Some(&image(&bitmap, Stretch::Fill, TileMode::None, RelativeRect::FILL, RelativeRect::FILL)),
        UNIT,
    );
    let texels = material.texture().unwrap().texels().unwrap();
    assert!(texels.premultiplied);
    assert_eq!(vec![64, 0, 128, 128], texels.rgba);
    // Sampled back as the straight colour.
    assert_close([64.0 / 128.0, 0.0, 1.0, 128.0 / 255.0], sample(&material, [0.5, 0.5]), 1e-6);
}

#[test]
fn the_key_of_an_image_is_its_bitmap_and_version_and_the_scene_inputs_follow_it() {
    let bitmap = quadrants();
    let brush = image(&bitmap, Stretch::Fill, TileMode::None, RelativeRect::FILL, RelativeRect::FILL);
    let key = MaterialBrush::of(Some(&brush), UNIT).texture().unwrap().key();
    let scene = {
        let mut layer = crate::test_scenes::diffuse(RED);
        layer.brush = Some(brush.clone());
        crate::test_scenes::snapshot(
            crate::test_scenes::orthographic(),
            vec![crate::test_scenes::model(
                crate::test_scenes::quad(1, -1.0, -1.0, 1.0, 1.0, 0.0),
                vec![layer],
                vec![],
            )],
            vec![],
        )
    };
    let inputs = scene_texture_inputs(&scene);
    assert_ne!(0, inputs);
    bitmap.set_pixels(vec![0; 64]);
    let new_key = MaterialBrush::of(Some(&brush), UNIT).texture().unwrap().key();
    assert_ne!(key, new_key);
    assert_ne!(inputs, scene_texture_inputs(&scene));
    // A scene of solid colours has none.
    let solid = crate::test_scenes::snapshot(crate::test_scenes::orthographic(), vec![], vec![]);
    assert_eq!(0, scene_texture_inputs(&solid));
}

#[test]
fn the_box_of_the_texture_coordinates_holds_them_all() {
    let mesh = MeshData {
        positions: vec![Vector3::new(0.0, 0.0, 0.0); 3],
        texture_coordinates: vec![Vector2::new(0.25, 0.5), Vector2::new(0.75, 0.0), Vector2::new(0.5, 2.0)],
        ..MeshData::default()
    };
    assert_eq!(Rect::new(0.25, 0.0, 0.5, 2.0), texture_coordinate_bounds(&mesh));
    assert_eq!(Rect::default(), texture_coordinate_bounds(&MeshData::default()));
}
