//! Port of the upstream `Inheritance` object tests.

use super::*;
use crate::data::BindingPriority;
use crate::*;

test_class!(Class1: FerroObject);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", s("foodefault"))
    });

    ferro_property!(pub fn baz_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>("Baz", StyledPropertyOptions::new(s("bazdefault")).inherits(true))
    });
}

#[repr(C)]
pub struct Class2 {
    base: Class1,
}

ferro_class!(Class2: Class1);
ferro_impl_classes!(Class2: FerroObjectImpl);

impl Class2 {
    fn class_init() {
        once_per_thread!({
            Class1::foo_property().override_default_value::<Class2>(s("foooverride"));
        });
    }

    pub fn new() -> Ref<Self> {
        Self::class_init();
        instantiate(Self { base: Class1::construct() })
    }

    pub fn with_parent<T: ObjectType + Upcast<FerroObject>>(parent: &Ref<T>) -> Ref<Self> {
        let result = Self::new();
        result.set_parent(parent);
        result
    }

    pub fn set_parent<T: ObjectType + Upcast<FerroObject>>(&self, parent: &Ref<T>) {
        self.set_inheritance_parent(parent);
    }
}

test_class!(AttachedOwner: FerroObject);

impl AttachedOwner {
    ferro_property!(pub fn attached_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached_with::<AttachedOwner, Class1, _>(
            "Attached",
            StyledPropertyOptions::new(None).inherits(true),
        )
    });
}

#[test]
fn get_value_returns_inherited_value_1() {
    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), s("changed"));

    let child = Class2::with_parent(&parent);
    assert_eq!("changed", child.get_value(Class1::baz_property()));
}

#[test]
fn get_value_returns_inherited_value_2() {
    let parent = Class1::new();
    let child = Class2::with_parent(&parent);

    parent.set_value(Class1::baz_property(), s("changed"));

    assert_eq!("changed", child.get_value(Class1::baz_property()));
}

#[test]
fn clear_value_clears_inherited_value() {
    let parent = Class1::new();
    let child = Class2::with_parent(&parent);

    parent.set_value(Class1::baz_property(), s("changed"));

    assert_eq!("changed", child.get_value(Class1::baz_property()));

    parent.clear_value(Class1::baz_property());

    assert_eq!("bazdefault", parent.get_value(Class1::baz_property()));
    assert_eq!("bazdefault", child.get_value(Class1::baz_property()));
}

#[test]
fn clear_value_on_parent_raises_property_changed_on_child() {
    let parent = Class1::new();
    let child = Class2::with_parent(&parent);
    let raised = Counter::new();

    parent.set_value(Class1::baz_property(), s("changed"));

    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        assert!(is_sender(e, &weak));
        assert_eq!(Some(s("changed")), old_string(e));
        assert_eq!("bazdefault", new_string(e));
        assert_eq!(BindingPriority::Inherited, e.priority());
        r.increment();
    });

    parent.clear_value(Class1::baz_property());

    assert_eq!(1, raised.get());
}

#[test]
fn clear_value_on_child_raises_property_changed_with_inherited_parent_value() {
    let parent = Class1::new();
    let child = Class2::with_parent(&parent);
    let raised = Counter::new();

    parent.set_value(Class1::baz_property(), s("parent"));
    child.set_value(Class1::baz_property(), s("child"));

    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        assert!(is_sender(e, &weak));
        assert_eq!(Some(s("child")), old_string(e));
        assert_eq!("parent", new_string(e));
        assert_eq!(BindingPriority::Inherited, e.priority());
        r.increment();
    });

    child.clear_value(Class1::baz_property());

    assert_eq!(1, raised.get());
}

#[test]
fn clear_value_on_parent_raises_property_changed_on_child_with_inherited_grandparent_value() {
    let grandparent = Class1::new();
    let parent = Class2::with_parent(&grandparent);
    let child = Class2::with_parent(&parent);
    let raised = Counter::new();

    grandparent.set_value(Class1::baz_property(), s("grandparent"));
    parent.set_value(Class1::baz_property(), s("parent"));

    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        assert!(is_sender(e, &weak));
        assert_eq!(Some(s("parent")), old_string(e));
        assert_eq!("grandparent", new_string(e));
        assert_eq!(BindingPriority::Inherited, e.priority());
        r.increment();
    });

    parent.clear_value(Class1::baz_property());

    assert_eq!(1, raised.get());
}

#[test]
fn setting_inheritance_parent_raises_property_changed_when_parent_has_value_set() {
    let raised = Flag::new();

    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), s("changed"));

    let child = Class2::new();
    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == Class1::baz_property().as_property()
                && old_string(e) == Some(s("bazdefault"))
                && new_string(e) == "changed"
                && e.priority() == BindingPriority::Inherited,
        );
    });

    child.set_parent(&parent);

    assert!(raised.get());
    assert_eq!("changed", child.get_value(Class1::baz_property()));
}

#[test]
fn setting_inheritance_parent_raises_property_changed_when_parent_and_grandparent_has_value_set() {
    let grandparent = Class1::new();
    let parent = Class2::with_parent(&grandparent);
    let raised = Flag::new();

    grandparent.set_value(Class1::baz_property(), s("changed1"));
    parent.set_value(Class1::baz_property(), s("changed2"));

    let child = Class2::new();
    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == Class1::baz_property().as_property()
                && old_string(e) == Some(s("bazdefault"))
                && new_string(e) == "changed2"
                && e.priority() == BindingPriority::Inherited,
        );
    });

    child.set_parent(&parent);

    assert!(raised.get());
    assert_eq!("changed2", child.get_value(Class1::baz_property()));
}

#[test]
fn setting_inheritance_parent_raises_property_changed_for_attached_property_when_parent_has_value_set() {
    let raised = Flag::new();

    let parent = Class1::new();
    parent.set_value(AttachedOwner::attached_property(), Some(s("changed")));

    let child = Class2::new();
    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == AttachedOwner::attached_property().as_property()
                && e.get_old_value::<Option<String>>() == Some(None)
                && e.get_new_value::<Option<String>>() == Some(s("changed")),
        );
    });

    child.set_parent(&parent);

    assert!(raised.get());
    assert_eq!(Some(s("changed")), child.get_value(AttachedOwner::attached_property()));
}

#[test]
fn setting_inheritance_parent_doesnt_raise_property_changed_when_local_value_set() {
    let raised = Flag::new();

    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), s("changed"));

    let child = Class2::new();
    child.set_value(Class1::baz_property(), s("localvalue"));
    let r = raised.clone();
    child.property_changed(move |_| r.set(true));

    child.set_parent(&parent);

    assert!(!raised.get());
    assert_eq!("localvalue", child.get_value(Class1::baz_property()));
}

#[test]
fn setting_value_in_inheritance_parent_raises_property_changed() {
    let raised = Flag::new();

    let parent = Class1::new();

    let child = Class2::new();
    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == Class1::baz_property().as_property()
                && old_string(e) == Some(s("bazdefault"))
                && new_string(e) == "changed",
        );
    });
    child.set_parent(&parent);

    parent.set_value(Class1::baz_property(), s("changed"));

    assert!(raised.get());
    assert_eq!("changed", child.get_value(Class1::baz_property()));
}

#[test]
fn setting_value_of_attached_property_in_inheritance_parent_raises_property_changed() {
    let raised = Flag::new();

    let parent = Class1::new();

    let child = Class2::new();
    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == AttachedOwner::attached_property().as_property()
                && e.get_old_value::<Option<String>>() == Some(None)
                && e.get_new_value::<Option<String>>() == Some(s("changed")),
        );
    });
    child.set_parent(&parent);

    parent.set_value(AttachedOwner::attached_property(), Some(s("changed")));

    assert!(raised.get());
    assert_eq!(Some(s("changed")), child.get_value(AttachedOwner::attached_property()));
}

#[test]
fn clearing_value_in_inheritance_parent_raises_property_changed() {
    let raised = Flag::new();

    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), s("changed"));

    let child = Class2::with_parent(&parent);

    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        r.set(
            is_sender(e, &weak)
                && e.property() == Class1::baz_property().as_property()
                && old_string(e) == Some(s("changed"))
                && new_string(e) == "bazdefault",
        );
    });

    parent.clear_value(Class1::baz_property());

    assert!(raised.get());
    assert_eq!("bazdefault", child.get_value(Class1::baz_property()));
}

#[test]
fn property_changed_is_raised_in_parent_before_child() {
    let parent = Class1::new();
    let child = Class2::with_parent(&parent);
    let result: Recorder<&'static str> = Recorder::new();

    let r = result.clone();
    parent.property_changed(move |_| r.push("parent"));
    let r = result.clone();
    child.property_changed(move |_| r.push("child"));

    parent.set_value(Class1::baz_property(), s("changed"));

    assert_eq!(vec!["parent", "child"], result.get());
}

#[test]
fn reparenting_raises_property_changed_for_old_and_new_inherited_values() {
    let old_parent = Class1::new();
    old_parent.set_value(Class1::baz_property(), s("oldvalue"));

    let new_parent = Class1::new();
    new_parent.set_value(Class1::baz_property(), s("newvalue"));

    let child = Class2::with_parent(&old_parent);
    let raised = Counter::new();

    let (r, weak) = (raised.clone(), child.downgrade());
    child.property_changed(move |e| {
        assert!(is_sender(e, &weak));
        assert_eq!(Some(s("oldvalue")), e.get_old_value::<String>());
        assert_eq!("newvalue", e.get_new_value::<String>());
        assert_eq!(BindingPriority::Inherited, e.priority());
        r.increment();
    });

    child.set_parent(&new_parent);

    assert_eq!(1, raised.get());
    assert_eq!("newvalue", child.get_value(Class1::baz_property()));
}

#[test]
fn reparenting_raises_property_changed_on_grand_child_for_old_and_new_inherited_values() {
    let old_parent = Class1::new();
    old_parent.set_value(Class1::baz_property(), s("oldvalue"));

    let new_parent = Class1::new();
    new_parent.set_value(Class1::baz_property(), s("newvalue"));

    let child = Class2::with_parent(&old_parent);
    let grandchild = Class2::with_parent(&child);
    let raised = Counter::new();

    let (r, weak) = (raised.clone(), grandchild.downgrade());
    grandchild.property_changed(move |e| {
        assert!(is_sender(e, &weak));
        assert_eq!(Some(s("oldvalue")), e.get_old_value::<String>());
        assert_eq!("newvalue", e.get_new_value::<String>());
        assert_eq!(BindingPriority::Inherited, e.priority());
        r.increment();
    });

    child.set_parent(&new_parent);

    assert_eq!(1, raised.get());
    assert_eq!("newvalue", grandchild.get_value(Class1::baz_property()));
}

#[test]
fn reparenting_retains_inherited_property_set_on_child() {
    let old_parent = Class1::new();
    old_parent.set_value(Class1::baz_property(), s("oldvalue"));

    let new_parent = Class1::new();
    new_parent.set_value(Class1::baz_property(), s("newvalue"));

    let child = Class2::with_parent(&old_parent);
    child.set_value(Class1::baz_property(), s("childvalue"));

    let grandchild = Class2::with_parent(&child);
    let raised = Counter::new();

    let r = raised.clone();
    grandchild.property_changed(move |_| r.increment());

    child.set_parent(&new_parent);

    assert_eq!(0, raised.get());
    assert_eq!("childvalue", child.get_value(Class1::baz_property()));
    assert_eq!("childvalue", grandchild.get_value(Class1::baz_property()));
}

/// The changes a recorded object raised: its name and the ID of the property.
type Changes = Recorder<(&'static str, u32)>;

fn record_changes(target: &FerroObject, name: &'static str, changes: &Changes) {
    let changes = changes.clone();
    target.property_changed(move |e| changes.push((name, e.property().id())));
}

/// The IDs of the two inherited properties of these tests, in the order in
/// which `changes` has them. The order of the properties follows their IDs,
/// which follow the order of registration: the tests do not depend on it.
#[track_caller]
fn first_and_second_property(changes: &[(&'static str, u32)]) -> (u32, u32) {
    let baz = Class1::baz_property().as_property().id();
    let attached = AttachedOwner::attached_property().as_property().id();
    let first = changes.first().expect("a change").1;
    assert!(first == baz || first == attached);
    (first, if first == baz { attached } else { baz })
}

/// A change of the inheritance parent raises the changes of the inherited
/// values property by property: the change of one property on the object and
/// on its inheritance children comes before the change of the next property
/// on any of them (upstream: `SetInheritanceParent` calls
/// `InheritedValueChanged` once for each property that differs, and each
/// call walks the subtree).
#[test]
fn changing_inheritance_parent_raises_the_changes_of_one_property_over_the_subtree_before_the_next_property() {
    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), s("parentbaz"));
    parent.set_value(AttachedOwner::attached_property(), Some(s("parentattached")));

    let child = Class2::new();
    let grandchild = Class2::with_parent(&child);
    let changes: Changes = Recorder::new();
    record_changes(&child, "child", &changes);
    record_changes(&grandchild, "grandchild", &changes);

    child.set_parent(&parent);

    let raised = changes.get();
    let (first, second) = first_and_second_property(&raised);
    assert_eq!(
        vec![("child", first), ("grandchild", first), ("child", second), ("grandchild", second)],
        raised
    );
    assert_eq!("parentbaz", grandchild.get_value(Class1::baz_property()));
    assert_eq!(Some(s("parentattached")), grandchild.get_value(AttachedOwner::attached_property()));

    // The removal is a change of its own, with its own notifications.
    changes.clear();
    child.set_inheritance_parent(None);

    let raised = changes.get();
    let (first, second) = first_and_second_property(&raised);
    assert_eq!(
        vec![("child", first), ("grandchild", first), ("child", second), ("grandchild", second)],
        raised
    );
    assert_eq!("bazdefault", grandchild.get_value(Class1::baz_property()));
    assert_eq!(None, grandchild.get_value(AttachedOwner::attached_property()));

    // And so is the return to the parent the object left.
    changes.clear();
    child.set_parent(&parent);

    let raised = changes.get();
    let (first, second) = first_and_second_property(&raised);
    assert_eq!(
        vec![("child", first), ("grandchild", first), ("child", second), ("grandchild", second)],
        raised
    );
    assert_eq!("parentbaz", grandchild.get_value(Class1::baz_property()));
}

/// An object that sets an inherited property locally stops the change of
/// that property, for itself and for what is below it, and of that property
/// only: the changes of the other inherited properties pass through it.
#[test]
fn changing_inheritance_parent_stops_at_a_local_value_for_that_property_only() {
    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), s("parentbaz"));
    parent.set_value(AttachedOwner::attached_property(), Some(s("parentattached")));

    let child = Class2::new();
    child.set_value(Class1::baz_property(), s("childbaz"));
    let grandchild = Class2::with_parent(&child);
    let changes: Changes = Recorder::new();
    record_changes(&child, "child", &changes);
    record_changes(&grandchild, "grandchild", &changes);

    child.set_parent(&parent);

    let attached = AttachedOwner::attached_property().as_property().id();
    assert_eq!(vec![("child", attached), ("grandchild", attached)], changes.get());
    assert_eq!("childbaz", child.get_value(Class1::baz_property()));
    assert_eq!("childbaz", grandchild.get_value(Class1::baz_property()));
    assert_eq!(Some(s("parentattached")), grandchild.get_value(AttachedOwner::attached_property()));

    changes.clear();
    child.set_inheritance_parent(None);

    assert_eq!(vec![("child", attached), ("grandchild", attached)], changes.get());
    assert_eq!("childbaz", grandchild.get_value(Class1::baz_property()));
    assert_eq!(None, grandchild.get_value(AttachedOwner::attached_property()));
}

/// A change of the inheritance parent made inside the notifications of
/// another one is complete in itself, and the outer one goes on where it
/// was: each pairs its old and new values in a list of its own.
#[test]
fn handler_of_an_inherited_change_can_change_the_inheritance_parent_of_another_subtree() {
    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), s("parentbaz"));
    parent.set_value(AttachedOwner::attached_property(), Some(s("parentattached")));
    let other_parent = Class1::new();
    other_parent.set_value(Class1::baz_property(), s("otherbaz"));
    other_parent.set_value(AttachedOwner::attached_property(), Some(s("otherattached")));

    let child = Class2::new();
    let grandchild = Class2::with_parent(&child);
    let other = Class2::new();
    let other_child = Class2::with_parent(&other);

    let changes: Changes = Recorder::new();
    let (r, weak_other, weak_other_parent, done) =
        (changes.clone(), other.downgrade(), other_parent.downgrade(), Flag::new());
    child.property_changed(move |e| {
        r.push(("child", e.property().id()));
        if !done.get() {
            done.set(true);
            weak_other.upgrade().unwrap().set_parent(&weak_other_parent.upgrade().unwrap());
        }
    });
    record_changes(&grandchild, "grandchild", &changes);
    record_changes(&other, "other", &changes);
    record_changes(&other_child, "other_child", &changes);

    child.set_parent(&parent);

    let raised = changes.get();
    let (first, second) = first_and_second_property(&raised);
    assert_eq!(
        vec![
            ("child", first),
            ("other", first),
            ("other_child", first),
            ("other", second),
            ("other_child", second),
            ("grandchild", first),
            ("child", second),
            ("grandchild", second),
        ],
        raised
    );
    assert_eq!("parentbaz", child.get_value(Class1::baz_property()));
    assert_eq!("parentbaz", grandchild.get_value(Class1::baz_property()));
    assert_eq!(Some(s("parentattached")), grandchild.get_value(AttachedOwner::attached_property()));
    assert_eq!("otherbaz", other.get_value(Class1::baz_property()));
    assert_eq!("otherbaz", other_child.get_value(Class1::baz_property()));
    assert_eq!(Some(s("otherattached")), other_child.get_value(AttachedOwner::attached_property()));

    // The lists both changes used serve the next ones.
    changes.clear();
    other.set_inheritance_parent(None);
    child.set_inheritance_parent(None);

    let raised = changes.get();
    let (first, second) = first_and_second_property(&raised);
    assert_eq!(
        vec![
            ("other", first),
            ("other_child", first),
            ("other", second),
            ("other_child", second),
            ("child", first),
            ("grandchild", first),
            ("child", second),
            ("grandchild", second),
        ],
        raised
    );
    assert_eq!("bazdefault", grandchild.get_value(Class1::baz_property()));
    assert_eq!("bazdefault", other_child.get_value(Class1::baz_property()));
}
