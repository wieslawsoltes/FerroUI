//! Public style-frame transactions for independently compiled controls.
use super::*;
use crate::{data::BindingPriority, *};

fn property() -> (&'static StyledProperty<bool>, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let property = StyledProperty::create(
        "Enabled",
        StyledElement::TYPE,
        StyledElement::TYPE,
        StyledPropertyMetadata::new(Some(true))
            .with_coerce(move |_, _| {
                count.set(count.get() + 1);
                false
            })
            .with_coerce_on_reevaluation(false),
        false,
        None,
        None,
        false,
    );
    (property, calls)
}

#[test]
fn styling_update_replaces_frames_without_an_intermediate_notification() {
    let (property, calls) = property();
    let owner = StyledElement::new();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let output = changes.clone();
    let _token = owner.property_changed(move |event| {
        if event.property() == property.as_property() && event.is_effective_value_change() {
            output
                .borrow_mut()
                .push(event.get_old_and_new_value::<bool>().1);
        }
    });
    let old = owner
        .set_value_with_priority(property, true, BindingPriority::Style)
        .unwrap();
    assert_eq!(*changes.borrow(), [false]);
    let replacement = owner.with_styling_update(|| {
        old.dispose();
        owner
            .set_value_with_priority(property, true, BindingPriority::Style)
            .unwrap()
    });
    assert_eq!(calls.get(), 2);
    assert_eq!(*changes.borrow(), [false]);
    replacement.dispose();
    assert_eq!(*changes.borrow(), [false, true]);
}

#[test]
fn styling_update_nests_and_returns_the_closure_result() {
    let (property, calls) = property();
    let owner = StyledElement::new();
    let old = owner
        .set_value_with_priority(property, true, BindingPriority::Style)
        .unwrap();
    let replacement = owner.with_styling_update(|| {
        old.dispose();
        owner.with_styling_update(|| {
            owner
                .set_value_with_priority(property, true, BindingPriority::Style)
                .unwrap()
        })
    });
    assert_eq!(calls.get(), 2);
    assert!(!owner.get_value(property));
    replacement.dispose();
    assert!(owner.get_value(property));
}

#[test]
fn styling_update_finishes_after_a_closure_panic_and_remains_usable() {
    let (property, calls) = property();
    let owner = StyledElement::new();
    let old = owner
        .set_value_with_priority(property, true, BindingPriority::Style)
        .unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        owner.with_styling_update(|| {
            old.dispose();
            panic!("update-failure");
        });
    }));
    assert!(result.is_err());
    assert!(owner.get_value(property));
    assert_eq!(calls.get(), 1);
    let replacement = owner.with_styling_update(|| {
        owner
            .set_value_with_priority(property, true, BindingPriority::Style)
            .unwrap()
    });
    assert!(!owner.get_value(property));
    assert_eq!(calls.get(), 2);
    replacement.dispose();
}
