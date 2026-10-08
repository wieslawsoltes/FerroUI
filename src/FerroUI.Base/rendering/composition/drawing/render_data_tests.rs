//! Tests of render data that are not from upstream: glyph runs and text
//! options in the stream, recording through the drawing context, and the
//! path of render data through a batch to the server. The upstream suites
//! of the stream (`Rendering/SceneGraph/*Tests.cs`) are ported in
//! `rendering/scene_graph/`.

use super::*;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{
    BoxShadows, Colors, DrawingContext, IBrush, ImmediateDrawingContext, IntersectionResult,
    MediaContext, RectangleGeometry,
};
use crate::media::text_formatting::testing::{utf16, TextTestScope};
use crate::media::{
    BaselinePixelAlignment, FormattedText, GlyphRun, TextHintingMode, TextOptions, TextRenderingMode, Typeface,
};
use crate::platform::{IDrawingContextImpl, IGlyphRunImpl};
use crate::rendering::composition::transport::{BatchStreamData, BatchStreamReader, BatchStreamWriter};
use crate::rendering::composition::Compositor;
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::rendering::testing::recorded_opcodes;
use crate::rendering::testing::{
    DrawingLog, ManualRenderLoop, MockDrawingContextImpl, MockGlyphRunImpl,
    MockPlatformRenderInterface,
};
use crate::threading::Dispatcher;
use crate::*;
use std::cell::Cell;
use std::rc::Rc;

fn brush() -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(Colors::RED))
}

fn b(brush: &Rc<dyn IBrush>) -> Option<RenderDataResource> {
    Some(RenderDataResource::Brush(brush.clone()))
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, w, h))
}

/// A glyph run of the default test font at an em size of 12.
fn glyph_run(text: &str) -> Rc<GlyphRun> {
    let glyph_typeface = Typeface::default_typeface().glyph_typeface();
    let glyphs: Vec<u16> =
        text.chars().map(|c| glyph_typeface.character_to_glyph_map().get_glyph(c as i32)).collect();
    GlyphRun::from_glyph_indices(glyph_typeface, 12.0, utf16(text), &glyphs, None, 0)
}

fn tile_brush() -> Rc<dyn crate::media::ITileBrush> {
    Rc::new(crate::media::immutable::ImmutableImageBrush::from_bitmap(None))
}

fn replay(stream: &RenderDataStream) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.log_transforms = false;
    stream.replay(&mut context);
    log.entries()
}

use crate::rendering::scene_graph::scene_graph_test_support::Counter;

struct TestOperation {
    bounds: Rect,
    rendered: Counter,
    disposed: Counter,
}

impl TestOperation {
    fn new(bounds: Rect) -> std::sync::Arc<TestOperation> {
        std::sync::Arc::new(TestOperation { bounds, rendered: Counter::new(0), disposed: Counter::new(0) })
    }
}

impl ICustomDrawOperation for TestOperation {
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn hit_test(&self, p: Point) -> bool {
        self.bounds.contains(p)
    }
    fn render(&self, _context: &mut ImmediateDrawingContext<'_>) {
        self.rendered.set(self.rendered.get() + 1);
    }
    fn equals(&self, other: &dyn ICustomDrawOperation) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const dyn ICustomDrawOperation)
    }
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}

// --- replay ---------------------------------------------------------------

#[test]
fn replay_forwards_glyph_run_and_skips_a_missing_one() {
    let glyph_run: std::sync::Arc<dyn IGlyphRunImpl> = MockGlyphRunImpl::new(Rect::new(1.0, 2.0, 30.0, 12.0));
    let mut stream = RenderDataStream::new();
    stream.draw_glyph_run(b(&brush()), Some(glyph_run.clone()));
    stream.draw_glyph_run(b(&brush()), None);
    assert_eq!(replay(&stream), ["DrawGlyphRun Red 1, 2, 30, 12"]);

    // The stream holds a reference to the platform glyph run until it is
    // disposed.
    assert_eq!(std::sync::Arc::strong_count(&glyph_run), 2);
    stream.dispose_resources();
    stream.dispose();
    assert_eq!(std::sync::Arc::strong_count(&glyph_run), 1);
}

#[test]
fn resources_are_interned_by_reference() {
    let (b1, b2) = (brush(), brush());
    let mut stream = RenderDataStream::new();
    for _ in 0..3 {
        stream.draw_ellipse(b(&b1), None, None, Rect::new(0.0, 0.0, 1.0, 1.0));
    }
    stream.draw_ellipse(b(&b2), None, None, Rect::new(0.0, 0.0, 1.0, 1.0));
    assert_eq!(stream.resource_count(), 2);
    let mut resources = RenderDataResources::default();
    assert_eq!(resources.intern(None), NULL_HANDLE);
    assert!(resources.get(NULL_HANDLE).is_none());
}

// --- bounds ---------------------------------------------------------------

#[test]
fn glyph_run_bounds_are_its_platform_bounds_inside_text_options_and_transforms() {
    let glyph_run: std::sync::Arc<dyn IGlyphRunImpl> = MockGlyphRunImpl::new(Rect::new(1.0, 2.0, 30.0, 12.0));
    let mut stream = RenderDataStream::new();
    stream.draw_glyph_run(b(&brush()), Some(glyph_run.clone()));
    assert_eq!(stream.calculate_bounds(), Some(Rect::new(1.0, 2.0, 30.0, 12.0)));

    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(b(&brush()), None, None, rect(0.0, 0.0, 5.0, 5.0), &BoxShadows::default());
    stream.push_text_options(TextOptions::default());
    stream.push_transform(Matrix::create_translation(100.0, 50.0));
    stream.draw_glyph_run(b(&brush()), Some(glyph_run));
    stream.pop();
    stream.pop();
    assert_eq!(stream.calculate_bounds(), Some(Rect::new(0.0, 0.0, 131.0, 64.0)));

    // A missing glyph run contributes the empty rectangle at the origin.
    let mut stream = RenderDataStream::new();
    stream.draw_glyph_run(b(&brush()), None);
    assert_eq!(stream.calculate_bounds(), Some(Rect::default()));

    // An empty text options scope contributes nothing.
    let mut stream = RenderDataStream::new();
    stream.push_text_options(TextOptions::default());
    stream.pop();
    assert_eq!(stream.calculate_bounds(), None);
}

// --- hit testing ----------------------------------------------------------

#[test]
fn glyph_run_is_hit_inside_its_bounds_also_inside_a_text_options_scope() {
    let glyph_run: std::sync::Arc<dyn IGlyphRunImpl> = MockGlyphRunImpl::new(Rect::new(10.0, 10.0, 30.0, 12.0));
    let mut stream = RenderDataStream::new();
    stream.push_text_options(TextOptions::default());
    stream.draw_glyph_run(b(&brush()), Some(glyph_run.clone()));
    stream.draw_glyph_run(b(&brush()), None);
    stream.pop();
    assert!(stream.hit_test(Point::new(10.0, 10.0)));
    assert!(stream.hit_test(Point::new(39.0, 21.0)));
    // The right and bottom edges are excluded.
    assert!(!stream.hit_test(Point::new(40.0, 15.0)));
    assert!(!stream.hit_test(Point::new(15.0, 22.0)));
    assert!(!stream.hit_test(Point::new(5.0, 15.0)));

    // The hit test does not depend on the brush.
    let mut stream = RenderDataStream::new();
    stream.draw_glyph_run(None, Some(glyph_run.clone()));
    assert!(stream.hit_test(Point::new(15.0, 15.0)));

    // A geometry is tested against the bounds of the run.
    let (scope, _render_interface) = MockPlatformRenderInterface::install();
    let mut stream = RenderDataStream::new();
    stream.draw_glyph_run(b(&brush()), Some(glyph_run));
    let inside = RectangleGeometry::with_rect(Rect::new(15.0, 12.0, 2.0, 2.0)).upcast();
    assert_eq!(stream.hit_test_geometry(&inside), IntersectionResult::FullyContains);
    scope.dispose();
}

#[test]
fn a_clip_restricts_the_hit_test_of_a_glyph_run_and_text_options_keep_it() {
    let glyph_run: std::sync::Arc<dyn IGlyphRunImpl> = MockGlyphRunImpl::new(Rect::new(0.0, 0.0, 30.0, 12.0));
    let mut stream = RenderDataStream::new();
    stream.push_clip(rect(0.0, 0.0, 10.0, 12.0));
    stream.push_text_options(TextOptions::default());
    stream.draw_glyph_run(b(&brush()), Some(glyph_run));
    stream.pop();
    stream.pop();
    assert!(stream.hit_test(Point::new(5.0, 5.0)));
    assert!(!stream.hit_test(Point::new(20.0, 5.0)));
}

#[test]
fn geometry_hit_test_reports_the_intersection() {
    let (scope, _render_interface) = MockPlatformRenderInterface::install();
    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(b(&brush()), None, None, rect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    let inside = RectangleGeometry::with_rect(Rect::new(2.0, 2.0, 2.0, 2.0)).upcast();
    let outside = RectangleGeometry::with_rect(Rect::new(50.0, 50.0, 2.0, 2.0)).upcast();
    // The hit test geometry lies inside the rectangle, which fully contains it.
    assert_eq!(stream.hit_test_geometry(&inside), IntersectionResult::FullyContains);
    assert_eq!(stream.hit_test_geometry(&outside), IntersectionResult::Empty);
    scope.dispose();
}

// --- serialization ----------------------------------------------------------

fn round_trip(stream: &RenderDataStream) -> RenderDataStream {
    let mut data = BatchStreamData::new();
    stream.serialize_to(&mut BatchStreamWriter::new(&mut data));
    let mut result = RenderDataStream::new();
    let mut reader = BatchStreamReader::new(&mut data);
    result.deserialize_from(&mut reader, &|_| None);
    assert!(reader.is_object_eof() && reader.is_struct_eof());
    result
}

#[test]
fn round_trip_preserves_text_options_and_glyph_runs() {
    let glyph_run: std::sync::Arc<dyn IGlyphRunImpl> = MockGlyphRunImpl::new(Rect::new(1.0, 2.0, 30.0, 12.0));
    let options = TextOptions {
        text_rendering_mode: TextRenderingMode::Alias,
        text_hinting_mode: TextHintingMode::Strong,
        baseline_pixel_alignment: BaselinePixelAlignment::Aligned,
    };
    let mut stream = RenderDataStream::new();
    stream.push_text_options(options);
    stream.draw_glyph_run(b(&brush()), Some(glyph_run.clone()));
    stream.pop();

    let copy = round_trip(&stream);
    assert_eq!(copy.max_depth(), 1);
    assert_eq!(
        replay(&copy),
        ["PushTextOptions Alias Strong Aligned", "DrawGlyphRun Red 1, 2, 30, 12", "PopTextOptions"]
    );
    assert_eq!(copy.calculate_bounds(), Some(Rect::new(1.0, 2.0, 30.0, 12.0)));
    assert!(copy.hit_test(Point::new(5.0, 5.0)));
    match copy.get_resource(1) {
        Some(RenderDataResource::GlyphRun(run)) => assert!(std::sync::Arc::ptr_eq(&**run, &glyph_run)),
        _ => panic!("the glyph run did not survive the round trip"),
    }
}

// --- recording --------------------------------------------------------------

#[test]
fn recording_elides_empty_scopes_and_no_op_pushes() {
    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        let empty = context.push_clip(Rect::new(0.0, 0.0, 5.0, 5.0));
        context.pop(empty);
        let identity = context.push_transform(Matrix::IDENTITY);
        let opaque = context.push_opacity(1.0);
        let clip = context.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
        context.fill_rectangle(&brush(), Rect::new(0.0, 0.0, 4.0, 4.0), 0.0);
        context.pop(clip);
        context.pop(opaque);
        context.pop(identity);
        // Empty rectangles and invisible content are not recorded at all.
        context.fill_rectangle(&brush(), Rect::new(0.0, 0.0, 0.0, 0.0), 0.0);
    }
    let content = recorder
        .get_immediate_scene_brush_content(tile_brush(), None, true)
        .expect("something was drawn");
    assert_eq!(
        content.with_stream(recorded_opcodes),
        Some(vec![RenderDataOpcode::PushClip, RenderDataOpcode::DrawRectangle, RenderDataOpcode::Pop])
    );
    assert_eq!(
        content.with_stream(replay).unwrap(),
        ["PushRoundedClip 0, 0, 10, 10", "DrawRectangle Red none 0, 0, 4, 4 shadows=0", "PopClip"]
    );
}

#[test]
fn recording_nothing_yields_no_stream_and_unpopped_scopes_are_flushed() {
    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        let _ = context.push_opacity(0.5);
    }
    assert!(recorder.get_immediate_scene_brush_content(tile_brush(), None, true).is_none());

    use crate::media::IDrawingContextCore;
    recorder.push_opacity_core(0.5);
    recorder.draw_ellipse_core(Some(&brush()), None, Rect::new(0.0, 0.0, 1.0, 1.0));
    let content = recorder
        .get_immediate_scene_brush_content(tile_brush(), None, true)
        .expect("something was drawn");
    assert_eq!(content.with_stream(RenderDataStream::depth), Some(0));
    assert_eq!(content.with_stream(replay).unwrap().last().map(String::as_str), Some("PopOpacity"));
}

#[test]
fn recording_elides_empty_text_options_scopes_and_glyph_runs_without_foreground() {
    let _text = TextTestScope::new();
    let run = glyph_run("abc");
    let bounds = run.platform_impl().bounds();

    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        let empty = context.push_text_options(TextOptions::default());
        context.draw_glyph_run(None, &run);
        context.pop(empty);
        let options = context
            .push_text_options(TextOptions { text_hinting_mode: TextHintingMode::Light, ..Default::default() });
        context.draw_glyph_run(Some(&brush()), &run);
        context.pop(options);
    }
    let content = recorder
        .get_immediate_scene_brush_content(tile_brush(), None, true)
        .expect("something was drawn");
    assert_eq!(
        content.with_stream(recorded_opcodes),
        Some(vec![RenderDataOpcode::PushTextOptions, RenderDataOpcode::DrawGlyphRun, RenderDataOpcode::Pop])
    );
    assert_eq!(
        content.with_stream(replay).unwrap(),
        [
            "PushTextOptions Unspecified Light Unspecified".to_string(),
            format!("DrawGlyphRun Red {bounds}"),
            "PopTextOptions".to_string()
        ]
    );
    // The recorded run is the platform glyph run of the glyph run.
    content.with_stream(|stream| match stream.get_resource(1) {
        Some(RenderDataResource::GlyphRun(recorded)) => assert!(std::sync::Arc::ptr_eq(&**recorded, &run.platform_impl())),
        _ => panic!("the glyph run was not recorded"),
    });
}

// --- end to end through the compositor -------------------------------------

struct Fixture {
    _dispatcher_scope: crate::threading::UnitTestDispatcherScope,
    locator_scope: Rc<dyn crate::reactive::IDisposable>,
    render_interface: Rc<MockPlatformRenderInterface>,
    render_loop: std::sync::Arc<ManualRenderLoop>,
    compositor: Rc<Compositor>,
}

impl Fixture {
    fn new() -> Fixture {
        let dispatcher_scope = Dispatcher::unit_test_scope();
        let (locator_scope, render_interface) = MockPlatformRenderInterface::install();
        let render_loop = ManualRenderLoop::new();
        let compositor = Compositor::with_scheduler(
            render_loop.clone(),
            None,
            false,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );
        Fixture { _dispatcher_scope: dispatcher_scope, locator_scope, render_interface, render_loop, compositor }
    }

    fn record(&self, draw: impl FnOnce(&mut DrawingContext<'_>)) -> Option<Rc<CompositionRenderData>> {
        let mut recorder = RenderDataDrawingContext::new(Some(self.compositor.clone()));
        {
            let mut context = DrawingContext::new(&mut recorder);
            draw(&mut context);
        }
        recorder.get_render_results()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.locator_scope.dispose();
    }
}

#[test]
fn render_data_reaches_the_server_through_a_batch() {
    let fixture = Fixture::new();
    let render_data = fixture
        .record(|context| {
            let clip = context.push_clip(Rect::new(0.0, 0.0, 10.0, 10.0));
            context.fill_rectangle(&brush(), Rect::new(1.2, 1.2, 4.0, 4.0), 0.0);
            context.pop(clip);
        })
        .expect("something was drawn");
    assert!(render_data.hit_test(Point::new(2.0, 2.0)));
    assert!(!render_data.hit_test(Point::new(8.0, 8.0)));

    // Nothing exists on the server until the batch has been applied.
    let server = fixture.compositor.server().clone();
    assert_eq!(server.object_count(), 0);
    let batch = fixture.compositor.commit();
    assert!(!batch.processed().is_completed());
    fixture.render_loop.tick();
    assert!(batch.processed().is_completed());
    assert!(batch.rendered().is_completed());
    assert_eq!(server.last_batch_id(), batch.sequence_id());

    let server_data = server.get::<ServerCompositionRenderData>(render_data.server()).expect("created by the batch");
    let bounds = server_data.bounds().expect("has content");
    assert_eq!((bounds.left, bounds.top, bounds.right, bounds.bottom), (1.0, 1.0, 6.0, 6.0));

    let log = fixture.render_interface.log().clone();
    let mut context = MockDrawingContextImpl::new(log.clone());
    server_data.render(&mut context);
    assert_eq!(
        log.entries(),
        ["PushRoundedClip 0, 0, 10, 10", "DrawRectangle Red none 1.2, 1.2, 4, 4 shadows=0", "PopClip"]
    );

    // Disposal travels in the next batch.
    render_data.dispose();
    fixture.compositor.commit();
    assert_eq!(server.object_count(), 1);
    fixture.render_loop.tick();
    assert_eq!(server.object_count(), 0);
    assert!(server_data.is_disposed());
}

#[test]
fn text_reaches_the_server_through_a_batch() {
    let _text = TextTestScope::new();
    let fixture = Fixture::new();
    let text = FormattedText::new(
        "abc",
        crate::utilities::CultureInfo::invariant_culture(),
        crate::media::FlowDirection::LeftToRight,
        Typeface::default_typeface(),
        12.0,
        Some(brush()),
    );
    let options = TextOptions { text_rendering_mode: TextRenderingMode::Antialias, ..Default::default() };
    let render_data = fixture
        .record(|context| {
            let state = context.push_text_options(options);
            context.draw_text(&text, Point::new(10.0, 20.0));
            context.pop(state);
        })
        .expect("something was drawn");
    // Three glyphs of 6 by a glyph run height of 12.
    assert!(render_data.hit_test(Point::new(11.0, 21.0)));
    assert!(render_data.hit_test(Point::new(27.0, 31.0)));
    assert!(!render_data.hit_test(Point::new(29.0, 21.0)));
    assert!(!render_data.hit_test(Point::new(9.0, 21.0)));

    let server = fixture.compositor.server().clone();
    let batch = fixture.compositor.commit();
    fixture.render_loop.tick();
    assert!(batch.processed().is_completed());

    let server_data = server.get::<ServerCompositionRenderData>(render_data.server()).expect("created by the batch");
    let bounds = server_data.bounds().expect("has content");
    assert_eq!((bounds.left, bounds.right), (10.0, 28.0));
    // The server bounds are rounded out to whole pixels.
    assert!((12.0..=13.0).contains(&(bounds.bottom - bounds.top)));
    assert!(bounds.top >= 20.0);

    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.log_transforms = false;
    server_data.render(&mut context);
    let entries = log.entries();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0], "PushTextOptions Antialias Unspecified Unspecified");
    assert!(entries[1].starts_with("DrawGlyphRun Red ") && entries[1].ends_with(", 18, 12"));
    assert_eq!(entries[2], "PopTextOptions");

    render_data.dispose();
    fixture.compositor.commit();
    fixture.render_loop.tick();
    assert!(server_data.is_disposed());
}

#[test]
fn commit_is_requested_through_the_scheduler_and_runs_with_the_frame() {
    let fixture = Fixture::new();
    let server = fixture.compositor.server().clone();
    let ran = Rc::new(Cell::new(0));
    let (before, job) = (ran.clone(), ran.clone());
    fixture.compositor.request_composition_update(move || before.set(before.get() + 1));
    fixture.compositor.post_server_job(move |_| job.set(job.get() + 10), false);
    assert!(MediaContext::instance().is_render_scheduled());

    Dispatcher::ui_thread().run_jobs(None);
    // The media context committed; the server has not run yet.
    assert_eq!(ran.get(), 1);
    fixture.render_loop.tick();
    assert_eq!(ran.get(), 11);
    assert!(server.last_batch_id() > 0);
}

#[test]
fn server_object_ids_are_reused_only_after_their_disposal_was_committed() {
    let fixture = Fixture::new();
    let first = fixture.record(|c| c.fill_rectangle(&brush(), Rect::new(0.0, 0.0, 1.0, 1.0), 0.0)).unwrap();
    first.dispose();
    let second = fixture.record(|c| c.fill_rectangle(&brush(), Rect::new(0.0, 0.0, 2.0, 2.0), 0.0)).unwrap();
    assert_ne!(first.server(), second.server());
    fixture.compositor.commit();
    fixture.render_loop.tick();
    let third = fixture.record(|c| c.fill_rectangle(&brush(), Rect::new(0.0, 0.0, 3.0, 3.0), 0.0)).unwrap();
    assert_eq!(third.server(), first.server());
    fixture.compositor.commit();
    fixture.render_loop.tick();
    let server = fixture.compositor.server();
    assert_eq!(server.object_count(), 2);
    let bounds = server.get::<ServerCompositionRenderData>(third.server()).unwrap().bounds().unwrap();
    assert_eq!(bounds.right, 3.0);
}

#[test]
fn compositor_unregisters_from_the_render_loop_when_dropped() {
    let fixture = Fixture::new();
    assert_eq!(fixture.render_loop.task_count(), 1);
    let render_loop = fixture.render_loop.clone();
    drop(fixture);
    assert_eq!(render_loop.task_count(), 0);
}

// --- immediate scene brush content, effects ------------------------------------

#[test]
fn immediate_scene_brush_content_uses_the_given_rect_or_the_rounded_bounds() {
    use crate::media::{IBrush, ISceneBrushContent};

    let record = |rect: Option<Rect>, scalable: bool| {
        let mut recorder = RenderDataDrawingContext::new(None);
        {
            let mut context = DrawingContext::new(&mut recorder);
            context.fill_rectangle(&brush(), Rect::new(0.25, 0.75, 4.0, 4.0), 0.0);
        }
        recorder.get_immediate_scene_brush_content(tile_brush(), rect, scalable).expect("something was drawn")
    };

    let content = record(None, true);
    assert_eq!(content.rect(), Rect::new(0.0, 0.0, 5.0, 5.0));
    assert!(content.use_scalable_rasterization());
    assert_eq!(content.opacity(), 1.0);
    assert!(content.transform().is_none());
    assert!(content.clone().into_immutable_brush().is_some());
    assert!(content.brush().as_tile_brush().is_some());

    let content = record(Some(Rect::new(1.0, 2.0, 3.0, 4.0)), false);
    assert_eq!(content.rect(), Rect::new(1.0, 2.0, 3.0, 4.0));
    assert!(!content.use_scalable_rasterization());

    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.set_transform(Matrix::create_translation(10.0, 0.0));
    log.clear();
    let transform = Matrix::create_scale(2.0, 2.0);
    content.render(&mut context, Some(transform));
    assert_eq!(
        log.entries(),
        [
            format!("SetTransform {}", transform * Matrix::create_translation(10.0, 0.0)),
            "DrawRectangle Red none 0.25, 0.75, 4, 4 shadows=0".to_string(),
            format!("SetTransform {}", Matrix::create_translation(10.0, 0.0)),
        ]
    );
}

#[test]
fn disposing_immediate_scene_brush_content_releases_the_custom_operations() {
    use crate::media::ISceneBrushContent;

    let operation = TestOperation::new(Rect::new(0.0, 0.0, 10.0, 10.0));
    let custom: std::sync::Arc<dyn ICustomDrawOperation> = operation.clone();
    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        context.custom(&custom);
    }
    let content = recorder.get_immediate_scene_brush_content(tile_brush(), None, true).expect("something was drawn");
    assert_eq!(content.rect(), Rect::new(0.0, 0.0, 10.0, 10.0));

    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    content.render(&mut context, None);
    assert_eq!(operation.rendered.get(), 1);
    assert_eq!(operation.disposed.get(), 0);

    content.dispose();
    assert_eq!(operation.disposed.get(), 1);
    assert!(content.with_stream(|_| ()).is_none());
    content.render(&mut context, None);
    assert_eq!(operation.rendered.get(), 1);
    content.dispose();
    assert_eq!(operation.disposed.get(), 1);
}

#[test]
fn replaying_an_effect_uses_the_effect_support_of_the_platform_context() {
    use crate::media::effects::{IEffect, ImmutableBlurEffect};

    let effect: Rc<dyn IEffect> = Rc::new(ImmutableBlurEffect::new(4.0));
    let mut stream = RenderDataStream::new();
    stream.push_effect(Some(effect), Rect::new(0.0, 0.0, 10.0, 10.0));
    let red = brush();
    stream.draw_rectangle(b(&red), None, None, rect(1.0, 1.0, 2.0, 2.0), &BoxShadows::default());
    stream.pop();

    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    stream.replay(&mut context);
    assert_eq!(
        log.entries(),
        ["PushEffect 0, 0, 10, 10", "DrawRectangle Red none 1, 1, 2, 2 shadows=0", "PopEffect"]
    );

    // Without effect support the content is drawn as is.
    assert_eq!(replay(&stream), ["DrawRectangle Red none 1, 1, 2, 2 shadows=0"]);
}

#[test]
fn dispatcher_processes_jobs_again_after_a_frame() {
    let fixture = Fixture::new();
    fixture.compositor.commit();
    fixture.render_loop.tick();
    // Rendering disables dispatcher processing for the frame only.
    let ran = Rc::new(Cell::new(false));
    let r = ran.clone();
    Dispatcher::ui_thread().post_local(move || r.set(true), crate::threading::DispatcherPriority::NORMAL);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(ran.get());
}
