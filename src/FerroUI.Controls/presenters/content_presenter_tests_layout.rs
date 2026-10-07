//! Port of `ContentPresenterTests_Layout.cs`.

use crate::presenters::ContentPresenter;
use crate::test_support::test_scope;
use crate::{Border, Control};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::{Rect, Size, Thickness};

/// Declares one test per data row of a parameterized test.
macro_rules! theory {
    ($func:ident: $($name:ident($($arg:expr),* $(,)?));+ $(;)?) => {
        $(
            #[test]
            fn $name() {
                $func($($arg),*)
            }
        )+
    };
}

fn content_alignment_is_applied_to_child_bounds(
    h: HorizontalAlignment,
    v: VerticalAlignment,
    expected_x: f64,
    expected_y: f64,
    expected_width: f64,
    expected_height: f64,
) {
    let _scope = test_scope();
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    let target = ContentPresenter::new();
    target.set_horizontal_content_alignment(h);
    target.set_vertical_content_alignment(v);
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(expected_x, expected_y, expected_width, expected_height));
}

theory!(content_alignment_is_applied_to_child_bounds:
    content_alignment_is_applied_to_child_bounds_1(HorizontalAlignment::Stretch, VerticalAlignment::Stretch, 0.0, 0.0, 100.0, 100.0);
    content_alignment_is_applied_to_child_bounds_2(HorizontalAlignment::Left, VerticalAlignment::Stretch, 0.0, 0.0, 16.0, 100.0);
    content_alignment_is_applied_to_child_bounds_3(HorizontalAlignment::Right, VerticalAlignment::Stretch, 84.0, 0.0, 16.0, 100.0);
    content_alignment_is_applied_to_child_bounds_4(HorizontalAlignment::Center, VerticalAlignment::Stretch, 42.0, 0.0, 16.0, 100.0);
    content_alignment_is_applied_to_child_bounds_5(HorizontalAlignment::Stretch, VerticalAlignment::Top, 0.0, 0.0, 100.0, 16.0);
    content_alignment_is_applied_to_child_bounds_6(HorizontalAlignment::Stretch, VerticalAlignment::Bottom, 0.0, 84.0, 100.0, 16.0);
    content_alignment_is_applied_to_child_bounds_7(HorizontalAlignment::Stretch, VerticalAlignment::Center, 0.0, 42.0, 100.0, 16.0));

fn content_alignment_and_padding_are_applied_to_child_bounds(
    h: HorizontalAlignment,
    v: VerticalAlignment,
    expected_x: f64,
    expected_y: f64,
    expected_width: f64,
    expected_height: f64,
) {
    let _scope = test_scope();
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    let target = ContentPresenter::new();
    target.set_horizontal_content_alignment(h);
    target.set_vertical_content_alignment(v);
    target.set_padding(Thickness::uniform(10.0));
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(expected_x, expected_y, expected_width, expected_height));
}

theory!(content_alignment_and_padding_are_applied_to_child_bounds:
    content_alignment_and_padding_are_applied_to_child_bounds_1(HorizontalAlignment::Stretch, VerticalAlignment::Stretch, 10.0, 10.0, 80.0, 80.0);
    content_alignment_and_padding_are_applied_to_child_bounds_2(HorizontalAlignment::Left, VerticalAlignment::Stretch, 10.0, 10.0, 16.0, 80.0);
    content_alignment_and_padding_are_applied_to_child_bounds_3(HorizontalAlignment::Right, VerticalAlignment::Stretch, 74.0, 10.0, 16.0, 80.0);
    content_alignment_and_padding_are_applied_to_child_bounds_4(HorizontalAlignment::Center, VerticalAlignment::Stretch, 42.0, 10.0, 16.0, 80.0);
    content_alignment_and_padding_are_applied_to_child_bounds_5(HorizontalAlignment::Stretch, VerticalAlignment::Top, 10.0, 10.0, 80.0, 16.0);
    content_alignment_and_padding_are_applied_to_child_bounds_6(HorizontalAlignment::Stretch, VerticalAlignment::Bottom, 10.0, 74.0, 80.0, 16.0);
    content_alignment_and_padding_are_applied_to_child_bounds_7(HorizontalAlignment::Stretch, VerticalAlignment::Center, 10.0, 42.0, 80.0, 16.0));

#[test]
fn should_correctly_align_child_with_fixed_size() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_horizontal_alignment(HorizontalAlignment::Left);
    content.set_vertical_alignment(VerticalAlignment::Bottom);
    content.set_width(16.0);
    content.set_height(16.0);
    let target = ContentPresenter::new();
    target.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
    target.set_vertical_content_alignment(VerticalAlignment::Stretch);
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    // Check correct result for Issue #1447.
    assert_eq!(content.bounds(), Rect::new(0.0, 84.0, 16.0, 16.0));
}

#[test]
fn content_can_be_stretched() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(0.0, 0.0, 100.0, 100.0));
}

#[test]
fn content_can_be_right_aligned() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    content.set_horizontal_alignment(HorizontalAlignment::Right);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(84.0, 0.0, 16.0, 100.0));
}

#[test]
fn content_can_be_bottom_aligned() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    content.set_vertical_alignment(VerticalAlignment::Bottom);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(0.0, 84.0, 100.0, 16.0));
}

#[test]
fn content_can_be_top_left_aligned() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    content.set_horizontal_alignment(HorizontalAlignment::Right);
    content.set_vertical_alignment(VerticalAlignment::Top);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(84.0, 0.0, 16.0, 16.0));
}

#[test]
fn content_can_be_top_right_aligned() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_min_width(16.0);
    content.set_min_height(16.0);
    content.set_horizontal_alignment(HorizontalAlignment::Right);
    content.set_vertical_alignment(VerticalAlignment::Top);
    let target = ContentPresenter::new();
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();
    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(84.0, 0.0, 16.0, 16.0));
}

#[test]
fn child_arrange_with_zero_height_when_padding_height_greater_than_child_height() {
    let _scope = test_scope();
    let content = Border::new();
    content.set_height(0.0);
    content.set_width(0.0);
    let target = ContentPresenter::new();
    target.set_padding(Thickness::uniform(32.0));
    target.set_max_height(32.0);
    target.set_max_width(32.0);
    target.set_horizontal_content_alignment(HorizontalAlignment::Center);
    target.set_vertical_content_alignment(VerticalAlignment::Center);
    target.set_content(Some(Control::boxed(&content)));

    target.update_child();

    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(32.0, 32.0, 0.0, 0.0));
}

/// The nested `UseLayoutRounding` test class.
mod use_layout_rounding {
    use crate::presenters::ContentPresenter;
    use crate::test_support::{test_scope, TestRoot};
    use crate::{Canvas, Control};
    use ferroui_base::{Ref, Size, Thickness};

    #[test]
    fn measure_rounds_padding() {
        let _scope = test_scope();
        let canvas = Canvas::new();
        canvas.set_width(101.0);
        canvas.set_height(101.0);
        let target = ContentPresenter::new();
        target.set_padding(Thickness::uniform(1.0));
        target.set_content(Some(Control::boxed(&canvas)));

        let root = created_root(1.5, &target, None);

        root.layout_manager().execute_initial_layout_pass();

        // - 1 pixel padding is rounded up to 1.3333; for both sides it is 2.6666
        // - Size of 101 gets rounded up to 101.3333
        // - Desired size = 101.3333 + 2.6666 = 104
        assert_eq!(target.desired_size(), Size::new(104.0, 104.0));
    }

    #[test]
    fn measure_rounds_border_thickness() {
        let _scope = test_scope();
        let canvas = Canvas::new();
        canvas.set_width(101.0);
        canvas.set_height(101.0);
        let target = ContentPresenter::new();
        target.set_border_thickness(Thickness::uniform(1.0));
        target.set_content(Some(Control::boxed(&canvas)));

        let root = created_root(1.5, &target, None);

        root.layout_manager().execute_initial_layout_pass();

        // - 1 pixel border thickness is rounded up to 1.3333; for both sides it is 2.6666
        // - Size of 101 gets rounded up to 101.3333
        // - Desired size = 101.3333 + 2.6666 = 104
        assert_eq!(target.desired_size(), Size::new(104.0, 104.0));
    }

    fn created_root(scaling: f64, child: &Control, constraint: Option<Size>) -> Ref<TestRoot> {
        let root = TestRoot::new();
        root.set_layout_scaling(scaling);
        root.set_use_layout_rounding(true);
        root.set_child(child.to_ref());
        root.set_client_size(constraint.unwrap_or(Size::new(1000.0, 1000.0)));
        root
    }
}
