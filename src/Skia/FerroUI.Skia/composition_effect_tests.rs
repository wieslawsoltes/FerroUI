//! The lights and the pixel operations of WinUI's composition drawn by the
//! drawing context (`draw_lighting`, `push_lighting_multiply_layer`,
//! `push_pixel_operation`: SkSL runtime effects) compared pixel by pixel
//! with the CPU reference of the compositor drawn with the same context
//! (the light map of `LightMap`, `PixelOperation::apply_to_premultiplied_rgba8`).
//!
//! The targets are raster surfaces: runtime effects run there as on the GPU
//! (the Graphite tests of `gpu/metal/tests.rs` draw some of the same scenes
//! on a Metal device).
//!
//! Each lighting scene is checked twice:
//!
//! - Against the exact light: the light of the compositor
//!   (`LightingParameters::lit_at`, in double precision) at the point of the
//!   visual each pixel shows, rounded to 8 bits. The GPU evaluates the same
//!   math in single precision per pixel: within 1 level (the rounding).
//! - Against the CPU path: the light map samples the light every 2 device
//!   pixels, rounds each sample to 8 bits and is drawn with bilinear
//!   interpolation, so it differs from the exact light by up to
//!   `h^2 / 8 * |f''|` (`h` = 2 pixels) plus the rounding, more where the
//!   light bends sharply (the smooth step between the cones of a spot
//!   light, the highlight of a high shine). At every pixel the GPU result
//!   is within the CPU path's own error plus 1 level of the CPU path: the
//!   two paths differ by what the sampling of the CPU path loses, no more.
//!   The largest CPU error of the scenes is asserted too (it stays a few
//!   levels).
//!
//! Left out: the edge of the lit area (2 device pixels: anti-aliasing, and
//! the sampled map holds its outermost sample there), and the pixels within
//! a sample of a discontinuity (the maximum cutoff of a point light), which
//! the sampled map blurs.
//!
//! The multiply layer of the CPU path also rounds its factors to 8 bits of
//! their scale (at most `scale / 510`); the same two checks apply, against
//! the content times the exact light.
//!
//! The pixel operations combine the same 8-bit sources with the same
//! formulas, in single precision on both sides: they differ by the rounding
//! of the result only (1 level).

use crate::composition_effects::pixel_image_count;
use crate::PlatformRenderInterface;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Color};
use ferroui_base::platform::{
    AlphaFormat, IDrawingContextImpl, IPlatformRenderInterface, IRenderTargetBitmapImpl, LightingParameters,
    PixelFormat, PixelFormats, PixelOperation,
};
use ferroui_base::rendering::composition::effects::BlendEffectMode;
use ferroui_base::rendering::composition::server::{
    LightAttenuation, LightColor, LightMap, LightSource, LightingMaterial,
};
use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Vector, Vector3D};
use std::sync::Arc;

pub(crate) const WIDTH: i32 = 160;
pub(crate) const HEIGHT: i32 = 120;

/// The premultiplied RGBA pixels of a raster target drawn by `draw` under
/// `transform`.
pub(crate) fn render(transform: Matrix, draw: impl FnOnce(&mut dyn IDrawingContextImpl)) -> Vec<u8> {
    let target = PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(WIDTH, HEIGHT), Vector::new(96.0, 96.0));
    {
        let mut context = target.create_drawing_context();
        context.set_transform(transform);
        draw(&mut *context);
        context.dispose();
    }
    read(&target)
}

fn read(target: &Arc<dyn IRenderTargetBitmapImpl>) -> Vec<u8> {
    let framebuffer = target.lock();
    let row_bytes = framebuffer.row_bytes() as usize;
    let bgra = framebuffer.format() == PixelFormat::BGRA8888;
    let mut rgba = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    framebuffer.with_data(&mut |data| {
        for y in 0..HEIGHT as usize {
            for x in 0..WIDTH as usize {
                let at = y * row_bytes + x * 4;
                let pixel = &data[at..at + 4];
                let out = (y * WIDTH as usize + x) * 4;
                rgba[out..out + 4].copy_from_slice(&if bgra {
                    [pixel[2], pixel[1], pixel[0], pixel[3]]
                } else {
                    [pixel[0], pixel[1], pixel[2], pixel[3]]
                });
            }
        }
    });
    framebuffer.dispose();
    rgba
}

/// The device pixels per unit of `matrix` along its longer axis, as the
/// compositor measures them for the light map.
fn device_scale(matrix: &Matrix) -> f64 {
    let x = (matrix.m11 * matrix.m11 + matrix.m12 * matrix.m12).sqrt();
    let y = (matrix.m21 * matrix.m21 + matrix.m22 * matrix.m22).sqrt();
    x.max(y)
}

/// The scene lighting of the CPU path (`paint_scene_lighting`): the light
/// map sampled every second device pixel, drawn as a bitmap.
fn cpu_scene(context: &mut dyn IDrawingContextImpl, rect: Rect, map: &LightMap, material: &LightingMaterial) {
    let size = LightMap::size_for_scale(rect, device_scale(&context.transform()));
    let pixels = map.scene_pixels(rect, size, material);
    let bitmap = PlatformRenderInterface::default().load_bitmap_from_pixels(
        PixelFormats::RGBA8888,
        AlphaFormat::Premul,
        &pixels,
        size,
        Vector::new(96.0, 96.0),
        size.width * 4,
    );
    context.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, size.width as f64, size.height as f64), rect);
}

fn gpu_scene(context: &mut dyn IDrawingContextImpl, rect: Rect, map: &LightMap, material: &LightingMaterial) {
    let layers = context.as_drawing_context_impl_with_layer_effects().expect("layer effects");
    assert!(layers.draw_lighting(rect, &LightingParameters::new(map, *material)), "the lights are drawn by Skia");
}

/// The pixels whose centre shows a point of `rect` at least `margin` device
/// pixels inside it and not `excluded`, with that point.
fn compared_pixels(
    transform: Matrix,
    rect: Rect,
    margin: f64,
    excluded: &dyn Fn(Point) -> bool,
) -> Vec<(usize, Point)> {
    let inverse = transform.try_invert().expect("invertible");
    let inset = margin / device_scale(&transform);
    let inner = Rect::new(rect.x + inset, rect.y + inset, rect.width - 2.0 * inset, rect.height - 2.0 * inset);
    let mut pixels = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let local = inverse.transform(Point::new(x as f64 + 0.5, y as f64 + 0.5));
            if inner.contains(local) && !excluded(local) {
                pixels.push((((y * WIDTH + x) * 4) as usize, local));
            }
        }
    }
    pixels
}

fn difference(a: &[u8], b: &[u8], at: usize) -> i32 {
    (0..4).map(|channel| (i32::from(a[at + channel]) - i32::from(b[at + channel])).abs()).max().unwrap()
}

/// What the comparison of a GPU result with the exact result and the CPU
/// result found: the largest difference from the exact result, the largest
/// excess of the difference from the CPU result over the CPU result's own
/// error, and that error.
#[derive(Debug)]
struct Agreement {
    gpu_error: i32,
    excess_over_cpu_error: i32,
    cpu_error: i32,
    compared: usize,
}

fn agreement(exact: &[u8], cpu: &[u8], gpu: &[u8], pixels: &[(usize, Point)]) -> Agreement {
    let mut result = Agreement { gpu_error: 0, excess_over_cpu_error: i32::MIN, cpu_error: 0, compared: pixels.len() };
    for &(at, _) in pixels {
        let cpu_error = difference(cpu, exact, at);
        result.gpu_error = result.gpu_error.max(difference(gpu, exact, at));
        result.cpu_error = result.cpu_error.max(cpu_error);
        result.excess_over_cpu_error = result.excess_over_cpu_error.max(difference(gpu, cpu, at) - cpu_error);
    }
    result
}

#[track_caller]
fn assert_agreement(agreement: &Agreement, cpu_error: i32) {
    assert!(agreement.compared > 1000, "enough pixels compared: {agreement:?}");
    assert!(agreement.gpu_error <= 1, "the GPU light is the exact light: {agreement:?}");
    assert!(agreement.excess_over_cpu_error <= 1, "the GPU and CPU paths differ by the sampling only: {agreement:?}");
    assert!(agreement.cpu_error <= cpu_error, "the sampling of the CPU path loses at most {cpu_error} levels: {agreement:?}");
}

fn byte(value: f64) -> u8 {
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

/// The exact light at each compared pixel, opaque.
fn exact_scene(lighting: &LightingParameters, pixels: &[(usize, Point)]) -> Vec<u8> {
    let mut rgba = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    for &(at, local) in pixels {
        let light = lighting.lit_at(local).clamped();
        rgba[at..at + 4].copy_from_slice(&[byte(light.r), byte(light.g), byte(light.b), 255]);
    }
    rgba
}

#[track_caller]
fn assert_scene(map: &LightMap, material: LightingMaterial, transform: Matrix, rect: Rect, cpu_error: i32) {
    assert_scene_excluding(map, material, transform, rect, cpu_error, |_| false);
}

#[track_caller]
fn assert_scene_excluding(
    map: &LightMap,
    material: LightingMaterial,
    transform: Matrix,
    rect: Rect,
    cpu_error: i32,
    excluded: impl Fn(Point) -> bool,
) {
    let cpu = render(transform, |context| cpu_scene(context, rect, map, &material));
    let gpu = render(transform, |context| gpu_scene(context, rect, map, &material));
    let pixels = compared_pixels(transform, rect, 2.0, &excluded);
    let exact = exact_scene(&LightingParameters::new(map, material), &pixels);
    assert_agreement(&agreement(&exact, &cpu, &gpu, &pixels), cpu_error);
}

const RECT: Rect = Rect { x: 10.0, y: 10.0, width: 140.0, height: 100.0 };

fn color(r: f64, g: f64, b: f64) -> LightColor {
    LightColor::new(r, g, b)
}

fn point_light(position: Vector3D, color: LightColor, attenuation: LightAttenuation) -> LightSource {
    LightSource::Point { color, position, attenuation }
}

fn spot_light() -> LightSource {
    LightSource::Spot {
        inner_color: color(1.0, 0.9, 0.7),
        outer_color: color(0.2, 0.3, 1.0),
        position: Vector3D::new(40.0, 30.0, 90.0),
        direction: Vector3D::new(0.5, 0.4, -1.0),
        inner_cone_angle: 0.25,
        outer_cone_angle: 0.6,
        attenuation: LightAttenuation { constant: 1.0, linear: 0.002, quadratic: 0.0, min_cutoff: 0.0, max_cutoff: f64::MAX },
    }
}

fn single(source: LightSource) -> LightMap {
    let mut map = LightMap::new();
    map.add(source, Matrix::IDENTITY);
    map
}

const DIFFUSE: LightingMaterial = LightingMaterial { ambient: 1.0, diffuse: 1.0, specular: 0.0, shine: 16.0 };
const SHINY: LightingMaterial = LightingMaterial { ambient: 0.5, diffuse: 0.7, specular: 0.6, shine: 24.0 };

#[test]
fn an_ambient_and_a_distant_light_are_the_same_everywhere() {
    let mut map = LightMap::new();
    map.add(LightSource::Ambient { color: color(0.2, 0.1, 0.4) }, Matrix::IDENTITY);
    map.add(LightSource::Distant { color: color(0.5, 0.6, 0.2), direction: Vector3D::new(0.3, -0.2, -1.0) }, Matrix::IDENTITY);
    assert_scene(&map, DIFFUSE, Matrix::IDENTITY, RECT, 1);
    assert_scene(&map, SHINY, Matrix::IDENTITY, RECT, 1);
    // It is drawn as a solid colour by the compositor; the shader agrees.
}

#[test]
fn a_point_light_with_attenuation_and_a_minimum_cutoff() {
    let attenuation =
        LightAttenuation { constant: 0.8, linear: 0.004, quadratic: 0.00002, min_cutoff: 70.0, max_cutoff: f64::MAX };
    let map = single(point_light(Vector3D::new(80.0, 60.0, 60.0), color(1.0, 0.8, 0.6), attenuation));
    assert_scene(&map, DIFFUSE, Matrix::IDENTITY, RECT, 2);
}

#[test]
fn a_point_light_lights_nothing_beyond_its_maximum_cutoff() {
    let center = Vector3D::new(80.0, 60.0, 60.0);
    let attenuation = LightAttenuation { constant: 1.0, linear: 0.0, quadratic: 0.0, min_cutoff: 0.0, max_cutoff: 80.0 };
    let map = single(point_light(center, LightColor::WHITE, attenuation));
    // The cutoff is a step: the sampled map blurs it over a sample.
    let near_the_cutoff = |p: Point| {
        let distance = ((p.x - center.x).powi(2) + (p.y - center.y).powi(2) + center.z * center.z).sqrt();
        (distance - 80.0).abs() < 2.5
    };
    assert_scene_excluding(&map, DIFFUSE, Matrix::IDENTITY, RECT, 3, near_the_cutoff);
    // The GPU light is dark beyond the cutoff and lit inside it.
    let gpu = render(Matrix::IDENTITY, |context| gpu_scene(context, RECT, &map, &DIFFUSE));
    let at = |x: usize, y: usize| gpu[(y * WIDTH as usize + x) * 4];
    assert!(at(80, 60) > 240, "under the light: {}", at(80, 60));
    assert_eq!(0, at(12, 12), "beyond the cutoff");
}

#[test]
fn a_spot_light_inside_between_and_outside_its_cones() {
    let map = single(spot_light());
    assert_scene(&map, DIFFUSE, Matrix::IDENTITY, RECT, 3);
    // The scene has all three: full light along the direction, the smooth
    // step, and darkness outside the outer cone.
    let gpu = render(Matrix::IDENTITY, |context| gpu_scene(context, RECT, &map, &DIFFUSE));
    let red = |x: usize, y: usize| gpu[(y * WIDTH as usize + x) * 4];
    let inner = red(85, 66);
    let outside = red(140, 15);
    assert!(inner > 150, "inside the inner cone: {inner}");
    assert_eq!(0, outside, "outside the outer cone");
}

#[test]
fn diffuse_and_specular_lighting() {
    let attenuation = LightAttenuation { constant: 1.0, linear: 0.003, quadratic: 0.0, min_cutoff: 0.0, max_cutoff: f64::MAX };
    let map = single(point_light(Vector3D::new(70.0, 50.0, 70.0), color(0.9, 0.9, 1.0), attenuation));
    assert_scene(&map, DIFFUSE, Matrix::IDENTITY, RECT, 2);
    assert_scene(&map, SHINY, Matrix::IDENTITY, RECT, 3);
    let specular_only = LightingMaterial { ambient: 0.0, diffuse: 0.0, specular: 1.0, shine: 40.0 };
    assert_scene(&map, specular_only, Matrix::IDENTITY, RECT, 3);
}

fn several_lights() -> LightMap {
    let mut map = LightMap::new();
    map.add(LightSource::Ambient { color: color(0.05, 0.05, 0.1) }, Matrix::IDENTITY);
    map.add(LightSource::Distant { color: color(0.1, 0.05, 0.0), direction: Vector3D::new(0.0, 1.0, -1.0) }, Matrix::IDENTITY);
    let attenuation = LightAttenuation { constant: 1.0, linear: 0.004, quadratic: 0.0, min_cutoff: 10.0, max_cutoff: f64::MAX };
    map.add(point_light(Vector3D::new(30.0, 30.0, 50.0), color(0.6, 0.2, 0.1), attenuation), Matrix::IDENTITY);
    // A light in a space of its own: shifted and scaled.
    map.add(
        point_light(Vector3D::new(50.0, 40.0, 60.0), color(0.1, 0.5, 0.3), attenuation),
        Matrix::create_scale(0.5, 0.5) * Matrix::create_translation(10.0, 5.0),
    );
    map.add(spot_light(), Matrix::IDENTITY);
    map.add(spot_light(), Matrix::create_translation(-60.0, 10.0));
    map
}

#[test]
fn several_lights_add_up() {
    let map = several_lights();
    assert_eq!(4, LightingParameters::new(&map, DIFFUSE).lights.len());
    // Two spot lights whose smooth steps cross the area: the sampled map
    // loses up to 6 levels there, the GPU none.
    assert_scene(&map, DIFFUSE, Matrix::IDENTITY, RECT, 6);
    assert_scene(&map, SHINY, Matrix::IDENTITY, RECT, 6);
}

#[test]
fn transformed_visuals_are_lit_where_their_points_are() {
    let map = several_lights();
    let rect = Rect::new(0.0, 0.0, 100.0, 70.0);
    // Translated, scaled, rotated: the lights stay with the visual.
    for transform in [
        Matrix::create_translation(25.5, 17.25),
        Matrix::create_scale(1.5, 1.5) * Matrix::create_translation(4.0, 6.0),
        Matrix::create_rotation(0.4) * Matrix::create_translation(50.0, 0.0),
        Matrix::create_scale(1.2, 0.9) * Matrix::create_rotation(-0.3) * Matrix::create_translation(10.0, 45.0),
    ] {
        assert_scene(&map, SHINY, transform, rect, 8);
    }
}

/// The content of a lit visual: opaque and translucent rectangles.
fn content(context: &mut dyn IDrawingContextImpl, rect: Rect) {
    let opaque = ImmutableSolidColorBrush::new(Color::from_rgb(200, 160, 90));
    context.draw_rectangle(Some(&opaque), None, RoundedRect::from_rect(rect), &BoxShadows::default());
    let translucent = ImmutableSolidColorBrush::new(Color::from_argb(128, 40, 120, 220));
    let half = Rect::new(rect.x, rect.y + rect.height / 2.0, rect.width, rect.height / 2.0);
    context.draw_rectangle(Some(&translucent), None, RoundedRect::from_rect(half), &BoxShadows::default());
}

#[track_caller]
fn assert_multiplied(map: &LightMap, transform: Matrix, rect: Rect, cpu_error: i32) {
    let cpu = render(transform, |context| {
        let size = LightMap::size_for_scale(rect, device_scale(&context.transform()));
        let factors = map.factors(rect, size, &LightingMaterial::VISUAL).expect("the light changes the content");
        let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
        layers.push_multiply_layer(rect, &factors);
        content(layers, rect);
        layers.pop_multiply_layer();
    });
    let lighting = LightingParameters::new(map, LightingMaterial::VISUAL);
    let gpu = render(transform, |context| {
        let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
        assert!(layers.push_lighting_multiply_layer(rect, &lighting));
        content(layers, rect);
        layers.pop_multiply_layer();
    });
    // The content times the exact light, clamped to the alpha.
    let unlit = render(transform, |context| content(context, rect));
    let pixels = compared_pixels(transform, rect, 2.0, &|_| false);
    let mut exact = vec![0u8; unlit.len()];
    for &(at, local) in &pixels {
        let light = lighting.lit_at(local);
        let alpha = f64::from(unlit[at + 3]);
        for (channel, factor) in [light.r, light.g, light.b].into_iter().enumerate() {
            let factor = factor.clamp(0.0, ferroui_base::rendering::composition::server::LIGHT_MAP_MAX_SCALE);
            exact[at + channel] = (f64::from(unlit[at + channel]) * factor).round().clamp(0.0, alpha) as u8;
        }
        exact[at + 3] = unlit[at + 3];
    }
    assert_agreement(&agreement(&exact, &cpu, &gpu, &pixels), cpu_error);
}

#[test]
fn a_multiply_layer_of_the_lights_matches_the_light_map() {
    // Below full light.
    assert_multiplied(&several_lights(), Matrix::IDENTITY, RECT, 3);
    // Above full light: the content is brightened, clamped per channel.
    let mut map = LightMap::new();
    map.add(LightSource::Ambient { color: color(0.4, 0.4, 0.4) }, Matrix::IDENTITY);
    let attenuation = LightAttenuation { constant: 1.0, linear: 0.002, quadratic: 0.0, min_cutoff: 0.0, max_cutoff: f64::MAX };
    map.add(point_light(Vector3D::new(80.0, 60.0, 80.0), color(1.8, 1.6, 1.2), attenuation), Matrix::IDENTITY);
    let factors = map.factors(RECT, LightMap::size_for_scale(RECT, 1.0), &LightingMaterial::VISUAL).unwrap();
    assert!(factors.scale > 1.5, "the light is above full: {}", factors.scale);
    assert_multiplied(&map, Matrix::IDENTITY, RECT, 3);
    assert_multiplied(&map, Matrix::create_rotation(0.3) * Matrix::create_translation(30.0, -10.0), Rect::new(0.0, 0.0, 120.0, 90.0), 3);
    // The same everywhere.
    let mut ambient = LightMap::new();
    ambient.add(LightSource::Ambient { color: color(0.5, 1.5, 0.25) }, Matrix::IDENTITY);
    assert_multiplied(&ambient, Matrix::IDENTITY, RECT, 1);
}

// --- the pixel operations ------------------------------------------------------------

/// The first source: quadrants of different colours and alphas.
fn first_source(context: &mut dyn IDrawingContextImpl) {
    let (w, h) = (WIDTH as f64 / 2.0, HEIGHT as f64 / 2.0);
    for (index, color) in [
        Color::from_argb(255, 200, 60, 30),
        Color::from_argb(128, 20, 200, 120),
        Color::from_argb(200, 250, 250, 250),
        Color::from_argb(60, 90, 30, 160),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = Rect::new((index % 2) as f64 * w, (index / 2) as f64 * h, w, h);
        context.draw_rectangle(Some(&ImmutableSolidColorBrush::new(color)), None, RoundedRect::from_rect(rect), &BoxShadows::default());
    }
}

/// The second source: stripes of different colours and alphas, with gaps.
fn second_source(context: &mut dyn IDrawingContextImpl) {
    let colors = [
        Color::from_argb(255, 30, 140, 230),
        Color::from_argb(160, 240, 200, 20),
        Color::from_argb(255, 128, 128, 128),
        Color::from_argb(90, 255, 0, 255),
        Color::from_argb(220, 10, 10, 10),
    ];
    for (index, color) in colors.into_iter().enumerate() {
        let rect = Rect::new(index as f64 * 30.0 + 4.0, 0.0, 24.0, HEIGHT as f64);
        context.draw_rectangle(Some(&ImmutableSolidColorBrush::new(color)), None, RoundedRect::from_rect(rect), &BoxShadows::default());
    }
}

/// The third source of a `MaskInvert`: a band across.
fn third_source(context: &mut dyn IDrawingContextImpl) {
    let rect = Rect::new(0.0, 40.0, WIDTH as f64, 30.0);
    let brush = ImmutableSolidColorBrush::new(Color::from_argb(180, 0, 0, 0));
    context.draw_rectangle(Some(&brush), None, RoundedRect::from_rect(rect), &BoxShadows::default());
}

fn full() -> Rect {
    Rect::new(0.0, 0.0, WIDTH as f64, HEIGHT as f64)
}

#[track_caller]
fn assert_operation(operation: PixelOperation, sources: &[fn(&mut dyn IDrawingContextImpl)], tolerance: i32) {
    let images: Vec<Vec<u8>> = sources.iter().map(|source| render(Matrix::IDENTITY, |context| source(context))).collect();
    let mut expected = images[0].clone();
    for image in &images[1..] {
        operation.apply_to_premultiplied_rgba8(&mut expected, image, WIDTH as usize);
    }
    let actual = render(Matrix::IDENTITY, |context| {
        let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
        assert!(layers.push_pixel_operation(full(), &operation), "{operation:?} is drawn by Skia");
        for (index, source) in sources.iter().enumerate() {
            if index > 0 {
                layers.next_pixel_operation_source();
            }
            source(layers);
        }
        layers.pop_pixel_operation();
    });
    let largest = expected.iter().zip(&actual).map(|(e, a)| (i32::from(*e) - i32::from(*a)).abs()).max().unwrap();
    assert!(largest <= tolerance, "{operation:?}: the GPU result is within {tolerance} levels of the CPU one: {largest}");
}

#[test]
fn the_blend_modes_the_layers_lack_match_the_cpu_formulas() {
    for mode in [
        BlendEffectMode::LinearBurn,
        BlendEffectMode::DarkerColor,
        BlendEffectMode::LighterColor,
        BlendEffectMode::VividLight,
        BlendEffectMode::LinearLight,
        BlendEffectMode::PinLight,
        BlendEffectMode::HardMix,
        BlendEffectMode::Subtract,
        BlendEffectMode::Division,
    ] {
        let operation = PixelOperation::Blend { mode, noise_grid: full(), noise_size: PixelSize::new(WIDTH, HEIGHT) };
        assert_operation(operation, &[first_source, second_source], 1);
    }
}

#[test]
fn dissolve_keeps_the_pixels_the_cpu_noise_keeps() {
    let operation = PixelOperation::Blend {
        mode: BlendEffectMode::Dissolve,
        noise_grid: full(),
        noise_size: PixelSize::new(WIDTH, HEIGHT),
    };
    // The same hash on the same grid: the same pixels, exactly.
    assert_operation(operation, &[first_source, second_source], 1);
}

#[test]
fn the_arithmetic_composite_matches_the_cpu_one() {
    for k in [[0.5, 0.75, 0.6, 0.1], [1.0, 0.0, 0.0, 0.0], [-0.5, 1.0, 1.0, 0.0], [0.0, 0.3, 0.3, -0.05]] {
        assert_operation(PixelOperation::Arithmetic { k }, &[first_source, second_source], 1);
    }
}

#[test]
fn mask_invert_matches_the_cpu_one() {
    assert_operation(PixelOperation::MaskInvert, &[first_source, second_source], 1);
    assert_operation(PixelOperation::MaskInvert, &[first_source, second_source, third_source], 1);
}

#[test]
fn a_pixel_operation_keeps_nothing_outside_its_bounds_and_composes_with_the_opacity() {
    let bounds = Rect::new(20.0, 20.0, 60.0, 40.0);
    let actual = render(Matrix::IDENTITY, |context| {
        context.push_opacity(0.5, None);
        let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
        // An offset alone: grey everywhere in the bounds.
        assert!(layers.push_pixel_operation(bounds, &PixelOperation::Arithmetic { k: [0.0, 0.0, 0.0, 1.0] }));
        first_source(layers);
        layers.next_pixel_operation_source();
        second_source(layers);
        layers.pop_pixel_operation();
        context.pop_opacity();
    });
    let at = |x: usize, y: usize| &actual[(y * WIDTH as usize + x) * 4..(y * WIDTH as usize + x) * 4 + 4];
    assert_eq!(&[0, 0, 0, 0], at(10, 10), "outside the bounds");
    let inside = at(50, 40);
    assert!((i32::from(inside[3]) - 127).abs() <= 1, "white at half opacity: {inside:?}");
}

#[test]
fn moving_lights_and_operations_make_no_image_of_pixels() {
    // The frames of a scroll: the lights move against the visual.
    let before = pixel_image_count();
    for frame in 0..8 {
        let mut map = several_lights();
        map.add(
            point_light(Vector3D::new(60.0, 4.0 * frame as f64, 50.0), LightColor::WHITE, LightAttenuation::NONE),
            Matrix::create_translation(0.0, 3.0 * frame as f64),
        );
        render(Matrix::create_translation(0.0, -2.5 * frame as f64), |context| {
            gpu_scene(context, RECT, &map, &SHINY);
            let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
            assert!(layers.push_lighting_multiply_layer(RECT, &LightingParameters::new(&map, LightingMaterial::VISUAL)));
            content(layers, RECT);
            layers.pop_multiply_layer();
            let operation = PixelOperation::Blend {
                mode: BlendEffectMode::Dissolve,
                noise_grid: RECT,
                noise_size: PixelSize::new(140, 100),
            };
            assert!(layers.push_pixel_operation(RECT, &operation));
            first_source(layers);
            layers.next_pixel_operation_source();
            second_source(layers);
            layers.pop_pixel_operation();
        });
    }
    assert_eq!(before, pixel_image_count(), "no image of pixels was made");
    // The CPU path makes one a frame.
    render(Matrix::IDENTITY, |context| cpu_scene(context, RECT, &several_lights(), &SHINY));
    assert_eq!(before + 1, pixel_image_count());
}

#[test]
fn without_runtime_effects_the_drawing_context_reports_it_did_not_draw() {
    let previous = crate::runtime_effects::set_enabled(false);
    let map = several_lights();
    render(Matrix::IDENTITY, |context| {
        let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
        assert!(!layers.draw_lighting(RECT, &LightingParameters::new(&map, SHINY)));
        assert!(!layers.push_lighting_multiply_layer(RECT, &LightingParameters::new(&map, SHINY)));
        assert!(!layers.push_pixel_operation(RECT, &PixelOperation::MaskInvert));
        assert!(!layers.push_pixel_operation(RECT, &PixelOperation::Arithmetic { k: [1.0, 0.0, 0.0, 0.0] }));
    });
    crate::runtime_effects::set_enabled(previous);
    // More lights than the shader has.
    let mut many = LightMap::new();
    for index in 0..=LightingParameters::MAX_LIGHTS {
        many.add(point_light(Vector3D::new(index as f64, 0.0, 10.0), LightColor::WHITE, LightAttenuation::NONE), Matrix::IDENTITY);
    }
    render(Matrix::IDENTITY, |context| {
        let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
        assert!(!layers.draw_lighting(RECT, &LightingParameters::new(&many, SHINY)));
    });
}

/// Scenes of every GPU form, for the backends that draw on a GPU device
/// (`gpu/metal/tests.rs`): each draws on a context of `WIDTH` by `HEIGHT`
/// pixels.
#[allow(dead_code)]
pub(crate) fn gpu_scenes() -> Vec<(&'static str, Box<dyn Fn(&mut dyn IDrawingContextImpl)>)> {
    fn operation(operation: PixelOperation, sources: &'static [fn(&mut dyn IDrawingContextImpl)]) -> Box<dyn Fn(&mut dyn IDrawingContextImpl)> {
        Box::new(move |context: &mut dyn IDrawingContextImpl| {
            let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
            assert!(layers.push_pixel_operation(full(), &operation));
            for (index, source) in sources.iter().enumerate() {
                if index > 0 {
                    layers.next_pixel_operation_source();
                }
                source(layers);
            }
            layers.pop_pixel_operation();
        })
    }
    const TWO: &[fn(&mut dyn IDrawingContextImpl)] = &[first_source, second_source];
    const THREE: &[fn(&mut dyn IDrawingContextImpl)] = &[first_source, second_source, third_source];
    let grid = (full(), PixelSize::new(WIDTH, HEIGHT));
    vec![
        (
            "scene lighting, rotated",
            Box::new(|context: &mut dyn IDrawingContextImpl| {
                context.set_transform(Matrix::create_rotation(0.3) * Matrix::create_translation(40.0, 0.0));
                gpu_scene(context, Rect::new(0.0, 0.0, 100.0, 80.0), &several_lights(), &SHINY);
            }),
        ),
        (
            "multiply layer above full light",
            Box::new(|context: &mut dyn IDrawingContextImpl| {
                let mut map = several_lights();
                map.add(LightSource::Ambient { color: color(0.8, 1.2, 0.6) }, Matrix::IDENTITY);
                let layers = context.as_drawing_context_impl_with_layer_effects().unwrap();
                assert!(layers.push_lighting_multiply_layer(RECT, &LightingParameters::new(&map, LightingMaterial::VISUAL)));
                content(layers, RECT);
                layers.pop_multiply_layer();
            }),
        ),
        ("vivid light", operation(PixelOperation::Blend { mode: BlendEffectMode::VividLight, noise_grid: grid.0, noise_size: grid.1 }, TWO)),
        ("darker colour", operation(PixelOperation::Blend { mode: BlendEffectMode::DarkerColor, noise_grid: grid.0, noise_size: grid.1 }, TWO)),
        ("dissolve", operation(PixelOperation::Blend { mode: BlendEffectMode::Dissolve, noise_grid: grid.0, noise_size: grid.1 }, TWO)),
        ("arithmetic", operation(PixelOperation::Arithmetic { k: [0.5, 0.75, 0.6, 0.1] }, TWO)),
        ("mask invert", operation(PixelOperation::MaskInvert, THREE)),
    ]
}
