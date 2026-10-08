//! Upstream has no dedicated tests for the converters, apart from the menu
//! scrolling case in the scroll viewer tests; these cover the port.

use super::*;
use crate::primitives::ScrollBarVisibility;
use ferroui_base::data::converters::{IMultiValueConverter, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingOperations;
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::media::{IBrush, VisualBrush};
use ferroui_base::{BoxedValue, CornerRadius, FerroProperty, Thickness};
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

fn some<T: PartialEq + 'static>(value: T) -> Option<BoxedValue> {
    Some(boxed(value))
}

fn get<T: Clone + 'static>(value: Option<BoxedValue>) -> T {
    value.expect("a value").downcast_ref::<T>().cloned().expect("a value of the expected type")
}

fn is_unset(value: &Option<BoxedValue>) -> bool {
    value.as_ref().is_some_and(|v| Rc::ptr_eq(v, &FerroProperty::unset_value()))
}

fn object() -> ValueType {
    ValueType::object()
}

#[test]
fn menu_scroll_bar_should_be_visible_when_specified_visible() {
    let converter = MenuScrollingVisibilityConverter::instance();
    let args = [some(ScrollBarVisibility::Visible), some(400.0f64), some(1800.0f64), some(500.0f64)];
    let result =
        converter.convert(&args, ValueType::of::<ScrollBarVisibility>(), Some(&boxed(String::from("0"))), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert!(get::<bool>(result));
}

#[test]
fn menu_scrolling_visibility_follows_the_scroll_position() {
    let converter = MenuScrollingVisibilityConverter::instance();
    let convert = |visibility: ScrollBarVisibility, offset: f64, parameter: BoxedValue| {
        let args = [some(visibility), some(offset), some(1800.0f64), some(800.0f64)];
        converter.convert(&args, ValueType::of::<bool>(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()
    };

    assert!(!get::<bool>(convert(ScrollBarVisibility::Auto, 0.0, boxed(String::from("0")))));
    assert!(get::<bool>(convert(ScrollBarVisibility::Auto, 500.0, boxed(String::from("0")))));
    assert!(!get::<bool>(convert(ScrollBarVisibility::Auto, 1000.0, boxed(100.0f64))));
    assert!(get::<bool>(convert(ScrollBarVisibility::Auto, 0.0, boxed(100.0f64))));
    assert!(!get::<bool>(convert(ScrollBarVisibility::Hidden, 500.0, boxed(0.0f64))));
    assert!(!get::<bool>(convert(ScrollBarVisibility::Disabled, 500.0, boxed(0.0f64))));
    assert!(is_unset(&convert(ScrollBarVisibility::Auto, 500.0, boxed(1i32))));

    // The extent fits the viewport.
    let args = [some(ScrollBarVisibility::Auto), some(0.0f64), some(500.0f64), some(500.0f64)];
    assert!(!get::<bool>(converter.convert(&args, object(), Some(&boxed(0.0f64)), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));

    // Invalid input.
    assert!(is_unset(&converter.convert(&args, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    assert!(is_unset(&converter.convert(&args[..3], object(), Some(&boxed(0.0f64)), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    let args = [some(1i32), some(0.0f64), some(500.0f64), some(500.0f64)];
    assert!(is_unset(&converter.convert(&args, object(), Some(&boxed(0.0f64)), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    let args = [some(ScrollBarVisibility::Auto), some(0.0f64), some(900.0f64), some(500.0f64)];
    assert!(converter.convert(&args, object(), Some(&boxed(String::from("x"))), &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
}

#[test]
fn corner_radius_filter_keeps_and_scales_the_filtered_corners() {
    let converter = CornerRadiusFilterConverter::new();
    assert_eq!(Corners::NONE, converter.filter());
    assert_eq!(1.0, converter.scale());
    let radius = CornerRadius::new(1.0, 2.0, 3.0, 4.0);

    let result = converter.convert(Some(&boxed(radius)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(CornerRadius::uniform(0.0), get::<CornerRadius>(result));

    converter.set_filter(Corners::TOP_LEFT | Corners::BOTTOM_RIGHT);
    converter.set_scale(2.0);
    let result = converter.convert(Some(&boxed(radius)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(CornerRadius::new(2.0, 0.0, 6.0, 0.0), get::<CornerRadius>(result));

    converter.set_filter(Corners::TOP_RIGHT | Corners::BOTTOM_LEFT);
    let result = converter.convert(Some(&boxed(radius)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(CornerRadius::new(0.0, 4.0, 0.0, 8.0), get::<CornerRadius>(result));

    // Other values are passed through.
    let other = boxed(5i32);
    let result = converter.convert(Some(&other), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert!(Rc::ptr_eq(&other, &result.unwrap()));
    assert!(converter.convert(None, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap().is_none());

    assert!(converter.convert_back(Some(&boxed(radius)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
}

#[test]
fn corner_radius_to_double_returns_the_selected_corner() {
    let converter = CornerRadiusToDoubleConverter::new();
    let radius = boxed(CornerRadius::new(1.0, 2.0, 3.0, 4.0));

    assert_eq!(0.0, get::<f64>(converter.convert(Some(&radius), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));

    for (corner, expected) in [
        (Corners::TOP_LEFT, 1.0),
        (Corners::TOP_RIGHT, 2.0),
        (Corners::BOTTOM_RIGHT, 3.0),
        (Corners::BOTTOM_LEFT, 4.0),
        (Corners::TOP_LEFT | Corners::TOP_RIGHT, 0.0),
    ] {
        converter.set_corner(corner);
        assert_eq!(corner, converter.corner());
        assert_eq!(expected, get::<f64>(converter.convert(Some(&radius), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    }

    assert!(is_unset(&converter.convert(Some(&boxed(1.0f64)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    assert!(is_unset(&converter.convert(None, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    assert!(converter.convert_back(Some(&boxed(1.0f64)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
}

#[test]
fn corners_have_the_expected_values() {
    assert_eq!(0, Corners::NONE.bits());
    assert_eq!(1, Corners::TOP_LEFT.bits());
    assert_eq!(2, Corners::TOP_RIGHT.bits());
    assert_eq!(4, Corners::BOTTOM_LEFT.bits());
    assert_eq!(8, Corners::BOTTOM_RIGHT.bits());
}

#[test]
fn enum_to_bool_compares_value_and_parameter() {
    let converter = EnumToBoolConverter::new();
    let convert = |value: Option<BoxedValue>, parameter: Option<BoxedValue>| {
        get::<bool>(converter.convert(value.as_ref(), ValueType::of::<bool>(), parameter.as_ref(), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap())
    };

    assert!(convert(None, None));
    assert!(!convert(some(ScrollBarVisibility::Auto), None));
    assert!(!convert(None, some(ScrollBarVisibility::Auto)));
    assert!(convert(some(ScrollBarVisibility::Auto), some(ScrollBarVisibility::Auto)));
    assert!(!convert(some(ScrollBarVisibility::Auto), some(ScrollBarVisibility::Visible)));
    assert!(!convert(some(ScrollBarVisibility::Auto), some(1i32)));
}

#[test]
fn enum_to_bool_converts_true_back_to_the_parameter() {
    let converter = EnumToBoolConverter::new();
    let parameter = boxed(ScrollBarVisibility::Visible);

    let result = converter.convert_back(Some(&boxed(true)), object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert!(Rc::ptr_eq(&parameter, &result.unwrap()));

    for value in [some(false), some(1i32), None] {
        let result = converter.convert_back(value.as_ref(), object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
        assert!(Rc::ptr_eq(&BindingOperations::do_nothing(), &result.unwrap()));
    }
}

#[test]
fn margin_multiplier_indents_the_selected_sides() {
    let converter = MarginMultiplierConverter::new();
    converter.set_indent(10.0);
    converter.set_left(true);

    let result = converter.convert(Some(&boxed(3i32)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(Thickness::new(30.0, 0.0, 0.0, 0.0), get::<Thickness>(result));

    converter.set_top(true);
    converter.set_right(true);
    converter.set_bottom(true);
    let result = converter.convert(Some(&boxed(2i32)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(Thickness::uniform(20.0), get::<Thickness>(result));

    converter.set_left(false);
    let depth = Thickness::new(1.0, 2.0, 3.0, 4.0);
    let result = converter.convert(Some(&boxed(depth)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(Thickness::new(0.0, 20.0, 30.0, 40.0), get::<Thickness>(result));

    let result = converter.convert(Some(&boxed(2.0f64)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(Thickness::uniform(0.0), get::<Thickness>(result));
    let result = converter.convert(None, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(Thickness::uniform(0.0), get::<Thickness>(result));

    assert!(converter.convert_back(Some(&boxed(depth)), object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
}

#[test]
fn string_format_formats_the_remaining_values() {
    let converter = StringFormatConverter::new();

    let values = [some(String::from("{0} of {1}")), some(2i32), some(String::from("five"))];
    assert_eq!("2 of five", get::<String>(converter.convert(&values, ValueType::of::<String>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));

    // An invalid format, a format that is not text and no values at all.
    let values = [some(String::from("{2}")), some(2i32)];
    assert!(is_unset(&converter.convert(&values, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    let values = [some(1i32), some(2i32)];
    assert!(is_unset(&converter.convert(&values, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    assert!(is_unset(&converter.convert(&[None], object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    assert!(is_unset(&converter.convert(&[], object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
}

#[test]
fn tree_view_item_indent_is_level_times_indent() {
    let converter = TreeViewItemIndentConverter::instance();

    let result = converter.convert(&[some(3i32), some(16.0f64)], object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(Thickness::new(48.0, 0.0, 0.0, 0.0), get::<Thickness>(result));

    for values in [vec![some(3i32)], vec![some(3.0f64), some(16.0f64)], vec![some(3i32), None], vec![]] {
        let result = converter.convert(&values, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
        assert_eq!(Thickness::uniform(0.0), get::<Thickness>(result));
    }
}

#[test]
fn platform_key_gesture_is_formatted_for_string_targets() {
    let converter = PlatformKeyGestureConverter::new();
    let gesture = KeyGesture::new(Key::S, KeyModifiers::CONTROL);
    let expected = gesture.to_platform_string(None);
    assert_eq!(expected, PlatformKeyGestureConverter::to_platform_string(&gesture));

    let result = converter.convert(Some(&boxed(gesture)), ValueType::of::<String>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(expected, get::<String>(result));

    assert!(converter.convert(None, ValueType::of::<String>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap().is_none());
    assert!(converter.convert(Some(&boxed(gesture)), ValueType::of::<i32>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
    assert!(converter.convert(Some(&boxed(1i32)), ValueType::of::<String>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
    assert!(converter.convert_back(Some(&boxed(expected)), ValueType::of::<KeyGesture>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
}

#[test]
fn border_gap_mask_validates_its_input() {
    let converter = BorderGapMaskConverter::new();
    let values = [some(20.0f64), some(100.0f64), some(50.0f64)];
    let parameter = boxed(7.0f64);

    assert!(is_unset(&converter.convert(&values, object(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    assert!(is_unset(&converter.convert(&values[..2], object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    assert!(is_unset(&converter.convert(&values, object(), Some(&boxed(7i32)), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    let invalid = [some(20i32), some(100.0f64), some(50.0f64)];
    assert!(is_unset(&converter.convert(&invalid, object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));
    let invalid = [some(20.0f64), None, some(50.0f64)];
    assert!(is_unset(&converter.convert(&invalid, object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap()));

    // A border without a width or a height has no mask.
    let empty = [some(20.0f64), some(0.0f64), some(50.0f64)];
    assert!(converter.convert(&empty, object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap().is_none());
    let empty = [some(20.0f64), some(100.0f64), some(0.0f64)];
    assert!(converter.convert(&empty, object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap().is_none());

    assert!(converter.convert(&values, object(), Some(&boxed(String::from("seven"))), &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
}

#[test]
fn border_gap_mask_is_a_visual_brush_of_a_grid_with_a_gap() {
    use crate::shapes::Rectangle;
    use crate::{Grid, GridLength, GridUnitType};

    let converter = BorderGapMaskConverter::new();
    let values = [some(20.0f64), some(100.0f64), some(50.0f64)];

    for parameter in [boxed(7.0f64), boxed(String::from("7"))] {
        let result = converter.convert(&values, object(), Some(&parameter), &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
        let brush = get::<Option<Rc<dyn IBrush>>>(result).expect("a brush");
        let visual_brush = brush.as_object().and_then(|o| o.downcast_ref::<VisualBrush>()).expect("a visual brush");
        let visual = visual_brush.visual().expect("a visual");
        let grid = visual.cast::<Grid>().expect("a grid");

        assert_eq!(100.0, grid.width());
        assert_eq!(50.0, grid.height());

        let widths: Vec<GridLength> = grid.column_definitions().snapshot().iter().map(|c| c.width()).collect();
        assert_eq!(
            vec![
                GridLength::new(7.0, GridUnitType::Pixel),
                GridLength::new(20.0, GridUnitType::Pixel),
                GridLength::new(1.0, GridUnitType::Star)
            ],
            widths
        );
        let heights: Vec<GridLength> = grid.row_definitions().snapshot().iter().map(|r| r.height()).collect();
        assert_eq!(vec![GridLength::new(25.0, GridUnitType::Pixel), GridLength::new(1.0, GridUnitType::Star)], heights);

        let cells: Vec<(i32, i32, i32)> = grid
            .children()
            .to_vec()
            .iter()
            .map(|child| {
                assert!(child.is::<Rectangle>());
                assert!(child.clone().cast::<Rectangle>().unwrap().fill().is_some());
                (Grid::get_column(child), Grid::get_row(child), Grid::get_row_span(child))
            })
            .collect();
        assert_eq!(vec![(0, 0, 2), (1, 1, 1), (2, 0, 2)], cells);
    }
}
