use super::AncestorFinder;
use crate::{Border, Control, Decorator};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{Ref, StyledElement};
use std::cell::RefCell;
use std::rc::Rc;

fn assert_ancestor(expected: Option<&Ref<Border>>, actual: &RefCell<Option<Ref<StyledElement>>>) {
    match (expected, actual.borrow().as_ref()) {
        (None, None) => {}
        (Some(expected), Some(actual)) => assert!(actual.ptr_eq(expected)),
        (expected, actual) => panic!("expected an ancestor: {}, found one: {}", expected.is_some(), actual.is_some()),
    }
}

#[test]
fn sanity_check() {
    let child = Control::new();
    let parent = Decorator::new();
    let grand_parent = Border::new();
    let grand_parent2 = Border::new();

    let current_parent: Rc<RefCell<Option<Ref<StyledElement>>>> = Rc::new(RefCell::new(None));
    let current = current_parent.clone();
    let subscription = AncestorFinder::create_for_type(&child.clone().upcast(), Border::TYPE)
        .subscribe_fn(move |s| *current.borrow_mut() = s);

    assert_ancestor(None, &current_parent);
    parent.set_child(&child);
    assert_ancestor(None, &current_parent);
    grand_parent.set_child(&parent);
    assert_ancestor(Some(&grand_parent), &current_parent);
    grand_parent.set_child(None);
    grand_parent2.set_child(&parent);
    assert_ancestor(Some(&grand_parent2), &current_parent);

    subscription.dispose();
    parent.set_child(None);
    assert_ancestor(Some(&grand_parent2), &current_parent);
}
