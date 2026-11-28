//! The route of a box shadow in the GPU modes (#1139): no shadow reaches a scene of the CPU
//! renderer, whatever its shape. The shapes are the ones with a closed form and the ones without:
//! a box too small for its blur, corners that are not circular, and an inset shadow.
//!
//! Skipped with a message on a machine without a graphics adapter.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::VelloWgpuDevice;
use crate::scene::{cpu_scenes_made, IVelloSceneSink, VelloCpuSceneSink};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::{BoxShadow, BoxShadows, Color};
use ferroui_base::platform::IDrawingContextImpl;
#[cfg(feature = "gpu")]
use ferroui_base::Point;
use ferroui_base::{Matrix, Rect, RoundedRect, Vector};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SIZE: u16 = 96;

fn device_modes(device: &Arc<VelloWgpuDevice>) -> Vec<VelloRenderingMode> {
    let mut modes = Vec::new();
    if cfg!(feature = "hybrid") {
        modes.push(VelloRenderingMode::Hybrid);
    }
    if cfg!(feature = "gpu") && device.supports_compute() {
        modes.push(VelloRenderingMode::Gpu);
    }
    modes
}

fn shadow(blur: f64, is_inset: bool) -> BoxShadows {
    BoxShadows::new(BoxShadow {
        offset_x: 2.0,
        offset_y: 3.0,
        blur,
        color: Color::from_argb(200, 0, 0, 0),
        is_inset,
        ..BoxShadow::default()
    })
}

/// A name, a box and its shadows.
fn shapes() -> Vec<(&'static str, RoundedRect, BoxShadows)> {
    let corner = Vector::new(16.0, 8.0);
    vec![
        ("closed form", RoundedRect::from_radius(Rect::new(24.0, 24.0, 48.0, 48.0), 6.0), shadow(12.0, false)),
        ("too small for its blur", RoundedRect::from_radius(Rect::new(20.0, 40.0, 56.0, 14.0), 7.0), shadow(20.0, false)),
        (
            "elliptical corners",
            RoundedRect::new(Rect::new(20.0, 24.0, 56.0, 48.0), corner, corner, corner, corner),
            shadow(12.0, false),
        ),
        ("inset", RoundedRect::from_radius(Rect::new(16.0, 16.0, 64.0, 64.0), 8.0), shadow(12.0, true)),
    ]
}

fn draw(device: &Arc<VelloWgpuDevice>, mode: VelloRenderingMode, shape: RoundedRect, shadows: &BoxShadows) -> Vec<u8> {
    draw_under(device, mode, shape, shadows, Matrix::IDENTITY)
}

fn draw_under(
    device: &Arc<VelloWgpuDevice>,
    mode: VelloRenderingMode,
    shape: RoundedRect,
    shadows: &BoxShadows,
    transform: Matrix,
) -> Vec<u8> {
    let sink: Box<dyn IVelloSceneSink> =
        crate::gpu::create_window_scene_sink(device, &[mode], SIZE, SIZE, wgpu::TextureFormat::Rgba8Unorm);
    draw_into_under(sink, mode, shape, shadows, transform)
}

#[cfg(feature = "gpu")]
fn draw_into(
    sink: Box<dyn IVelloSceneSink>,
    mode: VelloRenderingMode,
    shape: RoundedRect,
    shadows: &BoxShadows,
) -> Vec<u8> {
    draw_into_under(sink, mode, shape, shadows, Matrix::IDENTITY)
}

fn draw_into_under(
    sink: Box<dyn IVelloSceneSink>,
    mode: VelloRenderingMode,
    shape: RoundedRect,
    shadows: &BoxShadows,
    transform: Matrix,
) -> Vec<u8> {
    let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
    let target = pixels.clone();
    let mut drawing = DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| sink.render_to_pixels(&mut target.borrow_mut())),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes: vec![mode],
        },
        Vec::new(),
    );
    drawing.set_transform(transform);
    drawing.draw_rectangle(None, None, shape, shadows);
    drawing.dispose();
    pixels.take()
}

#[test]
fn no_shadow_reaches_a_scene_of_the_cpu_renderer_in_the_device_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    for mode in device_modes(&device) {
        for (name, shape, shadows) in shapes() {
            let before = cpu_scenes_made();
            let pixels = draw(&device, mode, shape, &shadows);
            assert_eq!(before, cpu_scenes_made(), "{mode:?}, {name}: a CPU scene was made");
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0), "{mode:?}, {name}: nothing was drawn");
        }
    }
}

/// `ALLOWED_ERROR` of the render tests.
const RENDER_TEST_ALLOWED_ERROR: f64 = 0.022;

/// The error of the render tests (`compare_images`) between two images of premultiplied pixels:
/// the root mean square of the difference of every channel, over the whole image.
fn render_test_error(actual: &[u8], expected: &[u8]) -> f64 {
    assert_eq!(actual.len(), expected.len());
    let squares: f64 = actual
        .iter()
        .zip(expected)
        .map(|(a, e)| ((f64::from(*a) - f64::from(*e)) / 255.0).powi(2))
        .sum();
    (squares / actual.len() as f64).sqrt()
}

/// The blur on the device of the compute mode against the CPU reference, at deviations it blurs
/// directly and at deviations it blurs at a reduced scale, away from the edge of the box.
#[cfg(feature = "gpu")]
#[test]
fn the_device_blur_of_the_compute_mode_agrees_with_the_cpu_reference() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    if !device.supports_compute() {
        println!("skipped: no compute");
        return;
    }
    let corner = Vector::new(16.0, 8.0);
    let thin = RoundedRect::from_radius(Rect::new(20.0, 40.0, 56.0, 14.0), 7.0);
    let elliptical = RoundedRect::new(Rect::new(20.0, 24.0, 56.0, 48.0), corner, corner, corner, corner);
    let inset = RoundedRect::from_radius(Rect::new(16.0, 16.0, 64.0, 64.0), 8.0);
    // Too long for one kept picture at its deviation: blurred for the area it shows.
    let long = RoundedRect::from_radius(Rect::new(-1500.0, 40.0, 3096.0, 14.0), 7.0);
    for (name, shape, blur, is_inset) in [
        ("too long to keep", long, 3.0, false),
        ("thin, direct", thin, 10.0, false),
        ("thin, reduced by 2", thin, 20.0, false),
        ("thin, reduced by 4", thin, 40.0, false),
        ("thin, reduced by 6", thin, 80.0, false),
        ("elliptical, direct", elliptical, 12.0, false),
        ("elliptical, reduced", elliptical, 40.0, false),
        ("inset, direct", inset, 12.0, true),
        ("inset, reduced", inset, 40.0, true),
    ] {
        let shadows = shadow(blur, is_inset);
        let compute = draw(&device, VelloRenderingMode::Gpu, shape, &shadows);
        let reference = draw_into(
            Box::new(VelloCpuSceneSink::single_threaded(SIZE, SIZE)),
            VelloRenderingMode::Cpu,
            shape,
            &shadows,
        );
        let edge = shape.rect.inflate(2.0);
        let inner = shape.rect.inflate(-2.0);
        let mut largest = 0;
        for y in 0..usize::from(SIZE) {
            for x in 0..usize::from(SIZE) {
                let at = Point::new(x as f64 + 0.5, y as f64 + 0.5);
                if edge.contains(at) && !inner.contains(at) {
                    continue;
                }
                let index = (y * usize::from(SIZE) + x) * 4;
                for channel in 0..4 {
                    largest = largest.max(compute[index + channel].abs_diff(reference[index + channel]));
                }
            }
        }
        assert!(largest <= 4, "{name}: {largest} of 255");
        let error = render_test_error(&compute, &reference);
        println!("{name}: {largest} of 255 away from the edge, error {error:.4} over the image");
        assert!(error <= RENDER_TEST_ALLOWED_ERROR, "{name}: error {error}");
    }
}

/// The closed form of the compute mode against the CPU reference, away from the edge of the box,
/// where the two renderers anti-alias the clip that keeps the shadow out of it differently.
#[cfg(feature = "gpu")]
#[test]
fn the_closed_form_of_the_compute_mode_agrees_with_the_cpu_reference() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    if !device.supports_compute() {
        println!("skipped: no compute");
        return;
    }
    let (_, shape, shadows) = shapes().swap_remove(0);
    let sink = crate::gpu::create_window_scene_sink(
        &device,
        &[VelloRenderingMode::Gpu],
        SIZE,
        SIZE,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    assert!(sink.filter_capabilities().blurred_rounded_rects, "the compute mode has the closed form");
    let compute = draw_into(sink, VelloRenderingMode::Gpu, shape, &shadows);
    let reference = draw_into(
        Box::new(VelloCpuSceneSink::single_threaded(SIZE, SIZE)),
        VelloRenderingMode::Cpu,
        shape,
        &shadows,
    );
    let edge = shape.rect.inflate(2.0);
    let inner = shape.rect.inflate(-2.0);
    let mut largest = 0;
    for y in 0..usize::from(SIZE) {
        for x in 0..usize::from(SIZE) {
            let at = Point::new(x as f64 + 0.5, y as f64 + 0.5);
            if edge.contains(at) && !inner.contains(at) {
                continue;
            }
            let index = (y * usize::from(SIZE) + x) * 4;
            for channel in 0..4 {
                largest = largest.max(compute[index + channel].abs_diff(reference[index + channel]));
            }
        }
    }
    assert!(largest <= 1, "{largest} of 255");
}

/// A shadow under an animated scale, and turned, is blurred once in each device mode and stays
/// within the error of the render tests of the CPU reference at every frame.
#[test]
fn an_animated_transform_reuses_one_picture_and_agrees_with_the_cpu_reference() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let corner = Vector::new(16.0, 8.0);
    let cases = [
        ("thin", RoundedRect::from_radius(Rect::new(20.0, 40.0, 56.0, 14.0), 7.0), shadow(20.0, false)),
        ("elliptical", RoundedRect::new(Rect::new(20.0, 24.0, 56.0, 48.0), corner, corner, corner, corner), shadow(12.0, false)),
        ("inset", RoundedRect::from_radius(Rect::new(16.0, 16.0, 64.0, 64.0), 8.0), shadow(12.0, true)),
    ];
    let about_centre = |matrix: Matrix| {
        Matrix::create_translation(-48.0, -48.0) * matrix * Matrix::create_translation(48.0, 48.0)
    };
    let frames: Vec<Matrix> = [0.85, 0.88, 0.91, 0.94, 0.97, 1.0, 1.13]
        .into_iter()
        .map(|scale| about_centre(Matrix::create_scale(scale, scale)))
        .chain([about_centre(Matrix::create_rotation(0.3))])
        .collect();
    for mode in device_modes(&device) {
        for (name, shape, shadows) in &cases {
            crate::gpu::shadow_blur::forget_pictures();
            let before = crate::gpu::shadow_blur::pictures_made();
            let scenes_before = cpu_scenes_made();
            let mut drawn = Vec::new();
            for frame in &frames {
                drawn.push(draw_under(&device, mode, *shape, shadows, *frame));
            }
            assert_eq!(scenes_before, cpu_scenes_made(), "{mode:?}, {name}: a CPU scene was made");
            let made = crate::gpu::shadow_blur::pictures_made() - before;
            assert_eq!(made, 1, "{mode:?}, {name}: {made} pictures for {} frames", frames.len());
            let mut worst: f64 = 0.0;
            for (frame, pixels) in frames.iter().zip(&drawn) {
                let reference = draw_into_under(
                    Box::new(VelloCpuSceneSink::single_threaded(SIZE, SIZE)),
                    VelloRenderingMode::Cpu,
                    *shape,
                    shadows,
                    *frame,
                );
                worst = worst.max(render_test_error(pixels, &reference));
            }
            println!("{mode:?}, {name}: {made} picture(s), worst error {worst:.4}");
            assert!(worst <= RENDER_TEST_ALLOWED_ERROR, "{mode:?}, {name}: error {worst}");
        }
    }
}

/// Coloured content with hard edges and a translucent part, for the blur of colours.
#[cfg(feature = "gpu")]
fn coloured_content(scene: &mut dyn IVelloSceneSink) {
    use kurbo::Shape;
    use peniko::{BlendMode, Fill};
    let mut fill = |rect: kurbo::Rect, color: peniko::Color| {
        let paint = crate::scene::VelloScenePaint::solid(color);
        scene.fill(&rect.to_path(0.1), Fill::NonZero, kurbo::Affine::IDENTITY, &paint, BlendMode::default(), false);
    };
    fill(kurbo::Rect::new(28.0, 28.0, 48.0, 68.0), peniko::Color::from_rgba8(255, 0, 0, 255));
    fill(kurbo::Rect::new(48.0, 28.0, 68.0, 48.0), peniko::Color::from_rgba8(0, 128, 255, 255));
    fill(kurbo::Rect::new(48.0, 48.0, 68.0, 68.0), peniko::Color::from_rgba8(255, 160, 0, 128));
}

/// The texture of the pool holding the content, drawn by the compute renderer.
#[cfg(feature = "gpu")]
fn content_texture(device: &Arc<VelloWgpuDevice>) -> Arc<crate::gpu::VelloDeviceTexture> {
    let mut scene = crate::scene::VelloGpuSceneSink::new(device.clone(), SIZE, SIZE);
    coloured_content(&mut scene);
    let texture = crate::gpu::effect_passes::pooled_texture(device, u32::from(SIZE), u32::from(SIZE));
    scene.render_to_texture(&crate::gpu::VelloGpuTexture { texture: texture.texture() }).unwrap();
    texture
}

/// The textures painted one texel per pixel in order, each at its offset, read back.
#[cfg(feature = "gpu")]
fn painted(device: &Arc<VelloWgpuDevice>, layers: &[(Arc<crate::gpu::VelloDeviceTexture>, (f64, f64))]) -> Vec<u8> {
    use kurbo::Shape;
    let mut scene = crate::scene::VelloGpuSceneSink::new(device.clone(), SIZE, SIZE);
    let whole = kurbo::Rect::new(0.0, 0.0, f64::from(SIZE), f64::from(SIZE)).to_path(0.1);
    for (texture, offset) in layers {
        let paint = crate::scene::VelloScenePaint {
            brush: crate::scene::VelloSceneBrush::Texture(crate::scene::VelloSceneTexture {
                texture: texture.clone(),
                x_extend: peniko::Extend::Pad,
                y_extend: peniko::Extend::Pad,
                quality: peniko::ImageQuality::Low,
                alpha: 1.0,
            }),
            transform: kurbo::Affine::translate(*offset),
        };
        scene.fill(&whole, peniko::Fill::NonZero, kurbo::Affine::IDENTITY, &paint, peniko::BlendMode::default(), false);
    }
    let mut pixels = vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4];
    scene.render_to_pixels(&mut pixels);
    pixels
}

/// The filter layer of the CPU renderer over the same content: the reference.
#[cfg(feature = "gpu")]
fn through_cpu_filter(filter: crate::scene::VelloSceneFilter) -> Vec<u8> {
    let mut scene = VelloCpuSceneSink::single_threaded(SIZE, SIZE);
    scene.push_filter_layer(&filter, kurbo::Affine::IDENTITY);
    coloured_content(&mut scene);
    scene.pop_layer();
    let mut pixels = vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4];
    scene.render_to_pixels(&mut pixels);
    pixels
}

#[cfg(feature = "gpu")]
fn largest_difference(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
}

/// The blur of colours and the shadow of a content on the device against the filters of the CPU
/// renderer, at deviations it blurs directly and at a reduced scale.
#[cfg(feature = "gpu")]
#[test]
fn the_blur_of_colours_on_the_device_agrees_with_the_cpu_filter() {
    use crate::scene::VelloSceneFilter;
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    if !device.supports_compute() {
        println!("skipped: no compute");
        return;
    }
    let content = content_texture(&device);
    for deviation in [1.5f32, 3.0, 4.0, 6.0, 9.0, 14.0, 24.0] {
        let blurred = crate::gpu::shadow_blur::blurred_content(
            &device,
            &content,
            (f64::from(deviation), f64::from(deviation)),
            None,
        );
        let drawn = painted(&device, &[(blurred, (0.0, 0.0))]);
        let reference = through_cpu_filter(VelloSceneFilter::Blur { std_deviation: deviation });
        let (largest, error) = (largest_difference(&drawn, &reference), render_test_error(&drawn, &reference));
        println!("blur {deviation}: {largest} of 255, error {error:.4}");
        assert!(error <= RENDER_TEST_ALLOWED_ERROR, "blur {deviation}: error {error}");

        let color = peniko::Color::from_rgba8(0, 0, 200, 204);
        let [red, green, blue, alpha] = color.components;
        let tint = [red * alpha, green * alpha, blue * alpha, alpha];
        let shadow = crate::gpu::shadow_blur::blurred_content(
            &device,
            &content,
            (f64::from(deviation), f64::from(deviation)),
            Some(tint),
        );
        let drawn = painted(&device, &[(shadow, (5.0, 3.0)), (content.clone(), (0.0, 0.0))]);
        let reference =
            through_cpu_filter(VelloSceneFilter::DropShadow { dx: 5.0, dy: 3.0, std_deviation: deviation, color });
        let (largest, error) = (largest_difference(&drawn, &reference), render_test_error(&drawn, &reference));
        println!("drop shadow {deviation}: {largest} of 255, error {error:.4}");
        assert!(error <= RENDER_TEST_ALLOWED_ERROR, "drop shadow {deviation}: error {error}");
    }
}

/// The offset of a drop shadow turns with its content in the compute mode, as in the CPU mode.
#[cfg(feature = "gpu")]
#[test]
fn the_drop_shadow_of_the_compute_mode_follows_a_turn() {
    use ferroui_base::media::effects::ImmutableDropShadowEffect;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::Colors;
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    if !device.supports_compute() {
        println!("skipped: no compute");
        return;
    }
    let drawn = |mode: VelloRenderingMode| {
        let sink = crate::gpu::create_window_scene_sink(&device, &[mode], SIZE, SIZE, wgpu::TextureFormat::Rgba8Unorm);
        let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
        let target = pixels.clone();
        let mut context = DrawingContextImpl::new(
            CreateInfo {
                sink,
                backdrop: None,
                on_finished: Box::new(move |sink| sink.render_to_pixels(&mut target.borrow_mut())),
                scale_drawing_to_dpi: false,
                dpi: Vector::new(96.0, 96.0),
                rendering_modes: vec![mode],
            },
            Vec::new(),
        );
        context.set_transform(Matrix::create_rotation(0.6) * Matrix::create_translation(40.0, 16.0));
        let effects = context.as_drawing_context_impl_with_effects().unwrap();
        effects.push_effect(None, &ImmutableDropShadowEffect::new(12.0, 0.0, 3.0, Colors::BLACK, 0.8));
        let brush = ImmutableSolidColorBrush::new(Colors::RED);
        effects.draw_rectangle(Some(&brush), None, RoundedRect::from_rect(Rect::new(0.0, 0.0, 30.0, 30.0)), &BoxShadows::default());
        effects.pop_effect();
        context.dispose();
        let result = pixels.borrow().clone();
        result
    };
    let error = render_test_error(&drawn(VelloRenderingMode::Gpu), &drawn(VelloRenderingMode::Cpu));
    println!("turned drop shadow: error {error:.4}");
    assert!(error <= RENDER_TEST_ALLOWED_ERROR, "error {error}");
}
