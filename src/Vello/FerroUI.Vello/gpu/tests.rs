//! Tests of the hybrid and the GPU mode on a device without a surface.
//!
//! They need a graphics adapter. On a machine without one every test here
//! prints `skipped: no adapter` with the reason and passes, so that the
//! suite can run anywhere; [`a_graphics_device_is_present_when_demanded`]
//! is the one test that fails without an adapter, and only when the
//! environment variable `FERROUI_VELLO_REQUIRE_GPU` demands one: a machine
//! that is meant to test the GPU modes sets it, and a run on it cannot be
//! green without having drawn.

use crate::gpu::VelloWgpuDevice;
use crate::helpers::pixel_format_helper::to_image;
use crate::scene::{
    IVelloSceneSink, VelloCpuSceneSink, VelloSceneBrush, VelloSceneImage, VelloScenePaint,
};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::PixelSize;
use kurbo::{Affine, BezPath, Cap, Circle, Ellipse, Join, Rect, RoundedRect, Shape, Stroke};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Color, ColorStop, Compose, Extend, Fill, Gradient, ImageQuality, Mix};
use std::sync::Arc;

/// The environment variable that demands a graphics adapter: set to
/// anything but nothing and `0`.
const REQUIRE_GPU: &str = "FERROUI_VELLO_REQUIRE_GPU";

const SIZE: u16 = 100;

fn gpu_is_demanded() -> bool {
    std::env::var(REQUIRE_GPU).is_ok_and(|value| !value.is_empty() && value != "0")
}

/// The device of the tests, or `None` with the message every skipped test
/// prints.
fn test_device(test: &str) -> Option<Arc<VelloWgpuDevice>> {
    match VelloWgpuDevice::headless() {
        Ok(device) => Some(device),
        Err(error) => {
            println!("skipped: no adapter ({test}): {error}");
            None
        }
    }
}

/// The sinks of the GPU modes that are built and that the device can run,
/// with their names.
fn gpu_sinks(test: &str) -> Vec<(&'static str, Box<dyn IVelloSceneSink>)> {
    let Some(device) = test_device(test) else {
        return Vec::new();
    };
    let mut sinks: Vec<(&'static str, Box<dyn IVelloSceneSink>)> = Vec::new();

    #[cfg(feature = "hybrid")]
    sinks.push(("hybrid", Box::new(crate::scene::VelloHybridSceneSink::new(device.clone(), SIZE, SIZE))));

    #[cfg(feature = "gpu")]
    if device.supports_compute() {
        sinks.push(("GPU", Box::new(crate::scene::VelloGpuSceneSink::new(device.clone(), SIZE, SIZE))));
    } else {
        println!("skipped: no compute shaders ({test}): the GPU mode needs them, {} has none", device.adapter_info().name);
    }

    let _ = device;
    sinks
}

fn render(sink: &mut dyn IVelloSceneSink) -> Vec<u8> {
    let mut pixels = vec![0u8; usize::from(sink.width()) * usize::from(sink.height()) * 4];
    sink.render_to_pixels(&mut pixels);
    pixels
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let index = (y * usize::from(SIZE) + x) * 4;
    [pixels[index], pixels[index + 1], pixels[index + 2], pixels[index + 3]]
}

/// The share of pixels (in percent) of which a channel differs by more
/// than an eighth of its range, and the largest difference of a channel.
fn difference(a: &[u8], b: &[u8]) -> (f64, u8) {
    let (mut differing, mut largest) = (0usize, 0u8);
    for (pixel_a, pixel_b) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let pixel_largest = pixel_a.iter().zip(pixel_b).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
        if pixel_largest > 32 {
            differing += 1;
        }
        largest = largest.max(pixel_largest);
    }
    (100.0 * differing as f64 / (a.len() / 4) as f64, largest)
}

/// Writes a rendering as a PNG file into the directory the environment
/// variable `FERROUI_VELLO_TEST_DUMP` names, when it names one: for a look
/// at what a mode drew.
fn dump(test: &str, mode: &str, pixels: &[u8]) {
    let Ok(directory) = std::env::var("FERROUI_VELLO_TEST_DUMP") else {
        return;
    };
    let side = (pixels.len() / 4).isqrt() as u32;
    let Ok(file) = std::fs::File::create(std::path::Path::new(&directory).join(format!("{test}-{mode}.png"))) else {
        return;
    };

    let mut straight = pixels.to_vec();
    for pixel in straight.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for channel in &mut pixel[..3] {
                *channel = (u32::from(*channel) * 255 / alpha).min(255) as u8;
            }
        }
    }

    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), side, side);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    if let Ok(mut writer) = encoder.write_header() {
        let _ = writer.write_image_data(&straight);
    }
}

fn close(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= tolerance)
}

/// Draws a scene with the CPU mode and with every GPU mode and asserts that
/// no more than `bound` percent of the pixels differ; returns the pixels of
/// each GPU mode for what a test asserts about them.
fn compare_with_cpu(test: &str, bound: f64, scene: &dyn Fn(&mut dyn IVelloSceneSink)) -> Vec<(&'static str, Vec<u8>)> {
    let mut cpu = VelloCpuSceneSink::new(SIZE, SIZE);
    scene(&mut cpu);
    let reference = render(&mut cpu);
    dump(test, "CPU", &reference);

    gpu_sinks(test)
        .into_iter()
        .map(|(name, mut sink)| {
            scene(&mut *sink);
            let pixels = render(&mut *sink);
            let (share, largest) = difference(&reference, &pixels);
            dump(test, name, &pixels);
            println!("{test}: {name} against CPU: {share:.3} % of the pixels differ, largest difference {largest}");
            assert!(share <= bound, "{test}: the {name} mode differs from the CPU mode in {share:.3} % of the pixels");
            (name, pixels)
        })
        .collect()
}

fn solid(red: u8, green: u8, blue: u8, alpha: u8) -> VelloScenePaint {
    VelloScenePaint::solid(Color::from_rgba8(red, green, blue, alpha))
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
    Rect::new(x0, y0, x1, y1).to_path(0.1)
}

fn target() -> BezPath {
    rect(0.0, 0.0, f64::from(SIZE), f64::from(SIZE))
}

fn fill(sink: &mut dyn IVelloSceneSink, path: &BezPath, paint: &VelloScenePaint) {
    sink.fill(path, Fill::NonZero, Affine::IDENTITY, paint, BlendMode::default(), true);
}

/// How the drawing context of the backend interpolates the stops of a
/// gradient: not premultiplied, as Skia does.
const ALPHA_SPACE: peniko::InterpolationAlphaSpace = peniko::InterpolationAlphaSpace::Unpremultiplied;

const COPY: BlendMode = BlendMode { mix: Mix::Normal, compose: Compose::Copy };

/// The one test that is not green without an adapter, when one is demanded.
#[test]
fn a_graphics_device_is_present_when_demanded() {
    match VelloWgpuDevice::headless() {
        Ok(device) => {
            let info = device.adapter_info();
            println!(
                "adapter: {} ({:?}, {:?}), compute shaders: {}",
                info.name,
                info.backend,
                info.device_type,
                device.supports_compute()
            );
            if gpu_is_demanded() && cfg!(feature = "gpu") {
                assert!(device.supports_compute(), "{REQUIRE_GPU} is set and the adapter runs no compute shaders");
            }
        }
        Err(error) => {
            println!("skipped: no adapter: {error}");
            assert!(!gpu_is_demanded(), "{REQUIRE_GPU} is set and there is no graphics adapter: {error}");
        }
    }
}

#[test]
fn the_device_is_shared_and_reads_back_what_it_was_given() {
    let Some(device) = test_device("the_device_is_shared_and_reads_back_what_it_was_given") else {
        return;
    };
    assert!(Arc::ptr_eq(&device, &VelloWgpuDevice::headless().unwrap()));
    assert!(!device.is_lost());

    let texture = device.create_rgba_texture(3, 2, wgpu::TextureUsages::COPY_DST);
    let written: Vec<u8> = (0..24).collect();
    device.queue().write_texture(
        wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &written,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(12), rows_per_image: Some(2) },
        wgpu::Extent3d { width: 3, height: 2, depth_or_array_layers: 1 },
    );

    let mut read = vec![0u8; 24];
    assert!(device.read_texture(&texture, &mut read));
    assert_eq!(written, read);
}

#[test]
fn a_mode_reports_itself_and_draws_nothing_as_transparent() {
    for (name, mut sink) in gpu_sinks("a_mode_reports_itself_and_draws_nothing_as_transparent") {
        let expected = if name == "hybrid" { VelloRenderingMode::Hybrid } else { VelloRenderingMode::Gpu };
        assert_eq!(expected, sink.rendering_mode());
        assert_eq!((SIZE, SIZE), (sink.width(), sink.height()));
        assert!(sink.capabilities().read_back);

        let pixels = render(&mut *sink);
        assert!(pixels.iter().all(|byte| *byte == 0), "{name}");

        // A scene that was reset draws nothing either.
        fill(&mut *sink, &target(), &solid(255, 0, 0, 255));
        sink.reset();
        assert!(render(&mut *sink).iter().all(|byte| *byte == 0), "{name}");
    }
}

#[test]
fn shapes_fills_and_strokes() {
    let modes = compare_with_cpu("shapes_fills_and_strokes", 1.0, &|sink| {
        fill(sink, &target(), &solid(255, 255, 255, 255));
        fill(sink, &rect(10.0, 10.0, 50.0, 40.0), &solid(20, 40, 120, 255));
        fill(sink, &Ellipse::new((60.0, 60.0), (30.0, 20.0), 0.3).to_path(0.01), &solid(240, 140, 20, 200));
        fill(sink, &RoundedRect::new(55.0, 8.0, 95.0, 38.0, 9.0).to_path(0.01), &solid(0, 150, 136, 255));

        let stroke = Stroke::new(5.0).with_caps(Cap::Round).with_join(Join::Round).with_dashes(2.0, [9.0, 6.0]);
        sink.stroke(&Circle::new((30.0, 70.0), 20.0).to_path(0.01), &stroke, Affine::IDENTITY, &solid(120, 0, 160, 255), true);
        sink.stroke(
            &rect(0.0, 0.0, 20.0, 10.0),
            &Stroke::new(3.0).with_join(Join::Miter),
            Affine::translate((70.0, 80.0)) * Affine::rotate(0.4),
            &solid(0, 0, 0, 255),
            true,
        );

        let mut star = BezPath::new();
        star.move_to((50.0, 45.0));
        for point in [(62.0, 80.0), (30.0, 58.0), (70.0, 58.0), (38.0, 80.0)] {
            star.line_to(point);
        }
        star.close_path();
        sink.fill(&star, Fill::EvenOdd, Affine::IDENTITY, &solid(200, 30, 30, 255), BlendMode::default(), true);
    });

    for (name, pixels) in modes {
        assert!(close(pixel(&pixels, 30, 25), [20, 40, 120, 255], 1), "{name}: {:?}", pixel(&pixels, 30, 25));
        assert!(close(pixel(&pixels, 3, 3), [255, 255, 255, 255], 1), "{name}");
    }
}

#[test]
fn gradients() {
    compare_with_cpu("gradients", 0.5, &|sink| {
        let stops = [
            ColorStop::from((0.0, Color::from_rgba8(255, 0, 0, 255))),
            ColorStop::from((0.5, Color::from_rgba8(255, 255, 0, 255))),
            ColorStop::from((0.7, Color::from_rgba8(0, 0, 255, 128))),
            // The stops end as they begin: a gradient that repeats has no
            // seam, where a pixel is one color or the other.
            ColorStop::from((1.0, Color::from_rgba8(255, 0, 0, 255))),
        ];
        fill(sink, &target(), &solid(255, 255, 255, 255));

        let linear = Gradient::new_linear((0.0, 0.0), (40.0, 0.0)).with_stops(stops).with_interpolation_alpha_space(ALPHA_SPACE).with_extend(Extend::Reflect);
        let paint = VelloScenePaint { brush: VelloSceneBrush::Gradient(linear), transform: Affine::rotate(0.3) };
        fill(sink, &rect(5.0, 5.0, 95.0, 30.0), &paint);

        let radial = Gradient::new_radial((25.0, 60.0), 20.0).with_stops(stops).with_interpolation_alpha_space(ALPHA_SPACE).with_extend(Extend::Repeat);
        let paint = VelloScenePaint { brush: VelloSceneBrush::Gradient(radial), transform: Affine::IDENTITY };
        fill(sink, &rect(5.0, 35.0, 45.0, 95.0), &paint);

        let sweep = Gradient::new_sweep((75.0, 65.0), 0.0, std::f32::consts::TAU).with_stops(stops).with_interpolation_alpha_space(ALPHA_SPACE);
        let paint = VelloScenePaint { brush: VelloSceneBrush::Gradient(sweep), transform: Affine::IDENTITY };
        fill(sink, &Circle::new((75.0, 65.0), 20.0).to_path(0.01), &paint);
    });
}

/// A linear gradient under a transform that does not keep angles: a scale
/// that differs in the two directions after a rotation.
#[test]
fn a_linear_gradient_under_a_transform_that_does_not_keep_angles() {
    compare_with_cpu("a_linear_gradient_under_a_skewing_transform", 0.5, &|sink| {
        let stops = [
            ColorStop::from((0.0, Color::from_rgba8(255, 0, 0, 255))),
            ColorStop::from((0.5, Color::from_rgba8(255, 255, 0, 255))),
            ColorStop::from((1.0, Color::from_rgba8(255, 0, 0, 255))),
        ];
        fill(sink, &target(), &solid(255, 255, 255, 255));

        let linear = Gradient::new_linear((10.0, 0.0), (30.0, 5.0))
            .with_stops(stops)
            .with_interpolation_alpha_space(ALPHA_SPACE)
            .with_extend(Extend::Repeat);
        let paint = VelloScenePaint { brush: VelloSceneBrush::Gradient(linear), transform: Affine::rotate(0.6) };
        let transform = Affine::translate((12.0, -4.0)) * Affine::scale_non_uniform(0.7, 1.4);
        sink.fill(&Ellipse::new((60.0, 40.0), (50.0, 25.0), 0.0).to_path(0.01), Fill::NonZero, transform, &paint, BlendMode::default(), true);
        sink.fill(&rect(5.0, 5.0, 30.0, 60.0), Fill::NonZero, transform, &paint, BlendMode::default(), true);
    });
}

/// A checkerboard of 4 by 4 pixels with a translucent corner.
fn checkerboard() -> peniko::ImageData {
    let mut rgba = Vec::new();
    for y in 0..4u8 {
        for x in 0..4u8 {
            match ((x + y) % 2 == 0, x == 0 && y == 0) {
                (_, true) => rgba.extend_from_slice(&[0, 64, 0, 64]),
                (true, _) => rgba.extend_from_slice(&[200, 30, 30, 255]),
                (false, _) => rgba.extend_from_slice(&[30, 30, 200, 255]),
            }
        }
    }
    to_image(rgba, PixelSize::new(4, 4))
}

#[test]
fn image_paints_with_extend_modes_and_alpha() {
    let image = checkerboard();
    let paint = |x_extend, y_extend, alpha, transform| VelloScenePaint {
        brush: VelloSceneBrush::Image(VelloSceneImage { image: image.clone(), x_extend, y_extend, quality: ImageQuality::Low, alpha }),
        transform,
    };

    let modes = compare_with_cpu("image_paints_with_extend_modes_and_alpha", 0.5, &|sink| {
        fill(sink, &target(), &solid(255, 255, 255, 255));
        // Tiles of 8 by 8 pixels: the image twice as large, repeated.
        fill(sink, &rect(4.0, 4.0, 48.0, 48.0), &paint(Extend::Repeat, Extend::Repeat, 1.0, Affine::translate((4.0, 4.0)) * Affine::scale(2.0)));
        fill(sink, &rect(52.0, 4.0, 96.0, 48.0), &paint(Extend::Reflect, Extend::Pad, 1.0, Affine::translate((52.0, 4.0)) * Affine::scale(3.0)));
        fill(sink, &rect(4.0, 52.0, 96.0, 96.0), &paint(Extend::Pad, Extend::Pad, 0.5, Affine::translate((40.0, 60.0)) * Affine::scale(5.0)));
        // The same image a second time in a frame is the same texture.
        fill(sink, &rect(60.0, 60.0, 68.0, 68.0), &paint(Extend::Pad, Extend::Pad, 1.0, Affine::translate((60.0, 60.0)) * Affine::scale(2.0)));
    });

    for (name, pixels) in modes {
        // The second texel of the first row of the first tile: blue.
        assert!(close(pixel(&pixels, 7, 5), [30, 30, 200, 255], 1), "{name}: {:?}", pixel(&pixels, 7, 5));
        // The translucent texel over white.
        assert!(close(pixel(&pixels, 5, 5), [191, 255, 191, 255], 2), "{name}: {:?}", pixel(&pixels, 5, 5));
    }
}

#[test]
fn a_copy_inside_a_clip_replaces_what_is_below_it_there_and_nowhere_else() {
    let modes = compare_with_cpu("a_copy_inside_a_clip", 0.5, &|sink| {
        fill(sink, &target(), &solid(255, 0, 0, 255));
        sink.push_clip(&rect(20.0, 20.0, 80.0, 80.0), Fill::NonZero, Affine::IDENTITY, false);
        sink.push_clip(&Circle::new((50.0, 50.0), 28.0).to_path(0.01), Fill::NonZero, Affine::IDENTITY, true);
        // What the drawing context's clear does: the whole target, copied.
        sink.fill(&target(), Fill::NonZero, Affine::IDENTITY, &solid(0, 0, 255, 128), COPY, false);
        sink.pop_clip();
        sink.pop_clip();
    });

    for (name, pixels) in modes {
        // Premultiplied translucent blue, not blue over red.
        assert!(close(pixel(&pixels, 50, 50), [0, 0, 128, 128], 1), "{name}: {:?}", pixel(&pixels, 50, 50));
        assert!(close(pixel(&pixels, 10, 10), [255, 0, 0, 255], 1), "{name}: {:?}", pixel(&pixels, 10, 10));
        // Inside the rectangle, outside the circle.
        assert!(close(pixel(&pixels, 22, 22), [255, 0, 0, 255], 1), "{name}: {:?}", pixel(&pixels, 22, 22));
    }
}

#[test]
fn a_transparent_copy_clears() {
    let modes = compare_with_cpu("a_transparent_copy_clears", 0.5, &|sink| {
        fill(sink, &target(), &solid(0, 128, 0, 255));
        sink.push_clip(&RoundedRect::new(10.0, 10.0, 90.0, 60.0, 12.0).to_path(0.01), Fill::NonZero, Affine::IDENTITY, true);
        sink.fill(&target(), Fill::NonZero, Affine::IDENTITY, &solid(0, 0, 0, 0), COPY, false);
        sink.pop_clip();
        // A copy without a clip around it.
        sink.fill(&rect(20.0, 70.0, 80.0, 90.0), Fill::NonZero, Affine::IDENTITY, &solid(255, 255, 0, 64), COPY, true);
    });

    for (name, pixels) in modes {
        assert_eq!([0, 0, 0, 0], pixel(&pixels, 50, 30), "{name}");
        assert!(close(pixel(&pixels, 50, 80), [64, 64, 0, 64], 1), "{name}: {:?}", pixel(&pixels, 50, 80));
        assert!(close(pixel(&pixels, 5, 95), [0, 128, 0, 255], 1), "{name}");
    }
}

#[test]
fn layers_with_opacity_and_a_destination_in_mask() {
    let modes = compare_with_cpu("layers_with_opacity_and_a_destination_in_mask", 1.0, &|sink| {
        fill(sink, &target(), &solid(255, 255, 255, 255));

        // An opacity layer: two overlapping shapes fade as one.
        sink.push_layer(BlendMode::default(), 0.5);
        fill(sink, &rect(10.0, 10.0, 50.0, 50.0), &solid(255, 0, 0, 255));
        fill(sink, &rect(30.0, 30.0, 70.0, 70.0), &solid(0, 0, 255, 255));
        sink.pop_layer();

        // What the drawing context makes of an opacity mask: a layer, and
        // in it a layer composed `DestIn` that holds the mask.
        sink.push_clip(&rect(40.0, 40.0, 100.0, 100.0), Fill::NonZero, Affine::IDENTITY, false);
        sink.push_layer(BlendMode::default(), 1.0);
        fill(sink, &rect(50.0, 50.0, 95.0, 95.0), &solid(0, 128, 0, 255));
        sink.push_layer(BlendMode::new(Mix::Normal, Compose::DestIn), 1.0);
        fill(sink, &Circle::new((72.0, 72.0), 15.0).to_path(0.01), &solid(0, 0, 0, 255));
        sink.pop_layer();
        sink.pop_layer();
        sink.pop_clip();
    });

    for (name, pixels) in modes {
        assert!(close(pixel(&pixels, 20, 20), [255, 128, 128, 255], 2), "{name}: {:?}", pixel(&pixels, 20, 20));
        // Where the shapes overlap the upper one alone shows through.
        assert!(close(pixel(&pixels, 40, 40), [128, 128, 255, 255], 2), "{name}: {:?}", pixel(&pixels, 40, 40));
        assert!(close(pixel(&pixels, 72, 72), [0, 128, 0, 255], 1), "{name}: {:?}", pixel(&pixels, 72, 72));
        // Inside the masked content, outside the mask: what was below.
        assert!(close(pixel(&pixels, 92, 92), [255, 255, 255, 255], 1), "{name}: {:?}", pixel(&pixels, 92, 92));
    }
}

#[test]
fn blended_shapes_inside_clips_see_what_is_below_the_clip() {
    let multiply = BlendMode::new(Mix::Multiply, Compose::SrcOver);
    let plus = BlendMode::new(Mix::Normal, Compose::Plus);

    let modes = compare_with_cpu("blended_shapes_inside_clips", 0.5, &|sink| {
        fill(sink, &target(), &solid(200, 200, 100, 255));
        sink.push_clip(&rect(10.0, 10.0, 90.0, 90.0), Fill::NonZero, Affine::IDENTITY, false);
        sink.fill(&rect(0.0, 0.0, 50.0, 100.0), Fill::NonZero, Affine::IDENTITY, &solid(128, 255, 255, 255), multiply, true);
        sink.fill(&rect(60.0, 20.0, 100.0, 80.0), Fill::NonZero, Affine::IDENTITY, &solid(40, 40, 40, 0), plus, true);
        sink.pop_clip();

        // A blended layer inside a clip.
        sink.push_clip(&rect(0.0, 92.0, 100.0, 100.0), Fill::NonZero, Affine::IDENTITY, false);
        sink.push_layer(multiply, 1.0);
        fill(sink, &rect(0.0, 0.0, 50.0, 100.0), &solid(255, 128, 255, 255));
        sink.pop_layer();
        sink.pop_clip();
    });

    for (name, pixels) in modes {
        assert!(close(pixel(&pixels, 30, 50), [100, 200, 100, 255], 2), "{name}: {:?}", pixel(&pixels, 30, 50));
        assert!(close(pixel(&pixels, 5, 50), [200, 200, 100, 255], 1), "{name}: {:?}", pixel(&pixels, 5, 50));
        assert!(close(pixel(&pixels, 30, 96), [200, 100, 100, 255], 2), "{name}: {:?}", pixel(&pixels, 30, 96));
        assert!(close(pixel(&pixels, 70, 96), [200, 200, 100, 255], 1), "{name}: {:?}", pixel(&pixels, 70, 96));
    }
}

#[test]
fn aliased_rectangles_cover_the_pixels_whose_centers_they_hold() {
    let scene = |sink: &mut dyn IVelloSceneSink| {
        fill(sink, &target(), &solid(255, 255, 255, 255));
        // Edges at 10.4 and 60.6: the pixels 10 to 60 are covered.
        sink.fill(&rect(10.4, 10.4, 60.6, 60.6), Fill::NonZero, Affine::IDENTITY, &solid(0, 0, 0, 255), BlendMode::default(), false);
        // A clip with a scale and a translation.
        sink.push_clip(&rect(0.0, 0.0, 10.3, 10.3), Fill::NonZero, Affine::translate((65.2, 65.2)) * Affine::scale(2.0), false);
        fill(sink, &target(), &solid(0, 0, 255, 255));
        sink.pop_clip();
    };

    let mut cpu = VelloCpuSceneSink::new(SIZE, SIZE);
    scene(&mut cpu);
    let reference = render(&mut cpu);
    assert_eq!([0, 0, 0, 255], pixel(&reference, 10, 30));
    assert_eq!([255, 255, 255, 255], pixel(&reference, 9, 30));
    assert_eq!([0, 0, 0, 255], pixel(&reference, 60, 30));
    assert_eq!([255, 255, 255, 255], pixel(&reference, 61, 30));

    for (name, mut sink) in gpu_sinks("aliased_rectangles") {
        scene(&mut *sink);
        let pixels = render(&mut *sink);
        let differing = reference.chunks_exact(4).zip(pixels.chunks_exact(4)).filter(|(a, b)| a != b).count();
        println!("aliased_rectangles: {name} against CPU: {differing} pixels differ");
        // The sparse-strips renderers paint a pixel of an aliased edge from
        // half of its area; the GPU mode has no aliased edges and gets a
        // rectangle snapped to the pixels whose centers it holds. The two
        // rules disagree about the pixel of a corner at most.
        let corners = if name == "GPU" { 8 } else { 0 };
        assert!(differing <= corners, "{name}: an aliased rectangle is the same pixels in every mode but for its corners");
        assert_eq!([0, 0, 0, 255], pixel(&pixels, 60, 30), "{name}");
        assert_eq!([255, 255, 255, 255], pixel(&pixels, 61, 30), "{name}");
        assert_eq!([0, 0, 255, 255], pixel(&pixels, 85, 70), "{name}");
    }
}

#[test]
fn a_scene_is_rendered_twice_to_the_same_pixels_and_sizes_may_differ() {
    let Some(device) = test_device("a_scene_is_rendered_twice") else {
        return;
    };
    let scene = |sink: &mut dyn IVelloSceneSink| {
        let (width, height) = (f64::from(sink.width()), f64::from(sink.height()));
        fill(sink, &rect(0.0, 0.0, width, height), &solid(255, 255, 255, 255));
        fill(sink, &Ellipse::new((width / 2.0, height / 2.0), (width / 3.0, height / 4.0), 0.0).to_path(0.01), &solid(200, 0, 0, 255));
    };

    for (width, height) in [(64u16, 48u16), (300, 20), (1, 1), (257, 513)] {
        let mut sinks: Vec<(&str, Box<dyn IVelloSceneSink>)> = Vec::new();
        #[cfg(feature = "hybrid")]
        sinks.push(("hybrid", Box::new(crate::scene::VelloHybridSceneSink::new(device.clone(), width, height))));
        #[cfg(feature = "gpu")]
        if device.supports_compute() {
            sinks.push(("GPU", Box::new(crate::scene::VelloGpuSceneSink::new(device.clone(), width, height))));
        }

        for (name, mut sink) in sinks {
            scene(&mut *sink);
            let first = render(&mut *sink);
            let second = render(&mut *sink);
            assert!(first == second, "{name} at {width}x{height}");
            assert_eq!(usize::from(width) * usize::from(height) * 4, first.len());
            let center = (usize::from(height) / 2 * usize::from(width) + usize::from(width) / 2) * 4;
            assert!(close([first[center], first[center + 1], first[center + 2], first[center + 3]], [200, 0, 0, 255], 1) || width == 1, "{name}");
        }
    }
}

/// The modes through the factory of the scene module: what a render target
/// asks for.
#[test]
fn the_factory_gives_each_mode_that_has_a_device() {
    let device = test_device("the_factory_gives_each_mode_that_has_a_device");

    for (mode, built) in [(VelloRenderingMode::Hybrid, cfg!(feature = "hybrid")), (VelloRenderingMode::Gpu, cfg!(feature = "gpu"))] {
        let sink = crate::scene::try_create_scene_sink(mode, 8, 8);
        let available = built && device.as_ref().is_some_and(|device| mode != VelloRenderingMode::Gpu || device.supports_compute());
        match sink {
            Ok(sink) => {
                assert!(available, "{mode:?}");
                assert_eq!(mode, sink.rendering_mode());
            }
            Err(unavailable) => {
                assert!(!available, "{unavailable}");
                assert_eq!(mode, unavailable.mode);
                println!("{unavailable}");
            }
        }
    }

    // Whatever the machine has, the default order ends at a mode that draws.
    let sink = crate::scene::create_scene_sink(&crate::VelloOptions::default().rendering_mode_order(), 8, 8);
    let _: AlphaColor<Srgb> = Color::TRANSPARENT;
    assert!(sink.capabilities().read_back);
}

/// The measure the anti-aliasing of the GPU mode was chosen by: the mean
/// difference from the CPU mode of a scene of curved and slanted edges,
/// for each method the compute renderer has.
#[cfg(feature = "gpu")]
#[test]
fn area_coverage_is_the_anti_aliasing_nearest_to_the_cpu_mode() {
    use crate::scene::{VelloGpuAntiAliasing, VelloGpuSceneSink};

    let Some(device) = test_device("area_coverage_is_the_anti_aliasing_nearest_to_the_cpu_mode") else {
        return;
    };
    if !device.supports_compute() {
        println!("skipped: no compute shaders (area_coverage_is_the_anti_aliasing_nearest_to_the_cpu_mode)");
        return;
    }

    let scene = |sink: &mut dyn IVelloSceneSink| {
        fill(sink, &target(), &solid(255, 255, 255, 255));
        fill(sink, &Ellipse::new((35.0, 35.0), (28.0, 17.0), 0.5).to_path(0.01), &solid(20, 40, 120, 255));
        sink.fill(
            &rect(0.0, 0.0, 40.0, 25.0),
            Fill::NonZero,
            Affine::translate((50.0, 55.0)) * Affine::rotate(0.2),
            &solid(200, 30, 30, 255),
            BlendMode::default(),
            true,
        );
        for index in 0..12 {
            let mut line = BezPath::new();
            line.move_to((5.0, 60.0 + f64::from(index) * 3.0));
            line.line_to((45.0, 62.0 + f64::from(index) * 3.3));
            sink.stroke(&line, &Stroke::new(1.0), Affine::IDENTITY, &solid(0, 0, 0, 255), true);
        }
    };

    let mut cpu = VelloCpuSceneSink::new(SIZE, SIZE);
    scene(&mut cpu);
    let reference = render(&mut cpu);

    let mean = |anti_aliasing| {
        let mut sink = VelloGpuSceneSink::with_anti_aliasing(device.clone(), SIZE, SIZE, anti_aliasing);
        scene(&mut sink);
        let pixels = render(&mut sink);
        let sum: u64 = reference.iter().zip(&pixels).map(|(a, b)| u64::from(a.abs_diff(*b))).sum();
        let (share, largest) = difference(&reference, &pixels);
        let mean = sum as f64 / reference.len() as f64;
        println!(
            "anti-aliasing {anti_aliasing:?} against CPU: mean difference {mean:.4}, largest {largest}, beyond the tolerance {share:.3} %"
        );
        mean
    };

    let area = mean(VelloGpuAntiAliasing::Area);
    let msaa8 = mean(VelloGpuAntiAliasing::Msaa8);
    let msaa16 = mean(VelloGpuAntiAliasing::Msaa16);
    assert!(area <= msaa8 && area <= msaa16, "area {area}, 8 samples {msaa8}, 16 samples {msaa16}");
}
