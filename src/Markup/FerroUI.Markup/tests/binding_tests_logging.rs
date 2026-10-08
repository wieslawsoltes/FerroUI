//! Ported from the upstream `BindingTests_Logging`.
//!
//! The nested test classes of the upstream suite are modules here. Type
//! names in messages are the names this port reports for the types.

use ferroui_base::utilities::CultureInfo;
use super::test_support::*;
use crate::data::Binding;
use ferroui_base::controls::{NameScope, NameScopeRef};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::parsers::TypeResolver;
use ferroui_base::data::core::plugins::PropertyInfoAccessorFactory;
use ferroui_base::data::core::{ClrPropertyInfo, Maybe, Value, ValueType, ValueTypes};
use ferroui_base::data::model::Model;
use ferroui_base::data::{
    BindingError, CompiledBinding, CompiledBindingPathBuilder, RelativeSource, RelativeSourceMode,
};
use ferroui_base::input::{Key, KeyBinding, KeyGesture, KeyModifiers};
use ferroui_base::layout::Layoutable;
use ferroui_base::logging::{LogArea, LogEventLevel};
use ferroui_base::*;
use std::fmt;
use std::rc::Rc;

/// Asserts, when dropped, that exactly one binding error was logged.
struct AssertLog {
    sink: Rc<TestLogSink>,
    target: *const (),
    expression: String,
    message: String,
    error_point: Option<String>,
    level: LogEventLevel,
    property: &'static FerroProperty,
}

fn assert_log<T: ObjectType + Upcast<FerroObject>>(
    target: &Ref<T>,
    expression: &str,
    message: &str,
    error_point: Option<&str>,
) -> AssertLog {
    assert_log_with(target, expression, message, error_point, LogEventLevel::Warning, None)
}

fn assert_log_with<T: ObjectType + Upcast<FerroObject>>(
    target: &Ref<T>,
    expression: &str,
    message: &str,
    error_point: Option<&str>,
    level: LogEventLevel,
    property: Option<&'static FerroProperty>,
) -> AssertLog {
    let object: &FerroObject = Upcast::<FerroObject>::upcast(&**target);
    AssertLog {
        sink: TestLogSink::start(level),
        target: object as *const FerroObject as *const (),
        expression: s(expression),
        message: s(message),
        error_point: error_point.map(s),
        level,
        property: property.unwrap_or(Control::tag_property().as_property()),
    }
}

impl Drop for AssertLog {
    fn drop(&mut self) {
        let logs = self.sink.stop();
        if std::thread::panicking() {
            return;
        }

        assert_eq!(logs.len(), 1, "{logs:?}");

        let l = &logs[0];
        let message_template = if self.error_point.is_some() {
            "An error occurred binding {Property} to {Expression} at {ExpressionErrorPoint}: {Message}"
        } else {
            "An error occurred binding {Property} to {Expression}: {Message}"
        };

        assert_eq!(l.level, self.level);
        assert_eq!(l.area, LogArea::BINDING);
        assert_eq!(l.source, Some(self.target));
        assert_eq!(l.message_template, message_template);
        assert_eq!(l.property_values[0], self.property.name());
        assert_eq!(l.property_values[1], self.expression);

        match &self.error_point {
            Some(error_point) => {
                assert_eq!(&l.property_values[2], error_point);
                assert_eq!(l.property_values[3], self.message);
            }
            None => assert_eq!(l.property_values[2], self.message),
        }
    }
}

/// Asserts, when dropped, that no warning or error was logged.
struct AssertNoLog {
    sink: Rc<TestLogSink>,
}

fn assert_no_log() -> AssertNoLog {
    AssertNoLog { sink: TestLogSink::start(LogEventLevel::Warning) }
}

impl Drop for AssertNoLog {
    fn drop(&mut self) {
        let logs = self.sink.stop();
        if !std::thread::panicking() {
            assert_eq!(logs.len(), 0, "{logs:?}");
        }
    }
}

fn resolve_type() -> TypeResolver {
    Rc::new(|_ns, type_name| match type_name {
        "TextBlock" => Some(CastTarget::Class(TextBlock::TYPE)),
        "TestRoot" => Some(CastTarget::Class(TestRoot::TYPE)),
        _ => None,
    })
}

struct TestClass {
    foo: Option<String>,
}

impl TestClass {
    fn new(foo: Option<&str>) -> Rc<Self> {
        Model::new_model(Self { foo: foo.map(s) })
    }
}

ferro_model!(TestClass, |b| b.read_only::<Maybe<String>>("Foo", |o| o.foo.clone()));

struct FooModel {
    foo: String,
}

ferro_model!(FooModel, |b| b.read_only::<Value<String>>("Foo", |o| o.foo.clone()));

struct BarModel {
    bar: Option<String>,
}

ferro_model!(BarModel, |b| b.read_only::<Maybe<String>>("Bar", |o| o.bar.clone()));

/// A value that no property type converts from.
#[derive(Clone, PartialEq, Default)]
struct Version {
    major: i32,
    minor: i32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

struct VersionModel {
    foo: Version,
}

impl VersionModel {
    fn new() -> Rc<Self> {
        ValueTypes::register_display::<Version>();
        Model::new_model(Self { foo: Version::default() })
    }
}

ferro_model!(VersionModel, |b| b.read_only::<Value<Version>>("Foo", |o| o.foo.clone()));

/// A data context without any declared property.
#[derive(PartialEq)]
struct PlainObject;

struct ThrowingConverter;

impl IValueConverter for ThrowingConverter {
    fn convert(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}

fn type_name<T: 'static>() -> &'static str {
    ValueType::of::<T>().name()
}

mod data_context {
    use super::*;

    #[test]
    fn should_not_log_missing_member_on_null_data_context() {
        let target = Decorator::new();
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("Foo");

        let _log = assert_no_log();
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_log_missing_member_on_data_context() {
        let target = Decorator::new();
        target.set_data_context(Some(TestClass::new(Some("foo"))));
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("Foo.Bar");

        let _log = assert_log(
            &target,
            &binding.path(),
            "Could not find a matching property accessor for 'Bar' on 'System.String'.",
            Some("Bar"),
        );
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_log_null_in_binding_chain() {
        let target = Decorator::new();
        target.set_data_context(Some(TestClass::new(None)));
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("Foo.Length");

        let _log = assert_log(&target, &binding.path(), "Value is null.", Some("Foo"));
        target.bind_binding(Control::tag_property(), &binding);
    }
}

mod source {
    use super::*;

    #[test]
    fn should_log_null_source() {
        let target = Decorator::new();
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("Foo");
        binding.set_source(None);

        let _log = assert_log(&target, &binding.path(), "Binding Source is null.", Some("(source)"));
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_log_null_source_for_unrooted_control() {
        let target = Decorator::new();
        let binding = Binding::with_path("Foo");
        binding.set_source(None);

        let _log = assert_log(&target, &binding.path(), "Binding Source is null.", Some("(source)"));
        target.bind_binding(Control::tag_property(), &binding);
    }
}

mod logical_ancestor {
    use super::*;

    #[test]
    fn should_log_ancestor_not_found() {
        let target = Decorator::new();
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("$parent[TextBlock]");
        binding.set_type_resolver(Some(resolve_type()));

        let _log = assert_log(&target, &binding.path(), "Ancestor not found.", Some("$parent[TextBlock]"));
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_not_log_ancestor_not_found_for_unrooted_control() {
        let target = Decorator::new();
        let binding = Binding::with_path("$parent[TextBlock]");
        binding.set_type_resolver(Some(resolve_type()));

        let _log = assert_no_log();
        target.bind_binding(Control::tag_property(), &binding);
    }
}

mod visual_ancestor {
    use super::*;

    fn find_ancestor(ancestor_type: &'static TypeInfo) -> Rc<RelativeSource> {
        let relative_source = RelativeSource::new(RelativeSourceMode::FindAncestor);
        relative_source.set_ancestor_type(Some(ancestor_type));
        relative_source
    }

    #[test]
    fn should_log_ancestor_not_found() {
        let target = Decorator::new();
        let _root = TestRoot::with_child(&target);
        let binding = Binding::new();
        binding.set_relative_source(Some(find_ancestor(TextBlock::TYPE)));

        let _log =
            assert_log(&target, "$visualParent[TextBlock]", "Ancestor not found.", Some("$visualParent[TextBlock]"));
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_log_ancestor_property_not_found() {
        let target = Decorator::new();
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("Foo");
        binding.set_relative_source(Some(find_ancestor(TestRoot::TYPE)));

        let _log = assert_log(
            &target,
            "$visualParent[TestRoot].Foo",
            "Could not find a matching property accessor for 'Foo' on 'TestRoot'.",
            Some("Foo"),
        );
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_not_log_ancestor_not_found_for_unrooted_control() {
        let target = Decorator::new();
        let binding = Binding::new();
        binding.set_relative_source(Some(find_ancestor(TestRoot::TYPE)));

        let _log = assert_no_log();
        target.bind_binding(Control::tag_property(), &binding);
    }
}

mod named_element {
    use super::*;

    #[test]
    fn should_log_name_scope_not_found() {
        let target = Decorator::new();
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("#source");
        binding.set_type_resolver(Some(resolve_type()));

        let _log = assert_log(&target, &binding.path(), "NameScope not found.", Some("#source"));
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_not_log_element_property_null_for_unrooted_control() {
        let ns = NameScopeRef::new(NameScope::new());
        let source = Canvas::new();
        source.set_name(Some(s("source")));
        let target = Decorator::new();
        let binding = Binding::with_path("#source.DataContext.Foo");
        binding.set_type_resolver(Some(resolve_type()));
        binding.set_name_scope(Some(Rc::downgrade(&ns.0)));

        let container = StackPanel::new();
        NameScope::set_name_scope(&container, Some(ns.clone()));
        container.add(&source);
        container.add(&target);

        ns.register("source", source.clone().upcast());

        {
            let _log = assert_no_log();
            target.bind_binding(Control::tag_property(), &binding);
        }

        // Sanity check that the binding works when rooted: make sure that
        // we're not just testing a broken binding!
        {
            let _log = assert_no_log();
            let root = TestRoot::with_child(&container);
            root.set_data_context(Some(Model::new_model(FooModel { foo: s("foo") })));
            assert_eq!(as_string(&target.tag()).as_deref(), Some("foo"));
        }
    }
}

mod converter {
    use super::*;

    #[test]
    fn should_log_error_for_unconvertible_type() {
        let target = Decorator::new();
        target.set_data_context(Some(VersionModel::new()));
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("Foo");

        let _log = assert_log_with(
            &target,
            &binding.path(),
            &format!("Could not convert '0.0' ({}) to '{}'.", type_name::<Version>(), type_name::<Thickness>()),
            None,
            LogEventLevel::Warning,
            Some(Layoutable::margin_property().as_property()),
        );
        target.bind_binding(Layoutable::margin_property(), &binding);
    }

    #[test]
    fn should_log_error_for_unconvertible_type_with_converter() {
        let target = Decorator::new();
        target.set_data_context(Some(VersionModel::new()));
        let _root = TestRoot::with_child(&target);
        let binding = Binding::with_path("Foo");
        binding.set_converter(Some(Rc::new(ThrowingConverter)));

        let _log = assert_log_with(
            &target,
            &binding.path(),
            &format!(
                "Could not convert '0.0' ({}) to '{}' using the value converter: \
                 The method or operation is not implemented.",
                type_name::<Version>(),
                type_name::<Thickness>()
            ),
            None,
            LogEventLevel::Warning,
            Some(Layoutable::margin_property().as_property()),
        );
        target.bind_binding(Layoutable::margin_property(), &binding);
    }
}

mod fallback {
    use super::*;

    fn should_log_invalid_fallback_value(rooted: bool) {
        let target = Decorator::new();
        let binding = Binding::with_path("foo");
        binding.set_fallback_value(bs("bar"));

        let _root = rooted.then(|| TestRoot::with_child(&target));

        // An invalid fallback value is invalid whether the control is rooted
        // or not.
        let _log = assert_log_with(
            &target,
            &binding.path(),
            &format!("Could not convert FallbackValue 'bar' to '{}'.", type_name::<f64>()),
            None,
            LogEventLevel::Error,
            Some(Visual::opacity_property().as_property()),
        );
        target.bind_binding(Visual::opacity_property(), &binding);
    }

    #[test]
    fn should_log_invalid_fallback_value_rooted() {
        should_log_invalid_fallback_value(true);
    }

    #[test]
    fn should_log_invalid_fallback_value_unrooted() {
        should_log_invalid_fallback_value(false);
    }

    fn should_log_invalid_target_null_value(rooted: bool) {
        let target = Decorator::new();
        target.set_data_context(Some(Model::new_model(BarModel { bar: None })));
        let binding = Binding::with_path("Bar");
        binding.set_target_null_value(bs("foo"));

        let _root = rooted.then(|| TestRoot::with_child(&target));

        // An invalid target null value is invalid whether the control is
        // rooted or not.
        let _log = assert_log_with(
            &target,
            &binding.path(),
            &format!("Could not convert TargetNullValue 'foo' to '{}'.", type_name::<f64>()),
            None,
            LogEventLevel::Error,
            Some(Visual::opacity_property().as_property()),
        );
        target.bind_binding(Visual::opacity_property(), &binding);
    }

    #[test]
    fn should_log_invalid_target_null_value_rooted() {
        should_log_invalid_target_null_value(true);
    }

    #[test]
    fn should_log_invalid_target_null_value_unrooted() {
        should_log_invalid_target_null_value(false);
    }
}

mod non_control_data_context {
    use super::*;

    fn add_key_binding(target: &Ref<TestRoot>, binding: &Binding) {
        let key_binding = KeyBinding::new();
        key_binding.set_gesture(Some(KeyGesture::new(Key::A, KeyModifiers::NONE)));
        key_binding.bind_binding(KeyBinding::command_property(), binding);
        target.key_bindings().add(key_binding);
    }

    #[test]
    fn should_not_log_missing_member_on_null_data_context() {
        let target = TestRoot::new();
        let binding = Binding::with_path("Foo");
        binding.set_default_anchor(Some(target.clone().upcast::<FerroObject>().downgrade()));

        add_key_binding(&target, &binding);

        let _log = assert_no_log();
        target.bind_binding(Control::tag_property(), &binding);
    }

    #[test]
    fn should_log_missing_member_on_data_context() {
        let target = TestRoot::new();
        let binding = Binding::with_path("Foo");
        binding.set_default_anchor(Some(target.clone().upcast::<FerroObject>().downgrade()));

        add_key_binding(&target, &binding);

        target.set_data_context(Some(Rc::new(PlainObject)));

        let _log = assert_log(
            &target,
            &binding.path(),
            &format!("Could not find a matching property accessor for 'Foo' on '{}'.", type_name::<PlainObject>()),
            Some("Foo"),
        );
        target.bind_binding(Control::tag_property(), &binding);
    }
}

mod compiled_binding {
    use super::*;

    #[test]
    fn should_log_for_invalid_data_context_type() {
        let target = TestRoot::new();
        target.set_data_context(Some(boxed(48)));
        let string_length_property =
            Rc::new(ClrPropertyInfo::read_only::<String, Value<i32>>("Length", |x| x.len() as i32));
        let binding_path = CompiledBindingPathBuilder::new()
            .property(string_length_property, PropertyInfoAccessorFactory::create_plain_property_accessor())
            .build();
        let binding = CompiledBinding::new(binding_path.clone());

        let _log = assert_log(
            &target,
            &binding_path.to_string(),
            &format!(
                "Unable to cast object of type '{}' to type '{}'.",
                type_name::<i32>(),
                type_name::<String>()
            ),
            Some("Length"),
        );
        target.bind_binding(Control::tag_property(), &binding);
    }
}
