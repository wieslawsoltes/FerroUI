//! The frames of a pipelined frame layer (`gpu/graphite/pipelined_layer.rs`,
//! docs/porting/desktop-performance.md, section 9.33) against the frames of
//! an ordinary layer of the same context.
//!
//! A pipelined frame is the frame played once per band under the clip of the
//! band, and that is all it is: the ordinary layer that draws the same frame
//! band by band, under the same clips, with the recorder of the context,
//! shows the same bytes, whichever threads drew the pipelined one
//! ([`Layers::draw`]). A frame the pipelined layer draws directly is compared
//! with the frame the ordinary layer draws as it always does
//! ([`Layers::draw_unclipped`]).
//!
//! What a clip costs is Skia's and is the same in every frame that is drawn
//! under one, a partial redraw of a dirty rectangle included: where a clip
//! cuts a shape, a shaded fill may round a channel the other way and the
//! coverage of an antialiased edge that crosses the boundary may differ in
//! the pixel at the crossing. The last test of the file measures it.

use super::*;
use crate::gpu::graphite::{band_clips, band_count, GraphiteGrContext, PipelinedLayerSurface};
use crate::gpu::{ISkiaGpu, ISkiaGrContext, ISkiaSurface};
use crate::{
    GlyphRunImpl, ISkiaApiLeaseFeature, SkiaFramePipelining, SkiaOptions, TextRenderingSettings,
    DEFAULT_FRAME_PIPELINING_BANDS, MAX_FRAME_PIPELINING_BANDS,
};
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::{Matrix, Point};

const WIDTH: i32 = 320;
const HEIGHT: i32 = 256;
/// The row the two bands of a layer of [`HEIGHT`] rows meet at.
const SEAM: f64 = 128.0;
/// The bands of the layers of most tests: the default of the options.
const BANDS: usize = DEFAULT_FRAME_PIPELINING_BANDS;

/// An ordinary layer and a pipelined layer of one context.
struct Layers {
    context: crate::SkiaContext,
    gpu: Rc<dyn ISkiaGpu>,
    gr_context: Rc<GraphiteGrContext>,
    ordinary: SurfaceRenderTarget,
    pipelined: SurfaceRenderTarget,
    /// The surface of `pipelined`.
    surface: Rc<PipelinedLayerSurface>,
}

fn create_info(gpu: &Rc<dyn ISkiaGpu>, gr_context: &Rc<GraphiteGrContext>) -> SurfaceRenderTargetCreateInfo {
    SurfaceRenderTargetCreateInfo {
        width: WIDTH,
        height: HEIGHT,
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

impl Layers {
    /// The layers, the pipelined one with the given mode and the default
    /// number of bands.
    fn new(device: &Rc<TestMetalDevice>, mode: SkiaFramePipelining) -> Self {
        Self::with_bands(device, mode, BANDS)
    }

    /// The layers, the pipelined one with the given mode, asked to divide
    /// its frames into `bands` bands.
    fn with_bands(device: &Rc<TestMetalDevice>, mode: SkiaFramePipelining, bands: usize) -> Self {
        let metal_gpu = crate::gpu::metal::SkiaMetalGpu::new(device.clone(), None, None);
        let gr_context = metal_gpu.gr_context();
        let gpu: Rc<dyn ISkiaGpu> = metal_gpu;
        let context = crate::SkiaContext::new(Some(gpu.clone()));

        let ordinary = SurfaceRenderTarget::new(create_info(&gpu, &gr_context));
        let info = create_info(&gpu, &gr_context);
        let surface = Rc::new(
            PipelinedLayerSurface::try_new(
                &gr_context,
                device.device(),
                PixelSize::new(WIDTH, HEIGHT),
                &info.text_rendering.surface_props(),
                mode,
                bands,
            )
            .expect("a layer that can be divided"),
        );
        let pipelined = SurfaceRenderTarget::with_surface(info, surface.clone());

        Self { context, gpu, gr_context, ordinary, pipelined, surface }
    }

    /// Another pipelined layer of the context, asked for `bands` bands.
    fn add_pipelined(
        &self,
        device: &Rc<TestMetalDevice>,
        bands: usize,
    ) -> (SurfaceRenderTarget, Rc<PipelinedLayerSurface>) {
        let info = create_info(&self.gpu, &self.gr_context);
        let surface = Rc::new(
            PipelinedLayerSurface::try_new(
                &self.gr_context,
                device.device(),
                PixelSize::new(WIDTH, HEIGHT),
                &info.text_rendering.surface_props(),
                SkiaFramePipelining::On,
                bands,
            )
            .expect("a layer that can be divided"),
        );
        (SurfaceRenderTarget::with_surface(info, surface.clone()), surface)
    }

    /// Draws one frame of each layer, a frame the pipelined layer hands to
    /// its threads. The ordinary layer draws it as the threads do, with the
    /// recorder of the context: once per band, under the clip of the band,
    /// each band a recording of its own, as the recording of a thread is
    /// (Graphite draws a recording in passes of its own, and what it draws
    /// at an antialiased edge depends on the draws that share the pass: a
    /// pass with a draw that needs multisampling is multisampled as a whole).
    fn draw(&self, draw: impl Fn(&mut dyn IDrawingContextImpl)) {
        self.draw_ordinary_in_clips(&self.surface.band_clips(), true, &draw);

        let mut drawing_context = self.pipelined.create_drawing_context();
        draw(&mut *drawing_context);
        drawing_context.dispose();
    }

    /// Draws one frame of the ordinary layer band by band, in `bands` bands:
    /// each band a recording of its own, or the frame one recording.
    fn draw_ordinary_in_bands(&self, bands: usize, apart: bool, draw: &impl Fn(&mut dyn IDrawingContextImpl)) {
        self.draw_ordinary_in_clips(&band_clips((WIDTH, HEIGHT), bands), apart, draw);
    }

    /// Draws one frame of the ordinary layer under each of `clips`: each a
    /// recording of its own, or the frame one recording.
    fn draw_ordinary_in_clips(
        &self,
        clips: &[(usize, skia_safe::IRect)],
        apart: bool,
        draw: &impl Fn(&mut dyn IDrawingContextImpl),
    ) {
        let recordings: Vec<&[(usize, skia_safe::IRect)]> =
            if apart { clips.chunks(1).collect() } else { vec![clips] };
        for clips in recordings {
            let mut drawing_context = self.ordinary.create_drawing_context();
            let feature = drawing_context
                .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
                .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned())
                .expect("the drawing context of a layer leases the Skia API");
            let with_canvas = |use_canvas: &mut dyn FnMut(&skia_safe::Canvas)| {
                let lease = feature.lease();
                lease.with_sk_canvas(use_canvas);
                lease.dispose();
            };
            for (_, clip) in clips {
                with_canvas(&mut |canvas| {
                    canvas.save();
                    canvas.clip_rect(skia_safe::Rect::from(*clip), skia_safe::ClipOp::Intersect, false);
                });
                draw(&mut *drawing_context);
                with_canvas(&mut |canvas| {
                    canvas.restore();
                });
            }
            drawing_context.dispose();
            self.gr_context.flush();
        }
    }

    /// Draws one frame of each layer, the ordinary one as it always draws: a
    /// frame the pipelined layer draws directly.
    fn draw_unclipped(&self, draw: impl Fn(&mut dyn IDrawingContextImpl)) {
        for layer in [&self.ordinary, &self.pipelined] {
            let mut drawing_context = layer.create_drawing_context();
            draw(&mut *drawing_context);
            drawing_context.dispose();
        }
    }

    /// Whether the threads are playing a frame of the pipelined layer.
    fn is_pending(&self) -> bool {
        self.pipelined.has_pending_frame()
    }

    /// Reads both layers back and compares every byte.
    fn assert_same_pixels(&self, what: &str) {
        self.assert_pixels_within(what, 0, 0);
    }

    /// Reads both layers back and compares them: at most `pixels` of them
    /// may differ, by at most `step` in a channel.
    fn assert_pixels_within(&self, what: &str, pixels: usize, step: u8) {
        let ordinary = bytes(&self.ordinary);
        let pipelined = bytes(&self.pipelined);
        assert_eq!(ordinary.len(), pipelined.len());
        assert!(ordinary.iter().any(|byte| *byte != ordinary[0]), "{what}: the frame shows something");

        let mut different = 0usize;
        let mut largest = 0u8;
        let mut first = Vec::new();
        for (index, (a, b)) in ordinary.chunks_exact(4).zip(pipelined.chunks_exact(4)).enumerate() {
            if a != b {
                different += 1;
                largest = a.iter().zip(b).fold(largest, |largest, (a, b)| largest.max(a.abs_diff(*b)));
                if first.len() < 16 {
                    first.push((index % WIDTH as usize, index / WIDTH as usize, a.to_vec(), b.to_vec()));
                }
            }
        }
        assert!(
            different <= pixels && largest <= step,
            "{what}: {different} pixels differ, by {largest} at most; the first at {first:?} (x, y, ordinary, pipelined)"
        );
    }

    fn dispose(self) {
        self.pipelined.dispose();
        self.ordinary.dispose();
        self.context.dispose();
    }
}

/// The premultiplied pixels of a layer, read back from the GPU, row by row
/// without padding.
fn bytes(layer: &SurfaceRenderTarget) -> Vec<u8> {
    let snapshot = layer.create_non_affined_snapshot();
    let bitmap = snapshot.as_any().downcast_ref::<ImmutableBitmap>().unwrap();
    let image = bitmap.image();
    let pixmap = image.peek_pixels().expect("the snapshot is a raster image");
    let row_bytes = pixmap.row_bytes();
    let all = pixmap.bytes().expect("the snapshot has pixels");
    let mut rows = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
    for row in all.chunks(row_bytes).take(HEIGHT as usize) {
        rows.extend_from_slice(&row[..(WIDTH * 4) as usize]);
    }
    rows
}

fn solid(color: u32) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::from_uint32(color)
}

fn gradient() -> ImmutableLinearGradientBrush {
    ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0.0, Colors::BLUE),
            ImmutableGradientStop::new(0.5, Colors::LIME),
            ImmutableGradientStop::new(1.0, Colors::RED),
        ],
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative)),
        None,
    )
}

/// A run of glyphs of the default typeface of the machine with its baseline
/// at `origin`; `None` on a machine without fonts.
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

/// A frame with every kind of drawing a window draws, antialiased, with
/// content on both sides of the row the bands meet at and across it.
fn draw_scene(context: &mut dyn IDrawingContextImpl) {
    context.clear(Colors::WHITE);

    // Filled and stroked shapes across the seam.
    context.draw_rectangle(
        Some(&solid(0xff2060c0)),
        None,
        RoundedRect::from_rect(Rect::new(8.5, 100.25, 60.0, 70.5)),
        &BoxShadows::default(),
    );
    context.draw_rectangle(
        Some(&gradient()),
        Some(&ImmutablePen::from_uint32(0xff102030, 3.0)),
        RoundedRect::from_radius(Rect::new(80.0, 90.0, 90.0, 80.0), 14.0),
        &BoxShadows::default(),
    );
    context.draw_ellipse(
        Some(&solid(0xc0e04020)),
        Some(&ImmutablePen::from_uint32(0xff000000, 1.5)),
        Rect::new(180.0, 96.0, 70.0, 64.0),
    );
    context.draw_line(
        Some(&ImmutablePen::from_uint32(0xff008040, 2.0)),
        Point::new(4.0, 20.0),
        Point::new(316.0, 250.0),
    );
    context.draw_line(
        Some(&ImmutablePen::from_uint32(0xff804000, 1.0)),
        Point::new(0.0, SEAM),
        Point::new(320.0, SEAM),
    );

    // A shadow that spreads across the seam.
    context.draw_rectangle(
        Some(&solid(0xfff0f0f0)),
        None,
        RoundedRect::from_radius(Rect::new(250.0, 110.0, 56.0, 30.0), 6.0),
        &BoxShadows::parse("0 4 12 2 #a0000000").unwrap(),
    );

    // Images, scaled, on both sides of the seam.
    let bitmap = two_tone_bitmap();
    let source = Rect::new(0.0, 0.0, 4.0, 4.0);
    context.draw_bitmap(&*bitmap, 1.0, source, Rect::new(10.0, 10.0, 48.0, 48.0));
    context.draw_bitmap(&*bitmap, 0.6, source, Rect::new(260.0, 104.0, 50.0, 50.0));

    // Text above, below and on the seam.
    for (text, size, origin) in [
        ("Frames in bands", 18.0, Point::new(70.0, 40.0)),
        ("Hxgjpqy 0123", 28.0, Point::new(20.0, SEAM + 9.0)),
        ("below the seam", 13.0, Point::new(120.0, 230.0)),
    ] {
        if let Some(run) = glyph_run(text, size, origin) {
            context.draw_glyph_run(Some(&solid(0xff101010)), &run);
        }
    }

    // A layer with opacity, a rounded clip and a rotation, all across the seam.
    context.push_opacity(0.55, Some(Rect::new(30.0, 60.0, 200.0, 150.0)));
    context.push_clip_rounded(RoundedRect::from_radius(Rect::new(40.5, 70.5, 180.0, 130.0), 22.0));
    let transform = context.transform();
    context.set_transform(Matrix::create_rotation_at(0.3, Point::new(130.0, SEAM)));
    context.draw_rectangle(
        Some(&gradient()),
        Some(&ImmutablePen::from_uint32(0xff400080, 2.5)),
        RoundedRect::from_radius(Rect::new(60.0, 80.0, 150.0, 100.0), 9.0),
        &BoxShadows::default(),
    );
    context.set_transform(transform);
    context.pop_clip();
    context.pop_opacity();
}

/// What a second frame draws on top of the first: a part of the layer only,
/// across the seam.
fn draw_update(context: &mut dyn IDrawingContextImpl) {
    context.push_clip(Rect::new(96.0, 84.0, 120.0, 90.0));
    context.clear(Color::from_uint32(0xfff8f4e0));
    context.draw_ellipse(Some(&solid(0xff2080a0)), None, Rect::new(100.0, 90.0, 110.0, 78.0));
    if let Some(run) = glyph_run("second", 22.0, Point::new(112.0, SEAM + 7.0)) {
        context.draw_glyph_run(Some(&solid(0xffffffff)), &run);
    }
    context.pop_clip();
}

#[test]
fn a_pipelined_frame_shows_what_the_layer_draws_band_by_band() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);

    layers.draw(draw_scene);
    assert!(layers.is_pending(), "the frame is with the threads when its drawing context is disposed");

    layers.assert_same_pixels("the first frame");
    assert!(!layers.is_pending(), "a read of the layer receives the frame");

    layers.dispose();
}

#[test]
fn the_frames_of_a_pipelined_layer_follow_each_other_in_order() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);

    // The second frame is drawn while the threads play the first, and keeps
    // what the first drew outside its clip.
    layers.draw(draw_scene);
    layers.draw(draw_update);
    assert!(layers.is_pending());
    layers.assert_same_pixels("the second frame over the first");

    // And again after a read: the layer draws over what was received.
    layers.draw(draw_scene);
    layers.draw(draw_update);
    layers.draw(draw_update);
    layers.assert_same_pixels("the fifth frame");

    layers.dispose();
}

#[test]
fn a_pending_frame_is_completed_on_request() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);

    assert!(!layers.is_pending(), "a new layer has no frame");
    layers.pipelined.complete_pending_frame();

    layers.draw(draw_scene);
    assert!(layers.is_pending());
    layers.pipelined.complete_pending_frame();
    assert!(!layers.is_pending());
    assert!(!layers.surface.has_pending_frame());
    layers.assert_same_pixels("the completed frame");

    // An ordinary layer never has one.
    assert!(!layers.ordinary.has_pending_frame());
    layers.ordinary.complete_pending_frame();

    layers.dispose();
}

#[test]
fn a_frame_with_an_image_filter_is_drawn_by_the_recorder_of_the_context() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);

    let blurred = |context: &mut dyn IDrawingContextImpl| {
        draw_scene(context);
        let blur = ferroui_base::media::ImmutableBlurEffect::new(6.0);
        let bounds = Rect::new(40.0, 80.0, 200.0, 100.0);
        context.as_drawing_context_impl_with_effects().expect("effects").push_effect(Some(bounds), &blur);
        context.draw_ellipse(Some(&solid(0xff2040e0)), None, Rect::new(70.0, 100.0, 140.0, 60.0));
        context.as_drawing_context_impl_with_effects().expect("effects").pop_effect();
    };
    layers.draw_unclipped(blurred);
    assert!(!layers.is_pending(), "a frame with an image filter is not handed to the threads");
    layers.assert_same_pixels("the frame with a blur");

    // The frames after it are pipelined again when the options ask for it always.
    layers.draw(draw_update);
    assert!(layers.is_pending());
    layers.assert_same_pixels("the frame after the blur");

    layers.dispose();
}

#[test]
fn a_frame_that_leases_the_skia_api_is_drawn_by_the_recorder_of_the_context() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);

    let leased = |context: &mut dyn IDrawingContextImpl| {
        draw_scene(context);
        let feature = context
            .get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
            .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned())
            .expect("the drawing context of a layer leases the Skia API");
        let lease = feature.lease();
        let mut paint = skia_safe::Paint::default();
        paint.set_anti_alias(true);
        paint.set_color(skia_safe::Color::from_argb(255, 200, 30, 120));
        lease.with_sk_canvas(&mut |canvas| {
            canvas.draw_circle((160.0, SEAM as f32), 44.5, &paint);
        });
        lease.dispose();
        // What is drawn after the lease is drawn over it.
        context.draw_rectangle(
            Some(&solid(0x80ffffff)),
            None,
            RoundedRect::from_rect(Rect::new(150.0, 100.0, 60.0, 56.0)),
            &BoxShadows::default(),
        );
    };
    layers.draw_unclipped(leased);
    assert!(!layers.is_pending(), "a frame with a lease is not handed to the threads");
    layers.assert_same_pixels("the frame with a lease");

    layers.dispose();
}

#[test]
fn an_adaptive_layer_draws_its_first_frames_directly() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::Adaptive);

    for _ in 0..3 {
        layers.draw_unclipped(draw_scene);
        assert!(!layers.is_pending(), "nothing is known about the frames of the stream yet");
    }
    layers.draw_unclipped(draw_update);
    assert!(!layers.is_pending());
    layers.assert_same_pixels("the direct frames of an adaptive layer");

    layers.dispose();
}

#[test]
fn a_layer_is_ordinary_when_pipelining_is_off_or_the_layer_is_too_low() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let gpu = crate::gpu::metal::SkiaMetalGpu::new(device.clone(), None, None);
    let gr_context = gpu.gr_context();
    let props = TextRenderingSettings::default().surface_props();
    let size = PixelSize::new(WIDTH, HEIGHT);

    let off =
        PipelinedLayerSurface::try_new(&gr_context, device.device(), size, &props, SkiaFramePipelining::Off, BANDS);
    assert!(off.is_none());
    let low = PipelinedLayerSurface::try_new(
        &gr_context,
        device.device(),
        PixelSize::new(WIDTH, 100),
        &props,
        SkiaFramePipelining::On,
        BANDS,
    );
    assert!(low.is_none(), "two bands need 64 rows each");
    let no_device = PipelinedLayerSurface::try_new(
        &gr_context,
        std::ptr::null_mut(),
        size,
        &props,
        SkiaFramePipelining::On,
        BANDS,
    );
    assert!(no_device.is_none());

    let layer =
        PipelinedLayerSurface::try_new(&gr_context, device.device(), size, &props, SkiaFramePipelining::On, BANDS)
            .expect("a layer");
    layer.dispose();
    assert!(layer.begin_recorded_frame().is_none(), "a disposed layer records no frame");

    let context = crate::SkiaContext::new(Some(gpu));
    context.dispose();
}

#[test]
fn a_layer_disposed_with_a_frame_pending_leaves_the_context_usable() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);

    layers.draw(draw_scene);
    assert!(layers.is_pending());
    layers.pipelined.dispose();
    assert!(!layers.is_pending());

    // The ordinary layer of the context still draws and reads back.
    let mut drawing_context = layers.ordinary.create_drawing_context();
    drawing_context.clear(Colors::BLUE);
    drawing_context.dispose();
    let pixels = bytes(&layers.ordinary);
    assert_eq!(&[0, 0, 255, 255], &rgba(&pixels[..4]));

    layers.ordinary.dispose();
    layers.context.dispose();
}

#[test]
fn a_frame_in_any_number_of_bands_shows_what_the_layer_draws_band_by_band() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };

    // One band is the whole frame on one thread; four are as many as a
    // layer of this height has room for.
    for bands in [1, 3, 4] {
        let layers = Layers::with_bands(&device, SkiaFramePipelining::On, bands);
        assert_eq!(bands, layers.surface.bands());

        layers.draw(draw_scene);
        assert!(layers.is_pending(), "{bands} bands: the frame is with the threads");
        layers.assert_same_pixels(&format!("{bands} bands: the first frame"));

        layers.draw(draw_update);
        layers.draw(draw_scene);
        layers.draw(draw_update);
        layers.assert_same_pixels(&format!("{bands} bands: the fourth frame"));

        layers.dispose();
    }
}

/// How many pixels of the scene a band that is a pass of its own may draw
/// otherwise than a frame that is one pass, and by how much in a channel:
/// the antialiased edges of the shapes Graphite draws with coverage it
/// computes, in the bands without a draw that needs multisampling.
const OWN_PASS_PIXELS: usize = (WIDTH * HEIGHT / 200) as usize;
const OWN_PASS_STEP: u8 = 96;

#[test]
fn a_band_is_drawn_as_a_frame_with_its_draws_alone_is() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::with_bands(&device, SkiaFramePipelining::On, 4);

    // The stroked ellipse of the scene needs multisampling, and lies in the
    // second and the third band of four. A frame that is one recording is
    // multisampled as a whole; of a pipelined frame, the bands with the
    // ellipse are.
    layers.draw_ordinary_in_bands(4, false, &draw_scene);
    let mut drawing_context = layers.pipelined.create_drawing_context();
    draw_scene(&mut *drawing_context);
    drawing_context.dispose();
    assert!(layers.is_pending());

    let ordinary = bytes(&layers.ordinary);
    let pipelined = bytes(&layers.pipelined);
    let mut different = 0usize;
    let mut largest = 0u8;
    for (band, clip) in band_clips((WIDTH, HEIGHT), 4) {
        let rows = (clip.top() * WIDTH * 4) as usize..(clip.bottom() * WIDTH * 4) as usize;
        let (ordinary, pipelined) = (&ordinary[rows.clone()], &pipelined[rows]);
        if band == 1 || band == 2 {
            assert!(ordinary == pipelined, "band {band} has the draw that needs multisampling in both frames");
            continue;
        }
        for (a, b) in ordinary.chunks_exact(4).zip(pipelined.chunks_exact(4)) {
            if a != b {
                different += 1;
                largest = a.iter().zip(b).fold(largest, |largest, (a, b)| largest.max(a.abs_diff(*b)));
            }
        }
    }
    assert!(
        different <= OWN_PASS_PIXELS && largest <= OWN_PASS_STEP,
        "{different} pixels of the bands without the ellipse differ, by {largest} at most"
    );

    layers.dispose();
}

#[test]
fn a_layer_has_the_bands_it_is_asked_for_as_far_as_they_fit_it() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };

    // Eight bands of 64 rows do not fit 256 rows: the layer has four.
    let layers = Layers::with_bands(&device, SkiaFramePipelining::On, 8);
    assert_eq!(4, layers.surface.bands());
    layers.draw(draw_scene);
    layers.assert_same_pixels("eight bands asked for, four drawn");
    layers.dispose();

    let gpu = crate::gpu::metal::SkiaMetalGpu::new(device.clone(), None, None);
    let gr_context = gpu.gr_context();
    let props = TextRenderingSettings::default().surface_props();
    let layer = |height: i32, bands: usize| {
        PipelinedLayerSurface::try_new(
            &gr_context,
            device.device(),
            PixelSize::new(WIDTH, height),
            &props,
            SkiaFramePipelining::On,
            bands,
        )
    };
    // A layer asked to be divided that has room for one band is ordinary;
    // a layer asked for one band needs room for that one.
    assert!(layer(100, 4).is_none());
    let one = layer(100, 1).expect("one band of 100 rows");
    assert_eq!(1, one.bands());
    one.dispose();
    assert!(layer(63, 1).is_none());
    let three = layer(200, 8).expect("three bands of 66 rows");
    assert_eq!(3, three.bands());
    three.dispose();

    let context = crate::SkiaContext::new(Some(gpu));
    context.dispose();
}

#[test]
fn a_layer_has_no_more_bands_than_its_context_has_threads() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };

    // The first layer that pipelines a frame makes the threads: two.
    let layers = Layers::with_bands(&device, SkiaFramePipelining::On, 2);
    layers.draw(draw_scene);
    layers.assert_same_pixels("the frame of the first layer");

    // A layer of the same context that asks for four draws in two.
    let (second, surface) = layers.add_pipelined(&device, 4);
    assert_eq!(4, surface.bands());
    layers.draw_ordinary_in_bands(2, true, &draw_scene);
    let mut drawing_context = second.create_drawing_context();
    draw_scene(&mut *drawing_context);
    drawing_context.dispose();
    assert!(second.has_pending_frame());
    assert!(bytes(&layers.ordinary) == bytes(&second), "the frame of the second layer is drawn in two bands");

    second.dispose();
    layers.dispose();
}

#[test]
fn the_options_ask_for_two_bands_by_default_and_for_sixteen_at_most() {
    assert_eq!(2, DEFAULT_FRAME_PIPELINING_BANDS);
    assert_eq!(DEFAULT_FRAME_PIPELINING_BANDS, SkiaOptions::default().effective_frame_pipelining_bands());

    let asked = |bands: usize| {
        SkiaOptions { frame_pipelining_bands: bands, ..SkiaOptions::default() }.effective_frame_pipelining_bands()
    };
    assert_eq!(1, asked(0));
    assert_eq!(1, asked(1));
    assert_eq!(8, asked(8));
    assert_eq!(MAX_FRAME_PIPELINING_BANDS, asked(64));
}

#[test]
fn the_bands_of_a_layer_are_disjoint_and_cover_it() {
    for height in [64, 100, 255, 256, 1080, 2161] {
        for asked in 1..=MAX_FRAME_PIPELINING_BANDS {
            let Some(bands) = band_count(height, asked) else {
                assert!(height / 64 < asked.min(2) as i32, "{height} rows have room for {asked} bands");
                continue;
            };
            assert!(bands <= asked && bands as i32 * 64 <= height);
            assert!(bands == asked || bands == (height / 64) as usize);

            let clips = band_clips((WIDTH, height), bands);
            assert_eq!(bands, clips.len());
            let mut top = 0;
            for (index, (band, clip)) in clips.iter().enumerate() {
                assert_eq!(index, *band);
                assert_eq!((0, top, WIDTH), (clip.left(), clip.top(), clip.right()));
                assert!(clip.height() >= 64, "{height} rows in {bands} bands: a band of {} rows", clip.height());
                top = clip.bottom();
            }
            assert_eq!(height, top);
        }
    }
}

/// The scene, and over it six thousand small shapes in the rows from 150 to 250: the lower band of two equal
/// bands draws all of them.
fn draw_scene_with_a_heavy_lower_part(context: &mut dyn IDrawingContextImpl) {
    draw_scene(context);
    for index in 0..6_000u32 {
        let (column, row) = (f64::from(index % 60), f64::from(index / 60));
        let bounds = Rect::new(column * 5.25 + 0.25, 150.5 + row, 6.0, 5.0);
        let color = 0xff00_0000 | (index.wrapping_mul(2_654_435_761) >> 8);
        let shape = RoundedRect::from_radius(bounds, 1.5);
        context.draw_rectangle(Some(&solid(color)), None, shape, &BoxShadows::default());
    }
}

#[test]
fn the_bands_move_to_the_rows_that_cost_and_every_frame_is_drawn_as_before() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);
    let equal = band_clips((WIDTH, HEIGHT), BANDS);
    assert_eq!(layers.surface.band_clips(), equal, "a layer begins with equal bands");

    // Each frame is received before the next is drawn, so both layers draw a frame under the same bands.
    for frame in 0..24 {
        layers.draw(draw_scene_with_a_heavy_lower_part);
        layers.pipelined.complete_pending_frame();
        if frame % 8 == 7 {
            layers.assert_same_pixels(&format!("frame {frame}"));
        }
    }
    let moved = layers.surface.band_clips();
    assert_eq!(moved.len(), equal.len());
    assert!(moved[0].1.bottom() > equal[0].1.bottom(), "the upper band grew: {moved:?}");
    assert_eq!((moved[0].1.top(), moved[0].1.bottom()), (0, moved[1].1.top()));
    assert_eq!(moved[1].1.bottom(), HEIGHT);
    assert!(moved[1].1.height() >= 64, "{moved:?}");

    // Frames that follow each other, and that keep what the frame before drew, under the moved bands.
    layers.draw(draw_scene);
    layers.draw(draw_update);
    layers.assert_same_pixels("an update over a frame, after the bands moved");

    layers.dispose();
}

/// How many pixels of the scene a frame drawn under the clips of the bands
/// may show differently from the frame drawn under no clip: one in two
/// thousand. Seven of the 81,920 did when this was written.
const CLIPPED_PIXELS: usize = (WIDTH * HEIGHT / 2000) as usize;

/// By how much a channel of such a pixel may differ: a sixteenth of the
/// range. The largest step was 6 of 255 when this was written, in the pixel
/// where the antialiased edge of a rectangle crosses the row the bands meet
/// at; the others were single steps inside a gradient the clip cuts.
const CLIPPED_STEP: u8 = 16;

/// What a clip costs in Skia, with or without a pipeline: the frame under the
/// clips of the bands against the frame under no clip. A partial redraw, which
/// draws under the clip of its dirty rectangle, differs from a full one in the
/// same way.
#[test]
fn a_pipelined_frame_differs_from_an_unclipped_frame_as_any_clipped_frame_does() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let layers = Layers::new(&device, SkiaFramePipelining::On);

    layers.draw_unclipped(draw_scene);
    assert!(layers.is_pending());
    layers.assert_pixels_within("the frame in bands against the frame under no clip", CLIPPED_PIXELS, CLIPPED_STEP);

    layers.dispose();
}

/// A pixel of a read-back in RGBA order, whichever order the native 32-bit
/// colour type has.
fn rgba(pixel: &[u8]) -> [u8; 4] {
    if ColorType::N32 == ColorType::BGRA8888 {
        [pixel[2], pixel[1], pixel[0], pixel[3]]
    } else {
        [pixel[0], pixel[1], pixel[2], pixel[3]]
    }
}
