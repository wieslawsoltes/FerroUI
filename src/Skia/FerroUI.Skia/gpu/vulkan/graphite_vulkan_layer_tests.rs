//! The pipelined frame layer on Graphite Vulkan (#1365, part 3) against an ordinary layer of the
//! same context, on the Vulkan implementation of the machine (lavapipe in the CI job
//! `graphite-vulkan.yml`).
//!
//! On Vulkan the band threads draw on no shared target: whatever mode the environment asks for,
//! the ordered insertion becomes the deferred one (`BandInsert::with_shared_target`), so each band
//! is a recording of its own, on a deferred canvas of the layer's size under the band's clip, placed
//! on the layer's image when it is ready. The frames are compared with the ordinary layer drawing the
//! same frame band by band, each band a recording of its own under the same clip, within the
//! tolerance of the Metal tests: at most a row of pixels per band edge, by at most [`STEP`].
//!
//! What the first runs on lavapipe showed (`diagnostic_where_the_frames_of_lavapipe_differ`): the
//! ordinary path draws a frame the same way twice, band by band and in one recording, and a pipelined
//! layer of one band matches the ordinary layer exactly. With 2 and 4 bands on canvases of the band's
//! size, the shapes that cross a band edge (the stroked line across the layer, a rounded rectangle)
//! were antialiased up to 19 steps apart along the whole shape; the canvases have the layer's size
//! since, so that such a shape is cut by the clip inside the target, as on the ordinary layer.
//!
//! Without a Vulkan loader or device the tests print why they do nothing and pass; with
//! `FERROUI_VULKAN_TEST_REQUIRE=1` they fail instead.

use super::GraphiteVulkanSkiaGpu;
use crate::gpu::graphite::{GraphiteGrContext, PipelinedLayerSurface};
use crate::gpu::{ISkiaGpu, ISkiaGrContext};
use crate::{
    GlyphRunImpl, ISkiaApiLeaseFeature, ImmutableBitmap, SkiaFramePipelining, SurfaceRenderTarget,
    SurfaceRenderTargetCreateInfo, TextRenderingSettings,
};
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{BoxShadows, Colors};
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IOptionalFeatureProvider, IPlatformGraphics,
    IPlatformRenderInterfaceContext,
};
use ferroui_base::{PixelSize, Point, Rect, RoundedRect, Vector};
use ferroui_vulkan::ash::{self, vk};
use ferroui_vulkan::{
    IVulkanPlatformGraphicsContext, VkGetInstanceProcAddressDelegate, VulkanInstanceCreationOptions, VulkanOptions,
    VulkanPlatformGraphics, VulkanPlatformSpecificOptions,
};
use std::any::TypeId;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::Arc;

const WIDTH: i32 = 320;
const HEIGHT: i32 = 256;

/// By how much a channel of a pixel may differ between the pipelined and the ordinary layer, as on Metal
/// (`gpu/metal/tests/pipelined_frames.rs`).
const STEP: u8 = 8;

fn skip(reason: &str) {
    if std::env::var("FERROUI_VULKAN_TEST_REQUIRE").is_ok_and(|value| value == "1") {
        panic!("{reason} (FERROUI_VULKAN_TEST_REQUIRE=1)");
    }
    eprintln!("passed over: {reason}");
}

fn loader() -> Result<VkGetInstanceProcAddressDelegate, String> {
    // SAFETY: the library that is opened is the Vulkan loader, whose initialisation has no
    // preconditions.
    let entry = unsafe { ash::Entry::load() }.map_err(|error| format!("no Vulkan loader: {error}"))?;
    let entry = Arc::new(entry);
    Ok(Arc::new(move |instance: isize, name: &str| {
        let Ok(name) = CString::new(name) else { return 0 };
        // SAFETY: the name is a zero-terminated string and the instance is null or an instance of
        // this loader, which the entry keeps open.
        let address = unsafe {
            (entry.static_fn().get_instance_proc_addr)(vk::Handle::from_raw(instance as u64), name.as_ptr())
        };
        address.map_or(0, |address| address as usize as isize)
    }))
}

/// A Vulkan context of the machine without a window system, the Graphite GPU of its device and the
/// Skia context of the GPU, which disposes the GPU (and the Vulkan context with it) when it is disposed.
fn graphite_gpu() -> Option<(crate::SkiaContext, Rc<GraphiteVulkanSkiaGpu>)> {
    let loader = match loader() {
        Ok(loader) => loader,
        Err(reason) => {
            skip(&reason);
            return None;
        }
    };
    let platform_options =
        VulkanPlatformSpecificOptions { get_proc_address_delegate: Some(loader), ..Default::default() };
    let mut options = VulkanOptions {
        vulkan_instance_creation_options: VulkanInstanceCreationOptions {
            application_name: Some("ferroui-skia Graphite Vulkan tests".to_string()),
            require_surface_extension: false,
            ..Default::default()
        },
        ..Default::default()
    };
    options.vulkan_device_creation_options.require_swapchain_extension = false;
    let Some(graphics) = VulkanPlatformGraphics::try_create(&options, platform_options) else {
        skip("the Vulkan implementation has no instance or no device");
        return None;
    };
    let context = graphics.create_context();
    let features: &dyn IOptionalFeatureProvider = &*context;
    let vulkan = features.try_get::<dyn IVulkanPlatformGraphicsContext>().expect("the context is a Vulkan context");
    let Some(gpu) = GraphiteVulkanSkiaGpu::try_new(vulkan, None) else {
        skip("Skia makes no Graphite context for the device");
        return None;
    };
    let skia_gpu: Rc<dyn ISkiaGpu> = gpu.clone();
    Some((crate::SkiaContext::new(Some(skia_gpu)), gpu))
}

fn create_info(
    gpu: &Rc<dyn ISkiaGpu>,
    gr_context: &Rc<GraphiteGrContext>,
    width: i32,
    height: i32,
) -> SurfaceRenderTargetCreateInfo {
    SurfaceRenderTargetCreateInfo {
        width,
        height,
        dpi: Vector::new(96.0, 96.0),
        format: None,
        disable_text_lcd_rendering: false,
        text_rendering: TextRenderingSettings::default(),
        gr_context: Some(gr_context.clone()),
        gpu: Some(gpu.clone()),
        frame_text_rendering: None,
        disable_manual_fbo: true,
        session: None,
        use_scaled_drawing: false,
    }
}

/// An ordinary layer and a pipelined layer of one Graphite Vulkan context.
struct Layers {
    gpu: Rc<GraphiteVulkanSkiaGpu>,
    gr_context: Rc<GraphiteGrContext>,
    size: (i32, i32),
    ordinary: SurfaceRenderTarget,
    pipelined: SurfaceRenderTarget,
    surface: Rc<PipelinedLayerSurface>,
}

impl Layers {
    fn new(gpu: &Rc<GraphiteVulkanSkiaGpu>, bands: usize, width: i32, height: i32) -> Self {
        let gr_context = gpu.gr_context();
        let skia_gpu: Rc<dyn ISkiaGpu> = gpu.clone();
        let ordinary = SurfaceRenderTarget::new(create_info(&skia_gpu, &gr_context, width, height));
        let info = create_info(&skia_gpu, &gr_context, width, height);
        let surface = Rc::new(
            gpu.try_create_pipelined_layer(
                PixelSize::new(width, height),
                &info.text_rendering.surface_props(),
                SkiaFramePipelining::On,
                bands,
            )
            .expect("a layer that can be divided, on an image of the device"),
        );
        let pipelined = SurfaceRenderTarget::with_surface(info, surface.clone());
        Self { gpu: gpu.clone(), gr_context, size: (width, height), ordinary, pipelined, surface }
    }

    /// One frame of each layer: the pipelined one hands it to its threads, the ordinary one draws it
    /// band by band under the same clips, each band a recording of its own.
    fn draw(&self, draw: impl Fn(&mut dyn IDrawingContextImpl)) {
        self.draw_ordinary(&self.ordinary, &self.surface.band_clips(), &draw);

        let mut drawing_context = self.pipelined.create_drawing_context();
        draw(&mut *drawing_context);
        drawing_context.dispose();
    }

    /// Draws a frame on an ordinary layer of the context under each of `clips`, each a recording of its own.
    fn draw_ordinary(
        &self,
        layer: &SurfaceRenderTarget,
        clips: &[(usize, skia_safe::IRect)],
        draw: &impl Fn(&mut dyn IDrawingContextImpl),
    ) {
        for (_, clip) in clips.iter().copied() {
            let mut drawing_context = layer.create_drawing_context();
            let feature = drawing_context
                .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
                .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned())
                .expect("the drawing context of a layer leases the Skia API");
            let with_canvas = |use_canvas: &mut dyn FnMut(&skia_safe::Canvas)| {
                let lease = feature.lease();
                lease.with_sk_canvas(use_canvas);
                lease.dispose();
            };
            with_canvas(&mut |canvas| {
                canvas.save();
                canvas.clip_rect(skia_safe::Rect::from(clip), skia_safe::ClipOp::Intersect, false);
            });
            draw(&mut *drawing_context);
            with_canvas(&mut |canvas| {
                canvas.restore();
            });
            drawing_context.dispose();
            self.gr_context.flush();
        }
    }

    /// Another ordinary layer of the size of the layers, on the same context.
    fn another_ordinary(&self) -> SurfaceRenderTarget {
        let skia_gpu: Rc<dyn ISkiaGpu> = self.gpu.clone();
        SurfaceRenderTarget::new(create_info(&skia_gpu, &self.gr_context, self.size.0, self.size.1))
    }

    /// Reads both layers back: at most `pixels` of them may differ, by at most `step` in a channel.
    fn assert_pixels_within(&self, what: &str, pixels: usize, step: u8) {
        let ordinary = bytes(&self.ordinary, self.size);
        let pipelined = bytes(&self.pipelined, self.size);
        assert!(ordinary.iter().any(|byte| *byte != ordinary[0]), "{what}: the frame shows something");
        let Difference { pixels: different, step: largest, first } = difference(&ordinary, &pipelined, self.size.0);
        assert!(
            different <= pixels && largest <= step,
            "{what}: {different} pixels differ, by {largest} at most; \
             the first at {first:?} (x, y, ordinary, pipelined)"
        );
    }

    fn dispose(self) {
        self.pipelined.dispose();
        self.ordinary.dispose();
        // The flush after a layer was let go of destroys its image once the GPU has finished with it.
        self.gr_context.flush();
    }
}

/// What differs between two frames of `width` pixels a row: how many pixels, by how much in a channel at most,
/// and the first of them (x, y, one, other).
struct Difference {
    pixels: usize,
    step: u8,
    first: Vec<(usize, usize, Vec<u8>, Vec<u8>)>,
}

fn difference(one: &[u8], other: &[u8], width: i32) -> Difference {
    assert_eq!(one.len(), other.len());
    let width = width as usize;
    let mut found = Difference { pixels: 0, step: 0, first: Vec::new() };
    for (index, (a, b)) in one.chunks_exact(4).zip(other.chunks_exact(4)).enumerate() {
        if a != b {
            found.pixels += 1;
            found.step = a.iter().zip(b).fold(found.step, |largest, (a, b)| largest.max(a.abs_diff(*b)));
            if found.first.len() < 24 {
                found.first.push((index % width, index / width, a.to_vec(), b.to_vec()));
            }
        }
    }
    found
}

/// Prints what differs between two frames, with the distance of each listed pixel to the nearest row where
/// two bands meet (`edges`), for the diagnosis of the CI job.
fn report(what: &str, found: &Difference, edges: &[i32]) {
    let near: Vec<(usize, usize, i32)> = found
        .first
        .iter()
        .map(|(x, y, ..)| (*x, *y, edges.iter().map(|edge| (*y as i32 - edge).abs()).min().unwrap_or(i32::MAX)))
        .collect();
    eprintln!(
        "diagnostic {what}: {} pixels differ, by {} at most; (x, y, rows to the nearest band edge) {near:?}; \
         pixels (x, y, one, other) {:?}",
        found.pixels, found.step, found.first
    );
}

/// The rows where the bands of a layer meet.
fn edges(clips: &[(usize, skia_safe::IRect)]) -> Vec<i32> {
    clips.iter().skip(1).map(|(_, clip)| clip.top).collect()
}

/// The premultiplied pixels of a layer, read back from the GPU, row by row without padding.
fn bytes(layer: &SurfaceRenderTarget, (width, height): (i32, i32)) -> Vec<u8> {
    let snapshot = layer.create_non_affined_snapshot();
    let bitmap = snapshot.as_any().downcast_ref::<ImmutableBitmap>().unwrap();
    let image = bitmap.image();
    let pixmap = image.peek_pixels().expect("the snapshot is a raster image");
    let row_bytes = pixmap.row_bytes();
    let all = pixmap.bytes().expect("the snapshot has pixels");
    let mut rows = Vec::with_capacity((width * height * 4) as usize);
    for row in all.chunks(row_bytes).take(height as usize) {
        rows.extend_from_slice(&row[..(width * 4) as usize]);
    }
    rows
}

fn solid(color: u32) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::from_uint32(color)
}

fn glyph_run(text: &str, size: f32, origin: Point) -> Option<GlyphRunImpl> {
    let typeface = skia_safe::FontMgr::new().legacy_make_typeface(None, skia_safe::FontStyle::normal())?;
    let font = skia_safe::Font::from_typeface(typeface, size);
    let mut glyph_ids = vec![0u16; text.chars().count()];
    font.str_to_glyphs(text, &mut glyph_ids);
    let mut widths = vec![0f32; glyph_ids.len()];
    font.get_widths(&glyph_ids, &mut widths);
    let glyphs: Vec<GlyphInfo> = glyph_ids
        .iter()
        .zip(widths)
        .enumerate()
        .map(|(cluster, (glyph_index, width))| GlyphInfo::new(*glyph_index, cluster as i32, f64::from(width)))
        .collect();
    Some(GlyphRunImpl::from_font(&font, f64::from(size), &glyphs, origin))
}

/// Antialiased shapes, a line and text across every row bands of the layer can meet at (the
/// quarters and the half of its height).
fn draw_scene(context: &mut dyn IDrawingContextImpl) {
    context.clear(Colors::WHITE);
    for (index, seam) in [64.0, 128.0, 192.0].into_iter().enumerate() {
        let left = 10.0 + index as f64 * 100.0;
        context.draw_ellipse(
            Some(&solid(0xc0e04020)),
            Some(&ImmutablePen::from_uint32(0xff000000, 1.5)),
            Rect::new(left, seam - 21.25, 70.0, 43.5),
        );
        context.draw_rectangle(
            Some(&solid(0xff2060c0)),
            None,
            RoundedRect::from_radius(Rect::new(left + 30.5, seam - 9.75, 52.0, 19.5), 7.0),
            &BoxShadows::default(),
        );
        if let Some(run) = glyph_run("Hxgjpqy", 22.0, Point::new(left, seam + 7.0)) {
            context.draw_glyph_run(Some(&solid(0xff101010)), &run);
        }
    }
    context.draw_line(
        Some(&ImmutablePen::from_uint32(0xff008040, 2.0)),
        Point::new(4.0, 6.0),
        Point::new(316.0, 250.0),
    );
}

/// What a later frame draws on top: part of the layer, across the half of its height.
fn draw_update(context: &mut dyn IDrawingContextImpl) {
    context.push_clip(Rect::new(96.0, 84.0, 120.0, 90.0));
    context.clear(ferroui_base::media::Color::from_uint32(0xfff8f4e0));
    context.draw_ellipse(Some(&solid(0xff2080a0)), None, Rect::new(100.0, 90.0, 110.0, 78.0));
    context.pop_clip();
}

#[test]
fn bands_on_graphite_vulkan_draw_on_no_shared_target_and_show_the_frames() {
    for bands in [2, 4] {
        // A context of its own for each count: the threads of a context are as many as its first layer asked for.
        let Some((context, gpu)) = graphite_gpu() else { return };
        let layers = Layers::new(&gpu, bands, WIDTH, HEIGHT);
        let tolerance = WIDTH as usize * (bands - 1);
        layers.draw(draw_scene);
        layers.assert_pixels_within(&format!("the first frame in {bands} bands"), tolerance, STEP);
        layers.draw(draw_update);
        layers.draw(draw_update);
        layers.assert_pixels_within(&format!("the third frame in {bands} bands"), tolerance, STEP);
        layers.dispose();
        context.dispose();
    }
}

#[test]
fn a_resized_layer_is_drawn_on_a_new_image_after_the_old_one_was_let_go_of() {
    let Some((context, gpu)) = graphite_gpu() else { return };
    // A window that grows and shrinks: each size a layer of its own, on an image of its own, and the
    // image of the layer before goes with the flush after it was let go of.
    for (width, height) in [(WIDTH, HEIGHT), (WIDTH + 64, HEIGHT + 80), (WIDTH - 40, HEIGHT - 64)] {
        let layers = Layers::new(&gpu, 2, width, height);
        layers.draw(draw_scene);
        layers.draw(draw_update);
        layers.assert_pixels_within(&format!("a frame at {width} x {height}"), width as usize, STEP);
        layers.dispose();
    }
    // The context goes on drawing after the images went.
    let layers = Layers::new(&gpu, 2, WIDTH, HEIGHT);
    layers.draw(draw_scene);
    layers.assert_pixels_within("a frame after the resizes", WIDTH as usize, STEP);
    layers.dispose();
    assert!(!gpu.gr_context().is_lost());
    context.dispose();
}

#[test]
fn a_layer_disposed_with_a_frame_pending_leaves_the_vulkan_context_usable() {
    let Some((context, gpu)) = graphite_gpu() else { return };
    let layers = Layers::new(&gpu, 2, WIDTH, HEIGHT);
    let mut drawing_context = layers.pipelined.create_drawing_context();
    draw_scene(&mut *drawing_context);
    drawing_context.dispose();
    // The frame may still be in the threads; the layer goes before it was read.
    layers.dispose();
    let layers = Layers::new(&gpu, 2, WIDTH, HEIGHT);
    layers.draw(draw_scene);
    layers.assert_pixels_within("a frame of a new layer", WIDTH as usize, STEP);
    layers.dispose();
    context.dispose();
}

/// Two pipelined layers of one context (two windows of one compositor): the one that goes releases its own image
/// and the surfaces the threads wrapped it in, and the other keeps drawing on the same threads.
#[test]
fn a_layer_that_goes_leaves_the_other_layer_of_the_context_drawing() {
    let Some((context, gpu)) = graphite_gpu() else { return };
    let staying = Layers::new(&gpu, 2, WIDTH, HEIGHT);
    let going = Layers::new(&gpu, 2, WIDTH - 32, HEIGHT - 16);
    staying.draw(draw_scene);
    going.draw(draw_scene);
    // The layer goes with a frame in the threads.
    let mut drawing_context = going.pipelined.create_drawing_context();
    draw_update(&mut *drawing_context);
    drawing_context.dispose();
    going.dispose();
    for frame in 0..3 {
        staying.draw(draw_update);
        staying.assert_pixels_within(&format!("frame {frame} of the layer that stays"), WIDTH as usize, STEP);
    }
    staying.dispose();
    assert!(!gpu.gr_context().is_lost());
    context.dispose();
}

/// Two Skia contexts on one Vulkan device (two compositors whose platform shares its device): each has a GPU, a
/// Graphite context and band threads of its own, and disposing one leaves the other drawing. Here both share
/// even the Vulkan context object, whose disposal releases only what the context made for itself (its external
/// objects feature, which the Graphite GPU does not use); the device stays.
#[test]
fn disposing_one_skia_context_leaves_another_on_the_same_device_drawing() {
    let Some((first, first_gpu)) = graphite_gpu() else { return };
    let Some(second_gpu) = GraphiteVulkanSkiaGpu::try_new(first_gpu.vulkan().clone(), None) else {
        return skip("Skia makes no second Graphite context for the device");
    };
    let skia_gpu: Rc<dyn ISkiaGpu> = second_gpu.clone();
    let second = crate::SkiaContext::new(Some(skia_gpu));
    let first_layers = Layers::new(&first_gpu, 2, WIDTH, HEIGHT);
    let second_layers = Layers::new(&second_gpu, 2, WIDTH, HEIGHT);
    first_layers.draw(draw_scene);
    second_layers.draw(draw_scene);
    first_layers.dispose();
    first.dispose();
    for frame in 0..3 {
        second_layers.draw(draw_update);
        second_layers.assert_pixels_within(&format!("frame {frame} of the context that stays"), WIDTH as usize, STEP);
    }
    second_layers.dispose();
    assert!(!second_gpu.gr_context().is_lost());
    second.dispose();
}

/// Diagnosis (printed by the CI job, nothing asserted): whether the ordinary path of lavapipe draws the same frame
/// the same way twice, band by band and in one recording; how one recording under the clips of the bands differs
/// from one without them; how a pipelined layer of one band (no band edge) differs from the ordinary layer; and,
/// for 2 and 4 bands, where the pixels that differ lie against the band edges.
#[test]
fn diagnostic_where_the_frames_of_lavapipe_differ() {
    for bands in [1, 2, 4] {
        let Some((context, gpu)) = graphite_gpu() else { return };
        let layers = Layers::new(&gpu, bands, WIDTH, HEIGHT);
        let clips = layers.surface.band_clips();
        let whole = vec![(0, skia_safe::IRect::from_wh(WIDTH, HEIGHT))];
        let edges = edges(&clips);

        layers.draw(draw_scene);
        let ordinary = bytes(&layers.ordinary, layers.size);
        let pipelined = bytes(&layers.pipelined, layers.size);
        let compare = |what: &str, one: &[u8], other: &[u8]| {
            report(&format!("{bands} bands: {what}"), &difference(one, other, WIDTH), &edges)
        };
        compare("pipelined against ordinary", &ordinary, &pipelined);

        let again = layers.another_ordinary();
        layers.draw_ordinary(&again, &clips, &draw_scene);
        let twice = bytes(&again, layers.size);
        compare("ordinary band by band, twice", &ordinary, &twice);

        let once = layers.another_ordinary();
        layers.draw_ordinary(&once, &whole, &draw_scene);
        let unbanded = bytes(&once, layers.size);
        let other = layers.another_ordinary();
        layers.draw_ordinary(&other, &whole, &draw_scene);
        let unbanded_twice = bytes(&other, layers.size);
        compare("ordinary in one recording, twice", &unbanded, &unbanded_twice);
        compare("ordinary band by band against one recording", &ordinary, &unbanded);
        compare("pipelined against one recording", &pipelined, &unbanded);

        for layer in [again, once, other] {
            layer.dispose();
        }
        layers.dispose();
        context.dispose();
    }
}
