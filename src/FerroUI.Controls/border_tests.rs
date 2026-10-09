use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Canvas, Control, Decorator};
use ferroui_base::layout::Layoutable;
use ferroui_base::{IntoRef, Rect, Ref, Size, StyledProperty, Thickness};
use std::panic::{catch_unwind, AssertUnwindSafe};

#[test]
fn measure_should_return_border_thickness_plus_padding_when_no_child_present() {
    let target = Border::new();
    target.set_padding(Thickness::uniform(6.0));
    target.set_border_thickness(Thickness::uniform(4.0));

    target.measure(Size::new(100.0, 100.0));

    assert_eq!(target.desired_size(), Size::new(20.0, 20.0));
}

#[test]
fn child_should_arrange_with_zero_height_width_if_padding_greater_than_child_size() {
    let content = Border::new();
    content.set_height(0.0);
    content.set_width(0.0);

    let target = Border::new();
    target.set_padding(Thickness::uniform(6.0));
    target.set_max_height(12.0);
    target.set_max_width(12.0);
    target.set_child(&content);

    target.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));

    assert_eq!(content.bounds(), Rect::new(6.0, 6.0, 0.0, 0.0));
}

#[test]
fn should_reject_nan_or_infinite_thicknesses() {
    fn set_values(target: &Ref<Border>, property: &'static StyledProperty<Thickness>) {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let result = catch_unwind(AssertUnwindSafe(|| {
                target.set_value(property, Thickness::new(0.0, 0.0, 0.0, value));
            }));
            assert!(result.is_err(), "{} accepted {value}", property.as_property().name());
        }
    }

    let target = Border::new();

    set_values(&target, Layoutable::margin_property());
    set_values(&target, Decorator::padding_property());
    set_values(&target, Border::border_thickness_property());
}

fn create_root(scaling: f64, child: impl IntoRef<Control>) -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.set_layout_scaling(scaling);
    root.set_use_layout_rounding(true);
    root.set_child(child.into_ref());
    root.set_client_size(Size::new(1000.0, 1000.0));
    root
}

fn canvas(width: f64, height: f64) -> Ref<Canvas> {
    let canvas = Canvas::new();
    canvas.set_width(width);
    canvas.set_height(height);
    canvas
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.0005, "{actual} != {expected}");
}

/// The reference `UseLayoutRounding`.
mod use_layout_rounding {
    use super::*;

    #[test]
    fn measure_rounds_padding() {
        let _scope = test_scope();
        let target = Border::new();
        target.set_padding(Thickness::uniform(1.0));
        target.set_child(canvas(101.0, 101.0));

        let root = create_root(1.5, &target);

        root.layout_manager().execute_initial_layout_pass();

        // - 1 pixel padding is rounded up to 1.3333; for both sides it is 2.6666
        // - Size of 101 gets rounded up to 101.3333
        // - Desired size = 101.3333 + 2.6666 = 104
        assert_eq!(target.desired_size(), Size::new(104.0, 104.0));
    }

    #[test]
    fn measure_rounds_border_thickness() {
        let _scope = test_scope();
        let target = Border::new();
        target.set_border_thickness(Thickness::uniform(1.0));
        target.set_child(canvas(101.0, 101.0));

        let root = create_root(1.5, &target);

        root.layout_manager().execute_initial_layout_pass();

        // - 1 pixel border thickness is rounded up to 1.3333; for both sides it is 2.6666
        // - Size of 101 gets rounded up to 101.3333
        // - Desired size = 101.3333 + 2.6666 = 104
        assert_eq!(target.desired_size(), Size::new(104.0, 104.0));
    }

    #[test]
    fn measure_arranges_child_to_rounded_border_thickness() {
        let _scope = test_scope();
        let child = Canvas::new();
        let target = Border::new();
        target.set_border_thickness(Thickness::uniform(1.0));
        target.set_width(82.0);
        target.set_height(82.0);
        target.set_child(&child);

        let root = create_root(1.5, &target);

        root.layout_manager().execute_initial_layout_pass();

        // - 1 pixel border thickness is rounded up to 1.3333; for both sides it is 2.6666
        // - Size of 82 needs no rounding
        // - Minus border thickness, space for child is 82 - 2.6666 = 79.3333
        assert_close(child.bounds().left(), 1.3333);
        assert_close(child.bounds().top(), 1.3333);
        assert_close(child.bounds().width, 79.3333);
        assert_close(child.bounds().height, 79.3333);
    }

    #[test]
    fn measure_arranges_child_with_rounded_margin() {
        let _scope = test_scope();
        let child = Border::new();
        child.set_margin(Thickness::new(0.0, 25.0, 25.0, 25.0));
        let target = Border::new();
        target.set_width(220.0);
        target.set_height(220.0);
        target.set_child(&child);

        let root = create_root(1.5, &target);

        root.layout_manager().execute_initial_layout_pass();

        // - 25 margin gets rounded up to 25.3333
        // - Size of 220 needs no rounding
        assert_close(child.bounds().left(), 0.0);
        assert_close(child.bounds().top(), 25.3333);
        assert_close(child.bounds().width, 194.6666);
        assert_close(child.bounds().height, 169.3333);
    }
}
