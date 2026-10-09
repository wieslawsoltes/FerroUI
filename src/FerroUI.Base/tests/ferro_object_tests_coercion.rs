//! Port of the upstream `Coercion` object tests.
//!
//! `Control1` is a layoutable where upstream's is a control (the control
//! library is another crate), and the initial layout pass of
//! `Deactivating_Style_Respects_Coerced_Default_Value` is the styling of the
//! target.

use super::*;
use crate::data::{BindingPriority, BindingValue};
use crate::layout::{Layoutable, LayoutableImpl};
use crate::*;
use std::cell::{Cell, RefCell};

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    min_foo: Cell<i32>,
    max_foo: Cell<i32>,
    coerce_foo_invocations: RefCell<Vec<i32>>,
    core_changes: RefCell<Vec<(Option<i32>, i32, BindingPriority, bool)>>,
}

ferro_class!(Class1: FerroObject);

impl FerroObjectImpl for Class1 {
    fn on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        this.core_changes.borrow_mut().push((
            change.get_old_value::<i32>(),
            change.get_new_value::<i32>(),
            change.priority(),
            change.is_effective_value_change(),
        ));
        Self::parent_on_property_changed_core(this, change);
    }
}

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Class1, _>("Foo", StyledPropertyOptions::new(11).coerce(Class1::coerce_foo))
    });

    ferro_property!(pub fn attached_property() -> AttachedProperty<i32> {
        FerroProperty::register_attached_with::<Class1, FerroObject, _>(
            "Attached",
            StyledPropertyOptions::new(11).coerce(Class1::coerce_foo),
        )
    });

    ferro_property!(pub fn inherited_property() -> AttachedProperty<i32> {
        FerroProperty::register_attached_with::<Class1, Class1, _>(
            "Attached",
            StyledPropertyOptions::new(11).inherits(true).coerce(Class1::coerce_foo),
        )
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
            min_foo: Cell::new(0),
            max_foo: Cell::new(100),
            coerce_foo_invocations: RefCell::new(Vec::new()),
            core_changes: RefCell::new(Vec::new()),
        })
    }

    pub fn with_foo(value: i32) -> Ref<Self> {
        let result = Self::new();
        result.set_foo(value);
        result
    }

    pub fn foo(&self) -> i32 {
        self.get_value(Self::foo_property())
    }

    pub fn set_foo(&self, value: i32) {
        self.set_value(Self::foo_property(), value);
    }

    pub fn inherited(&self) -> i32 {
        self.get_value(Self::inherited_property())
    }

    pub fn set_inherited(&self, value: i32) {
        self.set_value(Self::inherited_property(), value);
    }

    pub fn coerce_foo(instance: &FerroObject, value: i32) -> i32 {
        match instance.downcast_ref::<Class1>() {
            Some(o) => {
                o.coerce_foo_invocations.borrow_mut().push(value);
                value.clamp(o.min_foo.get(), o.max_foo.get())
            }
            None => value.clamp(0, 100),
        }
    }
}

test_class!(Class2: FerroObject);

impl Class2 {
    ferro_property!(pub fn foo_property() -> StyledProperty<i32> {
        let property = Class1::foo_property().add_owner::<Class2>();
        // The upstream static constructor.
        property.override_metadata::<Class2>(StyledPropertyMetadata::new(None).with_coerce(Class2::coerce_foo));
        property
    });

    pub fn create() -> Ref<Self> {
        let _ = Self::foo_property();
        Self::new()
    }

    pub fn foo(&self) -> i32 {
        self.get_value(Self::foo_property())
    }

    pub fn set_foo(&self, value: i32) {
        self.set_value(Self::foo_property(), value);
    }

    pub fn coerce_foo(_instance: &FerroObject, value: i32) -> i32 {
        -value
    }
}

test_class!(Class3: FerroObject);

impl Class3 {
    ferro_property!(pub fn foo_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Class3, _>("Foo", StyledPropertyOptions::new(11).coerce(Class3::coerce_foo))
    });

    pub fn foo(&self) -> i32 {
        self.get_value(Self::foo_property())
    }

    pub fn coerce_foo(instance: &FerroObject, value: i32) -> i32 {
        assert!(instance.is::<Class3>());
        value.clamp(50, 100)
    }
}

#[test]
fn coerces_set_value() {
    let target = Class1::new();

    target.set_foo(150);

    assert_eq!(100, target.foo());
}

#[test]
fn coerces_set_value_attached() {
    let target = Class1::new();

    target.set_value(Class1::attached_property(), 150);

    assert_eq!(100, target.get_value(Class1::attached_property()));
}

#[test]
fn coerces_set_value_attached_on_class_not_derived_from_owner() {
    let target = Class2::create();

    target.set_value(Class1::attached_property(), 150);

    assert_eq!(100, target.get_value(Class1::attached_property()));
}

#[test]
fn coerces_bound_value() {
    let target = Class1::new();
    let source: Subject<BindingValue<i32>> = Subject::new();

    target.bind_value(Class1::foo_property(), source.observable(), BindingPriority::LocalValue);
    source.on_next(BindingValue::new(150));

    assert_eq!(100, target.foo());
}

#[test]
fn coerce_value_updates_value() {
    let target = Class1::with_foo(99);

    assert_eq!(99, target.foo());

    target.max_foo.set(50);
    target.coerce_value(Class1::foo_property());

    assert_eq!(50, target.foo());
}

#[test]
fn coerce_value_updates_base_value() {
    let target = Class1::with_foo(99);

    target.set_value_with_priority(Class1::foo_property(), 88, BindingPriority::Animation);

    assert_eq!(88, target.foo());
    assert_eq!(Some(99), target.get_base_value(Class1::foo_property()));

    target.max_foo.set(50);
    target.coerce_value(Class1::foo_property());

    assert_eq!(50, target.foo());
    assert_eq!(Some(50), target.get_base_value(Class1::foo_property()));
}

#[test]
fn coerce_value_raises_property_changed() {
    let target = Class1::with_foo(99);
    let raised = Counter::new();

    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(Class1::foo_property().as_property(), e.property());
        assert_eq!(Some(99), e.get_old_value::<i32>());
        assert_eq!(50, e.get_new_value::<i32>());
        assert_eq!(BindingPriority::LocalValue, e.priority());
        r.increment();
    });

    assert_eq!(99, target.foo());

    target.max_foo.set(50);
    target.coerce_value(Class1::foo_property());

    assert_eq!(50, target.foo());
    assert_eq!(1, raised.get());
}

#[test]
fn coerce_value_raises_property_changed_core_for_base_value() {
    let target = Class1::with_foo(99);

    target.set_value_with_priority(Class1::foo_property(), 88, BindingPriority::Animation);

    assert_eq!(88, target.foo());
    assert_eq!(Some(99), target.get_base_value(Class1::foo_property()));

    target.max_foo.set(50);
    target.core_changes.borrow_mut().clear();
    target.coerce_value(Class1::foo_property());

    assert_eq!(2, target.core_changes.borrow().len());
}

#[test]
fn coerce_value_calls_coerce_callback_only_once() {
    let target = Class1::with_foo(99);

    target.max_foo.set(50);
    target.coerce_foo_invocations.borrow_mut().clear();
    target.coerce_value(Class1::foo_property());

    assert_eq!(vec![99], *target.coerce_foo_invocations.borrow());
}

#[test]
fn coerced_value_can_be_restored_if_limit_changed() {
    let target = Class1::new();

    target.set_foo(150);
    assert_eq!(100, target.foo());

    target.max_foo.set(200);
    target.coerce_value(Class1::foo_property());

    assert_eq!(150, target.foo());
}

#[test]
fn coerced_value_can_be_restored_from_previously_active_binding() {
    let target = Class1::new();
    let source1: Subject<BindingValue<i32>> = Subject::new();
    let source2: Subject<BindingValue<i32>> = Subject::new();

    target.bind_value(Class1::foo_property(), source1.observable(), BindingPriority::Style);
    source1.on_next(BindingValue::new(150));

    target.bind_value(Class1::foo_property(), source2.observable(), BindingPriority::LocalValue);
    source2.on_next(BindingValue::new(160));

    assert_eq!(100, target.foo());

    target.max_foo.set(200);
    source2.on_completed();

    assert_eq!(150, target.foo());
}

#[test]
fn coerce_value_updates_inherited_value() {
    let parent = Class1::new();
    parent.set_inherited(99);
    let child = FerroObject::new();
    child.set_inheritance_parent(&parent);
    let raised = Counter::new();

    child.set_inheritance_parent(&parent);
    let r = raised.clone();
    child.property_changed(move |e| {
        assert_eq!(Class1::inherited_property().as_property(), e.property());
        assert_eq!(Some(99), e.get_old_value::<i32>());
        assert_eq!(50, e.get_new_value::<i32>());
        assert_eq!(BindingPriority::Inherited, e.priority());
        r.increment();
    });

    assert_eq!(99, child.get_value(Class1::inherited_property()));

    parent.max_foo.set(50);
    parent.coerce_value(Class1::inherited_property());

    assert_eq!(50, child.get_value(Class1::inherited_property()));
    assert_eq!(1, raised.get());
}

#[test]
fn coercion_can_be_overridden() {
    let target = Class2::create();

    target.set_foo(150);

    assert_eq!(-150, target.foo());
}

#[test]
fn default_value_can_be_coerced() {
    let target = Class1::new();
    let raised = Counter::new();

    target.min_foo.set(20);
    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(Class1::foo_property().as_property(), e.property());
        assert_eq!(Some(11), e.get_old_value::<i32>());
        assert_eq!(20, e.get_new_value::<i32>());
        assert_eq!(BindingPriority::Unset, e.priority());
        r.increment();
    });

    target.coerce_value(Class1::foo_property());

    assert_eq!(20, target.foo());
    assert_eq!(1, raised.get());
}

#[test]
fn default_value_is_coerced_only_once() {
    let target = Class1::new();

    target.min_foo.set(20);
    target.coerce_foo_invocations.borrow_mut().clear();
    target.coerce_value(Class1::foo_property());

    assert_eq!(vec![11], *target.coerce_foo_invocations.borrow());
}

#[test]
fn second_coerce_of_default_value_is_passed_uncoerced_value() {
    let target = Class1::new();

    target.min_foo.set(20);
    target.coerce_foo_invocations.borrow_mut().clear();
    target.coerce_value(Class1::foo_property());
    target.coerce_value(Class1::foo_property());

    assert_eq!(vec![11, 11], *target.coerce_foo_invocations.borrow());
}

#[test]
fn clear_value_respects_coerced_default_value() {
    let target = Class1::new();
    let raised = Counter::new();

    target.set_foo(30);
    target.min_foo.set(20);

    let r = raised.clone();
    target.property_changed(move |e| {
        assert_eq!(Class1::foo_property().as_property(), e.property());
        assert_eq!(Some(30), e.get_old_value::<i32>());
        assert_eq!(20, e.get_new_value::<i32>());
        assert_eq!(BindingPriority::Unset, e.priority());
        r.increment();
    });

    target.clear_value(Class1::foo_property());

    assert_eq!(20, target.foo());
    assert_eq!(1, raised.get());
}

#[test]
fn if_initial_state_has_coerced_default_value_then_coerce_value_must_be_called() {
    // This test is just explicitly describing an edge-case. If the initial
    // state of the object results in a coerced property value then
    // `coerce_value` must be called before coercion takes effect.
    let target = Class3::new();

    assert_eq!(11, target.foo());

    target.coerce_value(Class3::foo_property());

    assert_eq!(50, target.foo());
}

#[repr(C)]
pub struct Control1 {
    base: Layoutable,
    min_foo: Cell<i32>,
    max_foo: Cell<i32>,
}

ferro_class!(Control1: Layoutable);
ferro_impl_classes!(Control1: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl Control1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<i32> {
        FerroProperty::register_with::<Control1, _>("Foo", StyledPropertyOptions::new(11).coerce(Control1::coerce_foo))
    });

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct(), min_foo: Cell::new(0), max_foo: Cell::new(100) })
    }

    pub fn foo(&self) -> i32 {
        self.get_value(Self::foo_property())
    }

    pub fn coerce_foo(instance: &FerroObject, value: i32) -> i32 {
        let o = instance.downcast_ref::<Control1>().expect("a Control1");
        value.clamp(o.min_foo.get(), o.max_foo.get())
    }
}

#[test]
fn deactivating_style_respects_coerced_default_value() {
    use crate::styling::test_support::TestRoot;
    use crate::styling::{Selectors, Setter, Style};

    let target = Control1::new();
    target.min_foo.set(20);

    let root = TestRoot::new();
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Control1>().class("foo"),
        [Setter::new(Control1::foo_property(), 50)],
    ));
    crate::styling::test_support::set_child(&root, &target);

    let raised = Rc::new(Cell::new(0));

    target.classes().add("foo");
    target.apply_styling();

    assert_eq!(50, target.foo());

    let r = raised.clone();
    target.property_changed(move |e| {
        assert!(e.property() == Control1::foo_property().as_property());
        assert_eq!(Some(50), e.get_old_value::<i32>());
        assert_eq!(20, e.get_new_value::<i32>());
        assert_eq!(BindingPriority::Unset, e.priority());
        r.set(r.get() + 1);
    });

    target.classes().remove("foo");

    assert_eq!(20, target.foo());
    assert_eq!(1, raised.get());
}
