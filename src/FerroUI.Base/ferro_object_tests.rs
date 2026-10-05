//! Smoke tests for the object model and property system.

use crate::data::BindingPriority;
use crate::reactive::{IObservable, IObserver, LightweightSubject};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    direct: RefCell<String>,
    changes: RefCell<Vec<String>>,
}

ferro_class! {
    Class1: FerroObject, virtuals Class1Impl: FerroObjectImpl {
        fn describe(this) -> String;
    }
}

impl FerroObjectImpl for Class1 {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        this.changes.borrow_mut().push(change.property().name().to_string());
        Self::parent_on_property_changed(this, change);
    }
}

impl Class1Impl for Class1 {
    fn describe(_this: &Self) -> String {
        "class1".into()
    }
}

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<Class1, _>("Foo", "foodefault".to_string())
    });

    ferro_property!(pub fn baz_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>(
            "Baz",
            StyledPropertyOptions::new("bazdefault".to_string()).inherits(true),
        )
    });

    ferro_property!(pub fn qux_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Class1, _>(
            "Qux",
            StyledPropertyOptions::new(5).coerce(|_, v| v.min(100)).validate(|v| *v >= 0),
        )
    });

    ferro_property!(pub fn direct_property() -> DirectProperty<Class1, String> {
        FerroProperty::register_direct::<Class1, _>(
            "Direct",
            |o| o.direct.borrow().clone(),
            Some(|o, v| { o.set_and_raise(Class1::direct_property(), &o.direct, v); }),
            String::new(),
        )
    });

    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), direct: RefCell::new("initial".into()), changes: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

#[repr(C)]
pub struct Class2 {
    base: Class1,
}

ferro_class!(Class2: Class1);
ferro_impl_classes!(Class2: FerroObjectImpl);

impl Class1Impl for Class2 {
    fn describe(this: &Self) -> String {
        format!("class2:{}", Self::parent_describe(this))
    }
}

impl Class2 {
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Class1::construct() })
    }
}

#[test]
fn get_value_returns_default_value() {
    let target = Class1::new();
    assert_eq!(target.get_value(Class1::foo_property()), "foodefault");
}

#[test]
fn set_value_sets_value_and_raises_property_changed() {
    let target = Class1::new();
    let raised = Rc::new(Cell::new(0));
    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(e.property(), Class1::foo_property().as_property());
        assert_eq!(e.get_old_and_new_value::<String>(), ("foodefault".to_string(), "newvalue".to_string()));
        assert_eq!(e.priority(), BindingPriority::LocalValue);
        r.set(r.get() + 1);
    });
    target.set_value(Class1::foo_property(), "newvalue".to_string());
    assert_eq!(target.get_value(Class1::foo_property()), "newvalue");
    assert_eq!(raised.get(), 1);
    // Setting the same value does not raise.
    target.set_value(Class1::foo_property(), "newvalue".to_string());
    assert_eq!(raised.get(), 1);
    assert_eq!(*target.changes.borrow(), vec!["Foo".to_string()]);
}

#[test]
fn clear_value_reverts_to_default() {
    let target = Class1::new();
    target.set_value(Class1::foo_property(), "newvalue".to_string());
    assert!(target.is_set(Class1::foo_property()));
    target.clear_value(Class1::foo_property());
    assert!(!target.is_set(Class1::foo_property()));
    assert_eq!(target.get_value(Class1::foo_property()), "foodefault");
}

#[test]
fn virtual_dispatch_and_inherited_overrides() {
    let c2 = Class2::new();
    assert_eq!(c2.describe(), "class2:class1");
    let c1: Ref<Class1> = c2.clone().upcast();
    assert_eq!(c1.describe(), "class2:class1");
    assert!(c1.cast::<Class2>().is_some());
    assert!(Class1::new().cast::<Class2>().is_none());
    assert_eq!(c1.get_type().name(), "Class2");
    // The intermediate override of on_property_changed in Class1 is inherited.
    c2.set_value(Class1::foo_property(), "x".to_string());
    assert_eq!(*c2.changes.borrow(), vec!["Foo".to_string()]);
}

#[test]
fn style_priority_value_is_overridden_by_local_value_and_restored() {
    let target = Class1::new();
    let style = target.set_value_with_priority(Class1::foo_property(), "style".to_string(), BindingPriority::Style);
    assert_eq!(target.get_value(Class1::foo_property()), "style");
    target.set_value(Class1::foo_property(), "local".to_string());
    assert_eq!(target.get_value(Class1::foo_property()), "local");
    target.clear_value(Class1::foo_property());
    assert_eq!(target.get_value(Class1::foo_property()), "style");
    style.unwrap().dispose();
    assert_eq!(target.get_value(Class1::foo_property()), "foodefault");
}

#[test]
fn animation_value_overrides_local_value_and_base_value_is_kept() {
    let target = Class1::new();
    target.set_value(Class1::foo_property(), "local".to_string());
    let anim = target.set_value_with_priority(Class1::foo_property(), "anim".to_string(), BindingPriority::Animation);
    assert_eq!(target.get_value(Class1::foo_property()), "anim");
    assert!(target.is_animating(Class1::foo_property()));
    assert_eq!(target.get_base_value(Class1::foo_property()), Some("local".to_string()));
    anim.unwrap().dispose();
    assert_eq!(target.get_value(Class1::foo_property()), "local");
    assert!(!target.is_animating(Class1::foo_property()));
}

#[test]
fn inherited_value_flows_to_children_and_raises() {
    let parent = Class1::new();
    let child = Class1::new();
    let grandchild = Class1::new();
    child.set_inheritance_parent(&parent);
    grandchild.set_inheritance_parent(&child);
    assert_eq!(grandchild.get_value(Class1::baz_property()), "bazdefault");

    parent.set_value(Class1::baz_property(), "changed".to_string());
    assert_eq!(child.get_value(Class1::baz_property()), "changed");
    assert_eq!(grandchild.get_value(Class1::baz_property()), "changed");
    assert_eq!(*grandchild.changes.borrow(), vec!["Baz".to_string()]);

    child.set_value(Class1::baz_property(), "child".to_string());
    assert_eq!(grandchild.get_value(Class1::baz_property()), "child");
    assert_eq!(parent.get_value(Class1::baz_property()), "changed");

    child.clear_value(Class1::baz_property());
    assert_eq!(grandchild.get_value(Class1::baz_property()), "changed");

    grandchild.set_inheritance_parent(None);
    assert_eq!(grandchild.get_value(Class1::baz_property()), "bazdefault");
}

#[test]
fn setting_inheritance_parent_raises_for_inherited_values() {
    let parent = Class1::new();
    parent.set_value(Class1::baz_property(), "changed".to_string());
    let child = Class1::new();
    child.set_inheritance_parent(&parent);
    assert_eq!(child.get_value(Class1::baz_property()), "changed");
    assert_eq!(*child.changes.borrow(), vec!["Baz".to_string()]);
}

#[test]
fn coercion_and_validation() {
    let target = Class1::new();
    target.set_value(Class1::qux_property(), 150);
    assert_eq!(target.get_value(Class1::qux_property()), 100);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        target.set_value(Class1::qux_property(), -1);
    }));
    assert!(result.is_err());
}

#[test]
fn direct_property_get_set_and_untyped_access() {
    let target = Class1::new();
    assert_eq!(target.get_direct_value(Class1::direct_property()), "initial");
    target.set_direct_value(Class1::direct_property(), "new".to_string());
    assert_eq!(target.get_direct_value(Class1::direct_property()), "new");
    assert_eq!(*target.changes.borrow(), vec!["Direct".to_string()]);

    let boxed = target.get_value_untyped(Class1::direct_property().as_property());
    assert_eq!(boxed.downcast_ref::<String>().unwrap(), "new");
    target.set_value_untyped(Class1::foo_property().as_property(), &"untyped".to_string(), BindingPriority::LocalValue);
    assert_eq!(target.get_value(Class1::foo_property()), "untyped");

    // Direct property declared on Class1 works on a derived class.
    let c2 = Class2::new();
    c2.set_direct_value(Class1::direct_property(), "derived".to_string());
    assert_eq!(c2.get_direct_value(Class1::direct_property()), "derived");
}

#[test]
fn local_value_binding_updates_and_completion_clears() {
    let target = Class1::new();
    let source: LightweightSubject<String> = LightweightSubject::new();
    let observable: Rc<dyn IObservable<String>> = Rc::new(source.clone());
    target.bind(Class1::foo_property(), observable, BindingPriority::LocalValue);
    assert_eq!(target.get_value(Class1::foo_property()), "foodefault");
    source.on_next("first".to_string());
    assert_eq!(target.get_value(Class1::foo_property()), "first");
    source.on_next("second".to_string());
    assert_eq!(target.get_value(Class1::foo_property()), "second");
    source.on_completed();
    assert_eq!(target.get_value(Class1::foo_property()), "foodefault");
}

#[test]
fn style_binding_is_overridden_by_local_value() {
    let target = Class1::new();
    let source: LightweightSubject<String> = LightweightSubject::new();
    let observable: Rc<dyn IObservable<String>> = Rc::new(source.clone());
    let binding = target.bind(Class1::foo_property(), observable, BindingPriority::Style);
    source.on_next("style".to_string());
    assert_eq!(target.get_value(Class1::foo_property()), "style");
    target.set_value(Class1::foo_property(), "local".to_string());
    source.on_next("style2".to_string());
    assert_eq!(target.get_value(Class1::foo_property()), "local");
    target.clear_value(Class1::foo_property());
    assert_eq!(target.get_value(Class1::foo_property()), "style2");
    binding.dispose();
    assert_eq!(target.get_value(Class1::foo_property()), "foodefault");
}

#[test]
fn set_current_value_keeps_priority() {
    let target = Class1::new();
    let style = target.set_value_with_priority(Class1::foo_property(), "style".to_string(), BindingPriority::Style);
    target.set_current_value(Class1::foo_property(), "current".to_string());
    assert_eq!(target.get_value(Class1::foo_property()), "current");
    target.clear_value(Class1::foo_property());
    assert_eq!(target.get_value(Class1::foo_property()), "style");
    drop(style);
}

#[test]
fn property_changed_observable_and_registry() {
    let seen = Rc::new(Cell::new(0));
    let s = seen.clone();
    let sub = Class1::foo_property().changed().add_class_handler::<Class2>(move |_, _| s.set(s.get() + 1));
    Class1::new().set_value(Class1::foo_property(), "a".to_string());
    assert_eq!(seen.get(), 0);
    Class2::new().set_value(Class1::foo_property(), "a".to_string());
    assert_eq!(seen.get(), 1);
    sub.dispose();

    let registry = FerroPropertyRegistry::instance();
    let _ = (Class1::baz_property(), Class1::qux_property(), Class1::direct_property());
    assert!(registry.find_registered(Class2::TYPE, "Foo").is_some());
    assert!(registry.find_registered(Class2::TYPE, "Direct").is_some());
    assert!(registry.find_registered(FerroObject::TYPE, "Foo").is_none());
}

#[test]
fn objects_report_the_dispatcher_of_their_thread() {
    use crate::threading::Dispatcher;
    use std::sync::Arc;

    let _scope = Dispatcher::unit_test_scope();
    let dispatcher = Dispatcher::current_dispatcher();
    let object = FerroObject::new();
    assert!(Arc::ptr_eq(object.dispatcher(), &dispatcher));
    assert!(object.check_access());
    object.verify_access();
    // Derived classes reach the accessors through the base object.
    assert!(Arc::ptr_eq(Class2::new().dispatcher(), &dispatcher));

    // The dispatcher is kept, like the field of the reference implementation.
    Dispatcher::reset_for_unit_tests();
    let replacement = Dispatcher::current_dispatcher();
    assert!(!Arc::ptr_eq(&replacement, &dispatcher));
    assert!(Arc::ptr_eq(object.dispatcher(), &dispatcher));
    assert!(Arc::ptr_eq(FerroObject::new().dispatcher(), &replacement));
}
