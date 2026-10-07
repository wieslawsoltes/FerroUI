//! Port of `ClassesTests.cs` of the controls unit tests.
//!
//! The `ArgumentException` of a pseudoclass passed to a method for standard
//! classes is the panic of `Classes`. An `IClassesChangedListener` is a
//! listener closure, removed by the token `add_listener` returns.

use crate::test_support::test_scope;
use ferroui_base::controls::{Classes, IPseudoClasses};
use std::rc::Rc;

fn items(target: &Classes) -> Vec<String> {
    target.snapshot().to_vec()
}

#[test]
fn duplicates_should_not_be_added() {
    let _scope = test_scope();
    let target = Classes::new();

    target.add("foo");
    target.add("foo");

    assert_eq!(items(&target), ["foo"]);
}

#[test]
fn duplicates_should_not_be_added_via_add_range() {
    let _scope = test_scope();
    let target = Classes::new();

    target.add("foo");
    target.add_range(["foo", "bar"]);

    assert_eq!(items(&target), ["foo", "bar"]);
}

#[test]
fn duplicates_should_not_be_added_via_pseudoclasses() {
    let _scope = test_scope();
    let target = Classes::new();
    let ps: &dyn IPseudoClasses = &target;

    ps.add_pseudo(":foo");
    ps.add_pseudo(":foo");

    assert_eq!(items(&target), [":foo"]);
}

#[test]
fn duplicates_should_not_be_inserted() {
    let _scope = test_scope();
    let target = Classes::new();

    target.add("foo");
    target.insert(0, "foo");

    assert_eq!(items(&target), ["foo"]);
}

#[test]
fn duplicates_should_not_be_inserted_via_insert_range() {
    let _scope = test_scope();
    let target = Classes::new();

    target.add("foo");
    target.insert_range(1, ["foo", "bar"]);

    assert_eq!(items(&target), ["foo", "bar"]);
}

#[test]
#[should_panic(expected = "The pseudoclass ':foo' may only be added by the control itself.")]
fn should_not_be_able_to_add_pseudoclass() {
    let _scope = test_scope();
    let target = Classes::new();

    target.add(":foo");
}

#[test]
#[should_panic(expected = "The pseudoclass ':bar' may only be added by the control itself.")]
fn should_not_be_able_to_add_pseudoclasses_via_add_range() {
    let _scope = test_scope();
    let target = Classes::new();

    target.add_range(["foo", ":bar"]);
}

#[test]
#[should_panic(expected = "The pseudoclass ':foo' may only be added by the control itself.")]
fn should_not_be_able_to_insert_pseudoclass() {
    let _scope = test_scope();
    let target = Classes::new();

    target.insert(0, ":foo");
}

#[test]
#[should_panic(expected = "The pseudoclass ':bar' may only be added by the control itself.")]
fn should_not_be_able_to_insert_pseudoclasses_via_insert_range() {
    let _scope = test_scope();
    let target = Classes::new();

    target.insert_range(0, ["foo", ":bar"]);
}

#[test]
#[should_panic(expected = "The pseudoclass ':foo' may only be removed by the control itself.")]
fn should_not_be_able_to_remove_pseudoclass() {
    let _scope = test_scope();
    let target = Classes::new();

    target.remove(":foo");
}

#[test]
#[should_panic(expected = "The pseudoclass ':bar' may only be removed by the control itself.")]
fn should_not_be_able_to_remove_pseudoclasses_via_remove_all() {
    let _scope = test_scope();
    let target = Classes::new();

    target.remove_all(["foo", ":bar"]);
}

// Upstream's `ArgumentException` here is the one `List<T>.RemoveRange` throws
// for a range past the end of the empty collection; the port panics on the
// out-of-range slice.
#[test]
#[should_panic(expected = "out of range")]
fn should_not_be_able_to_remove_pseudoclasses_via_remove_range() {
    let _scope = test_scope();
    let target = Classes::new();

    target.remove_range(0, 1);
}

#[test]
#[should_panic(expected = "The pseudoclass ':foo' may only be removed by the control itself.")]
fn should_not_be_able_to_remove_pseudoclass_via_remove_at() {
    let _scope = test_scope();
    let target = Classes::new();

    (&target as &dyn IPseudoClasses).add_pseudo(":foo");

    target.remove_at(0);
}

#[test]
fn replace_should_not_replace_pseudoclasses() {
    let _scope = test_scope();
    let target = Classes::from_names(["foo", "bar"]);

    (&target as &dyn IPseudoClasses).add_pseudo(":baz");

    target.replace(&["qux"]);

    assert_eq!(items(&target), [":baz", "qux"]);
}

#[test]
#[should_panic(expected = "The pseudoclass ':qux' may only be added by the control itself.")]
fn replace_should_not_accept_pseudoclasses() {
    let _scope = test_scope();
    let target = Classes::new();

    target.replace(&[":qux"]);
}

#[test]
fn clear_should_not_remove_pseudoclasses() {
    let _scope = test_scope();
    let target = Classes::from_names(["foo", "bar"]);

    (&target as &dyn IPseudoClasses).add_pseudo(":baz");

    target.clear();

    assert_eq!(items(&target), [":baz"]);
}

#[test]
fn remove_all_should_remove_classes() {
    let _scope = test_scope();
    let target = Classes::from_names(["foo", "bar", "baz"]);

    target.remove_all(["bar", "baz"]);

    assert_eq!(items(&target), ["foo"]);
}

#[test]
fn listeners_can_be_added_by_listener() {
    let _scope = test_scope();
    let classes = Classes::new();
    let listener1: Rc<dyn Fn()> = Rc::new(|| {});
    let listener2: Rc<dyn Fn()> = {
        let classes = classes.clone();
        Rc::new(move || {
            classes.add_listener(listener1.clone());
        })
    };

    classes.add_listener(listener2);
    classes.add("bar");
}

#[test]
fn listeners_can_be_removed_by_listener() {
    let _scope = test_scope();
    let classes = Classes::new();
    let listener1: Rc<dyn Fn()> = Rc::new(|| {});
    let listener1 = classes.add_listener(listener1);
    let listener2: Rc<dyn Fn()> = {
        let classes = classes.clone();
        Rc::new(move || {
            classes.remove_listener(listener1);
        })
    };

    classes.add_listener(listener2);
    classes.add("bar");
}
