use crate::{Border, ReversibleStackPanel};
use ferroui_base::{Rect, Size};

#[test]
fn arranges_in_reverse_order() {
    let target = ReversibleStackPanel::new();
    target.set_reverse_order(true);
    let first = Border::new();
    first.set_height(30.0);
    first.set_width(10.0);
    let second = Border::new();
    second.set_height(50.0);
    target.children().add(&first);
    target.children().add(&second);

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.children().get(0).bounds(), Rect::new(0.0, 50.0, 10.0, 30.0));
    assert_eq!(target.children().get(1).bounds(), Rect::new(0.0, 0.0, 10.0, 50.0));
}

#[test]
fn invalidates_arrange_on_reverse_order_change() {
    let target = ReversibleStackPanel::new();
    target.children().add(Border::new());
    target.children().add(Border::new());

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    target.set_reverse_order(true);

    assert!(target.is_measure_valid());
    assert!(!target.is_arrange_valid());
}
