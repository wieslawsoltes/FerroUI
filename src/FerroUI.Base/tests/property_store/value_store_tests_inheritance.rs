//! Port of the upstream value store inheritance tests.

use super::super::*;
use crate::*;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>("Foo", StyledPropertyOptions::new(s("foodefault")).inherits(true))
    });

    pub fn with_parent(parent: &Ref<Class1>) -> Ref<Self> {
        let result = Self::new();
        result.set_inheritance_parent(parent);
        result
    }

    pub fn foo(&self) -> String {
        self.get_value(Self::foo_property())
    }

    pub fn set_foo(&self, value: &str) {
        self.set_value(Self::foo_property(), s(value));
    }

    /// The object whose value store is this object's inheritance ancestor.
    fn ancestor(&self) -> Option<Ref<FerroObject>> {
        self.values().inheritance_ancestor()
    }
}

#[track_caller]
fn assert_ancestor(expected: &Ref<Class1>, actual: &Ref<Class1>) {
    match actual.ancestor() {
        Some(ancestor) => assert!(ancestor.ptr_eq(expected), "unexpected inheritance ancestor"),
        None => panic!("no inheritance ancestor"),
    }
}

#[test]
fn inheritance_ancestor_is_initially_null() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    assert!(parent.ancestor().is_none());
    assert!(child.ancestor().is_none());
    assert!(grandchild.ancestor().is_none());
}

#[test]
fn setting_value_in_parent_updates_inheritance_ancestor() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    parent.set_foo("changed");

    assert!(parent.ancestor().is_none());
    assert_ancestor(&parent, &child);
    assert_ancestor(&parent, &grandchild);
}

#[test]
fn setting_value_in_parent_doesnt_update_grandchild_inheritance_ancestor_if_child_has_value_set() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    child.set_foo("foochanged");
    parent.set_foo("changed");

    assert!(parent.ancestor().is_none());
    assert_ancestor(&parent, &child);
    assert_ancestor(&child, &grandchild);
}

#[test]
fn clearing_value_in_parent_updates_inheritance_ancestor() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    parent.set_foo("changed");
    parent.clear_value(Class1::foo_property());

    assert!(parent.ancestor().is_none());
    assert!(child.ancestor().is_none());
    assert!(grandchild.ancestor().is_none());
}

#[test]
fn clear_value_in_parent_doesnt_update_grandchild_inheritance_ancestor_if_child_has_value_set() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    child.set_foo("foochanged");
    parent.set_foo("changed");
    parent.clear_value(Class1::foo_property());

    assert!(parent.ancestor().is_none());
    assert!(child.ancestor().is_none());
    assert_ancestor(&child, &grandchild);
}

#[test]
fn clearing_value_in_child_updates_inheritance_ancestor() {
    let parent = Class1::new();
    let child = Class1::with_parent(&parent);
    let grandchild = Class1::with_parent(&child);

    parent.set_foo("changed");
    child.set_foo("foochanged");
    child.clear_value(Class1::foo_property());

    assert!(parent.ancestor().is_none());
    assert_ancestor(&parent, &child);
    assert_ancestor(&parent, &grandchild);
}

#[test]
fn child_notifies_about_setting_back_to_default_value() {
    let parent = Class1::new();
    let child = Class1::new();

    parent.set_foo("changed");
    child.set_inheritance_parent(&parent);

    let raised = Flag::new();

    let r = raised.clone();
    child.property_changed(move |args| {
        r.set(args.property() == Class1::foo_property().as_property() && new_string(args) == "foodefault");
    });

    assert_eq!("changed", child.foo()); // inherited from parent.

    child.set_foo("foodefault"); // reset back to default.

    assert!(raised.get()); // expect event to be raised, as actual value was changed.
}

#[test]
fn adding_child_sets_inheritance_ancestor() {
    let parent = Class1::new();
    let child = Class1::new();
    let grandchild = Class1::with_parent(&child);

    parent.set_foo("changed");
    child.set_inheritance_parent(&parent);

    assert!(parent.ancestor().is_none());
    assert_ancestor(&parent, &child);
    assert_ancestor(&parent, &grandchild);
}
