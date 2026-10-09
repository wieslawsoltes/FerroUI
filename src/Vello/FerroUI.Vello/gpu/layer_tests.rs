//! Tests of what stays on the device between frames: a texture a scene is
//! composed over, a texture as a paint, and the layer of the contract as a
//! texture ([`DeviceSurfaceRenderTarget`]). Every scene is drawn the same
//! way with surfaces in memory by the CPU mode, and the pixels compared.
//!
//! Skipped with a message on a machine without a graphics adapter, like the
//! tests of `tests.rs`.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::{DeviceSurfaceRenderTarget, VelloDeviceTexture, VelloGpuTexture, VelloTextureAlpha, VelloWgpuDevice};
use crate::scene::{
    IVelloSceneSink, VelloCpuSceneSink, VelloHybridSceneSink, VelloSceneBrush, VelloScenePaint, VelloScenePixelRect,
    VelloSceneTexture,
};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Color, Colors};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Matrix, PixelSize, Rect, RoundedRect, Vector};
use kurbo::{Affine, Shape};
use peniko::{BlendMode, Extend, Fill, ImageQuality};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SIZE: u16 = 64;

fn test_device(test: &str) -> Option<Arc<VelloWgpuDevice>> {
    match VelloWgpuDevice::headless() {
        Ok(device) => Some(device),
        Err(error) => {
            println!("skipped: no adapter ({test}): {error}");
            None
        }
    }
}

fn pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let index = (y * usize::from(SIZE) + x) * 4;
    [pixels[index], pixels[index + 1], pixels[index + 2], pixels[index + 3]]
}

fn largest_difference(a: &[u8], b: &[u8]) -> u8 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
}

fn solid(red: u8, green: u8, blue: u8, alpha: u8) -> VelloScenePaint {
    VelloScenePaint::solid(peniko::Color::from_rgba8(red, green, blue, alpha))
}

fn fill_rect(sink: &mut dyn IVelloSceneSink, rect: kurbo::Rect, paint: &VelloScenePaint) {
    sink.fill(&rect.to_path(0.1), Fill::NonZero, Affine::IDENTITY, paint, BlendMode::default(), false);
}

/// A scene that is rendered into a texture that is kept: the cleared
/// rectangles become transparent, the scene is composed over the rest, in
/// the hybrid mode on the texture as in the CPU mode on pixels.
#[test]
fn a_scene_is_composed_over_what_its_target_holds_but_for_the_cleared_rectangles() {
    let test = "a_scene_is_composed_over_what_its_target_holds_but_for_the_cleared_rectangles";
    let Some(device) = test_device(test) else { return };

    let first = |sink: &mut dyn IVelloSceneSink| {
        fill_rect(sink, kurbo::Rect::new(0.0, 0.0, 64.0, 64.0), &solid(200, 0, 0, 255));
        fill_rect(sink, kurbo::Rect::new(8.0, 8.0, 40.0, 40.0), &solid(0, 0, 200, 128));
    };
    let cleared = [
        VelloScenePixelRect { x0: 16, y0: 16, x1: 48, y1: 32 },
        VelloScenePixelRect { x0: 4, y0: 50, x1: 60, y1: 60 },
    ];
    let second = |sink: &mut dyn IVelloSceneSink| {
        sink.retain_target(&cleared);
        // Half translucent over the cleared and over the kept pixels.
        fill_rect(sink, kurbo::Rect::new(20.0, 10.0, 30.0, 56.0), &solid(0, 200, 0, 128));
    };

    // The CPU mode, on pixels.
    let mut reference = vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4];
    for scene in [&first as &dyn Fn(&mut dyn IVelloSceneSink), &second] {
        let mut sink = VelloCpuSceneSink::new(SIZE, SIZE);
        scene(&mut sink);
        sink.render_to_pixels(&mut reference);
    }
    // Kept, cleared, drawn over what was kept, drawn over what was cleared.
    assert_eq!([200, 0, 0, 255], pixel(&reference, 2, 2));
    assert_eq!([0, 0, 0, 0], pixel(&reference, 40, 20));
    assert_eq!([0, 0, 0, 0], pixel(&reference, 10, 55));
    assert_eq!(255, pixel(&reference, 25, 12)[3]);
    assert!(pixel(&reference, 25, 12)[1] > 90 && pixel(&reference, 25, 12)[0] > 40);
    assert_eq!([0, 100, 0, 128], pixel(&reference, 25, 20));

    // The hybrid mode, on a texture.
    let texture = VelloDeviceTexture::new(&device, u32::from(SIZE), u32::from(SIZE), VelloTextureAlpha::Premultiplied);
    for scene in [&first as &dyn Fn(&mut dyn IVelloSceneSink), &second] {
        let mut sink = VelloHybridSceneSink::new(device.clone(), SIZE, SIZE);
        scene(&mut sink);
        sink.render_to_texture(&VelloGpuTexture { texture: texture.texture() }).unwrap();
    }
    let pixels = texture.read_pixels().expect("the texture is read back");

    let largest = largest_difference(&reference, &pixels);
    println!("{test}: hybrid against CPU: largest difference {largest}");
    assert!(largest <= 2, "the texture holds what the pixels hold: largest difference {largest}");
}

/// A scene that is composed over its target and draws nothing leaves the
/// target as it was.
#[test]
fn a_retained_target_nothing_is_drawn_to_keeps_what_it_holds() {
    let test = "a_retained_target_nothing_is_drawn_to_keeps_what_it_holds";
    let Some(device) = test_device(test) else { return };

    let texture = VelloDeviceTexture::new(&device, u32::from(SIZE), u32::from(SIZE), VelloTextureAlpha::Premultiplied);
    let mut sink = VelloHybridSceneSink::new(device.clone(), SIZE, SIZE);
    fill_rect(&mut sink, kurbo::Rect::new(0.0, 0.0, 64.0, 32.0), &solid(10, 120, 230, 255));
    sink.render_to_texture(&VelloGpuTexture { texture: texture.texture() }).unwrap();
    let before = texture.read_pixels().unwrap();

    let mut sink = VelloHybridSceneSink::new(device.clone(), SIZE, SIZE);
    sink.retain_target(&[]);
    sink.render_to_texture(&VelloGpuTexture { texture: texture.texture() }).unwrap();

    assert_eq!(before, texture.read_pixels().unwrap());
    assert_eq!([10, 120, 230, 255], pixel(&before, 5, 5));
    assert_eq!([0, 0, 0, 0], pixel(&before, 5, 40));
}

/// A texture of the device as a paint: its pixels on the pixels of the
/// target, and with an alpha and a transform.
#[test]
fn a_texture_of_the_device_is_painted_with() {
    let test = "a_texture_of_the_device_is_painted_with";
    let Some(device) = test_device(test) else { return };

    let content = |sink: &mut dyn IVelloSceneSink| {
        fill_rect(sink, kurbo::Rect::new(0.0, 0.0, 32.0, 64.0), &solid(250, 200, 0, 255));
        fill_rect(sink, kurbo::Rect::new(32.0, 0.0, 64.0, 64.0), &solid(0, 60, 180, 128));
    };
    let texture = VelloDeviceTexture::new(&device, u32::from(SIZE), u32::from(SIZE), VelloTextureAlpha::Premultiplied);
    let mut sink = VelloHybridSceneSink::new(device.clone(), SIZE, SIZE);
    content(&mut sink);
    sink.render_to_texture(&VelloGpuTexture { texture: texture.texture() }).unwrap();
    let held = texture.read_pixels().unwrap();

    let brush = |alpha: f32| {
        VelloSceneBrush::Texture(VelloSceneTexture {
            texture: texture.clone(),
            x_extend: Extend::Pad,
            y_extend: Extend::Pad,
            quality: ImageQuality::Low,
            alpha,
        })
    };

    // Pixel on pixel.
    let mut sink = VelloHybridSceneSink::new(device.clone(), SIZE, SIZE);
    assert!(sink.capabilities().device_textures);
    fill_rect(
        &mut sink,
        kurbo::Rect::new(0.0, 0.0, 64.0, 64.0),
        &VelloScenePaint { brush: brush(1.0), transform: Affine::IDENTITY },
    );
    let mut drawn = vec![0u8; held.len()];
    sink.render_to_pixels(&mut drawn);
    assert!(largest_difference(&held, &drawn) <= 1, "the texture is drawn as it is");

    // Moved by eight pixels and at half its alpha, over white.
    let mut sink = VelloHybridSceneSink::new(device.clone(), SIZE, SIZE);
    fill_rect(&mut sink, kurbo::Rect::new(0.0, 0.0, 64.0, 64.0), &solid(255, 255, 255, 255));
    fill_rect(
        &mut sink,
        kurbo::Rect::new(8.0, 0.0, 64.0, 64.0),
        &VelloScenePaint { brush: brush(0.5), transform: Affine::translate((8.0, 0.0)) },
    );
    sink.render_to_pixels(&mut drawn);
    assert_eq!([255, 255, 255, 255], pixel(&drawn, 4, 10));
    let mixed = pixel(&drawn, 20, 10);
    assert!(mixed[0].abs_diff(252) <= 2 && mixed[1].abs_diff(227) <= 2 && mixed[2].abs_diff(127) <= 2, "{mixed:?}");
    assert!(pixel(&drawn, 50, 10)[3] == 255 && pixel(&drawn, 50, 10)[2] > pixel(&drawn, 50, 10)[0]);
}

/// A device and the GPU mode scenes are drawn in on it.
type DeviceMode<'a> = (&'a Arc<VelloWgpuDevice>, VelloRenderingMode);

/// The GPU modes that are built and that the device runs.
fn device_modes(device: &Arc<VelloWgpuDevice>) -> Vec<DeviceMode<'_>> {
    let mut modes = vec![(device, VelloRenderingMode::Hybrid)];
    if cfg!(feature = "gpu") && device.supports_compute() {
        modes.push((device, VelloRenderingMode::Gpu));
    }
    modes
}

/// A drawing context whose scene is rendered into `pixels` when it is
/// disposed, in a GPU mode on a device or, without one, in the CPU mode.
fn window_context(device: Option<DeviceMode<'_>>, pixels: &Rc<RefCell<Vec<u8>>>) -> DrawingContextImpl {
    let (sink, rendering_modes): (Box<dyn IVelloSceneSink>, _) = match device {
        Some((device, mode)) => (
            crate::gpu::create_window_scene_sink(device, &[mode], SIZE, SIZE, wgpu::TextureFormat::Rgba8Unorm),
            vec![mode],
        ),
        None => (Box::new(VelloCpuSceneSink::new(SIZE, SIZE)), vec![VelloRenderingMode::Cpu]),
    };
    let pixels = pixels.clone();

    DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| sink.render_to_pixels(&mut pixels.borrow_mut())),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes,
        },
        Vec::new(),
    )
}

fn brush(color: Color) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(color)
}

fn draw_rect(context: &mut dyn IDrawingContextImpl, rect: Rect, color: Color) {
    context.draw_rectangle(Some(&brush(color)), None, RoundedRect::from_rect(rect), &BoxShadows::default());
}

/// The frames of a layer as the compositor draws them, and one as a render
/// target bitmap is drawn into again: each returns the frame of the window
/// after the layer was blitted onto it.
fn layer_frames(device: Option<DeviceMode<'_>>) -> (Vec<Vec<u8>>, bool) {
    let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
    let mut frames = Vec::new();
    let size = PixelSize::new(i32::from(SIZE), i32::from(SIZE));

    // The layer is created by the context of the first frame of the window.
    let mut window = window_context(device, &pixels);
    let layer = window.create_layer(size);
    let on_device = layer.as_any().downcast_ref::<DeviceSurfaceRenderTarget>().is_some();
    assert_eq!(size, layer.pixel_size());
    assert!(layer.can_blit() && !layer.is_corrupted());

    let present = |window: &mut DrawingContextImpl, frames: &mut Vec<Vec<u8>>| {
        window.clear(Colors::TRANSPARENT);
        window.set_transform(Matrix::IDENTITY);
        layer.blit(window);
        window.dispose();
        frames.push(pixels.borrow().clone());
    };

    // Frame 1: everything is drawn.
    {
        let mut context = layer.create_drawing_context();
        context.push_clip(Rect::new(0.0, 0.0, 64.0, 64.0));
        context.clear(Colors::TRANSPARENT);
        draw_rect(&mut *context, Rect::new(0.0, 0.0, 64.0, 64.0), Color::from_rgb(240, 240, 240));
        draw_rect(&mut *context, Rect::new(4.0, 4.0, 24.0, 24.0), Color::from_rgb(200, 30, 30));
        draw_rect(&mut *context, Rect::new(34.0, 4.0, 24.0, 24.0), Color::from_argb(128, 30, 30, 200));
        context.pop_clip();
        context.dispose();
    }
    present(&mut window, &mut frames);

    // Frame 2: a dirty rectangle is cleared and drawn again, under a
    // transform; the rest of the layer is what frame 1 drew.
    {
        let mut context = layer.create_drawing_context();
        context.push_clip(Rect::new(8.0, 8.0, 40.0, 30.0));
        context.clear(Colors::TRANSPARENT);
        context.set_transform(Matrix::create_translation(2.0, 3.0));
        draw_rect(&mut *context, Rect::new(0.0, 0.0, 30.0, 20.0), Color::from_argb(160, 20, 160, 60));
        context.pop_clip();
        context.dispose();
    }
    let mut window = window_context(device, &pixels);
    present(&mut window, &mut frames);

    // Frame 3: two dirty rectangles as a region, and nothing drawn into the
    // second: it stays transparent.
    {
        let region = crate::VelloRegionImpl::new();
        use ferroui_base::platform::IPlatformRenderInterfaceRegion;
        region.add_rect(ferroui_base::platform::LtrbPixelRect::new(40, 36, 60, 60));
        region.add_rect(ferroui_base::platform::LtrbPixelRect::new(2, 40, 20, 62));
        let mut context = layer.create_drawing_context();
        context.push_clip_region(&region);
        context.clear(Colors::TRANSPARENT);
        draw_rect(&mut *context, Rect::new(30.0, 30.0, 40.0, 40.0), Color::from_rgb(250, 180, 0));
        context.pop_clip();
        context.dispose();
    }
    let mut window = window_context(device, &pixels);
    present(&mut window, &mut frames);

    // Frame 4: drawn into again without a clear, as a bitmap is: what the
    // layer held is under what is drawn, and a clear inside a rounded clip
    // takes out what the layer held there.
    {
        let mut context = layer.create_drawing_context();
        draw_rect(&mut *context, Rect::new(10.0, 44.0, 44.0, 10.0), Color::from_argb(128, 0, 0, 0));
        context.push_clip_rounded(RoundedRect::from_radius(Rect::new(40.0, 2.0, 20.0, 20.0), 8.0));
        context.clear(Colors::TRANSPARENT);
        context.pop_clip();
        context.dispose();
    }
    let mut window = window_context(device, &pixels);
    present(&mut window, &mut frames);

    // Frame 5: a context that draws nothing leaves the layer as it was.
    {
        let mut context = layer.create_drawing_context();
        context.push_clip(Rect::new(0.5, 0.5, 10.0, 10.0));
        context.pop_clip();
        context.dispose();
    }
    let mut window = window_context(device, &pixels);
    present(&mut window, &mut frames);

    // The layer in memory, as the contract asks for it.
    let snapshot = layer.create_shared_snapshot();
    assert_eq!(size, snapshot.pixel_size());
    layer.dispose();

    (frames, on_device)
}

/// The layer of a scene of a GPU mode is a texture of the device, and its
/// frames are those of a layer in memory drawn by the CPU mode: what
/// the layer held stays where nothing is cleared, a dirty rectangle and a
/// region are cleared and drawn again, a layer that is drawn into without a
/// clear keeps its content under what is drawn, and a clear inside a
/// rounded clip takes it out.
#[test]
fn the_layer_of_a_scene_on_a_device_is_a_texture_and_keeps_what_was_drawn_into_it() {
    let test = "the_layer_of_a_scene_on_a_device_is_a_texture_and_keeps_what_was_drawn_into_it";
    let (reference, in_memory_on_device) = layer_frames(None);
    assert!(!in_memory_on_device, "the layer of the CPU mode is in memory");

    // What the frames are, in the CPU mode.
    let frame = |index: usize, x: usize, y: usize| pixel(&reference[index], x, y);
    assert_eq!([200, 30, 30, 255], frame(0, 6, 6));
    // Frame 2: outside the dirty rectangle as before, inside it cleared and
    // the translucent rectangle alone.
    assert_eq!([200, 30, 30, 255], frame(1, 6, 6));
    assert_eq!([240, 240, 240, 255], frame(1, 50, 50));
    assert_eq!(160, frame(1, 20, 20)[3]);
    assert_eq!([0, 0, 0, 0], frame(1, 36, 26));
    // Frame 3: the first rectangle of the region drawn, the second cleared.
    assert_eq!([250, 180, 0, 255], frame(2, 50, 50));
    assert_eq!([0, 0, 0, 0], frame(2, 10, 50));
    assert_eq!([240, 240, 240, 255], frame(2, 30, 50));
    // Frame 4: the shade over what the layer held, and the hole.
    assert_eq!([120, 120, 120, 255], frame(3, 30, 50));
    assert_eq!([0, 0, 0, 0], frame(3, 50, 12));
    assert_eq!([200, 30, 30, 255], frame(3, 6, 6));
    // Frame 5: as frame 4.
    assert_eq!(reference[3], reference[4]);

    let Some(device) = test_device(test) else { return };
    for (device, mode) in device_modes(&device) {
        let (frames, on_device) = layer_frames(Some((device, mode)));
        assert!(on_device, "the layer of the {mode:?} mode is a texture of the device");

        for (index, (frame, reference)) in frames.iter().zip(&reference).enumerate() {
            let largest = largest_difference(frame, reference);
            println!("{test}: {mode:?}, frame {}: largest difference {largest}", index + 1);
            // The edge of the rounded clip of frame 4 is anti-aliased by
            // each renderer; everything else is on whole pixels. The GPU
            // mode keeps colors that are not premultiplied: a translucent
            // pixel is a digit of a color apart.
            let bound = if index >= 3 { 40 } else { 3 };
            assert!(largest <= bound, "{mode:?}: frame {} differs from the CPU mode by {largest}", index + 1);
        }
    }
}

/// A layer on a device is drawn with `draw_bitmap` from its texture, scaled
/// and at an opacity, as a layer in memory is from its pixels.
#[test]
fn a_layer_on_the_device_is_drawn_as_a_bitmap() {
    let test = "a_layer_on_the_device_is_drawn_as_a_bitmap";

    let frame = |device: Option<DeviceMode<'_>>| {
        let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
        let mut window = window_context(device, &pixels);
        let layer = window.create_layer(PixelSize::new(16, 16));
        {
            let mut context = layer.create_drawing_context();
            context.clear(Colors::TRANSPARENT);
            draw_rect(&mut *context, Rect::new(0.0, 0.0, 8.0, 16.0), Color::from_rgb(220, 40, 40));
            draw_rect(&mut *context, Rect::new(8.0, 0.0, 8.0, 16.0), Color::from_rgb(40, 40, 220));
            context.dispose();
        }
        window.clear(Colors::WHITE);
        window.push_opacity(0.5, None);
        window.draw_bitmap(&*layer, 1.0, Rect::new(0.0, 0.0, 16.0, 16.0), Rect::new(8.0, 8.0, 48.0, 48.0));
        window.pop_opacity();
        window.dispose();
        layer.dispose();
        let frame = pixels.borrow().clone();
        frame
    };

    let reference = frame(None);
    let red = pixel(&reference, 20, 30);
    assert!(red[0] > 230 && red[1].abs_diff(147) <= 3 && red[3] == 255, "{red:?}");
    assert_eq!([255, 255, 255, 255], pixel(&reference, 4, 4));

    let Some(device) = test_device(test) else { return };
    for (device, mode) in device_modes(&device) {
        let drawn = frame(Some((device, mode)));
        let largest = largest_difference(&reference, &drawn);
        println!("{test}: {mode:?}: largest difference {largest}");
        assert!(largest <= 3, "{mode:?}");
    }
}
