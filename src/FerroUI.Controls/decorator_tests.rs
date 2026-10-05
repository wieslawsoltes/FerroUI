use crate::{Control, Decorator};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::{Ref, Size, StyledElement, Thickness};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn setting_content_should_set_child_controls_parent() {
    let decorator = Decorator::new();
    let child = Control::new();

    decorator.set_child(&child);

    assert_eq!(child.parent().unwrap(), decorator);
}

#[test]
fn clearing_content_should_clear_child_controls_parent() {
    let decorator = Decorator::new();
    let child = Control::new();

    decorator.set_child(&child);
    decorator.set_child(None);

    assert!(child.parent().is_none());
}

#[test]
fn content_control_should_appear_in_logical_children() {
    let decorator = Decorator::new();
    let child = Control::new();

    decorator.set_child(&child);

    assert_eq!(decorator.logical_children().to_vec(), vec![child.upcast::<StyledElement>()]);
}

#[test]
fn clearing_content_should_remove_from_logical_children() {
    let decorator = Decorator::new();
    let child = Control::new();

    decorator.set_child(&child);
    decorator.set_child(None);

    assert!(decorator.logical_children().is_empty());
}

fn track(
    decorator: &Decorator,
    predicate: impl Fn(NotifyCollectionChangedAction) -> bool + 'static,
) -> Rc<Cell<bool>> {
    let called = Rc::new(Cell::new(false));
    let result = called.clone();
    decorator.logical_children().add_collection_changed(Rc::new(
        move |e: &NotifyCollectionChangedEventArgs<'_, Ref<StyledElement>>| called.set(predicate(e.action)),
    ));
    result
}

#[test]
fn setting_content_should_fire_logical_children_collection_changed() {
    let decorator = Decorator::new();
    let child = Control::new();
    let called = track(&decorator, |action| action == NotifyCollectionChangedAction::Add);

    decorator.set_child(&child);

    assert!(called.get());
}

#[test]
fn clearing_content_should_fire_logical_children_collection_changed() {
    let decorator = Decorator::new();
    let child = Control::new();

    decorator.set_child(&child);

    let called = track(&decorator, |action| action == NotifyCollectionChangedAction::Remove);

    decorator.set_child(None);

    assert!(called.get());
}

#[test]
fn changing_content_should_fire_logical_children_collection_changed() {
    let decorator = Decorator::new();
    let child1 = Control::new();
    let child2 = Control::new();

    decorator.set_child(&child1);

    let called = track(&decorator, |_| true);

    decorator.set_child(&child2);

    assert!(called.get());
}

#[test]
fn measure_should_return_padding_when_no_child_present() {
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(8.0));

    target.measure(Size::new(100.0, 100.0));

    assert_eq!(target.desired_size(), Size::new(16.0, 16.0));
}

#[test]
fn use_layout_rounding_measure_rounds_padding() {
    use crate::test_support::{test_scope, TestRoot};
    use crate::Canvas;

    let _scope = test_scope();
    let child = Canvas::new();
    child.set_width(101.0);
    child.set_height(101.0);
    let target = Decorator::new();
    target.set_padding(Thickness::uniform(1.0));
    target.set_child(child);

    let root = TestRoot::new();
    root.set_layout_scaling(1.5);
    root.set_use_layout_rounding(true);
    root.set_child(&target);
    root.set_client_size(Size::new(1000.0, 1000.0));

    root.layout_manager().execute_initial_layout_pass();

    // - 1 pixel padding is rounded up to 1.3333; for both sides it is 2.6666
    // - Size of 101 gets rounded up to 101.3333
    // - Desired size = 101.3333 + 2.6666 = 104
    assert_eq!(target.desired_size(), Size::new(104.0, 104.0));
}
