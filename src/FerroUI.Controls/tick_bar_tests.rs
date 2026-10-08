//! The reference implementation has no unit tests for `TickBar`; these tests
//! are specific to this port.

use crate::test_support::{test_scope, TestRoot};
use crate::{Slider, TickBar, TickBarPlacement, TickList};
use ferroui_base::layout::Orientation;
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, Geometry, GlyphRun, IBrush, IDrawingContextCore, IEffect, IPen,
    RenderOptions, TextOptions,
};
use ferroui_base::platform::{IBitmapImpl, IGeometryImpl};
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::{Matrix, Point, Rect, Ref, RoundedRect};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Default)]
struct RecordingDrawingContext {
    lines: Vec<(Point, Point)>,
    pens: Vec<Rc<dyn IPen>>,
}

impl IDrawingContextCore for RecordingDrawingContext {
    fn draw_line_core(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.lines.push((p1, p2));
        self.pens.push(pen.clone());
    }

    fn draw_geometry_impl_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        _geometry: &Arc<dyn IGeometryImpl>,
    ) {
    }

    fn draw_rectangle_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        _rrect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
    }

    fn draw_ellipse_core(&mut self, _brush: Option<&Rc<dyn IBrush>>, _pen: Option<&Rc<dyn IPen>>, _rect: Rect) {}
    fn draw_bitmap(&mut self, _source: &std::sync::Arc<ferroui_base::platform::SharedBitmapImpl>, _opacity: f64, _source_rect: Rect, _dest_rect: Rect) {}
    fn custom(&mut self, _custom: &Rc<dyn ICustomDrawOperation>) {}
    fn draw_glyph_run(&mut self, _foreground: Option<&Rc<dyn IBrush>>, _glyph_run: &Rc<GlyphRun>) {}
    fn push_clip_core(&mut self, _rect: Rect) {}
    fn push_rounded_clip_core(&mut self, _rect: RoundedRect) {}
    fn push_geometry_clip_core(&mut self, _clip: &Ref<Geometry>) {}
    fn push_opacity_core(&mut self, _opacity: f64) {}
    fn push_opacity_mask_core(&mut self, _mask: &Rc<dyn IBrush>, _bounds: Rect) {}
    fn push_transform_core(&mut self, _matrix: Matrix) {}
    fn push_render_options_core(&mut self, _render_options: RenderOptions) {}
    fn push_text_options_core(&mut self, _text_options: TextOptions) {}
    fn push_effect_core(&mut self, _effect: &Rc<dyn IEffect>, _bounds: Rect) {}
    fn pop_clip_core(&mut self) {}
    fn pop_geometry_clip_core(&mut self) {}
    fn pop_opacity_core(&mut self) {}
    fn pop_opacity_mask_core(&mut self) {}
    fn pop_transform_core(&mut self) {}
    fn pop_render_options_core(&mut self) {}
    fn pop_text_options_core(&mut self) {}
    fn pop_effect_core(&mut self) {}
    fn dispose_core(&mut self) {}
}

fn tick_bar(width: f64, height: f64) -> Ref<TickBar> {
    let target = TickBar::new();
    target.set_width(width);
    target.set_height(height);
    target.set_minimum(0.0);
    target.set_maximum(10.0);
    target.set_fill(Some(Brushes::black()));
    target
}

fn render(target: &Ref<TickBar>) -> RecordingDrawingContext {
    let root = TestRoot::with_child(target.clone());
    root.execute_initial_layout_pass();

    let mut core = RecordingDrawingContext::default();
    {
        let mut context = DrawingContext::new(&mut core);
        target.render(&mut context);
    }
    core
}

fn line(x1: f64, y1: f64, x2: f64, y2: f64) -> (Point, Point) {
    (Point::new(x1, y1), Point::new(x2, y2))
}

#[test]
fn tick_bar_defaults() {
    let _scope = test_scope();
    let target = TickBar::new();

    assert!(target.fill().is_none());
    assert_eq!(0.0, target.minimum());
    assert_eq!(0.0, target.maximum());
    assert_eq!(0.0, target.tick_frequency());
    assert_eq!(Orientation::Horizontal, target.orientation());
    assert!(target.ticks().is_none());
    assert!(!target.is_direction_reversed());
    assert_eq!(TickBarPlacement::Left, target.placement());
    assert_eq!(Rect::default(), target.reserved_space());
}

#[test]
fn tick_bar_bottom_draws_ticks_using_tick_frequency() {
    let _scope = test_scope();
    let target = tick_bar(100.0, 10.0);
    target.set_placement(TickBarPlacement::Bottom);
    target.set_tick_frequency(2.5);

    let drawn = render(&target);

    assert_eq!(
        vec![
            line(0.0, 0.0, 0.0, 10.0),
            line(100.0, 0.0, 100.0, 10.0),
            line(25.0, 0.0, 25.0, 7.5),
            line(50.0, 0.0, 50.0, 7.5),
            line(75.0, 0.0, 75.0, 7.5),
        ],
        drawn.lines
    );
    assert!(drawn.pens.iter().all(|pen| pen.thickness() == 1.0 && pen.brush().is_some()));
}

#[test]
fn tick_bar_top_draws_ticks_upwards_and_respects_reserved_space() {
    let _scope = test_scope();
    let target = tick_bar(120.0, 8.0);
    target.set_placement(TickBarPlacement::Top);
    target.set_reserved_space(Rect::new(0.0, 0.0, 20.0, 0.0));
    target.set_tick_frequency(5.0);

    let drawn = render(&target);

    assert_eq!(
        vec![line(10.0, 8.0, 10.0, 0.0), line(110.0, 8.0, 110.0, 0.0), line(60.0, 8.0, 60.0, 2.0)],
        drawn.lines
    );
}

#[test]
fn tick_bar_left_draws_ticks_using_ticks_collection() {
    let _scope = test_scope();
    let target = tick_bar(10.0, 100.0);
    target.set_orientation(Orientation::Vertical);
    target.set_placement(TickBarPlacement::Left);
    target.set_reserved_space(Rect::new(0.0, 0.0, 0.0, 20.0));
    // The tick frequency is ignored when the ticks collection is not empty;
    // ticks at or outside the minimum and maximum are skipped.
    target.set_tick_frequency(1.0);
    target.set_ticks(Some(TickList::from_items([0.0, 2.5, 5.0, 10.0, 12.0])));

    let drawn = render(&target);

    assert_eq!(
        vec![
            line(10.0, 90.0, 0.0, 90.0),
            line(10.0, 10.0, 0.0, 10.0),
            line(10.0, 70.0, 2.5, 70.0),
            line(10.0, 50.0, 2.5, 50.0),
        ],
        drawn.lines
    );
}

#[test]
fn tick_bar_right_reversed_draws_ticks_from_top() {
    let _scope = test_scope();
    let target = tick_bar(10.0, 100.0);
    target.set_orientation(Orientation::Vertical);
    target.set_placement(TickBarPlacement::Right);
    target.set_is_direction_reversed(true);
    target.set_ticks(Some(TickList::from_items([2.5])));

    let drawn = render(&target);

    assert_eq!(
        vec![line(0.0, 0.0, 10.0, 0.0), line(0.0, 100.0, 10.0, 100.0), line(0.0, 25.0, 7.5, 25.0)],
        drawn.lines
    );
}

#[test]
fn tick_bar_draws_nothing_when_reserved_space_exceeds_size() {
    let _scope = test_scope();
    let target = tick_bar(100.0, 10.0);
    target.set_placement(TickBarPlacement::Bottom);
    target.set_reserved_space(Rect::new(0.0, 0.0, 100.0, 0.0));
    target.set_tick_frequency(1.0);

    let drawn = render(&target);

    assert!(drawn.lines.is_empty());
}

#[test]
fn slider_shares_the_ticks_property_of_tick_bar() {
    let _scope = test_scope();

    assert!(std::ptr::eq(Slider::ticks_property(), TickBar::ticks_property()));

    let ticks = TickList::from_items([1.0, 2.0]);
    let slider = Slider::new();
    slider.set_ticks(Some(ticks.clone()));

    assert!(slider.get_value(TickBar::ticks_property()) == Some(ticks));
}

#[test]
fn tick_bar_without_fill_draws_nothing() {
    let _scope = test_scope();
    let target = tick_bar(100.0, 10.0);
    target.set_placement(TickBarPlacement::Bottom);
    target.set_tick_frequency(2.5);
    target.set_fill(None);

    let drawn = render(&target);

    // A pen without a brush is not visible: the drawing context skips it.
    assert!(drawn.lines.is_empty());
}
