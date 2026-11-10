//! Text on the device: glyph runs drawn from the glyph atlas of the engine, checked in the
//! read-back pixels and in what the engine says it uploaded.

use crate::geometry::ProGpuGeometry;
use crate::{ProGpuDevice, ProGpuDrawingContext, ProGpuEngine, ProGpuFontManager, ProGpuGlyphRun, ProGpuTextShaper, SceneBuilder, TargetFormat};
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableSolidColorBrush};
use ferroui_base::media::text_formatting::{GlyphInfo, TextShaperOptions};
use ferroui_base::media::{Color, FontSimulations, GlyphTypeface, GradientSpreadMethod, RenderOptions, TextOptions, TextRenderingMode};
use ferroui_base::platform::{IDrawingContextImpl, IFontManagerImpl, ITextShaperImpl};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{FerroLocator, Matrix, Point, Rect, RelativePoint, RelativeUnit};
use progpu_native_sys::progpu::progpu_native_scene_frame_metrics;
use std::rc::Rc;

const INTER: &[u8] = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/Inter-Regular.ttf");

const WIDTH: u32 = 160;
const HEIGHT: u32 = 64;

fn engine() -> Option<ProGpuEngine> {
    match ProGpuDevice::new().and_then(|device| ProGpuEngine::new(&Rc::new(device), TargetFormat::Rgba8)) {
        Ok(engine) => Some(engine),
        Err(error) => {
            eprintln!("skipped: {error}");
            None
        }
    }
}

/// A frame of the tests: the read-back RGBA8 pixels and what the engine says of the frame.
struct Frame {
    pixels: Vec<u8>,
    metrics: progpu_native_scene_frame_metrics,
}

impl Frame {
    fn rgba(&self, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * WIDTH + x) * 4) as usize;
        [self.pixels[index], self.pixels[index + 1], self.pixels[index + 2], self.pixels[index + 3]]
    }

    /// How dark a pixel of black text on white is: 0 for white, 255 for black.
    fn ink(&self, x: u32, y: u32) -> u32 {
        255 - u32::from(self.rgba(x, y)[1])
    }

    /// The ink of a box of pixels.
    fn ink_in(&self, x: std::ops::Range<u32>, y: std::ops::Range<u32>) -> u64 {
        y.flat_map(|row| x.clone().map(move |column| (column, row))).map(|(column, row)| u64::from(self.ink(column, row))).sum()
    }

    /// The horizontal middle of the ink of a box of pixels.
    fn ink_middle(&self, x: std::ops::Range<u32>, y: std::ops::Range<u32>) -> f64 {
        let mut weight = 0.0;
        let mut moment = 0.0;
        for row in y {
            for column in x.clone() {
                let ink = f64::from(self.ink(column, row));
                weight += ink;
                moment += ink * (f64::from(column) + 0.5);
            }
        }
        moment / weight
    }
}

/// Draws with `draw` on a white target of `scene` and renders it with `engine`.
fn render_scene(engine: &ProGpuEngine, scene: SceneBuilder, draw: impl FnOnce(&mut ProGpuDrawingContext)) -> (Frame, SceneBuilder) {
    let mut context = ProGpuDrawingContext::new(scene, None);
    context.clear(Color::from_rgb(255, 255, 255));
    draw(&mut context);
    let (mut scene, error) = context.finish();
    assert!(error.is_none(), "recording: {error:?}");
    let stream = scene.build().expect("the stream of the scene").to_vec();
    engine.update_scene(&stream).expect("the scene");
    let device = engine.device();
    let (texture, view) = device.create_target(WIDTH, HEIGHT);
    let metrics = engine.render_scene(scene.scene_id(), scene.generation(), view, WIDTH, HEIGHT, false).expect("the frame");
    let pixels = device.read_rgba8(texture, WIDTH, HEIGHT);
    device.release_texture(texture, view);
    (Frame { pixels, metrics }, scene)
}

fn render(engine: &ProGpuEngine, draw: impl FnOnce(&mut ProGpuDrawingContext)) -> Frame {
    render_scene(engine, SceneBuilder::new(), draw).0
}

fn glyph_typeface(simulations: FontSimulations) -> Rc<GlyphTypeface> {
    let typeface = ProGpuFontManager::new().try_create_glyph_typeface_from_stream(&mut &INTER[..], simulations).expect("the font is read");
    GlyphTypeface::new(typeface, simulations).expect("the tables of the font are read")
}

/// The run of `text` in Inter at `size`, from `origin`, shaped by the engine.
fn run_with(text: &str, size: f64, origin: Point, simulations: FontSimulations) -> ProGpuGlyphRun {
    let scope = FerroLocator::enter_scope();
    let shaper: Rc<dyn ITextShaperImpl> = Rc::new(ProGpuTextShaper::new());
    FerroLocator::current_mutable().bind::<dyn ITextShaperImpl>().to_constant(shaper.clone());
    let glyph_typeface = glyph_typeface(simulations);
    let units: Vec<u16> = text.encode_utf16().collect();
    let options = TextShaperOptions::with_all(glyph_typeface.clone(), size, 0, None, 0.0, 0.0, None);
    let glyphs: Vec<GlyphInfo> = shaper.shape_text(&ReadOnlyMemory::from_vec(units), &options).glyph_infos().to_vec();
    assert!(!glyphs.is_empty(), "{text:?} has glyphs");
    let run = ProGpuGlyphRun::new(&glyph_typeface, size, &glyphs, origin);
    glyph_typeface.dispose();
    scope.dispose();
    run
}

fn run(text: &str, size: f64, origin: Point) -> ProGpuGlyphRun {
    run_with(text, size, origin, FontSimulations::None)
}

fn black() -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(Color::from_rgb(0, 0, 0))
}

#[test]
fn text_covers_its_outlines_with_the_weight_of_the_platform() {
    let Some(engine) = engine() else { return };
    let black = black();
    let text = run("Hxg", 40.0, Point::new(8.0, 44.0));
    let atlas = render(&engine, |context| context.draw_glyph_run(Some(&black), &text));
    let geometry = ProGpuGeometry::new(text.outline_path().expect("the outlines of the run"));
    let path = render(&engine, |context| context.draw_geometry(Some(&black), None, geometry.as_ref()));

    let (atlas_ink, path_ink) = (atlas.ink_in(0..WIDTH, 0..HEIGHT), path.ink_in(0..WIDTH, 0..HEIGHT));
    assert!(atlas_ink > 100_000, "the text is drawn: {atlas_ink}");
    // The edges of the text lie a fifth of a pixel outside the outline sideways at this size,
    // twice that upwards and not at all downwards: more ink than the fill of the outlines,
    // and not a bold.
    let ratio = atlas_ink as f64 / path_ink as f64;
    assert!((1.04..1.25).contains(&ratio), "the ink of the text over the ink of its outlines: {atlas_ink} against {path_ink}");
    // The text covers its outlines: no pixel has clearly less ink than the fill, and only
    // edges differ. An edge that faces down is where the fill has it; the fill is sampled on
    // a grid of four and the text on a grid of eight, which reads the same edge up to an
    // eighth of a pixel apart, and the coverage transfer bends it by a fortieth more.
    let pixels = || (0..HEIGHT).flat_map(|y| (0..WIDTH).map(move |x| (x, y)));
    let lighter: Vec<_> = pixels().filter(|&(x, y)| atlas.ink(x, y) + 40 < path.ink(x, y)).map(|(x, y)| (x, y, atlas.ink(x, y), path.ink(x, y))).collect();
    assert!(lighter.is_empty(), "pixels of the text are lighter than the fill of its outlines: {lighter:?}");
    // The most an edge moves is upwards, not half a pixel at this size.
    let different = pixels().filter(|&(x, y)| atlas.ink(x, y).abs_diff(path.ink(x, y)) > 160).count();
    assert!(different < 8, "{different} pixels of the text are far from the fill of its outlines");
    assert_eq!([255, 255, 255, 255], atlas.rgba(2, 2), "outside the text");
    assert!(atlas.metrics.draw_call_count >= 1, "the run is a draw of the frame");
}

#[test]
fn glyphs_keep_the_quarter_pixel_of_their_place() {
    let Some(engine) = engine() else { return };
    let black = black();
    // Where the left edge of the stem of the l is on a row: the first pixel of full ink, less
    // the ink left of it.
    let left_edge = |frame: &Frame| {
        let full = (0..WIDTH).find(|&x| frame.ink(x, 30) == 255).expect("a stem of full ink");
        f64::from(full) - frame.ink_in(0..full, 30..31) as f64 / 255.0
    };
    let mut edges = Vec::new();
    let mut frames = Vec::new();
    for quarter in 0..5 {
        let text = run("l", 32.0, Point::new(20.0 + f64::from(quarter) * 0.25, 44.0));
        let frame = render(&engine, |context| context.draw_glyph_run(Some(&black), &text));
        edges.push(left_edge(&frame));
        frames.push(frame);
    }
    for (quarter, pair) in edges.windows(2).enumerate() {
        let step = pair[1] - pair[0];
        // The coverage transfer bends the ink of a partly covered pixel by up to a fortieth.
        assert!((0.22..0.28).contains(&step), "a quarter of a pixel moves the ink a quarter of a pixel: step {quarter} is {step} ({edges:?})");
    }
    // A whole pixel on: the same raster, one pixel right.
    for y in 0..HEIGHT {
        for x in 0..WIDTH - 1 {
            assert_eq!(frames[0].rgba(x, y), frames[4].rgba(x + 1, y), "pixel {x},{y}");
        }
    }

    // The baseline is on a whole pixel: a part of a pixel down does not blur the text.
    let above = run("l", 32.0, Point::new(20.0, 44.0));
    let below = run("l", 32.0, Point::new(20.0, 44.3));
    let frame_above = render(&engine, |context| context.draw_glyph_run(Some(&black), &above));
    let frame_below = render(&engine, |context| context.draw_glyph_run(Some(&black), &below));
    assert_eq!(frame_above.pixels, frame_below.pixels);
}

#[test]
fn aliased_text_has_no_grey() {
    let Some(engine) = engine() else { return };
    let black = black();
    let text = run("Rag", 24.0, Point::new(8.3, 40.0));
    let grey = |frame: &Frame| frame.pixels.chunks_exact(4).filter(|pixel| pixel[1] != 0 && pixel[1] != 255).count();
    let smooth = render(&engine, |context| context.draw_glyph_run(Some(&black), &text));
    assert!(grey(&smooth) > 50, "antialiased text has grey edges");

    let by_text_options = render(&engine, |context| {
        context.push_text_options(TextOptions { text_rendering_mode: TextRenderingMode::Alias, ..Default::default() });
        context.draw_glyph_run(Some(&black), &text);
        context.pop_text_options();
    });
    assert_eq!(0, grey(&by_text_options));
    assert!(by_text_options.ink_in(0..WIDTH, 0..HEIGHT) > 20_000, "the aliased text is drawn");

    let by_render_options = render(&engine, |context| {
        context.push_render_options(RenderOptions { text_rendering_mode: TextRenderingMode::Alias, ..Default::default() });
        context.draw_glyph_run(Some(&black), &text);
        context.pop_render_options();
    });
    assert_eq!(by_text_options.pixels, by_render_options.pixels);

    // The text options win over the render options.
    let antialiased_again = render(&engine, |context| {
        context.push_render_options(RenderOptions { text_rendering_mode: TextRenderingMode::Alias, ..Default::default() });
        context.push_text_options(TextOptions { text_rendering_mode: TextRenderingMode::Antialias, ..Default::default() });
        context.draw_glyph_run(Some(&black), &text);
        context.pop_text_options();
        context.pop_render_options();
    });
    assert_eq!(smooth.pixels, antialiased_again.pixels);
}

#[test]
fn text_is_clipped_by_a_rotated_clip() {
    let Some(engine) = engine() else { return };
    let text = run("HHHHHHHH", 32.0, Point::new(4.0, 44.0));
    let black = black();
    let whole = render(&engine, |context| context.draw_glyph_run(Some(&black), &text));
    // A square turned by 45 degrees about (80, 32): a diamond that reaches 28 pixels from there.
    let clipped = render(&engine, |context| {
        context.set_transform(Matrix::create_rotation_at(std::f64::consts::FRAC_PI_4, Point::new(80.0, 32.0)));
        context.push_clip(Rect::new(60.0, 12.0, 40.0, 40.0));
        context.set_transform(Matrix::IDENTITY);
        context.draw_glyph_run(Some(&black), &text);
        context.pop_clip();
    });
    // By the distance of a pixel from the middle of the diamond, along both axes together.
    let (mut outside, mut inside, mut cut) = (0u64, 0u64, 0u64);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let distance = (f64::from(x) + 0.5 - 80.0).abs() + (f64::from(y) + 0.5 - 32.0).abs();
            if distance > 30.0 {
                outside += u64::from(clipped.ink(x, y));
                cut += u64::from(whole.ink(x, y));
            } else if distance < 26.0 {
                assert_eq!(whole.ink(x, y), clipped.ink(x, y), "the text in the diamond is as without it, at ({x}, {y})");
                inside += u64::from(clipped.ink(x, y));
            }
        }
    }
    assert_eq!(0, outside, "nothing outside the diamond");
    assert!(inside > 20_000, "the text in the diamond: {inside}");
    assert!(cut > 20_000, "the text the diamond cuts off: {cut}");
    // In the box of the diamond, past its edge: the clip is the diamond, not its bounds.
    assert!(whole.ink_in(52..58, 40..44) + whole.ink_in(102..108, 40..44) > 500, "text in the corners of the box");
    assert_eq!(0, clipped.ink_in(52..58, 40..44) + clipped.ink_in(102..108, 40..44));
}

#[test]
fn text_takes_the_clip_the_colour_and_the_opacity_of_its_brush() {
    let Some(engine) = engine() else { return };
    let text = run("HHHH", 32.0, Point::new(8.0, 44.0));
    let black = black();
    let whole = render(&engine, |context| context.draw_glyph_run(Some(&black), &text));
    let clipped = render(&engine, |context| {
        // The clip is given under a transform; the text is drawn at whole pixels under it.
        context.set_transform(Matrix::create_translation(4.0, 0.0));
        context.push_clip(Rect::new(0.0, 0.0, 40.0, f64::from(HEIGHT)));
        context.set_transform(Matrix::IDENTITY);
        context.draw_glyph_run(Some(&black), &text);
        context.pop_clip();
    });
    assert_eq!(0, clipped.ink_in(44..WIDTH, 0..HEIGHT), "nothing right of the clip");
    assert_eq!(whole.ink_in(0..44, 0..HEIGHT), clipped.ink_in(0..44, 0..HEIGHT), "the text in the clip is as without it");
    assert!(whole.ink_in(44..WIDTH, 0..HEIGHT) > 20_000);

    // The colour of the brush, and its opacity over the white of the target.
    let red = ImmutableSolidColorBrush::new(Color::from_rgb(255, 0, 0));
    let coloured = render(&engine, |context| context.draw_glyph_run(Some(&red), &text));
    let half = ImmutableSolidColorBrush::with_opacity(Color::from_rgb(0, 0, 0), 0.5);
    let faded = render(&engine, |context| context.draw_glyph_run(Some(&half), &text));
    // A pixel in the left stem of the first H.
    let stem = (0..WIDTH).find(|&x| whole.rgba(x, 30) == [0, 0, 0, 255]).expect("a stem of full ink");
    assert_eq!([255, 0, 0, 255], coloured.rgba(stem, 30));
    let grey = faded.rgba(stem, 30);
    assert!((126..=129).contains(&grey[0]) && grey[0] == grey[1] && grey[1] == grey[2], "half the ink: {grey:?}");

    // A brush that is not one colour fills the outlines of the run.
    let gradient = ImmutableLinearGradientBrush::new(
        &[ImmutableGradientStop::new(0.0, Color::from_rgb(255, 0, 0)), ImmutableGradientStop::new(1.0, Color::from_rgb(0, 0, 255))],
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.5, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.5, RelativeUnit::Relative)),
        None,
    );
    let shaded = render(&engine, |context| context.draw_glyph_run(Some(&gradient), &text));
    let last_stem = (0..WIDTH).rev().find(|&x| whole.rgba(x, 30) == [0, 0, 0, 255]).expect("a stem of full ink");
    let (first, last) = (shaded.rgba(stem, 30), shaded.rgba(last_stem, 30));
    assert!(first[0] > 200 && first[2] < 60, "the start of the gradient: {first:?}");
    assert!(last[2] > 200 && last[0] < 60, "the end of the gradient: {last:?}");
}

#[test]
fn text_follows_the_transform() {
    let Some(engine) = engine() else { return };
    let black = black();
    let text = run("H", 16.0, Point::new(4.0, 16.0));
    let plain = render(&engine, |context| context.draw_glyph_run(Some(&black), &text));
    let plain_ink = plain.ink_in(0..WIDTH, 0..HEIGHT);

    // A scale and a translation keep the text upright: glyphs at whole pixels, made at the
    // scale.
    let doubled = render(&engine, |context| {
        context.set_transform(Matrix::create_scale(2.0, 2.0) * Matrix::create_translation(40.0, 10.0));
        context.draw_glyph_run(Some(&black), &text);
    });
    let doubled_ink = doubled.ink_in(0..WIDTH, 0..HEIGHT);
    let ratio = doubled_ink as f64 / plain_ink as f64;
    assert!((3.8..4.2).contains(&ratio), "twice the size is four times the ink: {plain_ink} then {doubled_ink}");
    assert_eq!(0, doubled.ink_in(0..46, 0..HEIGHT), "the text is moved");
    assert!(doubled.pixels.chunks_exact(4).any(|pixel| pixel == [0, 0, 0, 255]), "the stems of the scaled text are full ink");

    // A quarter turn about the origin of the run: the same ink, lying on its side.
    let turned = render(&engine, |context| {
        context.set_transform(
            Matrix::create_translation(-4.0, -16.0) * Matrix::create_rotation(std::f64::consts::FRAC_PI_2) * Matrix::create_translation(100.0, 20.0),
        );
        context.draw_glyph_run(Some(&black), &text);
    });
    let turned_ink = turned.ink_in(0..WIDTH, 0..HEIGHT);
    let ratio = turned_ink as f64 / plain_ink as f64;
    assert!((0.95..1.05).contains(&ratio), "a turn keeps the ink: {plain_ink} then {turned_ink}");
    // Upright the H is above its baseline and right of its origin; turned a quarter it is right
    // of the origin and below it.
    assert_eq!(turned_ink, turned.ink_in(98..116, 18..36), "the turned text is where the turn puts it");
}

#[test]
fn simulated_styles_are_drawn() {
    let Some(engine) = engine() else { return };
    let black = black();
    let origin = Point::new(20.0, 50.0);
    let regular = run("l", 48.0, origin);
    let bold = run_with("l", 48.0, origin, FontSimulations::Bold);
    let oblique = run_with("l", 48.0, origin, FontSimulations::Oblique);
    let plain = render(&engine, |context| context.draw_glyph_run(Some(&black), &regular));
    let heavy = render(&engine, |context| context.draw_glyph_run(Some(&black), &bold));
    let slanted = render(&engine, |context| context.draw_glyph_run(Some(&black), &oblique));

    let (plain_ink, heavy_ink, slanted_ink) =
        (plain.ink_in(0..WIDTH, 0..HEIGHT), heavy.ink_in(0..WIDTH, 0..HEIGHT), slanted.ink_in(0..WIDTH, 0..HEIGHT));
    // The stem of the l grows by the widening of the simulation: two pixels at 48.
    let stem_height = plain_ink as f64 / 255.0 / (plain.ink_in(0..WIDTH, 30..31) as f64 / 255.0);
    let widening = (heavy_ink - plain_ink) as f64 / 255.0 / stem_height;
    assert!((1.8..2.2).contains(&widening), "a simulated bold is wider by a twenty-fourth of the size: {widening}");
    let middle_shift = (heavy.ink_middle(0..WIDTH, 0..HEIGHT) - plain.ink_middle(0..WIDTH, 0..HEIGHT)).abs();
    assert!(middle_shift < 0.05, "the widening is on both sides: {middle_shift}");

    // The top of a simulated oblique is right of its foot, by the shear.
    let ratio = slanted_ink as f64 / plain_ink as f64;
    assert!((0.97..1.03).contains(&ratio), "a shear keeps the ink: {plain_ink} then {slanted_ink}");
    let lean = slanted.ink_middle(0..WIDTH, 18..22) - slanted.ink_middle(0..WIDTH, 44..48);
    assert!((7.0..8.6).contains(&lean), "26 pixels up lean 7.8 pixels right: {lean}");
    let upright = plain.ink_middle(0..WIDTH, 18..22) - plain.ink_middle(0..WIDTH, 44..48);
    assert!(upright.abs() < 0.05, "the l of the face is upright: {upright}");
}

#[test]
fn a_new_glyph_does_not_rasterize_the_others_again() {
    let Some(engine) = engine() else { return };
    let black = black();
    let first = run("atlas", 24.0, Point::new(8.0, 28.0));
    let second = run("Q", 24.0, Point::new(8.0, 56.0));

    // The first frame makes the rasters of its glyphs.
    let (one, scene) = render_scene(&engine, SceneBuilder::new(), |context| context.draw_glyph_run(Some(&black), &first));
    assert!(one.metrics.coverage_staging_bytes > 0, "the first frame rasterizes its glyphs");

    // The same text again: nothing is rasterized.
    let (again, scene) = render_scene(&engine, scene, |context| context.draw_glyph_run(Some(&black), &first));
    assert_eq!(0, again.metrics.coverage_staging_bytes, "glyphs in the atlas are not rasterized again");
    assert_eq!(one.pixels, again.pixels);

    // A new glyph: its raster only, and the others are drawn as before.
    let (more, scene) = render_scene(&engine, scene, |context| {
        context.draw_glyph_run(Some(&black), &first);
        context.draw_glyph_run(Some(&black), &second);
    });
    assert!(
        more.metrics.coverage_staging_bytes > 0 && more.metrics.coverage_staging_bytes < one.metrics.coverage_staging_bytes / 2,
        "one new glyph is one raster: {} bytes after {}",
        more.metrics.coverage_staging_bytes,
        one.metrics.coverage_staging_bytes
    );
    let row = (WIDTH * 4 * 36) as usize;
    assert_eq!(one.pixels[..row], more.pixels[..row], "the first line is as it was");
    assert!(more.ink_in(0..WIDTH, 36..HEIGHT) > 5_000, "the new glyph is drawn");

    // The first text leaves and comes back: its rasters are still there.
    let (_, scene) = render_scene(&engine, scene, |context| context.draw_glyph_run(Some(&black), &second));
    let (back, _) = render_scene(&engine, scene, |context| context.draw_glyph_run(Some(&black), &first));
    assert_eq!(0, back.metrics.coverage_staging_bytes, "glyphs stay in the atlas while they are not drawn");
    assert_eq!(one.pixels, back.pixels);
}
