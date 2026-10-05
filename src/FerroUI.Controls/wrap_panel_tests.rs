use crate::{Border, WrapPanel, WrapPanelItemsAlignment};
use ferroui_base::layout::{LayoutHelper, Orientation};
use ferroui_base::{Rect, Ref, Size};

fn border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

fn min_width_border(min_width: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_min_width(min_width);
    border
}

fn min_height_border(min_height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_min_height(min_height);
    border
}

fn bounds(target: &WrapPanel, index: usize) -> Rect {
    target.children().get(index).bounds()
}

fn measure_and_arrange(target: &WrapPanel) {
    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
}

/// Swaps the axes of a rect.
fn transpose(rect: Rect) -> Rect {
    Rect::new(rect.y, rect.x, rect.height, rect.width)
}

#[test]
fn lays_out_horizontally_on_separate_lines() {
    let target = WrapPanel::new();
    target.set_width(100.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(100.0, 100.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 50.0, 100.0, 50.0));
}

#[test]
fn lays_out_horizontally_on_a_single_line() {
    let target = WrapPanel::new();
    target.set_width(200.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(200.0, 50.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(100.0, 0.0, 100.0, 50.0));
}

#[test]
fn lays_out_with_items_alignment() {
    use WrapPanelItemsAlignment::*;

    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        for items_alignment in [Start, Center, End, Justify, Stretch, StretchAll] {
            let line_height = 50.0;
            let target = WrapPanel::new();
            target.set_width(200.0);
            target.set_height(200.0);
            target.set_orientation(orientation);
            target.set_items_alignment(items_alignment);
            target.set_use_layout_rounding(false);

            if orientation == Orientation::Horizontal {
                target.set_item_height(line_height);
                target.children().add(min_width_border(50.0));
                target.children().add(min_width_border(100.0));
                target.children().add(min_width_border(150.0));
            } else {
                target.set_item_width(line_height);
                target.children().add(min_height_border(50.0));
                target.children().add(min_height_border(100.0));
                target.children().add(min_height_border(150.0));
            }

            measure_and_arrange(&target);

            assert_eq!(target.bounds().size(), Size::new(200.0, 200.0));

            let mut row1_bounds = bounds(&target, 0).union(bounds(&target, 1));
            let mut row2_bounds = bounds(&target, 2);

            row1_bounds = Rect::new(
                row1_bounds.x.round_ties_even(),
                row1_bounds.y.round_ties_even(),
                row1_bounds.width.round_ties_even(),
                row1_bounds.height.round_ties_even(),
            );

            if orientation == Orientation::Vertical {
                row1_bounds = transpose(row1_bounds);
                row2_bounds = transpose(row2_bounds);
            }

            let context = format!("{orientation:?} {items_alignment:?}");

            assert_eq!(
                row1_bounds,
                match items_alignment {
                    Stretch | StretchAll | Justify => Rect::new(0.0, 0.0, 200.0, line_height),
                    Center => Rect::new(25.0, 0.0, 150.0, line_height),
                    End => Rect::new(50.0, 0.0, 150.0, line_height),
                    Start => Rect::new(0.0, 0.0, 150.0, line_height),
                },
                "{context}"
            );

            assert_eq!(
                row2_bounds,
                match items_alignment {
                    StretchAll => Rect::new(0.0, line_height, 200.0, line_height),
                    Center => Rect::new(25.0, line_height, 150.0, line_height),
                    End => Rect::new(50.0, line_height, 150.0, line_height),
                    _ => Rect::new(0.0, 50.0, 150.0, 50.0),
                },
                "{context}"
            );
        }
    }
}

#[test]
fn justify_positions_unequal_items() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let target = WrapPanel::new();
        target.set_width(100.0);
        target.set_height(100.0);
        target.set_orientation(orientation);
        target.set_items_alignment(WrapPanelItemsAlignment::Justify);
        target.set_use_layout_rounding(false);

        if orientation == Orientation::Horizontal {
            target.children().add(border(40.0, 50.0));
            target.children().add(border(20.0, 50.0));
        } else {
            target.children().add(border(50.0, 40.0));
            target.children().add(border(50.0, 20.0));
        }

        measure_and_arrange(&target);

        let mut first_bounds = bounds(&target, 0);
        let mut second_bounds = bounds(&target, 1);
        if orientation == Orientation::Vertical {
            first_bounds = transpose(first_bounds);
            second_bounds = transpose(second_bounds);
        }

        assert_eq!(first_bounds, Rect::new(0.0, 0.0, 40.0, 50.0));
        assert_eq!(second_bounds, Rect::new(80.0, 0.0, 20.0, 50.0));
        assert_eq!(second_bounds.x - first_bounds.right(), 40.0);
    }
}

#[test]
fn stretch_handles_a_zero_width_line() {
    let target = WrapPanel::new();
    target.set_width(100.0);
    target.set_item_spacing(1.0);
    target.set_items_alignment(WrapPanelItemsAlignment::Stretch);
    target.children().add(border(0.0, 50.0));
    target.children().add(border(100.0, 50.0));

    measure_and_arrange(&target);

    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 0.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 50.0, 100.0, 50.0));
}

#[test]
fn lays_out_vertically_children_on_a_single_line() {
    let target = WrapPanel::new();
    target.set_orientation(Orientation::Vertical);
    target.set_height(120.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(100.0, 120.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 50.0, 100.0, 50.0));
}

#[test]
fn lays_out_vertically_on_separate_lines() {
    let target = WrapPanel::new();
    target.set_orientation(Orientation::Vertical);
    target.set_height(60.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(200.0, 60.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(100.0, 0.0, 100.0, 50.0));
}

#[test]
fn lays_out_horizontally_on_separate_lines_with_spacing() {
    let target = WrapPanel::new();
    target.set_width(100.0);
    target.set_item_spacing(10.0);
    target.set_line_spacing(20.0);
    target.children().add(border(60.0, 50.0)); // line 0
    target.children().add(border(30.0, 50.0)); // line 0
    target.children().add(border(70.0, 50.0)); // line 1
    target.children().add(border(30.0, 50.0)); // line 2

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(100.0, 190.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 60.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(70.0, 0.0, 30.0, 50.0));
    assert_eq!(bounds(&target, 2), Rect::new(0.0, 70.0, 70.0, 50.0));
    assert_eq!(bounds(&target, 3), Rect::new(0.0, 140.0, 30.0, 50.0));
}

#[test]
fn measure_respects_layout_rounding_at_wrap_boundary() {
    for (use_layout_rounding, expected_height) in [(true, 50.0), (false, 110.0)] {
        let target = WrapPanel::new();
        target.set_use_layout_rounding(use_layout_rounding);
        target.set_item_spacing(10.0);
        target.set_line_spacing(10.0);
        target.children().add(border(45.0, 50.0));
        target.children().add(border(45.0, 50.0));

        target.measure(Size::new(100.0 - LayoutHelper::LAYOUT_EPSILON / 2.0, f64::INFINITY));

        assert_eq!(target.desired_size().height, expected_height);
    }
}

#[test]
fn arrange_respects_layout_rounding_at_wrap_boundary() {
    for (use_layout_rounding, expected_second_child_y) in [(true, 0.0), (false, 60.0)] {
        let target = WrapPanel::new();
        target.set_use_layout_rounding(use_layout_rounding);
        target.set_item_spacing(10.0);
        target.set_line_spacing(10.0);
        let first = border(45.0, 50.0);
        first.set_use_layout_rounding(false);
        let second = border(45.0 + LayoutHelper::LAYOUT_EPSILON / 2.0, 50.0);
        second.set_use_layout_rounding(false);
        target.children().add(first);
        target.children().add(second);

        target.measure(Size::INFINITY);
        target.arrange(Rect::new(0.0, 0.0, 100.0, 110.0));

        assert_eq!(bounds(&target, 1).y, expected_second_child_y);
    }
}

#[test]
fn lays_out_horizontally_on_separate_lines_with_spacing_invisible() {
    let target = WrapPanel::new();
    target.set_item_spacing(10.0);
    target.children().add(border(60.0, 50.0)); // line 0
    let invisible = border(30.0, 50.0); // line 0
    invisible.set_is_visible(false);
    target.children().add(invisible);
    target.children().add(border(50.0, 50.0)); // line 0

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(120.0, 50.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 60.0, 50.0));
    assert_eq!(bounds(&target, 2), Rect::new(70.0, 0.0, 50.0, 50.0));
}

#[test]
fn lays_out_horizontally_on_separate_lines_with_spacing_vertical() {
    let target = WrapPanel::new();
    target.set_height(100.0);
    target.set_orientation(Orientation::Vertical);
    target.set_item_spacing(10.0);
    target.set_line_spacing(20.0);
    target.children().add(border(50.0, 60.0)); // line 0
    target.children().add(border(50.0, 30.0)); // line 0
    target.children().add(border(50.0, 70.0)); // line 1
    target.children().add(border(50.0, 30.0)); // line 2

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(190.0, 100.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 50.0, 60.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 70.0, 50.0, 30.0));
    assert_eq!(bounds(&target, 2), Rect::new(70.0, 0.0, 50.0, 70.0));
    assert_eq!(bounds(&target, 3), Rect::new(140.0, 0.0, 50.0, 30.0));
}

#[test]
fn applies_item_width_and_item_height_properties() {
    let target = WrapPanel::new();
    target.set_orientation(Orientation::Horizontal);
    target.set_width(50.0);
    target.set_item_width(20.0);
    target.set_item_height(15.0);
    target.children().add(Border::new());
    target.children().add(Border::new());

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(50.0, 15.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 20.0, 15.0));
    assert_eq!(bounds(&target, 1), Rect::new(20.0, 0.0, 20.0, 15.0));
}

#[test]
fn zero_size_visible_child() {
    let target = WrapPanel::new();
    target.set_orientation(Orientation::Horizontal);
    target.set_width(50.0);
    target.set_item_spacing(10.0);
    target.set_line_spacing(10.0);
    target.children().add(Border::new()); // line 0
    target.children().add(border(50.0, 50.0)); // line 1

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(50.0, 60.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 0.0, 0.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 10.0, 50.0, 50.0));
}

#[test]
fn item_width_trigger_invalidate_measure() {
    let target = WrapPanel::new();

    target.measure(Size::new(10.0, 10.0));

    assert!(target.is_measure_valid());

    target.set_item_width(1.0);

    assert!(!target.is_measure_valid());
}

#[test]
fn item_height_trigger_invalidate_measure() {
    let target = WrapPanel::new();

    target.measure(Size::new(10.0, 10.0));

    assert!(target.is_measure_valid());

    target.set_item_height(1.0);

    assert!(!target.is_measure_valid());
}
