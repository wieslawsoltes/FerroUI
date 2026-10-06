//! Port of `Rendering/SceneGraph/RenderDataStreamTests.cs`.
//!
//! The mocked drawing context of upstream (`Mock<IDrawingContextImpl>` and
//! its `Verify` calls) is the recording drawing context of the test doubles:
//! a verified call is an entry of its log. The `RefCount` of the counted
//! bitmap and glyph run references is the strong count of their handles:
//! the stream holds one reference per recorded draw until it is disposed.

use super::scene_graph_test_support::{
    b, black, black_pen, p, replay, rrect, TestBitmapImpl, TestCustomOperation, TestGeometryImpl,
};
use crate::media::{BoxShadow, BoxShadows, EdgeMode, RenderOptions, TextOptions, TextRenderingMode};
use crate::platform::{IBitmapImpl, IDrawingContextImpl, IGlyphRunImpl};
use crate::rendering::composition::drawing::{RenderDataResource, RenderDataStream};
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl, MockGlyphRunImpl};
use crate::{Matrix, Point, Rect};
use std::rc::Rc;

#[test]
fn replay_forwards_line() {
    let pen = black_pen(1.0);

    let mut stream = RenderDataStream::new();
    stream.draw_line(p(&pen), p(&pen), Point::new(1.0, 2.0), Point::new(3.0, 4.0));
    let calls = replay(&stream);

    assert_eq!(calls, ["DrawLine Black@1 1, 2 3, 4"]);
}

#[test]
fn replay_forwards_rectangle_with_box_shadows() {
    let brush = black();
    let pen = black_pen(1.0);
    let rect = rrect(0.0, 0.0, 10.0, 20.0);
    let shadows = BoxShadows::with_rest(
        BoxShadow { blur: 1.0, ..Default::default() },
        &[BoxShadow { blur: 2.0, ..Default::default() }],
    );

    let mut stream = RenderDataStream::new();
    stream.draw_rectangle(b(&brush), p(&pen), p(&pen), rect, &shadows);
    let calls = replay(&stream);

    assert_eq!(calls, ["DrawRectangle Black Black@1 0, 0, 10, 20 shadows=2"]);
}

#[test]
fn replay_forwards_custom_operation() {
    let operation = TestCustomOperation::new();

    let mut stream = RenderDataStream::new();
    stream.draw_custom(Some(operation.clone()));
    replay(&stream);

    assert_eq!(operation.render_count.get(), 1);
}

#[test]
fn replay_pop_dispatches_to_matching_pop_in_lifo_order() {
    let mut stream = RenderDataStream::new();
    stream.push_clip(rrect(0.0, 0.0, 10.0, 10.0));
    stream.push_opacity(0.5);
    stream.pop();
    stream.pop();
    let calls = recorded_calls(&stream);

    assert_eq!(calls, ["PushClip", "PushOpacity", "PopOpacity", "PopClip"]);
}

#[test]
fn replay_applies_and_restores_transform() {
    let mut context = MockDrawingContextImpl::new(DrawingLog::new());
    context.set_transform(Matrix::IDENTITY);
    let matrix = Matrix::create_translation(5.0, 7.0);

    let mut stream = RenderDataStream::new();
    stream.push_transform(matrix);
    stream.pop();

    stream.replay(&mut context);

    assert_eq!(Matrix::IDENTITY, context.transform());
}

#[test]
fn replay_skips_opacity_one_push() {
    let mut stream = RenderDataStream::new();
    stream.push_opacity(1.0);
    stream.pop();
    let calls = recorded_calls(&stream);

    assert!(calls.is_empty());
}

#[test]
fn replay_skips_null_geometry_clip() {
    let mut stream = RenderDataStream::new();
    stream.push_geometry_clip(None);
    stream.pop();
    let calls = recorded_calls(&stream);

    assert!(calls.is_empty());
}

#[test]
fn replay_walks_nested_pushes_in_order() {
    let geometry = TestGeometryImpl::new();

    let mut stream = RenderDataStream::new();
    stream.push_clip(rrect(0.0, 0.0, 10.0, 10.0));
    stream.push_geometry_clip(Some(RenderDataResource::GeometryImpl(geometry)));
    stream.pop();
    stream.pop();
    let calls = recorded_calls(&stream);

    assert_eq!(calls, ["PushClip", "PushGeometryClip", "PopGeometryClip", "PopClip"]);
}

#[test]
fn replay_forwards_render_options() {
    let options = RenderOptions { edge_mode: EdgeMode::Aliased, ..Default::default() };

    let mut stream = RenderDataStream::new();
    stream.push_render_options(options);
    stream.pop();
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.log_transforms = false;
    context.log_render_options = true;
    stream.replay(&mut context);

    assert_eq!(
        log.entries(),
        ["PushRenderOptions Unspecified Unspecified Aliased Unspecified None", "PopRenderOptions"]
    );
}

#[test]
fn replay_forwards_text_options() {
    let options = TextOptions { text_rendering_mode: TextRenderingMode::Antialias, ..Default::default() };

    let mut stream = RenderDataStream::new();
    stream.push_text_options(options);
    stream.pop();
    let calls = replay(&stream);

    assert_eq!(calls, ["PushTextOptions Antialias Unspecified Unspecified", "PopTextOptions"]);
}

#[test]
fn dispose_resources_disposes_owned_resources() {
    let bitmap: Rc<dyn IBitmapImpl> = Rc::new(TestBitmapImpl);
    let glyph_run: Rc<dyn IGlyphRunImpl> = MockGlyphRunImpl::new(Rect::default());
    let operation = TestCustomOperation::new();

    {
        let mut stream = RenderDataStream::new();
        stream.draw_bitmap(Some(bitmap.clone()), 1.0, Rect::new(0.0, 0.0, 1.0, 1.0), Rect::new(0.0, 0.0, 1.0, 1.0));
        stream.draw_glyph_run(None, Some(glyph_run.clone()));
        stream.draw_custom(Some(operation.clone()));

        assert_eq!(2, Rc::strong_count(&bitmap));
        assert_eq!(2, Rc::strong_count(&glyph_run));

        stream.dispose_resources();
        stream.dispose();
    }

    assert_eq!(1, Rc::strong_count(&bitmap));
    assert_eq!(1, Rc::strong_count(&glyph_run));
    assert_eq!(1, operation.dispose_count.get());
}

#[test]
fn replay_handles_deeply_nested_scopes() {
    let mut stream = RenderDataStream::new();
    for _ in 0..100 {
        stream.push_opacity(0.5);
    }
    stream.draw_rectangle(b(&black()), None, None, rrect(0.0, 0.0, 10.0, 10.0), &BoxShadows::default());
    for _ in 0..100 {
        stream.pop();
    }

    let calls = replay(&stream);

    assert_eq!(1, calls.iter().filter(|call| call.starts_with("DrawRectangle Black none ")).count());
}

/// `RecordingContext(calls)`: the calls of the clip, geometry clip and
/// opacity scopes, by name. The rounded clip of the stream is the
/// `PushClip(RoundedRect)` overload of upstream.
fn recorded_calls(stream: &RenderDataStream) -> Vec<String> {
    const RECORDED: [&str; 6] =
        ["PushClip", "PopClip", "PushGeometryClip", "PopGeometryClip", "PushOpacity", "PopOpacity"];
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.log_transforms = false;
    stream.replay(&mut context);
    log.entries()
        .iter()
        .map(|entry| entry.split(' ').next().unwrap_or_default())
        .map(|name| if name == "PushRoundedClip" { "PushClip" } else { name })
        .filter(|name| RECORDED.contains(name))
        .map(str::to_owned)
        .collect()
}
