use super::{
    Flex, FlexAlignItems, FlexBasis, FlexBasisKind, FlexDirection, FlexJustifyContent, FlexPanel, FlexWrap,
};
use crate::presenters::ScrollContentPresenter;
use crate::{Border, Control, StackPanel};
use ferroui_base::{Point, Rect, Ref, Size};

fn border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_height(height);
    border.set_width(width);
    border
}

fn bounds(target: &FlexPanel, index: usize) -> Rect {
    target.children().get(index).bounds()
}

fn measure_and_arrange(target: &FlexPanel) {
    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
}

const DIRECTIONS: [FlexDirection; 4] =
    [FlexDirection::Row, FlexDirection::RowReverse, FlexDirection::Column, FlexDirection::ColumnReverse];

#[test]
fn lays_items_in_a_single_row() {
    let target = FlexPanel::new();
    target.set_width(200.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(200.0, 50.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(100.0, 0.0, 100.0, 50.0));
}

#[test]
fn lays_items_in_a_single_column() {
    let target = FlexPanel::new();
    target.set_direction(FlexDirection::Column);
    target.set_height(120.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(100.0, 120.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 50.0, 100.0, 50.0));
}

#[test]
fn can_wrap_items_into_next_row() {
    let target = FlexPanel::new();
    target.set_width(100.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));
    target.set_wrap(FlexWrap::Wrap);

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(100.0, 100.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 50.0, 100.0, 50.0));
}

#[test]
fn can_wrap_items_into_next_row_in_reverse_wrap() {
    let target = FlexPanel::new();
    target.set_width(100.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));
    target.set_wrap(FlexWrap::WrapReverse);

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(100.0, 100.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 50.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 0.0, 100.0, 50.0));
}

#[test]
fn can_wrap_items_into_next_column() {
    let target = FlexPanel::new();
    target.set_height(60.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));
    target.set_wrap(FlexWrap::Wrap);
    target.set_direction(FlexDirection::Column);

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(200.0, 60.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(100.0, 0.0, 100.0, 50.0));
}

#[test]
fn can_wrap_items_into_next_column_in_reverse_wrap() {
    let target = FlexPanel::new();
    target.set_height(60.0);
    target.children().add(border(100.0, 50.0));
    target.children().add(border(100.0, 50.0));
    target.set_wrap(FlexWrap::WrapReverse);
    target.set_direction(FlexDirection::Column);

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(200.0, 60.0));
    assert_eq!(bounds(&target, 0), Rect::new(100.0, 0.0, 100.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 0.0, 100.0, 50.0));
}

#[test]
fn lays_out_with_items_alignment() {
    use FlexAlignItems::{Center, FlexEnd, FlexStart, Stretch};
    use FlexDirection::{Column, ColumnReverse, Row, RowReverse};

    for direction in DIRECTIONS {
        for items_alignment in [FlexStart, FlexEnd, Center, Stretch] {
            let target = FlexPanel::new();
            target.set_width(200.0);
            target.set_height(200.0);
            target.set_direction(direction);
            target.set_align_items(items_alignment);
            target.children().add(border(50.0, 50.0));
            target.children().add(border(50.0, 50.0));

            measure_and_arrange(&target);

            assert_eq!(target.bounds().size(), Size::new(200.0, 200.0));

            let row_bounds = bounds(&target, 0).union(bounds(&target, 1));

            let expected_size = match direction {
                Row | RowReverse => Size::new(100.0, 50.0),
                Column | ColumnReverse => Size::new(50.0, 100.0),
            };
            assert_eq!(row_bounds.size(), expected_size, "{direction:?} {items_alignment:?}");

            let expected_position = match (direction, items_alignment) {
                (Row, FlexStart) => Point::new(0.0, 0.0),
                (Column, FlexStart) => Point::new(0.0, 0.0),
                (Row, Center) => Point::new(0.0, 75.0),
                (Column, Center) => Point::new(75.0, 0.0),
                (Row, FlexEnd) => Point::new(0.0, 150.0),
                (Column, FlexEnd) => Point::new(150.0, 0.0),
                (Row, Stretch) => Point::new(0.0, 75.0),
                (Column, Stretch) => Point::new(75.0, 0.0),
                (RowReverse, FlexStart) => Point::new(100.0, 0.0),
                (ColumnReverse, FlexStart) => Point::new(0.0, 100.0),
                (RowReverse, Center) => Point::new(100.0, 75.0),
                (ColumnReverse, Center) => Point::new(75.0, 100.0),
                (RowReverse, FlexEnd) => Point::new(100.0, 150.0),
                (ColumnReverse, FlexEnd) => Point::new(150.0, 100.0),
                (RowReverse, Stretch) => Point::new(100.0, 75.0),
                (ColumnReverse, Stretch) => Point::new(75.0, 100.0),
            };
            assert_eq!(row_bounds.position(), expected_position, "{direction:?} {items_alignment:?}");
        }
    }
}

#[test]
fn lays_out_with_justify_content() {
    use FlexDirection::{Column, ColumnReverse, Row, RowReverse};
    use FlexJustifyContent::{Center, FlexEnd, FlexStart, SpaceAround, SpaceBetween, SpaceEvenly};

    for direction in DIRECTIONS {
        for justify in [FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly] {
            let target = FlexPanel::new();
            target.set_width(200.0);
            target.set_height(200.0);
            target.set_direction(direction);
            target.set_justify_content(justify);
            target.set_align_items(FlexAlignItems::FlexStart);
            target.children().add(border(50.0, 50.0));
            target.children().add(border(50.0, 50.0));

            measure_and_arrange(&target);

            assert_eq!(target.bounds().size(), Size::new(200.0, 200.0));

            let row_bounds = bounds(&target, 0).union(bounds(&target, 1));

            let expected_position = match (direction, justify) {
                (Row, FlexStart) => Point::new(0.0, 0.0),
                (Column, FlexStart) => Point::new(0.0, 0.0),
                (Row, Center) => Point::new(50.0, 0.0),
                (Column, Center) => Point::new(0.0, 50.0),
                (Row, FlexEnd) => Point::new(100.0, 0.0),
                (Column, FlexEnd) => Point::new(0.0, 100.0),
                (Row, SpaceAround) => Point::new(25.0, 0.0),
                (Column, SpaceAround) => Point::new(0.0, 25.0),
                (Row, SpaceBetween) => Point::new(0.0, 0.0),
                (Column, SpaceBetween) => Point::new(0.0, 0.0),
                (Row, SpaceEvenly) => Point::new(33.0, 0.0),
                (Column, SpaceEvenly) => Point::new(0.0, 33.0),
                (RowReverse, FlexStart) => Point::new(100.0, 0.0),
                (ColumnReverse, FlexStart) => Point::new(0.0, 100.0),
                (RowReverse, Center) => Point::new(50.0, 0.0),
                (ColumnReverse, Center) => Point::new(0.0, 50.0),
                (RowReverse, FlexEnd) => Point::new(0.0, 0.0),
                (ColumnReverse, FlexEnd) => Point::new(0.0, 0.0),
                (RowReverse, SpaceAround) => Point::new(25.0, 0.0),
                (ColumnReverse, SpaceAround) => Point::new(0.0, 25.0),
                (RowReverse, SpaceBetween) => Point::new(0.0, 0.0),
                (ColumnReverse, SpaceBetween) => Point::new(0.0, 0.0),
                (RowReverse, SpaceEvenly) => Point::new(33.0, 0.0),
                (ColumnReverse, SpaceEvenly) => Point::new(0.0, 33.0),
            };
            assert_eq!(row_bounds.position(), expected_position, "{direction:?} {justify:?}");
        }
    }
}

#[test]
fn can_wrap_items_into_next_row_with_spacing() {
    let target = FlexPanel::new();
    target.set_width(110.0);
    target.set_column_spacing(10.0);
    target.set_row_spacing(20.0);
    target.children().add(border(60.0, 50.0)); // line 0
    target.children().add(border(30.0, 50.0)); // line 0
    target.children().add(border(70.0, 50.0)); // line 1
    target.children().add(border(30.0, 50.0)); // line 2
    target.set_wrap(FlexWrap::Wrap);

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(110.0, 190.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 60.0, 50.0));
    assert_eq!(bounds(&target, 1), Rect::new(70.0, 0.0, 30.0, 50.0));
    assert_eq!(bounds(&target, 2), Rect::new(0.0, 70.0, 70.0, 50.0));
    assert_eq!(bounds(&target, 3), Rect::new(0.0, 140.0, 30.0, 50.0));
}

#[test]
fn can_wrap_items_into_next_row_with_spacing_and_invisible_content() {
    let target = FlexPanel::new();
    target.set_column_spacing(10.0);
    target.children().add(border(60.0, 50.0)); // line 0
    let invisible = border(30.0, 50.0); // line 0
    invisible.set_is_visible(false);
    target.children().add(invisible);
    target.children().add(border(50.0, 50.0)); // line 0
    target.set_wrap(FlexWrap::Wrap);

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(120.0, 50.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 60.0, 50.0));
    assert_eq!(bounds(&target, 2), Rect::new(70.0, 0.0, 50.0, 50.0));
}

#[test]
fn can_wrap_items_into_next_column_with_spacing() {
    let target = FlexPanel::new();
    target.set_height(110.0);
    target.set_row_spacing(10.0);
    target.set_column_spacing(20.0);
    target.children().add(border(50.0, 60.0)); // line 0
    target.children().add(border(50.0, 30.0)); // line 0
    target.children().add(border(50.0, 70.0)); // line 1
    target.children().add(border(50.0, 30.0)); // line 2
    target.set_wrap(FlexWrap::Wrap);
    target.set_direction(FlexDirection::Column);

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(190.0, 110.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 50.0, 60.0));
    assert_eq!(bounds(&target, 1), Rect::new(0.0, 70.0, 50.0, 30.0));
    assert_eq!(bounds(&target, 2), Rect::new(70.0, 0.0, 50.0, 70.0));
    assert_eq!(bounds(&target, 3), Rect::new(140.0, 0.0, 50.0, 30.0));
}

fn border_with_basis(basis: FlexBasis) -> Ref<Border> {
    let border = Border::new();
    border.set_value(Flex::basis_property(), basis);
    border.set_height(15.0);
    border
}

#[test]
fn applies_absolute_flex_basis_properties() {
    let target = FlexPanel::new();
    target.set_width(50.0);
    target.children().add(border_with_basis(FlexBasis::absolute(20.0)));
    target.children().add(border_with_basis(FlexBasis::absolute(20.0)));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(50.0, 15.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 20.0, 15.0));
    assert_eq!(bounds(&target, 1), Rect::new(20.0, 0.0, 20.0, 15.0));
}

#[test]
fn applies_relative_flex_basis_properties() {
    let target = FlexPanel::new();
    target.set_width(50.0);
    target.children().add(border_with_basis(FlexBasis::new(50.0, FlexBasisKind::Relative)));
    target.children().add(border_with_basis(FlexBasis::new(50.0, FlexBasisKind::Relative)));

    measure_and_arrange(&target);

    assert_eq!(target.bounds().size(), Size::new(50.0, 15.0));
    assert_eq!(bounds(&target, 0), Rect::new(0.0, 0.0, 25.0, 15.0));
    assert_eq!(bounds(&target, 1), Rect::new(25.0, 0.0, 25.0, 15.0));
}

#[test]
fn empty_panel_does_not_take_up_available_space() {
    let target = FlexPanel::new();
    let stack = StackPanel::new();
    stack.children().add(border(100.0, 50.0));
    stack.children().add(target.clone());

    let presenter = ScrollContentPresenter::new();
    presenter.set_can_vertically_scroll(true);
    presenter.set_content(Some(Control::boxed(stack.clone())));

    presenter.update_child();
    presenter.measure(Size::new(100.0, 100.0));

    assert_eq!(target.desired_size(), Size::default());
    assert_eq!(stack.desired_size(), Size::new(100.0, 50.0));
}

// Additional tests (not ports).

/// As upstream, growing along an unconstrained main axis gives an item
/// without a grow factor an undefined length (infinity times zero), which a
/// measure pass rejects.
#[test]
#[should_panic(expected = "NaN")]
fn additional_unconstrained_main_axis_with_mixed_grow_factors_is_rejected() {
    let target = FlexPanel::new();
    let growing = border(50.0, 50.0);
    growing.set_value(Flex::grow_property(), 1.0);
    target.children().add(growing);
    target.children().add(border(50.0, 50.0));

    target.measure(Size::INFINITY);
}
