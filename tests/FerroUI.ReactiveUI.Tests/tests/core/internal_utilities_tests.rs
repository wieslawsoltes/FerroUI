//! `InternalUtilitiesTests` (`ReactiveUI.Tests/Core/InternalUtilitiesTests.cs`): `NotAWeakReference`.

use ferroui_reactiveui::NotAWeakReference;
use std::any::Any;
use std::rc::Rc;

const TARGET: &str = "test target";

/// There is no collector to force: the reference is strong, so the target stays alive while the
/// test drops every other reference to it.
#[test]
fn not_a_weak_reference_after_gc_still_alive() {
    let weak_ref = NotAWeakReference::new(Rc::new(TARGET.to_owned()) as Rc<dyn Any>);

    assert!(weak_ref.is_alive());
    assert_eq!(weak_ref.target().downcast_ref::<String>().map(String::as_str), Some(TARGET));
}

#[test]
fn not_a_weak_reference_constructor_stores_target() {
    let target: Rc<dyn Any> = Rc::new(TARGET.to_owned());

    let weak_ref = NotAWeakReference::new(target.clone());

    assert!(Rc::ptr_eq(weak_ref.target(), &target));
    assert!(weak_ref.is_alive());
}
