//! Tests of setters whose value is a binding, ported from the upstream
//! `SetterTests` and `StyleTests` (the tests that need binding descriptions;
//! the others are in `styling/style_tests.rs`).
//!
//! The control library is a separate crate, so the tests use minimal styled
//! element classes in place of the controls used by the upstream tests.

use super::*;
use crate::data::converters::IValueConverter;
use crate::data::core::{Maybe, Value, ValueType};
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{
    BindingError, BindingMode, BindingPriority, ReflectionBinding, RelativeSource, RelativeSourceMode,
    TemplateBinding,
};
use crate::media::{Brushes, IBrush};
use crate::styling::test_support::{record_values, remove_child, set_child, try_attach, Class1, TestRoot};
use crate::styling::{Selectors, Setter, Style};
use crate::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, ClassBindingManager, DirectProperty,
    FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl, StyledProperty,
};

/// An element with a tag.
#[repr(C)]
pub struct Decorator {
    base: StyledElement,
}

ferro_class!(Decorator: StyledElement);
ferro_impl_classes!(Decorator: FerroObjectImpl, StyledElementImpl);

impl Decorator {
    ferro_property!(pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<Decorator, _>("Tag", None)
    });

    pub fn construct() -> Self {
        Self { base: StyledElement::construct() }
    }

    pub fn new() -> Ref<Self> {
        Self::tag_property();
        instantiate(Self::construct())
    }

    pub fn tag(&self) -> Option<BoxedValue> {
        self.get_value(Self::tag_property())
    }
}

/// A decorator with a background.
#[repr(C)]
pub struct Border {
    base: Decorator,
}

ferro_class!(Border: Decorator);
ferro_impl_classes!(Border: FerroObjectImpl, StyledElementImpl);

impl Border {
    ferro_property!(pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<Border, _>("Background", None)
    });

    pub fn new() -> Ref<Self> {
        Decorator::tag_property();
        Self::background_property();
        instantiate(Self { base: Decorator::construct() })
    }

    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value)
    }
}

#[repr(C)]
pub struct DirectPropertyClass {
    base: StyledElement,
    foo: RefCell<Option<String>>,
}

ferro_class!(DirectPropertyClass: StyledElement);
ferro_impl_classes!(DirectPropertyClass: FerroObjectImpl, StyledElementImpl);

impl DirectPropertyClass {
    ferro_property!(pub fn foo_property() -> DirectProperty<DirectPropertyClass, Option<String>> {
        FerroProperty::register_direct::<DirectPropertyClass, _>(
            "Foo",
            |o| o.foo(),
            Some(|o, v| o.set_foo(v)),
            None,
        )
    });

    pub fn new() -> Ref<Self> {
        Self::foo_property();
        instantiate(Self { base: StyledElement::construct(), foo: RefCell::new(None) })
    }

    pub fn foo(&self) -> Option<String> {
        self.foo.borrow().clone()
    }

    pub fn set_foo(&self, value: Option<String>) {
        self.set_and_raise(Self::foo_property(), &self.foo, value);
    }
}

/// A view model whose properties do not raise change notifications.
pub struct Data {
    foo: RefCell<Option<String>>,
    bar: RefCell<Option<Rc<dyn IBrush>>>,
    property_changed: Event<str>,
}

impl Data {
    fn new(foo: Option<&str>, bar: Option<Rc<dyn IBrush>>) -> Rc<Self> {
        Model::new_model(Self {
            foo: RefCell::new(foo.map(s)),
            bar: RefCell::new(bar),
            property_changed: Event::new(),
        })
    }

    fn foo(&self) -> Option<String> {
        self.foo.borrow().clone()
    }

    fn bar(&self) -> Option<Rc<dyn IBrush>> {
        self.bar.borrow().clone()
    }

    fn property_changed_subscription_count(&self) -> usize {
        self.property_changed.handler_count()
    }
}

impl INotifyPropertyChanged for Data {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Data, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("Foo", |o| o.foo(), |o, v| {
        o.foo.replace(v);
    })
    .property::<Maybe<Rc<dyn IBrush>>>("Bar", |o| o.bar(), |o, v| {
        o.bar.replace(v);
    }));

pub struct FooBar {
    foo: String,
    bar: String,
}

ferro_model!(FooBar, |b| b
    .read_only::<Value<String>>("Foo", |o| o.foo.clone())
    .read_only::<Value<String>>("Bar", |o| o.bar.clone()));

struct TestConverter;

impl IValueConverter for TestConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let value = value.and_then(|v| v.downcast_ref::<String>().cloned()).unwrap_or_default();
        Ok(Some(boxed(value + "bar")))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        panic!("The method or operation is not implemented.");
    }
}

fn red() -> Option<Rc<dyn IBrush>> {
    Some(Brushes::red())
}

fn green() -> Option<Rc<dyn IBrush>> {
    Some(Brushes::green())
}

fn blue() -> Option<Rc<dyn IBrush>> {
    Some(Brushes::blue())
}

fn binding_style<T: crate::ObjectType + crate::Upcast<StyledElement>>(
    class: Option<&str>,
    property: &'static FerroProperty,
    binding: Rc<ReflectionBinding>,
) -> Ref<Style> {
    let selector = Selectors::of_type::<T>();
    let selector = match class {
        Some(class) => selector.class(class),
        None => selector,
    };
    Style::with_setters(selector, [Setter::new_binding_base(property, binding)])
}

fn green_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<Border>().class("foo"),
        [Setter::new(Border::background_property(), green())],
    )
}

// --- upstream `SetterTests` ---------------------------------------------------

#[test]
fn does_not_call_converter_convert_back_on_one_way_binding() {
    let control = Decorator::new();
    control.set_name(Some(s("foo")));
    control.classes().add("foo");

    let binding = ReflectionBinding::new("Name").with_mode(BindingMode::OneWay);
    binding.set_converter(Some(Rc::new(TestConverter)));
    binding.set_relative_source(Some(RelativeSource::new(RelativeSourceMode::SelfMode)));

    let style = binding_style::<Decorator>(Some("foo"), Decorator::tag_property().as_property(), binding);

    try_attach(&style, &control, None);

    assert_eq!(control.tag().and_then(|v| v.downcast_ref::<String>().cloned()).as_deref(), Some("foobar"));

    // Issue #1218 caused the converter's `convert_back` to be called here.
    control.classes().remove("foo");
    assert!(control.tag().is_none());
}

#[test]
fn setter_should_apply_binding_without_activator_with_style_priority() {
    let control = Border::new();
    control.set_data_context(Some(boxed(s("foo"))));

    let style =
        binding_style::<Border>(None, Decorator::tag_property().as_property(), ReflectionBinding::empty());

    let raised = Counter::new();
    let r = raised.clone();
    control.property_changed(move |e| {
        assert_eq!(e.property(), Decorator::tag_property().as_property());
        assert_eq!(e.priority(), BindingPriority::Style);
        r.increment();
    });

    try_attach(&style, &control, None);

    assert_eq!(raised.get(), 1);
}

#[test]
fn setter_should_apply_binding_with_activator_with_style_trigger_priority() {
    let control = Border::new();
    control.classes().add("foo");
    control.set_data_context(Some(boxed(s("foo"))));

    let style =
        binding_style::<Border>(Some("foo"), Decorator::tag_property().as_property(), ReflectionBinding::empty());

    let raised = Counter::new();
    let r = raised.clone();
    control.property_changed(move |e| {
        assert_eq!(e.property(), Decorator::tag_property().as_property());
        assert_eq!(e.priority(), BindingPriority::StyleTrigger);
        r.increment();
    });

    try_attach(&style, &control, None);

    assert_eq!(raised.get(), 1);
}

#[test]
fn direct_property_setter_with_two_way_binding_should_update_source() {
    let data = Data::new(Some("foo"), None);
    let control = DirectPropertyClass::new();
    control.set_data_context(Some(data.clone()));

    let style = binding_style::<DirectPropertyClass>(
        None,
        DirectPropertyClass::foo_property().as_property(),
        ReflectionBinding::new("Foo").with_mode(BindingMode::TwoWay),
    );

    try_attach(&style, &control, None);
    assert_eq!(control.foo().as_deref(), Some("foo"));

    control.set_foo(Some(s("bar")));
    assert_eq!(data.foo().as_deref(), Some("bar"));
}

#[test]
fn styled_property_setter_with_two_way_binding_should_update_source() {
    let data = Data::new(None, red());
    let control = Border::new();
    control.set_data_context(Some(data.clone()));

    let style = binding_style::<Border>(
        None,
        Border::background_property().as_property(),
        ReflectionBinding::new("Bar").with_mode(BindingMode::TwoWay),
    );

    try_attach(&style, &control, None);
    assert_eq!(control.background(), red());

    control.set_background(green());
    assert_eq!(data.bar(), green());
}

#[test]
fn non_active_styled_property_binding_should_be_unsubscribed() {
    let data = Data::new(None, red());
    let control = Border::new();
    control.set_data_context(Some(data.clone()));

    let style1 =
        binding_style::<Border>(None, Border::background_property().as_property(), ReflectionBinding::new("Bar"));
    let style2 = green_style();

    try_attach(&style1, &control, None);
    try_attach(&style2, &control, None);

    // `style1` is initially active.
    assert_eq!(control.background(), red());
    assert_eq!(data.property_changed_subscription_count(), 1);

    // Activate `style2`.
    control.classes().add("foo");
    assert_eq!(control.background(), green());

    // The binding from `style1` is now inactive and so should be
    // unsubscribed.
    assert_eq!(data.property_changed_subscription_count(), 0);
}

#[test]
fn non_active_styled_property_setter_with_two_way_binding_should_not_update_source() {
    let data = Data::new(None, red());
    let control = Border::new();
    control.set_data_context(Some(data.clone()));

    let style1 = binding_style::<Border>(
        None,
        Border::background_property().as_property(),
        ReflectionBinding::new("Bar").with_mode(BindingMode::TwoWay),
    );
    let style2 = green_style();

    try_attach(&style1, &control, None);
    try_attach(&style2, &control, None);

    // `style1` is initially active.
    assert_eq!(control.background(), red());

    // Activate `style2`.
    control.classes().add("foo");
    assert_eq!(control.background(), green());

    // The two-way binding from `style1` is now inactive and so should not
    // write back to the data context.
    assert_eq!(data.bar(), red());
}

#[test]
fn styled_property_setter_with_two_way_binding_updates_source_when_made_active() {
    let data = Data::new(None, red());
    let control = Border::new();
    control.classes().add("foo");
    control.set_data_context(Some(data.clone()));

    let style1 = binding_style::<Border>(
        None,
        Border::background_property().as_property(),
        ReflectionBinding::new("Bar").with_mode(BindingMode::TwoWay),
    );
    let style2 = green_style();

    try_attach(&style1, &control, None);
    try_attach(&style2, &control, None);

    // `style2` is initially active.
    assert_eq!(control.background(), green());

    // Deactivate `style2`.
    control.classes().remove("foo");
    assert_eq!(control.background(), red());

    // The two-way binding from `style1` is now active and so should write
    // back to the data context.
    control.set_background(blue());
    assert_eq!(data.bar(), blue());
}

// --- upstream `StyleTests` ----------------------------------------------------

#[test]
fn style_with_class_selector_should_update_and_restore_value_with_template_binding() {
    let style = Style::with_setters(
        Selectors::of_type::<Class1>().class("foo"),
        [Setter::new(Class1::foo_property(), s("Foo"))],
    );

    let templated_parent = Class1::new();
    templated_parent.set_foo("unset-foo");
    let target = Class1::new();
    target.set_templated_parent(&templated_parent);
    // A template binding binds with template priority.
    target.bind_binding(Class1::foo_property(), &TemplateBinding::new(Class1::foo_property()));

    try_attach(&style, &target, None);
    assert_eq!(target.foo(), "unset-foo");
    target.classes().add("foo");
    assert_eq!(target.foo(), "Foo");
    target.classes().remove("foo");
    assert_eq!(target.foo(), "unset-foo");
}

fn root_with_two_binding_styles() -> Ref<TestRoot> {
    let root = TestRoot::new();
    root.styles().add(binding_style::<Class1>(
        None,
        Class1::foo_property().as_property(),
        ReflectionBinding::new("Foo"),
    ));
    root.styles().add(binding_style::<Class1>(
        None,
        Class1::foo_property().as_property(),
        ReflectionBinding::new("Bar"),
    ));
    root
}

#[test]
fn inactive_bindings_should_not_be_made_active_during_style_attach() {
    let root = root_with_two_binding_styles();

    let target = Class1::new();
    target.set_data_context(Some(Model::new_model(FooBar { foo: s("Foo"), bar: s("Bar") })));

    let values = record_values(&target, Class1::foo_property());
    set_child(&root, &target);

    assert_eq!(*values.borrow(), vec!["foodefault", "Bar"]);
}

#[test]
fn inactive_bindings_should_not_be_made_active_during_style_detach() {
    let root = root_with_two_binding_styles();

    let target = Class1::new();
    target.set_data_context(Some(Model::new_model(FooBar { foo: s("Foo"), bar: s("Bar") })));

    set_child(&root, &target);

    let values = record_values(&target, Class1::foo_property());
    remove_child(&root, &target);

    assert_eq!(*values.borrow(), vec!["Bar", "foodefault"]);
}

// --- class bindings in setters (specific to this port) ---------------------------

/// The upstream setter rejects a class binding property in a style with an
/// activator; there is no upstream test for it.
#[test]
fn cannot_set_class_binding_property_in_style_with_activator() {
    let control = Border::new();
    control.classes().add("foo");
    control.set_data_context(Some(boxed(true)));

    let style = binding_style::<Border>(
        Some("foo"),
        ClassBindingManager::get_class_property("bar"),
        ReflectionBinding::empty(),
    );

    assert_panics(|| {
        try_attach(&style, &control, None);
    });
}

#[test]
fn can_set_class_binding_property_in_style_without_activator() {
    let control = Border::new();
    control.set_data_context(Some(boxed(true)));

    let style =
        binding_style::<Border>(None, ClassBindingManager::get_class_property("bar"), ReflectionBinding::empty());

    try_attach(&style, &control, None);

    assert!(control.classes().contains("bar"));
}
