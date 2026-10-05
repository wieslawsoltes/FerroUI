//! Helpers of the binding and markup-extension suites: untyped values of
//! assertions, the stand-ins of anonymous objects, a behavior subject and a
//! log sink that records what bindings report.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::logging::{ILogSink, LogEventLevel, Logger};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, PropertyValue, Ref, Upcast};

/// A value as an untyped (`object`) value.
pub fn obj<T: PropertyValue>(value: T) -> Option<BoxedValue> {
    Some(Rc::new(value))
}

/// `Assert.Same(expected, value)` for an untyped value: whether it holds the object `expected`
/// (through whichever handle of the class hierarchy).
pub fn is_same<T: ObjectType + Upcast<FerroObject>>(value: &Option<BoxedValue>, expected: &Ref<T>) -> bool {
    let Some(boxed) = value.as_ref() else { return false };
    ValueTypes::as_object(&**boxed).is_some_and(|object| object == expected.clone().upcast::<FerroObject>())
}

/// Declares the stand-in of an anonymous object (`new { Foo = "foo" }`): a
/// shared object with read-only properties, published to markup.
///
/// ```ignore
/// anonymous_object!(AnonymousFoo { Foo: Option<BoxedValue> => foo });
/// ```
macro_rules! anonymous_object {
    ($name:ident { $($property:ident : $type_:ty => $field:ident),* $(,)? }) => {
        pub struct $name {
            $(pub $field: $type_,)*
        }

        impl ::std::cmp::PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                ::std::ptr::eq(self, other)
            }
        }

        ::ferroui_base::ferro_markup_type!(class $name {
            this: ::std::rc::Rc<$name>,
            handles: [$name, ::std::rc::Rc<$name>, Option<::std::rc::Rc<$name>>],
            properties: [
                $($property: $type_ { get: |this: &::std::rc::Rc<$name>| this.$field.clone() },)*
            ],
        });
    };
}
pub(crate) use anonymous_object;

/// A subject that replays its latest value to new subscribers.
pub struct BehaviorSubject<T> {
    value: RefCell<T>,
    subject: LightweightSubject<T>,
}

impl<T: Clone + 'static> BehaviorSubject<T> {
    pub fn new(value: T) -> Rc<Self> {
        Rc::new(Self { value: RefCell::new(value), subject: LightweightSubject::new() })
    }

    /// The latest value.
    pub fn value(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn on_next(&self, value: T) {
        self.value.replace(value.clone());
        IObserver::on_next(&self.subject, value);
    }
}

impl<T: Clone + 'static> IObservable<T> for BehaviorSubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        observer.on_next(self.value());
        self.subject.subscribe(observer)
    }
}

/// A log sink that hands every event logged on the calling thread to a
/// callback while it is installed; dropping it restores the previous sink.
pub struct TestLogSink {
    callback: Box<dyn Fn(LogEventLevel, &str, &str, &[String])>,
}

/// The installed [`TestLogSink`].
pub struct TestLogSinkScope {
    previous: Option<Option<Rc<dyn ILogSink>>>,
}

impl TestLogSink {
    /// Installs a sink that calls `callback` with the level, the area, the
    /// message template and the formatted property values of each event.
    pub fn start(callback: impl Fn(LogEventLevel, &str, &str, &[String]) + 'static) -> TestLogSinkScope {
        let sink: Rc<dyn ILogSink> = Rc::new(Self { callback: Box::new(callback) });
        TestLogSinkScope { previous: Some(Logger::set_thread_sink(Some(sink))) }
    }
}

impl Drop for TestLogSinkScope {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.take() {
            Logger::set_thread_sink(previous);
        }
    }
}

impl ILogSink for TestLogSink {
    fn is_enabled(&self, _level: LogEventLevel, _area: &str) -> bool {
        true
    }

    fn log(&self, level: LogEventLevel, area: &str, source: Option<&dyn std::any::Any>, message_template: &str) {
        self.log_with_values(level, area, source, message_template, &[]);
    }

    fn log_with_values(
        &self,
        level: LogEventLevel,
        area: &str,
        _source: Option<&dyn std::any::Any>,
        message_template: &str,
        property_values: &[&dyn std::fmt::Display],
    ) {
        let values: Vec<String> = property_values.iter().map(|value| value.to_string()).collect();
        (self.callback)(level, area, message_template, &values);
    }
}

/// The test files of the binding and markup-extension suites that declare
/// test types.
const MODULES: &[&crate::support::TypeModule] = &[
    // FerroUI.Markup.Xaml.UnitTests.Xaml
    &crate::xaml::assign_binding_tests::MODULE,
    &crate::xaml::binding_tests::MODULE,
    &crate::xaml::binding_tests_relative_source::MODULE,
    &crate::xaml::control_binding_tests::MODULE,
    // FerroUI.Markup.Xaml.UnitTests.Data
    &crate::data::binding_tests::MODULE,
    &crate::data::binding_tests_method::MODULE,
    &crate::data::binding_tests_templated_parent::MODULE,
    // FerroUI.Markup.Xaml.UnitTests.MarkupExtensions
    &crate::markup_extensions::test_value_converter::MODULE,
    &crate::markup_extensions::binding_extension_tests::MODULE,
    &crate::markup_extensions::compiled_binding_extension_tests::MODULE,
    &crate::markup_extensions::dynamic_resource_extension_tests::MODULE,
    &crate::markup_extensions::on_form_factor_extension_tests::MODULE,
    &crate::markup_extensions::options_markup_extension_tests::MODULE,
    &crate::markup_extensions::resource_include_tests::MODULE,
    &crate::markup_extensions::static_resource_extension_tests::MODULE,
];

/// The dotted namespaces of the modules of the binding and
/// markup-extension suites.
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_markup_xaml_tests::data", "FerroUI.Markup.Xaml.UnitTests.Data"),
    ("ferroui_markup_xaml_tests::markup_extensions", "FerroUI.Markup.Xaml.UnitTests.MarkupExtensions"),
];

/// Registers the namespaces, the types and the value knowledge of the test
/// types the binding and markup-extension suites declare. Called once by
/// `register_types()` of the crate.
pub(crate) fn register() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_base::metadata::MarkupType;
    use ferroui_base::TypeInfo;

    TypeInfo::register_namespaces(NAMESPACES);
    for module in MODULES {
        TypeInfo::register_all(module.types);
        MarkupType::register_all(module.markup_types);
    }
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    for module in MODULES {
        (module.value_types)();
    }
}
