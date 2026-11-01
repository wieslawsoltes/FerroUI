//! `TextBlock.LayoutRoundingPlacement`, a FerroUI addition (docs/porting/DEVIATIONS.md, Text
//! block; docs/porting/text-rendering.md, section 6): where a text block puts the height layout
//! rounding adds to it. The reference has no such property; these tests are specific to the port.
//!
//! The test fonts (`ferroui_base::media::text_formatting::testing`) make a line 1.1 em high, so a
//! line of the default size of 12 is 13.2 high: a height that layout rounding rounds up at every
//! scaling.

use crate::documents::InlineUIContainer;
use crate::primitives::AccessText;
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, SelectableTextBlock, TextBlock, TextLayoutRoundingPlacement};
use ferroui_base::layout::{HorizontalAlignment, LayoutHelper, VerticalAlignment};
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, Geometry, GlyphRun, IBrush, IDrawingContextCore, IEffect, IPen,
    RenderOptions, TextOptions,
};
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::{Matrix, Point, Rect, Ref, RoundedRect, Thickness, Visual};
use std::rc::Rc;
use std::sync::Arc;

const SCALINGS: [f64; 4] = [1.0, 1.25, 1.5, 2.0];

/// What a text block draws, in the order it draws it: the vertical translation of every pushed
/// transform (the origin of a run of text, the origin of the selection), the top of every
/// rectangle (the selection) and the vertical position of every line (the access key underline).
#[derive(Default)]
struct Recording {
    transforms: Vec<f64>,
    rectangles: Vec<f64>,
    lines: Vec<f64>,
}

impl IDrawingContextCore for Recording {
    fn draw_line_core(&mut self, _pen: &Rc<dyn IPen>, p1: Point, _p2: Point) {
        self.lines.push(p1.y);
    }
    fn draw_geometry_impl_core(&mut self, _: Option<&Rc<dyn IBrush>>, _: Option<&Rc<dyn IPen>>, _: &Arc<dyn IGeometryImpl>) {}
    fn draw_rectangle_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
        self.rectangles.push(rrect.rect.y);
    }
    fn draw_ellipse_core(&mut self, _: Option<&Rc<dyn IBrush>>, _: Option<&Rc<dyn IPen>>, _: Rect) {}
    fn draw_bitmap(
        &mut self,
        _: &ferroui_base::utilities::RefCounted<ferroui_base::platform::SharedBitmapImpl>,
        _: f64,
        _: Rect,
        _: Rect,
    ) {
    }
    fn custom(&mut self, _: &Arc<dyn ICustomDrawOperation>) {}
    fn draw_glyph_run(&mut self, _: Option<&Rc<dyn IBrush>>, _: &Rc<GlyphRun>) {}
    fn push_clip_core(&mut self, _: Rect) {}
    fn push_rounded_clip_core(&mut self, _: RoundedRect) {}
    fn push_geometry_clip_core(&mut self, _: &Ref<Geometry>) {}
    fn push_opacity_core(&mut self, _: f64) {}
    fn push_opacity_mask_core(&mut self, _: &Rc<dyn IBrush>, _: Rect) {}
    fn push_transform_core(&mut self, matrix: Matrix) {
        self.transforms.push(matrix.m32);
    }
    fn push_render_options_core(&mut self, _: RenderOptions) {}
    fn push_text_options_core(&mut self, _: TextOptions) {}
    fn push_effect_core(&mut self, _: &Rc<dyn IEffect>, _: Rect) {}
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

fn render(target: &Visual) -> Recording {
    let mut core = Recording::default();
    {
        let mut context = DrawingContext::new(&mut core);
        target.render(&mut context);
    }
    core
}

/// `target` laid out alone in a root at `scaling`.
fn root_at(target: Ref<Control>, scaling: f64) -> Ref<TestRoot> {
    let root = TestRoot::with_child(target);
    root.set_layout_scaling(scaling);
    root.execute_initial_layout_pass();
    root
}

fn text_block(text: &str, placement: TextLayoutRoundingPlacement) -> Ref<TextBlock> {
    let target = TextBlock::new();
    target.set_text(Some(text));
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    target.set_vertical_alignment(VerticalAlignment::Top);
    target.set_layout_rounding_placement(placement);
    target
}

/// What layout rounding adds to `height` at `scaling`.
fn added(height: f64, scaling: f64) -> f64 {
    LayoutHelper::round_layout_value_up(height, scaling) - height
}

fn assert_near(expected: f64, actual: f64, what: &str) {
    assert!((expected - actual).abs() < 1e-9, "{what}: expected {expected}, got {actual}");
}

#[test]
fn below_is_the_default_and_is_inherited() {
    let _scope = test_scope();
    let target = TextBlock::new();
    assert_eq!(TextLayoutRoundingPlacement::Below, target.layout_rounding_placement());

    // A theme sets it once on a top-level: the text blocks below inherit it.
    let border = Border::new();
    border.set_child(Some(target.clone().upcast()));
    let root = TestRoot::with_child(border.clone());
    TextBlock::set_layout_rounding_placement_on(&root, TextLayoutRoundingPlacement::Above);
    assert_eq!(TextLayoutRoundingPlacement::Above, target.layout_rounding_placement());
    assert_eq!(TextLayoutRoundingPlacement::Above, TextBlock::get_layout_rounding_placement(&border));

    // A value of the text block wins over the inherited one.
    target.set_layout_rounding_placement(TextLayoutRoundingPlacement::Below);
    assert_eq!(TextLayoutRoundingPlacement::Below, target.layout_rounding_placement());
}

#[test]
fn below_draws_the_text_at_the_top_of_the_content() {
    for scaling in SCALINGS {
        let _scope = test_scope();
        let target = text_block("Text", TextLayoutRoundingPlacement::Below);
        target.set_padding(Thickness::new(0.0, 2.0, 0.0, 0.0));
        let _root = root_at(target.clone().upcast(), scaling);

        let padding = LayoutHelper::round_layout_thickness(target.padding(), scaling);
        assert_eq!(0.0, target.layout_rounding_offset(), "{scaling}");
        let drawn = render(&target);
        assert_near(padding.top, drawn.transforms[0], &format!("the text at {scaling}"));
    }
}

#[test]
fn above_moves_the_text_down_by_what_layout_rounding_adds() {
    for scaling in SCALINGS {
        for padding in [Thickness::default(), Thickness::new(0.0, 2.0, 0.0, 1.0)] {
            let _scope = test_scope();
            let below = text_block("Text", TextLayoutRoundingPlacement::Below);
            below.set_padding(padding);
            let above = text_block("Text", TextLayoutRoundingPlacement::Above);
            above.set_padding(padding);
            let _below_root = root_at(below.clone().upcast(), scaling);
            let _above_root = root_at(above.clone().upcast(), scaling);

            // The block has the same size either way: the property moves the text only.
            assert_eq!(below.bounds(), above.bounds(), "{scaling}");

            let rounded_padding = LayoutHelper::round_layout_thickness(padding, scaling);
            let height = above.text_layout().height() + rounded_padding.top + rounded_padding.bottom;
            let offset = added(height, scaling);
            assert!(offset > 0.0, "13.2 high text is rounded up at {scaling}");
            assert_near(offset, above.layout_rounding_offset(), &format!("the offset at {scaling}"));
            // The text ends at the bottom of the rounded box.
            assert_near(
                above.desired_size().height,
                rounded_padding.top + offset + above.text_layout().height() + rounded_padding.bottom,
                &format!("the bottom of the text at {scaling}"),
            );

            let below_drawn = render(&below);
            let above_drawn = render(&above);
            assert_near(rounded_padding.top, below_drawn.transforms[0], &format!("below at {scaling}"));
            assert_near(rounded_padding.top + offset, above_drawn.transforms[0], &format!("above at {scaling}"));
        }
    }
}

#[test]
fn above_changes_nothing_without_layout_rounding_or_on_whole_pixels() {
    let _scope = test_scope();
    let target = text_block("Text", TextLayoutRoundingPlacement::Above);
    target.set_use_layout_rounding(false);
    let _root = root_at(target.clone().upcast(), 2.0);
    assert_eq!(0.0, target.layout_rounding_offset());
    assert_eq!(0.0, render(&target).transforms[0]);

    // A line of 20 at a size of 20 / 1.1 is whole pixels at a scaling of 1.
    let target = text_block("Text", TextLayoutRoundingPlacement::Above);
    target.set_font_size(20.0 / 1.1);
    let _root = root_at(target.clone().upcast(), 1.0);
    assert_near(0.0, target.layout_rounding_offset(), "whole pixels");
}

#[test]
fn changing_the_placement_invalidates_the_visual_and_not_the_measure() {
    let _scope = test_scope();
    let target = text_block("Text", TextLayoutRoundingPlacement::Below);
    let _root = root_at(target.clone().upcast(), 2.0);
    let desired = target.desired_size();

    target.set_layout_rounding_placement(TextLayoutRoundingPlacement::Above);
    assert!(target.is_measure_valid());
    assert!(!target.is_arrange_valid());
    target.update_layout();
    assert_eq!(desired, target.desired_size());
    assert!(target.layout_rounding_offset() > 0.0);
}

#[test]
fn above_moves_the_embedded_controls_with_the_text() {
    for scaling in SCALINGS {
        let _scope = test_scope();
        let child = |placement| {
            let target = TextBlock::new();
            target.set_horizontal_alignment(HorizontalAlignment::Left);
            target.set_vertical_alignment(VerticalAlignment::Top);
            target.set_layout_rounding_placement(placement);
            let border = Border::new();
            border.set_width(10.0);
            border.set_height(5.0);
            // Arranged where the text block puts it, not rounded to the pixel grid on its own.
            border.set_use_layout_rounding(false);
            let inlines = target.inlines().unwrap();
            inlines.add_text("ab");
            inlines.add(InlineUIContainer::with_child(border.clone()));
            (target, border)
        };
        let (below, below_child) = child(TextLayoutRoundingPlacement::Below);
        let (above, above_child) = child(TextLayoutRoundingPlacement::Above);
        let _below_root = root_at(below.clone().upcast(), scaling);
        let _above_root = root_at(above.clone().upcast(), scaling);

        let offset = above.layout_rounding_offset();
        assert!(offset > 0.0, "{scaling}");
        assert_near(below_child.bounds().y + offset, above_child.bounds().y, &format!("the child at {scaling}"));
        assert_eq!(below_child.bounds().x, above_child.bounds().x);
    }
}

#[test]
fn the_selection_moves_with_the_text() {
    for scaling in SCALINGS {
        let _scope = test_scope();
        let target = SelectableTextBlock::new();
        target.set_text(Some("Selected"));
        target.set_horizontal_alignment(HorizontalAlignment::Left);
        target.set_vertical_alignment(VerticalAlignment::Top);
        target.set_selection_brush(Some(Brushes::blue() as Rc<dyn IBrush>));
        target.set_layout_rounding_placement(TextLayoutRoundingPlacement::Above);
        let _root = root_at(target.clone().upcast(), scaling);
        target.set_selection_start(0);
        target.set_selection_end(3);

        let offset = target.layout_rounding_offset();
        assert!(offset > 0.0, "{scaling}");
        let drawn = render(&target);
        // The origin of the selection is the origin of the text.
        assert_near(offset, drawn.transforms[0], &format!("the selection at {scaling}"));
        assert_near(offset, drawn.transforms[1], &format!("the text at {scaling}"));
    }
}

#[test]
fn the_access_key_underline_moves_with_the_text() {
    for scaling in SCALINGS {
        let _scope = test_scope();
        let underline = |placement| {
            let target = AccessText::new();
            target.set_text(Some("_File"));
            target.set_show_access_key(true);
            target.set_horizontal_alignment(HorizontalAlignment::Left);
            target.set_vertical_alignment(VerticalAlignment::Top);
            target.set_layout_rounding_placement(placement);
            let _root = root_at(target.clone().upcast(), scaling);
            let offset = target.layout_rounding_offset();
            let rect = target.text_layout().hit_test_text_position(0);
            let drawn = render(&target);
            (offset, rect, drawn.lines)
        };
        let (none, rect, below) = underline(TextLayoutRoundingPlacement::Below);
        assert_eq!(0.0, none);
        assert_eq!(vec![rect.bottom().round() - 1.5], below, "below at {scaling}");

        let (offset, rect, above) = underline(TextLayoutRoundingPlacement::Above);
        assert!(offset > 0.0, "{scaling}");
        assert_eq!(vec![(rect.bottom() + offset).round() - 1.5], above, "above at {scaling}");
    }
}
