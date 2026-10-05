//! Port of the upstream `TypedBindingExpressionTests` and of its
//! `Compatibility` part.
//!
//! `Should_Not_Produce_TypedBindingExpression_For_Non_StyledElement_Target`
//! reads the data context of an anchor, as only elements provide one. The
//! `null` run of `OneWay_Binding_Updates_Target_When_Source_Raises_PropertyChanged_For_All_Properties`
//! is the empty-name run (there is no null name). A getter that throws is a
//! getter that returns an error.

use super::*;
use crate::data::converters::FuncValueConverter;
use crate::data::core::plugins::PropertyInfoAccessorFactory;
use crate::data::core::{
    BindingExpression, ClrPropertyInfo, IPropertyInfo, Maybe, TypedBindingExpression, TypedClrPropertyInfo, Value,
    ValueType, ValueTypes,
};
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{
    BindingBase, BindingError, BindingExpressionBase, BindingMode, BindingPriority, CompiledBinding, CompiledBindingPathBuilder,
};
use crate::layout::{Layoutable, LayoutableImpl};
use crate::logging::{ILogSink, LogArea, LogEventLevel, Logger};
use crate::styling::test_support::TestRoot;
use crate::styling::{Selectors, Setter, Style};
use crate::{
    ferro_class, ferro_impl_classes, ferro_model, ferro_property, instantiate, AnyValue, AttachedProperty, DirectProperty,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledElement, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, VisualImpl,
};
use std::any::Any;
use std::fmt::Display;

struct ViewModel {
    string_value: RefCell<Option<String>>,
    double_value: Cell<f64>,
    /// Counts every setter invocation so that tests can assert that the
    /// binding does not write spurious values back to the source. The change
    /// notification is only raised on a real change.
    string_value_set_count: Cell<i32>,
    /// When set, the getter fails so that tests can verify that getter
    /// failures don't escape the binding's property-changed handler.
    throw_on_get: Cell<bool>,
    property_changed: Event<str>,
}

impl ViewModel {
    fn new() -> Rc<Self> {
        Model::new_model(Self {
            string_value: RefCell::new(None),
            double_value: Cell::new(0.0),
            string_value_set_count: Cell::new(0),
            throw_on_get: Cell::new(false),
            property_changed: Event::new(),
        })
    }

    fn with_string(value: &str) -> Rc<Self> {
        let result = Self::new();
        result.set_string_value(Some(s(value)));
        result
    }

    fn string_value(&self) -> Option<String> {
        self.string_value.borrow().clone()
    }

    /// The getter of the upstream property: fails when `throw_on_get` is set.
    fn try_string_value(&self) -> Result<Option<String>, BindingError> {
        if self.throw_on_get.get() {
            return Err(BindingError::message("Getter failed."));
        }
        Ok(self.string_value())
    }

    fn set_string_value(&self, value: Option<String>) {
        self.string_value_set_count.set(self.string_value_set_count.get() + 1);
        if *self.string_value.borrow() != value {
            self.string_value.replace(value);
            self.property_changed.raise("StringValue");
        }
    }

    fn double_value(&self) -> f64 {
        self.double_value.get()
    }

    fn set_double_value(&self, value: f64) {
        if self.double_value.get() != value {
            self.double_value.set(value);
            self.property_changed.raise("DoubleValue");
        }
    }

    /// Mutates the backing field without raising a change notification, so
    /// that tests can then raise an "all properties changed" notification
    /// and observe the binding react.
    fn set_string_value_without_notification(&self, value: Option<&str>) {
        self.string_value.replace(value.map(s));
    }

    fn raise_property_changed(&self, name: &str) {
        self.property_changed.raise(name);
    }

    fn property_changed_subscription_count(&self) -> usize {
        self.property_changed.handler_count()
    }
}

impl INotifyPropertyChanged for ViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(ViewModel, |b| b
    .notify_property_changed()
    .property::<Maybe<String>>("StringValue", |o| o.string_value(), |o, v| o.set_string_value(v))
    .property::<Value<f64>>("DoubleValue", |o| o.double_value(), |o, v| o.set_double_value(v)));

/// `new { Baz = "baz" }`.
struct BazData {
    baz: String,
}

ferro_model!(BazData, |b| b.read_only::<Value<String>>("Baz", |o| o.baz.clone()));

/// `new { StringValue = 1.5 }`.
struct DoubleStringData {
    string_value: f64,
}

ferro_model!(DoubleStringData, |b| b.read_only::<Value<f64>>("StringValue", |o| o.string_value));

static_type!(TextElement);

impl TextElement {
    ferro_property!(pub fn font_size_property() -> AttachedProperty<f64> {
        FerroProperty::register_attached_with::<TextElement, StyledElement, _>(
            "FontSize",
            StyledPropertyOptions::new(12.0).inherits(true),
        )
    });
}

/// The equivalent of the upstream text block (and selectable text block).
#[repr(C)]
pub struct TextBlock {
    base: Layoutable,
    selected_text: RefCell<Option<String>>,
}

ferro_class!(TextBlock: Layoutable);
ferro_impl_classes!(TextBlock: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl);

impl TextBlock {
    ferro_property!(pub fn text_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<TextBlock, _>("Text", None)
    });
    ferro_property!(pub fn tag_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<TextBlock, _>("Tag", None)
    });
    ferro_property!(pub fn opacity_property() -> StyledProperty<f64> {
        FerroProperty::register::<TextBlock, _>("TextOpacity", 1.0)
    });
    ferro_property!(pub fn selected_text_property() -> DirectProperty<TextBlock, Option<String>> {
        FerroProperty::register_direct::<TextBlock, _>(
            "SelectedText",
            |o| o.selected_text.borrow().clone(),
            None,
            None,
        )
    });

    fn new() -> Ref<Self> {
        instantiate(Self { base: Layoutable::construct(), selected_text: RefCell::new(None) })
    }

    fn with_data_context(data: Option<&Rc<ViewModel>>) -> Ref<Self> {
        let result = Self::new();
        result.set_data_context(data.map(|d| d.clone() as BoxedValue));
        result
    }

    fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    fn set_text(&self, value: Option<&str>) {
        self.set_value(Self::text_property(), value.map(s))
    }

    fn tag(&self) -> Option<BoxedValue> {
        self.get_value(Self::tag_property())
    }

    fn font_size(&self) -> f64 {
        self.get_value(TextElement::font_size_property())
    }

    fn set_selected_text(&self, value: Option<&str>) {
        self.set_and_raise(Self::selected_text_property(), &self.selected_text, value.map(s));
    }

    /// The priority of the effective value of a property: what the upstream
    /// diagnostics report. A value that is not set on the object itself is
    /// inherited (or the default).
    fn priority_of(&self, property: &'static FerroProperty) -> BindingPriority {
        self.values().get_effective_value(property).map_or(BindingPriority::Inherited, |v| v.priority())
    }
}

test_class!(NonStyledTarget: FerroObject);

impl NonStyledTarget {
    ferro_property!(pub fn value_property() -> StyledProperty<Option<String>> {
        FerroProperty::register::<NonStyledTarget, _>("Value", None)
    });
}

struct TestLogSink {
    errors: RefCell<Vec<String>>,
}

impl TestLogSink {
    fn start() -> Rc<Self> {
        let result = Rc::new(Self { errors: RefCell::new(Vec::new()) });
        Logger::set_thread_sink(Some(result.clone()));
        result
    }
}

impl ILogSink for TestLogSink {
    fn is_enabled(&self, level: LogEventLevel, area: &str) -> bool {
        level >= LogEventLevel::Warning && area == LogArea::BINDING
    }

    fn log(&self, _level: LogEventLevel, _area: &str, _source: Option<&dyn Any>, message_template: &str) {
        self.errors.borrow_mut().push(message_template.to_string());
    }

    fn log_with_values(
        &self,
        _level: LogEventLevel,
        _area: &str,
        _source: Option<&dyn Any>,
        message_template: &str,
        _property_values: &[&dyn Display],
    ) {
        self.errors.borrow_mut().push(message_template.to_string());
    }
}

type Typed = TypedBindingExpression<ViewModel, Option<String>>;

#[track_caller]
fn bind_and_assert(target: &TextBlock, binding: &dyn BindingBase) -> Rc<dyn BindingExpressionBase> {
    let expression = target.bind_binding(TextBlock::text_property(), binding);
    assert!(expression.as_any().is::<Typed>());
    expression
}

fn create_binding_with_setter(mode: BindingMode, settable: bool) -> Rc<CompiledBinding> {
    let setter: Option<Rc<dyn Fn(&ViewModel, Option<String>)>> =
        if settable { Some(Rc::new(|o, v| o.set_string_value(v))) } else { None };
    let property_info =
        TypedClrPropertyInfo::<ViewModel, Option<String>>::new_fallible("StringValue", |v| v.try_string_value(), setter);
    let path = CompiledBindingPathBuilder::new().typed_property_info(property_info).build();
    CompiledBinding::new(path).with_mode(mode)
}

fn create_binding(mode: BindingMode) -> Rc<CompiledBinding> {
    create_binding_with_setter(mode, true)
}

fn create_target(data: Option<&Rc<ViewModel>>, mode: BindingMode) -> Ref<TextBlock> {
    let result = TextBlock::with_data_context(data);
    let binding = create_binding(mode);
    bind_and_assert(&result, &binding);
    result
}

#[test]
fn should_produce_typed_binding_expression() {
    let binding = create_binding(BindingMode::OneWay);
    let target = TextBlock::new();
    bind_and_assert(&target, &binding);
}

#[test]
fn should_bind_string_value() {
    let data = ViewModel::with_string("Hello");
    let target = create_target(Some(&data), BindingMode::OneWay);

    assert_eq!(target.text(), Some(s("Hello")));
}

#[test]
fn one_way_binding_should_track_string_value() {
    let data = ViewModel::with_string("Hello");
    let target = create_target(Some(&data), BindingMode::OneWay);

    assert_eq!(target.text(), Some(s("Hello")));

    data.set_string_value(Some(s("World")));

    assert_eq!(target.text(), Some(s("World")));
}

#[test]
fn one_way_binding_should_track_data_context() {
    let data1 = ViewModel::with_string("Hello");
    let data2 = ViewModel::with_string("World");
    let target = create_target(Some(&data1), BindingMode::OneWay);

    assert_eq!(target.text(), Some(s("Hello")));

    target.set_data_context(Some(data2.clone()));

    assert_eq!(target.text(), Some(s("World")));
}

// The name of this test matches the name of the test in the binding
// expression tests.
#[test]
fn one_way_binding_updates_target_when_changes_and_source_raises_property_changed() {
    let data = ViewModel::with_string("foo");
    let target = create_target(Some(&data), BindingMode::OneWay);

    assert_eq!(target.text(), Some(s("foo")));

    target.set_current_value(TextBlock::text_property(), Some(s("bar")));
    assert_eq!(target.text(), Some(s("bar")));

    data.raise_property_changed("StringValue");
    assert_eq!(target.text(), Some(s("foo")));
}

#[test]
fn two_way_binding_writes_value_to_source() {
    let source = ViewModel::with_string("Hello");
    let target = create_target(Some(&source), BindingMode::TwoWay);

    assert_eq!(target.text(), Some(s("Hello")));

    source.set_string_value(Some(s("World")));

    assert_eq!(target.text(), Some(s("World")));

    target.set_text(Some("Goodbye"));

    assert_eq!(source.string_value(), Some(s("Goodbye")));
}

#[test]
fn two_way_binding_does_not_write_back_to_source_on_attach() {
    let source = ViewModel::with_string("Hello");
    let sets_after_construction = source.string_value_set_count.get();
    let target = create_target(Some(&source), BindingMode::TwoWay);

    assert_eq!(target.text(), Some(s("Hello")));

    // Pushing the source value to the target must not echo it straight back
    // to the source.
    assert_eq!(source.string_value_set_count.get(), sets_after_construction);
}

#[test]
fn two_way_binding_does_not_echo_source_change_back_to_source() {
    let source = ViewModel::with_string("Hello");
    let target = create_target(Some(&source), BindingMode::TwoWay);
    let before = source.string_value_set_count.get();

    source.set_string_value(Some(s("World"))); // One setter call: this assignment.

    assert_eq!(target.text(), Some(s("World")));
    assert_eq!(source.string_value_set_count.get(), before + 1);
}

#[test]
fn one_time_binding_sets_target_only_once_if_data_context_does_not_change() {
    let data = ViewModel::with_string("foo");
    let target = create_target(Some(&data), BindingMode::OneTime);

    assert_eq!(target.text(), Some(s("foo")));

    data.set_string_value(Some(s("bar")));
    assert_eq!(target.text(), Some(s("foo")));
}

#[test]
fn one_time_binding_sets_target_when_data_context_changes() {
    let data = ViewModel::with_string("foo");
    let target = create_target(Some(&data), BindingMode::OneTime);

    assert_eq!(target.text(), Some(s("foo")));

    target.set_data_context(Some(ViewModel::with_string("bar")));
    assert_eq!(target.text(), Some(s("bar")));
}

#[test]
fn one_time_binding_waits_for_data_context() {
    let target = create_target(None, BindingMode::OneTime);

    assert_eq!(target.text(), None);
}

#[test]
fn one_time_binding_waits_for_data_context_with_matching_property_name() {
    let data1 = Model::new_model(BazData { baz: s("baz") });
    let data2 = ViewModel::with_string("foo");
    let target = create_target(None, BindingMode::OneTime);

    target.set_data_context(Some(data1));
    assert_eq!(target.text(), None);

    target.set_data_context(Some(data2.clone()));
    assert_eq!(target.text(), Some(s("foo")));

    data2.set_string_value(Some(s("bar")));
    assert_eq!(target.text(), Some(s("foo")));
}

#[test]
fn one_time_binding_waits_for_data_context_with_matching_property_type() {
    let data1 = Model::new_model(DoubleStringData { string_value: 1.5 });
    let data2 = ViewModel::with_string("foo");
    let target = create_target(None, BindingMode::OneTime);

    target.set_data_context(Some(data1));
    assert_eq!(target.text(), None);

    target.set_data_context(Some(data2.clone()));
    assert_eq!(target.text(), Some(s("foo")));

    data2.set_string_value(Some(s("bar")));
    assert_eq!(target.text(), Some(s("foo")));
}

#[test]
fn one_way_to_source_binding_updates_source_when_target_changes() {
    let data = ViewModel::new();
    let target = create_target(Some(&data), BindingMode::OneWayToSource);

    assert_eq!(data.string_value(), None);

    target.set_text(Some("foo"));

    assert_eq!(data.string_value(), Some(s("foo")));
}

#[test]
fn one_way_to_source_binding_does_not_update_target_when_source_changes() {
    let data = ViewModel::new();
    let target = create_target(Some(&data), BindingMode::OneWayToSource);

    target.set_text(Some("foo"));
    assert_eq!(data.string_value(), Some(s("foo")));

    data.set_string_value(Some(s("bar")));
    assert_eq!(target.text(), Some(s("foo")));
}

#[test]
fn one_way_to_source_binding_updates_source_when_data_context_changes() {
    let data1 = ViewModel::new();
    let data2 = ViewModel::new();
    let target = create_target(Some(&data1), BindingMode::OneWayToSource);

    target.set_text(Some("foo"));
    assert_eq!(data1.string_value(), Some(s("foo")));

    target.set_data_context(Some(data2.clone()));
    assert_eq!(data2.string_value(), Some(s("foo")));
}

#[test]
fn can_bind_readonly_property_one_way_to_source() {
    let data = ViewModel::new();
    let target = TextBlock::with_data_context(Some(&data));
    target.set_text(Some("foobar"));
    target.set_selected_text(Some("foo"));

    let binding = create_binding(BindingMode::OneWayToSource);
    target.bind_binding(TextBlock::selected_text_property(), &binding);

    assert_eq!(data.string_value(), Some(s("foo")));
}

#[test]
fn can_bind_string_to_object() {
    let _logger = TestLogSink::start();
    let source = ViewModel::with_string("Hello");
    let binding = create_binding(BindingMode::OneWay);
    let target = TextBlock::with_data_context(Some(&source));

    let expression = target.bind_binding(TextBlock::tag_property(), &binding);

    assert!(expression.as_any().is::<Typed>());
    assert_eq!(target.tag().and_then(|v| v.downcast_ref::<String>().cloned()), Some(s("Hello")));
}

#[test]
fn disposing_binding_unsubscribes_from_source() {
    let data = ViewModel::with_string("foo");
    let target = TextBlock::with_data_context(Some(&data));
    let binding = create_binding(BindingMode::OneWay);

    let expression = target.bind_binding(TextBlock::text_property(), &binding);

    assert_eq!(target.text(), Some(s("foo")));
    assert_eq!(data.property_changed_subscription_count(), 1);

    expression.dispose();

    assert_eq!(data.property_changed_subscription_count(), 0);

    // Source changes no longer propagate to the (now unbound) target.
    data.set_string_value(Some(s("bar")));
    assert_ne!(target.text(), Some(s("bar")));
}

#[test]
fn rebinding_same_property_unsubscribes_previous_binding() {
    let data = ViewModel::with_string("foo");
    let target = TextBlock::with_data_context(Some(&data));

    target.bind_binding(TextBlock::text_property(), &create_binding(BindingMode::OneWay));
    target.bind_binding(TextBlock::text_property(), &create_binding(BindingMode::OneWay));

    // The first binding is disposed when the second is applied, leaving a
    // single subscription rather than two.
    assert_eq!(data.property_changed_subscription_count(), 1);
}

#[test]
fn should_not_produce_typed_binding_expression_when_binding_string_to_double() {
    let _logger = TestLogSink::start();
    let source = ViewModel::with_string("Hello");
    let binding = create_binding(BindingMode::OneWay);
    let target = TextBlock::with_data_context(Some(&source));

    let expression = target.bind_binding(TextBlock::opacity_property(), &binding);

    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn should_not_produce_typed_binding_expression_when_converter_is_present() {
    let binding = create_binding(BindingMode::OneWay);
    binding.set_converter(Some(Rc::new(FuncValueConverter::<Option<String>, Option<String>>::new(|s| s))));
    let target = TextBlock::new();

    let expression = target.bind_binding(TextBlock::text_property(), &binding);

    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn should_not_produce_typed_binding_expression_when_binding_data_context() {
    let _logger = TestLogSink::start();
    let binding = create_binding(BindingMode::OneWay);
    let target = TextBlock::new();

    let expression = target.bind_binding(StyledElement::data_context_property(), &binding);

    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn one_way_binding_updates_target_when_source_raises_property_changed_for_all_properties() {
    // An empty property name means "all properties changed", so the binding
    // must re-read its source value.
    let data = ViewModel::with_string("foo");
    let target = create_target(Some(&data), BindingMode::OneWay);

    assert_eq!(target.text(), Some(s("foo")));

    data.set_string_value_without_notification(Some("bar"));
    data.raise_property_changed("");

    assert_eq!(target.text(), Some(s("bar")));
}

#[test]
fn getter_exception_does_not_propagate_when_source_raises_property_changed() {
    // The untyped binding path swallows getter failures; the typed path must
    // do the same rather than letting them escape into the event handler.
    let data = ViewModel::with_string("foo");
    let target = create_target(Some(&data), BindingMode::OneWay);

    assert_eq!(target.text(), Some(s("foo")));

    data.throw_on_get.set(true);

    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| data.raise_property_changed("StringValue")));

    assert!(result.is_ok());
}

#[test]
fn should_not_produce_typed_binding_expression_for_read_only_source_in_two_way() {
    // A read-only source property cannot be written back to in two-way and
    // one-way-to-source modes, so the untyped path (which fails silently)
    // must be used instead.
    let binding = create_binding_with_setter(BindingMode::TwoWay, false);
    let target = TextBlock::with_data_context(Some(&ViewModel::new()));

    let expression = target.bind_binding(TextBlock::text_property(), &binding);

    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn should_not_produce_typed_binding_expression_when_target_type_is_wider_in_two_way() {
    // The source is a string but the target property is untyped. The forward
    // assignment is valid but writing an arbitrary value back to the string
    // source could fail, so the untyped path must be used.
    let source = ViewModel::with_string("Hello");
    let binding = create_binding(BindingMode::TwoWay);
    let target = TextBlock::with_data_context(Some(&source));

    let expression = target.bind_binding(TextBlock::tag_property(), &binding);

    assert!(expression.as_any().is::<BindingExpression>());
}

#[test]
fn should_not_produce_typed_binding_expression_for_non_styled_element_target() {
    // The typed expression only supports element targets; other objects must
    // use the untyped path rather than failing at run time.
    let binding = create_binding(BindingMode::OneWay);
    let anchor = TextBlock::with_data_context(Some(&ViewModel::with_string("Hello")));
    let target = NonStyledTarget::new();

    let expression =
        target.bind_binding_with_anchor(NonStyledTarget::value_property(), &binding, Some(&anchor.clone().upcast()));

    assert!(expression.as_any().is::<BindingExpression>());
    assert_eq!(target.get_value(NonStyledTarget::value_property()), Some(s("Hello")));
}

// --- compatibility ----------------------------------------------------------
//
// Tests which compare the behaviour of the typed binding expression with the
// untyped one for bindings which are eligible for the typed path. Each test
// runs twice: once with a binding which produces a typed expression and once
// with an equivalent binding which produces an untyped expression. The
// assertions describe the behaviour of the untyped expression.

#[track_caller]
fn assert_expression_type(typed: bool, expression: &Rc<dyn BindingExpressionBase>) {
    assert_eq!(!expression.as_any().is::<BindingExpression>(), typed);
}

/// The untyped description of `ViewModel.StringValue`, whose getter fails
/// when the view model says so.
struct StringValuePropertyInfo(ClrPropertyInfo);

impl IPropertyInfo for StringValuePropertyInfo {
    fn name(&self) -> &str {
        self.0.name()
    }

    fn get(&self, target: &dyn AnyValue) -> Option<BoxedValue> {
        self.0.get(target)
    }

    fn try_get(&self, target: &dyn AnyValue) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(data) = target.downcast_ref::<ViewModel>() {
            data.try_string_value()?;
        }
        self.0.try_get(target)
    }

    fn set(&self, target: &dyn AnyValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        self.0.set(target, value)
    }

    fn can_set(&self) -> bool {
        self.0.can_set()
    }

    fn can_get(&self) -> bool {
        self.0.can_get()
    }

    fn property_type(&self) -> ValueType {
        self.0.property_type()
    }
}

fn create_string_binding(typed: bool) -> Rc<CompiledBinding> {
    if typed {
        return create_binding(BindingMode::OneWay);
    }
    ValueTypes::register_reference::<ViewModel>();
    let path = CompiledBindingPathBuilder::new()
        .property(
            Rc::new(StringValuePropertyInfo(ClrPropertyInfo::read_write::<ViewModel, Maybe<String>>(
                "StringValue",
                |o| o.string_value(),
                |o, v| o.set_string_value(v),
            ))),
            PropertyInfoAccessorFactory::create_inpc_property_accessor::<ViewModel>(),
        )
        .build();
    CompiledBinding::new(path).with_mode(BindingMode::OneWay)
}

fn create_double_binding(typed: bool) -> Rc<CompiledBinding> {
    let builder = CompiledBindingPathBuilder::new();
    let builder = if typed {
        builder.typed_property::<ViewModel, f64>(
            "DoubleValue",
            |o| o.double_value(),
            Some(Rc::new(|o, v| o.set_double_value(v))),
        )
    } else {
        builder.notifying_property::<ViewModel, Value<f64>>("DoubleValue", |o| o.double_value(), |o, v| {
            o.set_double_value(v)
        })
    };
    CompiledBinding::new(builder.build()).with_mode(BindingMode::OneWay)
}

#[test]
fn null_data_context_should_not_override_style_setter() {
    for typed in [true, false] {
        // A binding which has no value must not contribute a value to the
        // target property, otherwise the property's default value is applied
        // at local value priority, hiding the value from the style setter.
        let target = TextBlock::new();
        let root = TestRoot::new();
        root.styles().add(Style::with_setters(
            Selectors::of_type::<TextBlock>(),
            [Setter::new(TextBlock::text_property(), Some(s("styled")))],
        ));
        crate::styling::test_support::set_child(&root, &target);

        assert_expression_type(typed, &target.bind_binding(TextBlock::text_property(), &create_string_binding(typed)));

        assert_eq!(target.text(), Some(s("styled")));
        assert_eq!(target.priority_of(TextBlock::text_property()), BindingPriority::Style);
    }
}

#[test]
fn null_data_context_should_not_break_property_inheritance() {
    for typed in [true, false] {
        // As above, but for an inherited property: applying the property's
        // default value at local value priority stops the value being
        // inherited from the parent.
        let target = TextBlock::new();
        let root = TestRoot::with_child(&target);
        root.set_value(TextElement::font_size_property(), 30.0);

        assert_expression_type(
            typed,
            &target.bind_binding(TextElement::font_size_property(), &create_double_binding(typed)),
        );

        assert_eq!(target.font_size(), 30.0);
        assert_eq!(target.priority_of(TextElement::font_size_property()), BindingPriority::Inherited);
    }
}

#[test]
fn style_priority_binding_with_null_data_context_should_not_break_property_inheritance() {
    for typed in [true, false] {
        // The same problem occurs for bindings at a priority other than
        // local value, such as a binding in a style setter.
        let target = TextBlock::new();
        let root = TestRoot::with_child(&target);
        root.set_value(TextElement::font_size_property(), 30.0);

        let binding = create_double_binding(typed).with_priority(BindingPriority::Style);

        assert_expression_type(typed, &target.bind_binding(TextElement::font_size_property(), &binding));

        assert_eq!(target.font_size(), 30.0);
        assert_eq!(target.priority_of(TextElement::font_size_property()), BindingPriority::Inherited);
    }
}

#[test]
fn setting_object_target_property_to_a_different_type_should_not_throw() {
    for typed in [true, false] {
        // The binding value type only needs to be convertible to the target
        // property type, so a string can be bound to an untyped property.
        // Writing a value of any other type to that property must not fail
        // when the binding reads the new target value.
        let data = ViewModel::with_string("foo");
        let target = TextBlock::with_data_context(Some(&data));
        let _root = TestRoot::with_child(&target);

        assert_expression_type(typed, &target.bind_binding(TextBlock::tag_property(), &create_string_binding(typed)));

        assert_eq!(target.tag().and_then(|v| v.downcast_ref::<String>().cloned()), Some(s("foo")));

        target.set_value(TextBlock::tag_property(), Some(boxed(5)));

        assert_eq!(target.tag().and_then(|v| v.downcast_ref::<i32>().copied()), Some(5));
    }
}

#[test]
fn setting_object_target_property_to_null_should_not_throw() {
    for typed in [true, false] {
        // As above, but with a number binding: writing null to the untyped
        // target property must not fail when the binding reads the new
        // target value.
        let data = ViewModel::new();
        data.set_double_value(1.0);
        let target = TextBlock::with_data_context(Some(&data));
        let _root = TestRoot::with_child(&target);

        assert_expression_type(typed, &target.bind_binding(TextBlock::tag_property(), &create_double_binding(typed)));

        assert_eq!(target.tag().and_then(|v| v.downcast_ref::<f64>().copied()), Some(1.0));

        target.set_value(TextBlock::tag_property(), None);

        assert!(target.tag().is_none());
    }
}

#[test]
fn incompatible_data_context_should_log_a_binding_error() {
    for typed in [true, false] {
        // When the data context isn't of the expected type a binding error
        // is logged.
        let sink = TestLogSink::start();

        let target = TextBlock::with_data_context(Some(&ViewModel::with_string("foo")));
        let _root = TestRoot::with_child(&target);

        assert_expression_type(typed, &target.bind_binding(TextBlock::text_property(), &create_string_binding(typed)));

        assert_eq!(target.text(), Some(s("foo")));

        target.set_data_context(Some(boxed(())));

        assert!(!sink.errors.borrow().is_empty(), "typed: {typed}");
        Logger::set_thread_sink(None);
    }
}

#[test]
fn source_getter_exception_should_clear_the_target_value() {
    for typed in [true, false] {
        // When the source getter fails, a binding error is reported and the
        // target reverts to its default value.
        let sink = TestLogSink::start();

        let data = ViewModel::with_string("foo");
        let target = TextBlock::with_data_context(Some(&data));
        let _root = TestRoot::with_child(&target);

        assert_expression_type(typed, &target.bind_binding(TextBlock::text_property(), &create_string_binding(typed)));

        assert_eq!(target.text(), Some(s("foo")));

        data.throw_on_get.set(true);
        data.raise_property_changed("StringValue");

        assert_eq!(target.text(), None, "typed: {typed}");
        assert!(!sink.errors.borrow().is_empty(), "typed: {typed}");
        Logger::set_thread_sink(None);
    }
}

// --- assignability (not an upstream test) -----------------------------------
//
// The rule that decides whether the typed expression is used: the equivalent
// of the runtime's "is assignable to" for value types.

#[test]
fn value_type_assignability_follows_the_managed_rules() {
    let of = ValueType::of::<String>;
    ValueTypes::register_reference::<ViewModel>();

    // Identity, "any value" and nullable wrapping.
    assert!(ValueTypes::is_assignable(of(), of()));
    assert!(ValueTypes::is_assignable(of(), ValueType::object()));
    assert!(ValueTypes::is_assignable(ValueType::of::<f64>(), ValueType::object()));
    assert!(ValueTypes::is_assignable(of(), ValueType::of::<Option<String>>()));
    assert!(!ValueTypes::is_assignable(ValueType::of::<Option<String>>(), of()));
    assert!(!ValueTypes::is_assignable(ValueType::object(), of()));

    // Conversions are not assignability.
    assert!(!ValueTypes::is_assignable(ValueType::of::<i32>(), ValueType::of::<f64>()));
    assert!(!ValueTypes::is_assignable(ValueType::of::<i32>(), ValueType::of::<Option<f64>>()));
    assert!(!ValueTypes::is_assignable(of(), ValueType::of::<f64>()));
    assert!(!ValueTypes::is_assignable(ValueType::of::<f64>(), of()));

    // Handles: derived to base, also through nullable handles.
    let derived = ValueType::of::<Ref<StyledElement>>();
    let base = ValueType::of::<Ref<FerroObject>>();
    let layoutable = ValueType::of::<Ref<Layoutable>>();
    assert!(ValueTypes::is_assignable(derived, base));
    assert!(ValueTypes::is_assignable(layoutable, derived));
    assert!(ValueTypes::is_assignable(layoutable, ValueType::of::<Option<Ref<StyledElement>>>()));
    assert!(ValueTypes::is_assignable(ValueType::of::<Option<Ref<Layoutable>>>(), ValueType::of::<Option<Ref<FerroObject>>>()));
    assert!(!ValueTypes::is_assignable(base, derived));
    assert!(!ValueTypes::is_assignable(ValueType::of::<Option<Ref<Layoutable>>>(), derived));

    // Model references.
    assert!(ValueTypes::is_assignable(ValueType::of::<Rc<ViewModel>>(), ValueType::of::<Option<Rc<ViewModel>>>()));
    assert!(!ValueTypes::is_assignable(ValueType::of::<Option<Rc<ViewModel>>>(), ValueType::of::<Rc<ViewModel>>()));
}

#[test]
fn value_type_cast_produces_exactly_the_target_type() {
    let cast = |value: BoxedValue, target: ValueType| ValueTypes::try_cast(&value, target);

    let value = cast(boxed(s("a")), ValueType::of::<Option<String>>()).unwrap();
    assert_eq!(value.downcast_ref::<Option<String>>(), Some(&Some(s("a"))));

    // A nullable value is null or its contents as "any value".
    let value = cast(boxed(Some(s("a"))), ValueType::object()).unwrap();
    let inner = value.downcast_ref::<Option<BoxedValue>>().unwrap().clone().unwrap();
    assert_eq!(inner.downcast_ref::<String>(), Some(&s("a")));
    let value = cast(boxed(Option::<String>::None), ValueType::object()).unwrap();
    assert!(value.downcast_ref::<Option<BoxedValue>>().unwrap().is_none());

    // Handles are cast to base handles, never to derived ones.
    let element: Ref<Layoutable> = TextBlock::new().upcast();
    let value = cast(boxed(Some(element.clone())), ValueType::of::<Option<Ref<StyledElement>>>()).unwrap();
    assert!(value.downcast_ref::<Option<Ref<StyledElement>>>().unwrap().is_some());
    let value = cast(boxed(Option::<Ref<Layoutable>>::None), ValueType::of::<Option<Ref<StyledElement>>>()).unwrap();
    assert!(value.downcast_ref::<Option<Ref<StyledElement>>>().unwrap().is_none());
    let root: Ref<FerroObject> = element.upcast();
    assert!(cast(boxed(root), ValueType::of::<Ref<StyledElement>>()).is_none());

    // Conversions are not casts.
    assert!(cast(boxed(1i32), ValueType::of::<f64>()).is_none());
    assert!(cast(boxed(s("1")), ValueType::of::<i32>()).is_none());
    assert!(cast(boxed(1i32), ValueType::of::<String>()).is_none());
}
