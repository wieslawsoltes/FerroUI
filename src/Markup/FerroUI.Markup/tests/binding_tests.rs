//! Ported from the upstream `BindingTests`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::data::core::{Maybe, Untyped, Value};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingMode, BindingPriority};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

// --- models -----------------------------------------------------------------

pub struct Source {
    foo: RefCell<Option<String>>,
    foo_set_count: Cell<i32>,
    property_changed: Event<str>,
}

impl Source {
    pub fn new() -> Rc<Self> {
        Model::new_model(Self { foo: RefCell::new(None), foo_set_count: Cell::new(0), property_changed: Event::new() })
    }

    pub fn with_foo(foo: Option<&str>) -> Rc<Self> {
        let result = Self::new();
        result.set_foo(foo.map(str::to_string));
        result
    }

    pub fn foo(&self) -> Option<String> {
        self.foo.borrow().clone()
    }

    pub fn set_foo(&self, value: Option<String>) {
        self.foo.replace(value);
        self.foo_set_count.set(self.foo_set_count.get() + 1);
        self.property_changed.raise("Foo");
    }

    pub fn foo_set_count(&self) -> i32 {
        self.foo_set_count.get()
    }

    pub fn subscriber_count(&self) -> usize {
        self.property_changed.handler_count()
    }
}

impl INotifyPropertyChanged for Source {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Source, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("Foo", |o| o.foo(), |o, v| o.set_foo(v)));

/// A value that is cloned when it is read from [`WeakRefSource`].
#[derive(Clone, PartialEq, Debug)]
struct DummyObject(Option<String>);

struct WeakRefSource {
    foo: RefCell<Option<Weak<dyn AnyValue>>>,
    property_changed: Event<str>,
}

impl WeakRefSource {
    fn new() -> Rc<Self> {
        Model::new_model(Self { foo: RefCell::new(None), property_changed: Event::new() })
    }

    fn foo(&self) -> Option<BoxedValue> {
        let target = self.foo.borrow().as_ref()?.upgrade()?;
        match target.downcast_ref::<DummyObject>() {
            Some(cloneable) => Some(Rc::new(cloneable.clone())),
            None => Some(target),
        }
    }

    fn set_foo(&self, value: Option<BoxedValue>) {
        self.foo.replace(value.as_ref().map(Rc::downgrade));
        self.property_changed.raise("Foo");
    }
}

impl INotifyPropertyChanged for WeakRefSource {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(WeakRefSource, |b| b
    .notify_property_changed()
    .property::<Untyped>("Foo", |o| o.foo(), |o, v| o.set_foo(v)));

struct Headered {
    header: Option<BoxedValue>,
}

ferro_model!(Headered, |b| b.read_only::<Untyped>("Header", |o| o.header.clone()));

struct FooModel {
    foo: String,
}

impl FooModel {
    fn new(foo: &str) -> Rc<Self> {
        Model::new_model(Self { foo: s(foo) })
    }
}

ferro_model!(FooModel, |b| b.read_only::<Value<String>>("Foo", |o| o.foo.clone()));

struct NullableValuesViewModel {
    nullable_double: Cell<Option<f64>>,
    property_changed: Event<str>,
}

impl NullableValuesViewModel {
    fn new(value: Option<f64>) -> Rc<Self> {
        Model::new_model(Self { nullable_double: Cell::new(value), property_changed: Event::new() })
    }

    fn set_nullable_double(&self, value: Option<f64>) {
        self.nullable_double.set(value);
        self.property_changed.raise("NullableDouble");
    }
}

impl INotifyPropertyChanged for NullableValuesViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(NullableValuesViewModel, |b| b
    .notify_property_changed()
    .property::<Maybe<f64>>("NullableDouble", |o| o.nullable_double.get(), |o, v| o.set_nullable_double(v)));

struct TestStackOverflowViewModel {
    setter_invoked_count: Cell<i32>,
    bool_value: Cell<bool>,
    value: Cell<f64>,
    property_changed: Event<str>,
}

impl TestStackOverflowViewModel {
    const MAX_INVOKED_COUNT: i32 = 1000;

    fn new() -> Rc<Self> {
        Model::new_model(Self {
            setter_invoked_count: Cell::new(0),
            bool_value: Cell::new(false),
            value: Cell::new(0.0),
            property_changed: Event::new(),
        })
    }

    fn setter_invoked_count(&self) -> i32 {
        self.setter_invoked_count.get()
    }

    fn bool_value(&self) -> bool {
        self.bool_value.get()
    }

    fn set_bool_value(&self, value: bool) {
        if self.bool_value.get() != value {
            self.bool_value.set(value);
            self.setter_invoked_count.set(self.setter_invoked_count.get() + 1);
            self.property_changed.raise("BoolValue");
        }
    }

    fn value(&self) -> f64 {
        self.value.get()
    }

    fn set_value(&self, value: f64) {
        if self.value.get() != value {
            self.setter_invoked_count.set(self.setter_invoked_count.get() + 1);
            if self.setter_invoked_count.get() < Self::MAX_INVOKED_COUNT {
                self.value.set(value.trunc().clamp(25.0, 75.0));
            } else {
                self.value.set(value);
            }
            self.property_changed.raise("Value");
        }
    }

    fn reset_setter_invoked_count(&self) {
        self.setter_invoked_count.set(0);
    }
}

impl INotifyPropertyChanged for TestStackOverflowViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(TestStackOverflowViewModel, |b| b
    .notify_property_changed()
    .property::<Value<bool>>("BoolValue", |o| o.bool_value(), |o, v| o.set_bool_value(v))
    .property::<Value<f64>>("Value", |o| o.value(), |o, v| o.set_value(v)));

struct OldDataContextViewModel {
    foo: Cell<i32>,
    bar: Cell<i32>,
}

ferro_model!(OldDataContextViewModel, |b| b
    .property::<Value<i32>>("Foo", |o| o.foo.get(), |o, v| o.foo.set(v))
    .property::<Value<i32>>("Bar", |o| o.bar.get(), |o, v| o.bar.set(v)));

// --- classes ------------------------------------------------------------------

#[repr(C)]
struct StyledPropertyClass {
    base: FerroObject,
}

ferro_class!(StyledPropertyClass: FerroObject);
ferro_impl_classes!(StyledPropertyClass: FerroObjectImpl);

impl StyledPropertyClass {
    ferro_property!(fn double_value_property() -> StyledProperty<f64> {
        FerroProperty::register::<StyledPropertyClass, _>("DoubleValue", 12.3)
    });

    ferro_property!(fn nullable_double_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register::<StyledPropertyClass, _>("NullableDoubleProperty", Some(-1.0))
    });

    fn new() -> Ref<Self> {
        let result = instantiate(Self { base: FerroObject::construct() });
        Self::double_value_property();
        result
    }

    fn double_value(&self) -> f64 {
        self.get_value(Self::double_value_property())
    }

    fn set_double_value(&self, value: f64) {
        self.set_value(Self::double_value_property(), value)
    }

    fn nullable_double(&self) -> Option<f64> {
        self.get_value(Self::nullable_double_property())
    }
}

#[repr(C)]
struct DirectPropertyClass {
    base: FerroObject,
    double_value: Cell<f64>,
}

ferro_class!(DirectPropertyClass: FerroObject);
ferro_impl_classes!(DirectPropertyClass: FerroObjectImpl);

impl DirectPropertyClass {
    ferro_property!(fn double_value_property() -> DirectProperty<DirectPropertyClass, f64> {
        FerroProperty::register_direct::<DirectPropertyClass, _>(
            "DoubleValue",
            |o| o.double_value.get(),
            Some(|o, v| o.set_double_value(v)),
            0.0,
        )
    });

    fn new() -> Ref<Self> {
        let result = instantiate(Self { base: FerroObject::construct(), double_value: Cell::new(0.0) });
        Self::double_value_property();
        result
    }

    fn double_value(&self) -> f64 {
        self.double_value.get()
    }

    fn set_double_value(&self, value: f64) {
        self.set_and_raise_cell(Self::double_value_property(), &self.double_value, value);
    }
}

#[repr(C)]
struct TwoWayBindingTest {
    base: Control,
}

ferro_class!(TwoWayBindingTest: Control);
ferro_impl_classes!(
    TwoWayBindingTest: StyledElementImpl,
    VisualImpl,
    ferroui_base::layout::LayoutableImpl,
    ferroui_base::interactivity::InteractiveImpl,
    ferroui_base::input::InputElementImpl
);

impl FerroObjectImpl for TwoWayBindingTest {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::always_false_property().as_property() {
            this.set_current_value(Self::always_false_property(), false);
        }
    }
}

impl TwoWayBindingTest {
    ferro_property!(fn always_false_property() -> StyledProperty<bool> {
        FerroProperty::register::<TwoWayBindingTest, _>("AlwaysFalse", false)
    });

    ferro_property!(fn two_way_property() -> StyledProperty<Option<String>> {
        FerroProperty::register_with::<TwoWayBindingTest, _>(
            "TwoWay",
            StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
        )
    });

    fn new() -> Ref<Self> {
        {
        Control::class_init();
        instantiate(Self { base: Control::construct() })
    }
    }

    fn always_false(&self) -> bool {
        self.get_value(Self::always_false_property())
    }

    fn two_way(&self) -> Option<String> {
        self.get_value(Self::two_way_property())
    }

    fn set_two_way(&self, value: &str) {
        self.set_value(Self::two_way_property(), Some(s(value)))
    }
}

#[repr(C)]
struct TestControl {
    base: Control,
    value: RefCell<Option<BoxedValue>>,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(
    TestControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    ferroui_base::layout::LayoutableImpl,
    ferroui_base::interactivity::InteractiveImpl,
    ferroui_base::input::InputElementImpl
);

impl TestControl {
    ferro_property!(fn value_property() -> DirectProperty<TestControl, Option<BoxedValue>> {
        FerroProperty::register_direct::<TestControl, _>(
            "Value",
            |o| o.value.borrow().clone(),
            Some(|o, v| o.set_value_field(v)),
            None,
        )
    });

    fn new() -> Ref<Self> {
        {
        Control::class_init();
        Self::value_property();
        instantiate(Self { base: Control::construct(), value: RefCell::new(None) })
    }
    }

    fn set_value_field(&self, value: Option<BoxedValue>) {
        self.set_and_raise(Self::value_property(), &self.value, value);
    }
}

#[repr(C)]
struct OldDataContextTest {
    base: Control,
}

ferro_class!(OldDataContextTest: Control);
ferro_impl_classes!(
    OldDataContextTest: StyledElementImpl,
    VisualImpl,
    ferroui_base::layout::LayoutableImpl,
    ferroui_base::interactivity::InteractiveImpl,
    ferroui_base::input::InputElementImpl
);

impl FerroObjectImpl for OldDataContextTest {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let object: &FerroObject = this;
        this.bind(Self::bar_property(), object.get_observable(Self::foo_property()), BindingPriority::LocalValue);
    }
}

impl OldDataContextTest {
    ferro_property!(fn foo_property() -> StyledProperty<i32> {
        FerroProperty::register::<OldDataContextTest, _>("Foo", 0)
    });

    ferro_property!(fn bar_property() -> StyledProperty<i32> {
        FerroProperty::register::<OldDataContextTest, _>("Bar", 0)
    });

    fn new() -> Ref<Self> {
        {
        Control::class_init();
        instantiate(Self { base: Control::construct() })
    }
    }
}

fn binding(path: &str, mode: BindingMode) -> Rc<Binding> {
    Binding::with_path_and_mode(path, mode)
}

// --- tests --------------------------------------------------------------------

#[test]
fn one_way_binding_should_be_set_up() {
    let source = Source::with_foo(Some("foo"));
    let target = TextBlock::new();
    target.set_data_context(Some(source.clone()));
    let binding = binding("Foo", BindingMode::OneWay);

    target.bind_binding(TextBox::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
    source.set_foo(Some(s("bar")));
    assert_eq!(target.text().as_deref(), Some("bar"));
    target.set_text(Some("baz"));
    assert_eq!(source.foo().as_deref(), Some("bar"));
}

#[test]
fn two_way_binding_should_be_set_up() {
    let source = Source::with_foo(Some("foo"));
    let target = TextBlock::new();
    target.set_data_context(Some(source.clone()));
    let binding = binding("Foo", BindingMode::TwoWay);

    target.bind_binding(TextBox::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
    source.set_foo(Some(s("bar")));
    assert_eq!(target.text().as_deref(), Some("bar"));
    target.set_text(Some("baz"));
    assert_eq!(source.foo().as_deref(), Some("baz"));
}

fn assign_value(source: &TestControl, val: &str) -> Weak<dyn AnyValue> {
    let obj: BoxedValue = Rc::new(DummyObject(Some(s(val))));
    source.set_value_field(Some(obj.clone()));
    Rc::downgrade(&obj)
}

/// The upstream test forces garbage collections between the steps; values
/// are released deterministically here, so the steps are the same without
/// them.
#[test]
fn two_way_binding_should_be_set_up_gc_collect() {
    let source = WeakRefSource::new();
    let target = TestControl::new();
    target.set_data_context(Some(source.clone()));

    let binding = binding("Foo", BindingMode::TwoWay);

    target.bind_binding(TestControl::value_property(), &binding);

    let ref1 = assign_value(&target, "ref1");

    let expected = ref1.upgrade().expect("the value is alive");
    let actual = source.foo().expect("a value");
    assert!(expected.value_eq(&*actual));

    let _ref2 = assign_value(&target, "ref2");

    target.set_value_field(None);

    assert!(source.foo().is_none());
}

#[test]
fn one_time_binding_should_be_set_up() {
    let source = Source::with_foo(Some("foo"));
    let target = TextBlock::new();
    target.set_data_context(Some(source.clone()));
    let binding = binding("Foo", BindingMode::OneTime);

    target.bind_binding(TextBox::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
    source.set_foo(Some(s("bar")));
    assert_eq!(target.text().as_deref(), Some("foo"));
    target.set_text(Some("baz"));
    assert_eq!(source.foo().as_deref(), Some("bar"));
}

#[test]
fn one_way_to_source_binding_should_be_set_up() {
    let source = Source::with_foo(Some("foo"));
    let target = TextBlock::new();
    target.set_data_context(Some(source.clone()));
    target.set_text(Some("bar"));
    let binding = binding("Foo", BindingMode::OneWayToSource);

    target.bind_binding(TextBox::text_property(), &binding);

    assert_eq!(source.foo().as_deref(), Some("bar"));
    target.set_text(Some("baz"));
    assert_eq!(source.foo().as_deref(), Some("baz"));
    source.set_foo(Some(s("quz")));
    assert_eq!(target.text().as_deref(), Some("baz"));
}

#[test]
fn one_way_to_source_binding_should_react_to_data_context_changed() {
    let target = TextBlock::new();
    target.set_text(Some("bar"));
    let binding = binding("Foo", BindingMode::OneWayToSource);

    target.bind_binding(TextBox::text_property(), &binding);

    let source = Source::with_foo(Some("foo"));
    target.set_data_context(Some(source.clone()));

    assert_eq!(source.foo().as_deref(), Some("bar"));
    target.set_text(Some("baz"));
    assert_eq!(source.foo().as_deref(), Some("baz"));
    source.set_foo(Some(s("quz")));
    assert_eq!(target.text().as_deref(), Some("baz"));
}

#[test]
fn one_way_to_source_binding_should_not_stack_overflow_with_null_value() {
    // Issue #2912
    let target = TextBlock::new();
    target.set_text(None);
    let binding = binding("Foo", BindingMode::OneWayToSource);

    target.bind_binding(TextBox::text_property(), &binding);

    let source = Source::with_foo(Some("foo"));
    target.set_data_context(Some(source.clone()));

    assert!(source.foo().is_none());

    // Detect a runaway update by making sure the property was only set once
    // by the binding.
    assert_eq!(source.foo_set_count(), 2);
}

#[test]
fn default_binding_mode_should_be_used() {
    let source = Source::with_foo(Some("foo"));
    let target = TwoWayBindingTest::new();
    target.set_data_context(Some(source.clone()));
    let binding = Binding::with_path("Foo");

    target.bind_binding(TwoWayBindingTest::two_way_property(), &binding);

    assert_eq!(target.two_way().as_deref(), Some("foo"));
    source.set_foo(Some(s("bar")));
    assert_eq!(target.two_way().as_deref(), Some("bar"));
    target.set_two_way("baz");
    assert_eq!(source.foo().as_deref(), Some("baz"));
}

#[test]
fn data_context_binding_should_use_parent_data_context() {
    let parent_data_context = Model::new_model(Headered { header: bs("Foo") });

    let child = Control::new();
    let parent = Decorator::with_child(&child);
    parent.set_data_context(Some(parent_data_context));

    let binding = Binding::with_path("Header");

    child.bind_binding(StyledElement::data_context_property(), &binding);

    assert_eq!(as_string(&child.data_context()).as_deref(), Some("Foo"));

    let parent_data_context = Model::new_model(Headered { header: bs("Bar") });
    parent.set_data_context(Some(parent_data_context));
    assert_eq!(as_string(&child.data_context()).as_deref(), Some("Bar"));
}

#[test]
fn data_context_binding_should_track_parent() {
    let parent = Decorator::new();
    parent.set_data_context(Some(FooModel::new("foo")));

    let child = Control::new();

    let binding = Binding::with_path("Foo");

    child.bind_binding(StyledElement::data_context_property(), &binding);

    assert!(child.data_context().is_none());
    parent.set_child(Some(child.clone()));
    assert_eq!(as_string(&child.data_context()).as_deref(), Some("foo"));
}

#[test]
fn data_context_binding_should_produce_correct_results() {
    let view_model = FooModel::new("bar");
    let root = Decorator::new();
    root.set_data_context(Some(view_model));

    let child = Control::new();
    let values = Rc::new(RefCell::new(Vec::new()));

    let recorded = values.clone();
    let object: &FerroObject = &child;
    object
        .get_observable(StyledElement::data_context_property())
        .subscribe_fn(move |x| recorded.borrow_mut().push(as_string(&x)));
    child.bind_binding(StyledElement::data_context_property(), &Binding::with_path("Foo"));

    // When binding to the data context and the source isn't found, the
    // binding should produce null rather than the unset marker in order to
    // not propagate incorrect data contexts from parent controls while things
    // are being set up.
    assert!(child.is_set(StyledElement::data_context_property().as_property()));

    root.set_child(Some(child.clone()));

    assert_eq!(*values.borrow(), vec![None, Some(s("bar"))]);
}

#[test]
fn should_return_fallback_value_when_path_not_resolved() {
    let target = TextBlock::new();
    let source = Source::new();
    let binding = Binding::with_path("BadPath");
    binding.set_source(Some(source.clone()));
    binding.set_fallback_value(bs("foofallback"));

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foofallback"));
}

#[test]
fn should_return_fallback_value_when_invalid_source_type() {
    let target = ProgressBar::new();
    let source = Source::with_foo(Some("foo"));
    let binding = Binding::with_path("Foo");
    binding.set_source(Some(source.clone()));
    binding.set_fallback_value(Some(boxed(42)));

    target.bind_binding(ProgressBar::value_property(), &binding);

    assert_eq!(target.value(), 42.0);
}

#[test]
fn should_return_target_null_value_when_value_is_null() {
    let target = TextBlock::new();
    let source = Source::with_foo(None);

    let binding = Binding::with_path("Foo");
    binding.set_source(Some(source.clone()));
    binding.set_target_null_value(bs("(null)"));

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("(null)"));
}

#[test]
fn null_path_should_bind_to_data_context() {
    let target = TextBlock::new();
    target.set_data_context(bs("foo"));
    let binding = Binding::new();

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
}

#[test]
fn empty_path_should_bind_to_data_context() {
    let target = TextBlock::new();
    target.set_data_context(bs("foo"));
    let binding = Binding::with_path("");

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
}

#[test]
fn dot_path_should_bind_to_data_context() {
    let target = TextBlock::new();
    target.set_data_context(bs("foo"));
    let binding = Binding::with_path(".");

    target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text().as_deref(), Some("foo"));
}

/// Tests a problem discovered with a list box with selection.
///
/// - Items is bound to the data context first, followed by say the selected
///   index
/// - When the list box is removed from the logical tree, the data context
///   becomes null (as it's inherited)
/// - This changes the items to null, which changes the selected index to
///   null as there are no longer any items
/// - However, the news that the data context is now null hasn't yet reached
///   the selected index binding and so the unselection is sent back to the
///   view model
#[test]
fn should_not_write_to_old_data_context() {
    let vm = Model::new_model(OldDataContextViewModel { foo: Cell::new(1), bar: Cell::new(2) });
    let target = OldDataContextTest::new();

    let foo_binding = binding("Foo", BindingMode::TwoWay);
    let bar_binding = binding("Bar", BindingMode::TwoWay);

    // Bind Foo and Bar to the VM.
    target.bind_binding(OldDataContextTest::foo_property(), &foo_binding);
    target.bind_binding(OldDataContextTest::bar_property(), &bar_binding);
    target.set_data_context(Some(vm.clone()));

    // Make sure the control's Foo and Bar properties are read from the VM.
    assert_eq!(target.get_value(OldDataContextTest::foo_property()), 1);
    assert_eq!(target.get_value(OldDataContextTest::bar_property()), 2);

    // Set the data context to null.
    target.set_data_context(None);

    // Foo and Bar are no longer bound so they return 0, their default value.
    assert_eq!(target.get_value(OldDataContextTest::foo_property()), 0);
    assert_eq!(target.get_value(OldDataContextTest::bar_property()), 0);

    // The problem was here - the data context is now null, setting Foo to 0.
    // Bar is bound to Foo so Bar also gets set to 0. However the Bar binding
    // still had a reference to the VM and so vm.Bar was set to 0 erroneously.
    assert_eq!(vm.foo.get(), 1);
    assert_eq!(vm.bar.get(), 2);
}

/// The upstream test assigns the binding through the indexer of the object;
/// binding a property is the equivalent here.
#[test]
fn ferro_object_this_operator_accepts_binding() {
    let target = ContentControl::new();
    target.set_data_context(Some(FooModel::new("foo")));

    target.bind_binding(ContentControl::content_property(), &Binding::with_path("Foo"));

    assert_eq!(as_string(&target.content()).as_deref(), Some("foo"));
}

#[test]
fn styled_property_set_value_should_not_cause_stack_overflow_and_have_correct_values() {
    let view_model = TestStackOverflowViewModel::new();
    view_model.set_value(50.0);

    let target = StyledPropertyClass::new();

    let binding = binding("Value", BindingMode::TwoWay);
    binding.set_source(Some(view_model.clone()));
    target.bind_binding(StyledPropertyClass::double_value_property(), &binding);

    let child = StyledPropertyClass::new();

    let binding = self::binding("DoubleValue", BindingMode::TwoWay);
    binding.set_source(object_source(&target));
    child.bind_binding(StyledPropertyClass::double_value_property(), &binding);

    assert_eq!(view_model.setter_invoked_count(), 1);

    // Here in real life a stack overflow happened: issues #855 and #824.
    target.set_double_value(51.001);

    assert_eq!(view_model.setter_invoked_count(), 2);

    let expected = 51.0;

    assert_eq!(view_model.value(), expected);
    assert_eq!(target.double_value(), expected);
    assert_eq!(child.double_value(), expected);
}

#[test]
fn set_value_should_not_cause_stack_overflow_and_have_correct_values() {
    let view_model = TestStackOverflowViewModel::new();
    view_model.set_value(50.0);

    let target = DirectPropertyClass::new();

    let binding = binding("Value", BindingMode::TwoWay);
    binding.set_source(Some(view_model.clone()));
    target.bind_binding(DirectPropertyClass::double_value_property(), &binding);

    let child = DirectPropertyClass::new();

    let binding = self::binding("DoubleValue", BindingMode::TwoWay);
    binding.set_source(object_source(&target));
    child.bind_binding(DirectPropertyClass::double_value_property(), &binding);

    assert_eq!(view_model.setter_invoked_count(), 1);

    // Here in real life a stack overflow happened: issues #855 and #824.
    target.set_double_value(51.001);

    assert_eq!(view_model.setter_invoked_count(), 2);

    let expected = 51.0;

    assert_eq!(view_model.value(), expected);
    assert_eq!(target.double_value(), expected);
    assert_eq!(child.double_value(), expected);
}

#[test]
fn combined_one_time_and_one_way_to_source_bindings_should_release_subscriptions() {
    let target1 = TextBlock::new();
    let target2 = TextBlock::new();
    let root = Panel::new();
    root.add(&target1);
    root.add(&target2);
    let source = Source::with_foo(Some("foo"));

    let binding1 = target1.bind_binding(TextBlock::text_property(), &binding("Foo", BindingMode::OneTime));
    let binding2 = target2.bind_binding(TextBlock::text_property(), &binding("Foo", BindingMode::OneWayToSource));
    root.set_data_context(Some(source.clone()));
    binding2.dispose();
    binding1.dispose();

    assert_eq!(source.subscriber_count(), 0);
}

#[test]
fn binding_producing_default_value_should_result_in_correct_priority() {
    let default_value = StyledPropertyClass::nullable_double_property().get_default_value(StyledPropertyClass::TYPE);

    let vm = NullableValuesViewModel::new(default_value);
    let target = StyledPropertyClass::new();

    let binding = Binding::with_path("NullableDouble");
    binding.set_source(Some(vm.clone()));
    target.bind_binding(StyledPropertyClass::nullable_double_property(), &binding);

    // The value is set with local value priority.
    assert!(target.is_set(StyledPropertyClass::nullable_double_property().as_property()));
    assert_eq!(target.get_value(StyledPropertyClass::nullable_double_property()), default_value);
}

#[test]
fn binding_non_nullable_value_type_to_null_reverts_to_default_value() {
    let source = NullableValuesViewModel::new(Some(42.0));
    let target = StyledPropertyClass::new();
    let binding = Binding::with_path("NullableDouble");
    binding.set_source(Some(source.clone()));

    target.bind_binding(StyledPropertyClass::double_value_property(), &binding);
    assert_eq!(target.double_value(), 42.0);

    source.set_nullable_double(None);

    assert_eq!(target.double_value(), 12.3);
}

#[test]
fn binding_nullable_value_type_to_null_sets_value_to_null() {
    let source = NullableValuesViewModel::new(Some(42.0));
    let target = StyledPropertyClass::new();
    let binding = Binding::with_path("NullableDouble");
    binding.set_source(Some(source.clone()));

    target.bind_binding(StyledPropertyClass::nullable_double_property(), &binding);
    assert_eq!(target.nullable_double(), Some(42.0));

    source.set_nullable_double(None);

    assert_eq!(target.nullable_double(), None);
}

#[test]
fn one_way_to_source_binding_does_not_override_two_way_binding() {
    // Issue #2983
    let target1 = TextBlock::new();
    let target2 = TextBlock::new();
    target2.set_text(Some("OneWayToSource"));
    let source = Source::with_foo(Some("foo"));
    let root = Panel::new();
    root.set_data_context(Some(source.clone()));
    root.add(&target1);
    root.add(&target2);

    target1.bind_binding(TextBlock::text_property(), &binding("Foo", BindingMode::TwoWay));
    target2.bind_binding(TextBlock::text_property(), &binding("Foo", BindingMode::OneWayToSource));

    assert_eq!(source.foo().as_deref(), Some("OneWayToSource"));

    target1.set_text(Some("TwoWay"));

    assert_eq!(source.foo().as_deref(), Some("TwoWay"));
}

#[test]
fn target_undoing_property_change_during_two_way_binding_does_not_cause_stack_overflow() {
    let source = TestStackOverflowViewModel::new();
    source.set_bool_value(true);
    let target = TwoWayBindingTest::new();

    source.reset_setter_invoked_count();

    // The AlwaysFalse property is set to false in the property changed
    // callback. Ensure that binding it to an initial `true` value with a
    // two-way binding does not cause a stack overflow.
    target.bind_binding(TwoWayBindingTest::always_false_property(), &binding("BoolValue", BindingMode::TwoWay));

    target.set_data_context(Some(source.clone()));

    assert_eq!(source.setter_invoked_count(), 1);
    assert!(!source.bool_value());
    assert!(!target.always_false());
}
