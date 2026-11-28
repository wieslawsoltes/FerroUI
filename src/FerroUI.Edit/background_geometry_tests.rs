use crate::background_geometry_builder::{BackgroundGeometryBuilder, BackgroundGeometryError};
use crate::background_geometry_rects::{
    BackgroundGeometryLine, BackgroundGeometryViewport, get_rects_for_segment,
    get_rects_for_selection, get_rects_from_visual_segment,
};
use crate::{
    BackgroundRendererRef, IBackgroundRenderer, ITextViewConnect, KnownLayer, TextArea, TextEditor,
    VisualLine, VisualLineBuildOptions, VisualLineBuilder, VisualLineError,
};
use ferroui_base::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, TextLayout, TextRunProperties,
};
use ferroui_base::media::{
    DrawingContext, FillRule, FontFamily, SweepDirection, TextAlignment, TextWrapping, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Point, Rect, Size};
use ferroui_edit_core::document::{SimpleSegment, TextSnapshot};
use ferroui_edit_core::editing::SelectionSegment;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
enum Command {
    Fill(FillRule),
    Begin(Point, bool),
    Line(Point),
    Arc(Point, Size, SweepDirection),
    End(bool),
}
#[derive(Default)]
struct Recorder(Vec<Command>);
impl IGeometryContext for Recorder {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        angle: f64,
        large: bool,
        direction: SweepDirection,
        stroked: bool,
    ) {
        assert_eq!(angle, 0.0);
        assert!(!large);
        assert!(stroked);
        self.0.push(Command::Arc(point, size, direction));
    }
    fn begin_figure(&mut self, start: Point, filled: bool) {
        self.0.push(Command::Begin(start, filled));
    }
    fn line_to(&mut self, point: Point, stroked: bool) {
        assert!(stroked);
        self.0.push(Command::Line(point));
    }
    fn end_figure(&mut self, closed: bool) {
        self.0.push(Command::End(closed));
    }
    fn set_fill_rule(&mut self, rule: FillRule) {
        self.0.push(Command::Fill(rule));
    }
    fn cubic_bezier_to(&mut self, _: Point, _: Point, _: Point, _: bool) {
        panic!("unexpected cubic");
    }
    fn quadratic_bezier_to(&mut self, _: Point, _: Point, _: bool) {
        panic!("unexpected quadratic");
    }
    fn dispose(&mut self) {
        panic!("the builder must not dispose a caller-owned context");
    }
}
fn commands(builder: &mut BackgroundGeometryBuilder) -> Vec<Command> {
    let mut recorder = Recorder::default();
    builder.write_to(&mut recorder);
    recorder.0
}
fn approx(a: f64, b: f64) {
    assert!((a - b).abs() < 0.00001, "{a} != {b}");
}
fn rectangle(actual: Rect, expected: Rect) {
    approx(actual.x, expected.x);
    approx(actual.y, expected.y);
    approx(actual.width, expected.width);
    approx(actual.height, expected.height);
}

#[test]
fn rounded_rectangle_writes_filled_native_arcs_and_a_closed_outline() {
    use SweepDirection::Clockwise;
    let mut builder = BackgroundGeometryBuilder::new();
    assert!(builder.create_geometry().is_none());
    builder.corner_radius = 2.0;
    builder
        .add_rectangle(Rect::new(10.0, 20.0, 20.0, 20.0))
        .unwrap();
    assert_eq!(
        commands(&mut builder),
        vec![
            Command::Fill(FillRule::EvenOdd),
            Command::Begin(Point::new(10.0, 22.0), true),
            Command::Arc(Point::new(12.0, 20.0), Size::new(2.0, 2.0), Clockwise),
            Command::Line(Point::new(28.0, 20.0)),
            Command::Arc(Point::new(30.0, 22.0), Size::new(2.0, 2.0), Clockwise),
            Command::Line(Point::new(30.0, 38.0)),
            Command::Arc(Point::new(28.0, 40.0), Size::new(2.0, 2.0), Clockwise),
            Command::Line(Point::new(12.0, 40.0)),
            Command::Arc(Point::new(10.0, 38.0), Size::new(2.0, 2.0), Clockwise),
            Command::Line(Point::new(10.0, 22.0)),
            Command::End(true),
        ]
    );
}

#[test]
fn expanding_and_contracting_rows_keep_inner_and_outer_corner_directions() {
    use SweepDirection::{Clockwise, CounterClockwise};
    let mut builder = BackgroundGeometryBuilder::new();
    builder.corner_radius = 2.0;
    for rect in [
        Rect::new(10.0, 0.0, 20.0, 10.0),
        Rect::new(5.0, 10.0, 30.0, 10.0),
        Rect::new(15.0, 20.0, 10.0, 10.0),
    ] {
        builder.add_rectangle(rect).unwrap();
    }
    let path = commands(&mut builder);
    assert_eq!(
        path.iter()
            .filter(|command| matches!(command, Command::Begin(..)))
            .count(),
        1
    );
    for (point, direction) in [
        (Point::new(32.0, 10.0), CounterClockwise),
        (Point::new(35.0, 12.0), Clockwise),
        (Point::new(33.0, 20.0), Clockwise),
        (Point::new(25.0, 22.0), CounterClockwise),
        (Point::new(13.0, 20.0), CounterClockwise),
        (Point::new(5.0, 18.0), Clockwise),
        (Point::new(7.0, 10.0), Clockwise),
        (Point::new(10.0, 8.0), CounterClockwise),
    ] {
        assert!(path.contains(&Command::Arc(point, Size::new(2.0, 2.0), direction)));
    }
    assert_eq!(path.last(), Some(&Command::End(true)));
}

#[test]
fn vertical_tolerance_and_explicit_closure_control_separate_figures() {
    let mut builder = BackgroundGeometryBuilder::new();
    builder
        .add_rectangle(Rect::new(0.0, 0.0, 10.0, 10.0))
        .unwrap();
    builder
        .add_rectangle(Rect::new(0.005, 10.005, 10.0, 10.0))
        .unwrap();
    builder
        .add_rectangle(Rect::new(0.0, 20.03, 10.0, 10.0))
        .unwrap();
    builder.close_figure();
    builder
        .add_rectangle(Rect::new(0.0, 30.03, 10.0, 10.0))
        .unwrap();
    let path = commands(&mut builder);
    assert_eq!(
        path.iter()
            .filter(|command| matches!(command, Command::Begin(..)))
            .count(),
        3
    );
    assert!(
        path.iter()
            .all(|command| !matches!(command, Command::Arc(..)))
    );
    assert_eq!(path, commands(&mut builder));
    builder.clear();
    assert!(builder.is_empty());
    assert!(commands(&mut builder).is_empty());
}

#[test]
fn whole_pixel_alignment_uses_outer_border_edges_and_away_from_zero_midpoints() {
    let mut builder = BackgroundGeometryBuilder::new();
    builder.align_to_whole_pixels = true;
    builder.border_thickness = 1.0;
    builder
        .add_rectangle_aligned(Rect::new(0.24, 0.26, 3.5, 4.25), Size::new(0.5, 0.25))
        .unwrap();
    let path = commands(&mut builder);
    assert!(path.contains(&Command::Begin(Point::new(0.0, 0.25), true)));
    assert!(path.contains(&Command::Line(Point::new(3.5, 4.5))));
    builder.clear();
    builder.border_thickness = 0.0;
    builder
        .add_rectangle_aligned(Rect::new(-0.25, -0.125, 1.0, 1.0), Size::new(0.5, 0.25))
        .unwrap();
    let path = commands(&mut builder);
    assert!(path.contains(&Command::Begin(Point::new(-0.5, -0.25), true)));
    assert!(path.contains(&Command::Line(Point::new(1.0, 1.0))));
}

#[test]
fn invalid_geometry_is_rejected_without_damaging_prepared_figures() {
    let mut builder = BackgroundGeometryBuilder::new();
    builder
        .add_rectangle(Rect::new(0.0, 0.0, 10.0, 10.0))
        .unwrap();
    let valid = commands(&mut builder);
    assert_eq!(
        builder.add_rectangle(Rect::new(f64::NAN, 0.0, 1.0, 1.0)),
        Err(BackgroundGeometryError::InvalidRectangle)
    );
    builder.corner_radius = -1.0;
    assert_eq!(
        builder.add_rectangle(Rect::new(0.0, 10.0, 10.0, 10.0)),
        Err(BackgroundGeometryError::InvalidCornerRadius)
    );
    builder.corner_radius = 0.0;
    builder.align_to_whole_pixels = true;
    assert_eq!(
        builder.add_rectangle_aligned(Rect::new(0.0, 10.0, 10.0, 10.0), Size::new(0.0, 1.0)),
        Err(BackgroundGeometryError::InvalidPixelSize)
    );
    assert_eq!(valid, commands(&mut builder));
}

#[test]
fn narrow_rectangles_keep_the_upstream_cap_rule_and_options_do_not_corrupt_closed_paths() {
    let mut builder = BackgroundGeometryBuilder::new();
    builder.corner_radius = 2.0;
    builder
        .add_rectangle(Rect::new(0.0, 0.0, 1.0, 10.0))
        .unwrap();
    builder.corner_radius = f64::NAN;
    let path = commands(&mut builder);
    assert_eq!(
        path,
        vec![
            Command::Fill(FillRule::EvenOdd),
            Command::Begin(Point::new(0.0, 2.0), true),
            Command::Line(Point::new(1.0, 8.0)),
            Command::Line(Point::new(0.0, 2.0)),
            Command::End(true),
        ]
    );
}

#[test]
fn native_stream_geometry_snapshots_survive_builder_reuse() {
    let _scope = crate::tests::test_scope();
    let mut builder = BackgroundGeometryBuilder::new();
    builder.corner_radius = 2.0;
    builder
        .add_rectangle(Rect::new(10.0, 20.0, 20.0, 20.0))
        .unwrap();
    let first = builder.create_geometry().unwrap();
    rectangle(first.bounds(), Rect::new(10.0, 20.0, 20.0, 20.0));
    builder
        .add_rectangle(Rect::new(100.0, 100.0, 10.0, 10.0))
        .unwrap();
    let second = builder.create_geometry().unwrap();
    rectangle(second.bounds(), Rect::new(10.0, 20.0, 100.0, 90.0));
    rectangle(first.bounds(), Rect::new(10.0, 20.0, 20.0, 20.0));
}

fn visual(text: &str, width: f64, wrap: TextWrapping) -> (Rc<VisualLine>, TextLayout) {
    let properties = Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::new(FontFamily::new("Test Sans")),
        12.0,
    )) as Rc<dyn TextRunProperties>;
    let line = VisualLineBuilder::generate(
        TextSnapshot::from_text(text),
        1,
        properties.clone(),
        &[],
        VisualLineBuildOptions::default(),
    )
    .unwrap()
    .finish()
    .unwrap();
    let layout = line
        .create_text_layout(
            Rc::new(GenericTextParagraphProperties::with_options(
                properties,
                TextAlignment::Left,
                wrap,
                0.0,
                0.0,
            )),
            width,
            f64::INFINITY,
            0,
            None,
            None,
        )
        .unwrap();
    (line, layout)
}
fn viewport() -> BackgroundGeometryViewport {
    BackgroundGeometryViewport {
        full_width: 80.0,
        default_line_height: 12.0,
        wide_space_width: 6.0,
        empty_line_selection_width: 2.0,
    }
}

#[test]
fn native_projection_uses_utf16_and_translates_shaped_bounds() {
    let _scope = crate::tests::test_scope();
    let (visual_line, layout) = visual("ab😀cd", f64::INFINITY, TextWrapping::NoWrap);
    let line = BackgroundGeometryLine {
        visual_line: &visual_line,
        layout: &layout,
        origin: Point::new(8.0, -3.0),
    };
    let rectangles =
        get_rects_for_segment([line], &SimpleSegment::new(2, 2), viewport(), false).unwrap();
    assert_eq!(rectangles.len(), 1);
    rectangle(rectangles[0], Rect::new(20.0, -3.0, 12.0, layout.height()));
    let caret = get_rects_for_segment(
        [line],
        &SimpleSegment::new(usize::MAX, 10),
        viewport(),
        false,
    )
    .unwrap();
    assert_eq!(caret.len(), 1);
    rectangle(caret[0], Rect::new(44.0, -3.0, 2.0, layout.height()));
}

#[test]
fn bidi_projection_preserves_separate_native_run_rectangles() {
    let _scope = crate::tests::test_scope();
    let (visual_line, layout) = visual("abc אבג xyz", f64::INFINITY, TextWrapping::NoWrap);
    let line = BackgroundGeometryLine {
        visual_line: &visual_line,
        layout: &layout,
        origin: Point::new(-4.0, 2.0),
    };
    let rectangles =
        get_rects_for_segment([line], &SimpleSegment::new(2, 4), viewport(), false).unwrap();
    assert!(
        rectangles.len() > 1,
        "mixed-direction range must keep its disjoint shaped runs"
    );
    assert!(
        rectangles
            .iter()
            .all(|rect| rect.width >= 0.0 && rect.y == 2.0)
    );
    approx(rectangles.iter().map(|rect| rect.width).sum(), 24.0);
}

#[test]
fn wrapped_ranges_extend_to_content_extent_and_skip_duplicate_empty_boundary_markers() {
    let _scope = crate::tests::test_scope();
    let (visual_line, layout) = visual("abc def ghi", 24.0, TextWrapping::Wrap);
    assert_eq!(layout.text_lines().len(), 3);
    let line = BackgroundGeometryLine {
        visual_line: &visual_line,
        layout: &layout,
        origin: Point::default(),
    };
    let rectangles = get_rects_from_visual_segment(line, 0, 11, viewport()).unwrap();
    let height = layout.text_lines()[0].height();
    for y in [0.0, height] {
        approx(
            rectangles
                .iter()
                .filter(|rect| (rect.y - y).abs() < 0.001)
                .map(Rect::right)
                .fold(0.0, f64::max),
            80.0,
        );
    }
    approx(rectangles.last().unwrap().right(), 18.0);
    let (visual_line, layout) = visual("abcdef", 18.0, TextWrapping::Wrap);
    let line = BackgroundGeometryLine {
        visual_line: &visual_line,
        layout: &layout,
        origin: Point::default(),
    };
    let first = get_rects_from_visual_segment(line, 0, 3, viewport()).unwrap();
    assert_eq!(first.len(), 1);
    approx(first[0].y, 0.0);
    let second = get_rects_from_visual_segment(line, 3, 6, viewport()).unwrap();
    assert_eq!(second.len(), 1);
    approx(second[0].y, layout.text_lines()[0].height());
}

#[test]
fn empty_lines_and_selected_newlines_have_visible_rectangles() {
    let _scope = crate::tests::test_scope();
    let (visual_line, layout) = visual("\nnext", f64::INFINITY, TextWrapping::NoWrap);
    let line = BackgroundGeometryLine {
        visual_line: &visual_line,
        layout: &layout,
        origin: Point::new(5.0, 7.0),
    };
    let marker =
        get_rects_for_segment([line], &SimpleSegment::new(0, 1), viewport(), false).unwrap();
    assert_eq!(marker.len(), 1);
    rectangle(marker[0], Rect::new(5.0, 7.0, 2.0, layout.height()));
    let full = get_rects_for_segment([line], &SimpleSegment::new(0, 1), viewport(), true).unwrap();
    assert_eq!(full.len(), 1);
    rectangle(full[0], Rect::new(5.0, 7.0, 80.0, layout.height()));
}

#[test]
fn reversed_virtual_selection_follows_the_physical_right_edge_of_rtl_text() {
    let _scope = crate::tests::test_scope();
    let (visual_line, layout) = visual("aאב", f64::INFINITY, TextWrapping::NoWrap);
    let line = BackgroundGeometryLine {
        visual_line: &visual_line,
        layout: &layout,
        origin: Point::new(10.0, -2.0),
    };
    let rectangles = get_rects_for_selection(
        [line],
        SelectionSegment::with_virtual_space(3, 4, 3, 1),
        viewport(),
        false,
    )
    .unwrap();
    assert_eq!(rectangles.len(), 1);
    rectangle(rectangles[0], Rect::new(34.0, -2.0, 18.0, layout.height()));
}

#[test]
fn background_renderer_values_retain_identity_and_forward_connection_hooks() {
    struct Renderer(Rc<Cell<usize>>);
    impl ITextViewConnect for Renderer {
        fn add_to_text_view(&self, _: &TextArea) {
            self.0.set(self.0.get() + 1);
        }
        fn remove_from_text_view(&self, _: &TextArea) {
            self.0.set(self.0.get() - 1);
        }
    }
    impl IBackgroundRenderer for Renderer {
        fn layer(&self) -> KnownLayer {
            KnownLayer::Selection
        }
        fn draw(&self, _: &TextArea, _: &mut DrawingContext) -> Result<(), VisualLineError> {
            Ok(())
        }
    }
    let _scope = crate::tests::test_scope();
    let count = Rc::new(Cell::new(0));
    let renderer = BackgroundRendererRef::new(Renderer(count.clone()));
    let duplicate = renderer.clone();
    assert_eq!(renderer, duplicate);
    assert_ne!(
        renderer,
        BackgroundRendererRef::new(Renderer(count.clone()))
    );
    let editor = TextEditor::new();
    duplicate.add_to_text_view(&editor.text_area());
    assert_eq!(count.get(), 1);
    renderer.remove_from_text_view(&editor.text_area());
    assert_eq!(count.get(), 0);
    assert!(KnownLayer::Background < KnownLayer::Selection);
    assert!(KnownLayer::Selection < KnownLayer::Text);
    assert!(KnownLayer::Text < KnownLayer::Caret);
}
