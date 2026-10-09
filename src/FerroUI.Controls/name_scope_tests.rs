//! Port of `NameScopeTests.cs` of the controls unit tests.
//!
//! `new object()` is a plain `FerroObject`, and the `ArgumentException` and
//! `InvalidOperationException` of a rejected registration are the panics of
//! `INameScope::register`.

use crate::test_support::test_scope;
use ferroui_base::controls::{ChildNameScope, INameScope, NameScope, NameScopeRef};
use ferroui_base::utilities::SynchronousCompletionAsyncResult;
use ferroui_base::{FerroObject, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Found = Rc<RefCell<Option<Ref<FerroObject>>>>;

/// `async void FindAsync(INameScope scope, string name)`: awaits the search
/// and stores its result in `found`. The continuation runs synchronously.
fn find_async(found: &Found, scope: &dyn INameScope, name: &str) {
    await_search(scope.find_async(name), {
        let found = found.clone();
        move |result| *found.borrow_mut() = result
    });
}

/// `await` of a search: runs `continuation` with the result now if the search
/// is completed, otherwise when it completes.
fn await_search(
    search: SynchronousCompletionAsyncResult<Option<Ref<FerroObject>>>,
    continuation: impl FnOnce(Option<Ref<FerroObject>>) + 'static,
) {
    if search.is_completed() {
        continuation(search.get_result());
    } else {
        let result = search.clone();
        search.on_completed(move || continuation(result.get_result()));
    }
}

fn same(found: &Found, element: &Ref<FerroObject>) -> bool {
    found.borrow().as_ref() == Some(element)
}

#[test]
fn register_registers_element() {
    let _scope = test_scope();
    let target = NameScope::new();
    let element = FerroObject::new();

    target.register("foo", element.clone());

    assert_eq!(target.find("foo"), Some(element));
}

#[test]
#[should_panic(expected = "Control with the name 'foo' already registered.")]
fn cannot_register_new_element_with_existing_name() {
    let _scope = test_scope();
    let target = NameScope::new();

    target.register("foo", FerroObject::new());
    target.register("foo", FerroObject::new());
}

#[test]
fn can_register_same_element_more_than_once() {
    let _scope = test_scope();
    let target = NameScope::new();
    let element = FerroObject::new();

    target.register("foo", element.clone());
    target.register("foo", element.clone());

    assert_eq!(target.find("foo"), Some(element));
}

#[test]
#[should_panic(expected = "NameScope is completed, no further registrations are allowed")]
fn cannot_register_new_element_for_completed_scope() {
    let _scope = test_scope();
    let target = NameScope::new();
    let element = FerroObject::new();

    target.register("foo", element.clone());
    target.complete();
    target.register("bar", element);
}

#[test]
fn find_async_should_find_controls_added_earlier() {
    let _scope = test_scope();
    let found = Found::default();
    let scope = NameScope::new();
    let element = FerroObject::new();
    scope.register("foo", element.clone());
    find_async(&found, &scope, "foo");
    assert!(same(&found, &element));
}

#[test]
fn find_async_should_find_controls_added_later() {
    let _scope = test_scope();
    let found = Found::default();
    let scope = NameScope::new();
    let element = FerroObject::new();

    find_async(&found, &scope, "foo");
    assert!(found.borrow().is_none());
    scope.register("foo", element.clone());
    assert!(same(&found, &element));
}

#[test]
fn find_async_should_return_null_after_scope_completion() {
    let _scope = test_scope();
    let scope = NameScope::new();
    let element = FerroObject::new();
    let finished = Rc::new(Cell::new(false));
    let find = |name: &str| {
        let finished = finished.clone();
        await_search(scope.find_async(name), move |result| {
            assert!(result.is_none());
            finished.set(true);
        });
    };
    find("foo");
    assert!(!finished.get());
    scope.register("bar", element);
    assert!(!finished.get());
    scope.complete();
    assert!(finished.get());
}

#[test]
fn child_scope_should_not_find_control_in_parent_scope_unless_completed() {
    let _scope = test_scope();
    let scope = NameScopeRef::new(NameScope::new());
    let child_scope = ChildNameScope::new(scope.clone());
    let element = FerroObject::new();
    scope.register("foo", element.clone());
    assert!(child_scope.find("foo").is_none());
    child_scope.complete();
    assert_eq!(child_scope.find("foo"), Some(element));
}

#[test]
fn child_scope_should_prefer_own_elements() {
    let _scope = test_scope();
    let scope = NameScopeRef::new(NameScope::new());
    let child_scope = ChildNameScope::new(scope.clone());
    let element = FerroObject::new();
    let child_element = FerroObject::new();
    scope.register("foo", element);
    child_scope.register("foo", child_element.clone());
    child_scope.complete();
    assert_eq!(child_scope.find("foo"), Some(child_element));
}

#[test]
fn child_scope_find_async_should_find_elements_in_parent_scope_when_child_is_completed() {
    let _scope = test_scope();
    let found = Found::default();
    let scope = NameScopeRef::new(NameScope::new());
    let child_scope = ChildNameScope::new(scope.clone());
    let element = FerroObject::new();
    scope.register("foo", element.clone());
    find_async(&found, &child_scope, "foo");
    assert!(found.borrow().is_none());
    child_scope.complete();
    assert!(same(&found, &element));
}

#[test]
fn child_scope_find_async_should_prefer_own_elements() {
    let _scope = test_scope();
    let found = Found::default();
    let scope = NameScopeRef::new(NameScope::new());
    let child_scope = ChildNameScope::new(scope.clone());
    let element = FerroObject::new();
    let child_element = FerroObject::new();
    find_async(&found, &child_scope, "foo");
    scope.register("foo", element);
    assert!(found.borrow().is_none());
    child_scope.register("foo", child_element.clone());
    assert_eq!(child_scope.find("foo"), Some(child_element.clone()));
    child_scope.complete();
    find_async(&found, &child_scope, "foo");
    assert_eq!(child_scope.find("foo"), Some(child_element));
}

// Not ports: the scope holds the element it is attached to weakly (the
// managed original holds it as every other element and leaves the cycle to
// its collector).

fn element_and_scope() -> (Ref<crate::Border>, NameScopeRef) {
    (crate::Border::new(), NameScopeRef::new(NameScope::new()))
}

#[test]
fn scope_does_not_keep_the_element_it_is_attached_to_alive() {
    let _scope = test_scope();
    // Named before the scope is attached, as the root of a document is.
    let (element, scope) = element_and_scope();
    scope.register("root", element.clone().upcast());
    NameScope::set_name_scope(&element, Some(scope.clone()));
    assert!(scope.find("root").is_some_and(|found| found == element));
    let weak = element.downgrade();
    drop(element);
    assert!(weak.upgrade().is_none());
    assert!(scope.find("root").is_none());

    // Named after the scope is attached.
    let (element, scope) = element_and_scope();
    NameScope::set_name_scope(&element, Some(scope.clone()));
    scope.register("root", element.clone().upcast());
    assert!(scope.find("root").is_some_and(|found| found == element));
    let weak = element.downgrade();
    drop(element);
    assert!(weak.upgrade().is_none());
}

#[test]
fn scope_keeps_the_other_elements_and_one_it_was_detached_from() {
    let _scope = test_scope();
    let (element, scope) = element_and_scope();
    let other = crate::Border::new();
    scope.register("root", element.clone().upcast());
    scope.register("other", other.clone().upcast());
    NameScope::set_name_scope(&element, Some(scope.clone()));
    let other_weak = other.downgrade();
    drop(other);
    assert!(other_weak.upgrade().is_some());

    NameScope::set_name_scope(&element, None);
    let weak = element.downgrade();
    drop(element);
    assert!(scope.find("root").is_some_and(|found| weak.points_to(&found)));
}

#[test]
fn child_scope_does_not_keep_the_element_it_is_attached_to_alive() {
    let _scope = test_scope();
    let element = crate::Border::new();
    let scope = NameScopeRef::new(ChildNameScope::new(NameScopeRef::new(NameScope::new())));
    scope.register("root", element.clone().upcast());
    NameScope::set_name_scope(&element, Some(scope.clone()));
    let weak = element.downgrade();
    drop(element);
    assert!(weak.upgrade().is_none());
}

#[test]
fn name_of_a_dropped_element_stays_taken() {
    let _scope = test_scope();
    let (element, scope) = element_and_scope();
    scope.register("root", element.clone().upcast());
    NameScope::set_name_scope(&element, Some(scope.clone()));
    drop(element);
    assert!(scope.try_register("root", FerroObject::new()).is_err());
}
