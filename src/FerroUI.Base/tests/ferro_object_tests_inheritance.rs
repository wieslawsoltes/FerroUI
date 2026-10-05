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
