//! Ported from the upstream `BindingTests_Converters`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::converters::{IValueConverter, StringConverters};
use ferroui_base::data::core::{BindingExpression, Value, ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingBase, BindingError, BindingMode};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::utilities::{CultureInfo, Uri, UriKind, UriRef};
use ferroui_base::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

struct Class1 {
    foo: RefCell<String>,
}

impl Class1 {
    fn new() -> Rc<Self> {
        Model::new_model(Self { foo: RefCell::new(s("foo")) })
    }
}

ferro_model!(Class1, |b| b.property::<Value<String>>("Foo", |o| o.foo.borrow().clone(), |o, v| {
    o.foo.replace(v);
}));

#[test]
fn converter_should_be_used() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = Binding::with_path("Foo");
    target.set_converter(Some(StringConverters::is_null_or_empty()));

    let expression = target.create_instance(&text_block, Some(TextBlock::text_property().as_property()), None);
    let expression = expression.as_any().downcast_ref::<BindingExpression>().expect("a binding expression");

    assert!(Rc::ptr_eq(&StringConverters::is_null_or_empty(), expression.converter().expect("a converter")));
}

#[test]
fn string_format_should_be_applied() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = Binding::with_path("Foo");
    target.set_string_format(Some(s("Hello {0}")));

    text_block.bind_binding(TextBlock::text_property(), &target);

    assert_eq!(text_block.text().as_deref(), Some("Hello foo"));
}

#[test]
fn string_format_should_be_applied_after_converter() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let target = Binding::with_path("Foo");
    target.set_converter(Some(StringConverters::is_not_null_or_empty()));
    target.set_string_format(Some(s("Hello {0}")));

    text_block.bind_binding(TextBlock::text_property(), &target);

    assert_eq!(text_block.text().as_deref(), Some("Hello True"));
}

/// One call of a converter: the method, the text of the value, the target type, whether
/// there was a parameter, and the culture.
type ConverterCall = (&'static str, Option<String>, ValueType, bool, CultureInfo);

/// The mock of the upstream tests: records its calls and returns null.
#[derive(Default)]
struct RecordingConverter {
    calls: RefCell<Vec<ConverterCall>>,
}

impl RecordingConverter {
    fn record(
        &self,
        method: &'static str,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) {
        let text = value.and_then(|value| value.downcast_ref::<String>().cloned());
        self.calls.borrow_mut().push((method, text, target_type, parameter.is_some(), culture.clone()));
    }

    fn calls_of(&self, method: &'static str) -> Vec<ConverterCall> {
        self.calls.borrow().iter().filter(|call| call.0 == method).cloned().collect()
    }
}

impl IValueConverter for RecordingConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.record("Convert", value, target_type, parameter, culture);
        Ok(None)
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.record("ConvertBack", value, target_type, parameter, culture);
        Ok(None)
    }
}

#[test]
fn converter_culture_should_be_passed_to_converter_convert() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let culture = CultureInfo::get_culture_info("ar-SA");
    let converter = Rc::new(RecordingConverter::default());
    let target = Binding::with_path("Foo");
    target.set_converter(Some(converter.clone()));
    target.set_converter_culture(Some(culture.clone()));

    text_block.bind_binding(TextBlock::text_property(), &target);

    let expected: ConverterCall = ("Convert", Some(s("foo")), ValueType::of::<Option<String>>(), false, culture);
    assert_eq!(converter.calls_of("Convert"), [expected]);
}

#[test]
fn converter_culture_should_be_passed_to_converter_convert_back() {
    let text_block = TextBlock::new();
    text_block.set_data_context(Some(Class1::new()));

    let culture = CultureInfo::get_culture_info("ar-SA");
    let converter = Rc::new(RecordingConverter::default());
    let target = Binding::with_path("Foo");
    target.set_converter(Some(converter.clone()));
    target.set_converter_culture(Some(culture.clone()));
    target.set_mode(BindingMode::TwoWay);

    text_block.bind_binding(TextBlock::text_property(), &target);
    text_block.set_text(Some("bar"));

    let expected: ConverterCall = ("ConvertBack", Some(s("bar")), ValueType::of::<String>(), false, culture);
    assert_eq!(converter.calls_of("ConvertBack"), [expected]);
}

// --- the two Rust forms of a time span ------------------------------------------------
//
// Not from upstream (C5 of #423, docs/idiomatic-rust-signatures.md): `SignedDuration` and
// `Duration` are both `System.TimeSpan`, and a binding converts between them with the casts
// of the untyped value conversions. A negative time span is not a duration: its conversion
// fails like any conversion a binding cannot make (decision O2), never clamped to zero.

control_class!(
    /// A control with a property of each Rust form of a time span.
    DurationControl: Control [interval_property, span_property]
);

impl DurationControl {
    ferro_property!(pub fn interval_property() -> StyledProperty<Duration> {
        FerroProperty::register::<DurationControl, _>("Interval", Duration::from_millis(500))
    });

    ferro_property!(pub fn span_property() -> StyledProperty<SignedDuration> {
        FerroProperty::register::<DurationControl, _>("Span", SignedDuration::ZERO)
    });

    fn interval(&self) -> Duration {
        self.get_value(Self::interval_property())
    }

    fn span(&self) -> SignedDuration {
        self.get_value(Self::span_property())
    }
}

/// A view model with a time span and a duration.
struct Timing {
    span: Cell<SignedDuration>,
    interval: Cell<Duration>,
    property_changed: Event<str>,
}

impl Timing {
    fn new(span: SignedDuration, interval: Duration) -> Rc<Self> {
        Model::new_model(Self { span: Cell::new(span), interval: Cell::new(interval), property_changed: Event::new() })
    }

    fn set_span(&self, value: SignedDuration) {
        self.span.set(value);
        self.property_changed.raise("Span");
    }

    fn set_interval(&self, value: Duration) {
        self.interval.set(value);
        self.property_changed.raise("Interval");
    }
}

impl INotifyPropertyChanged for Timing {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Timing, |b| b
    .notify_property_changed()
    .property::<Value<SignedDuration>>("Span", |o| o.span.get(), |o, v| o.set_span(v))
    .property::<Value<Duration>>("Interval", |o| o.interval.get(), |o, v| o.set_interval(v)));

#[test]
fn a_binding_casts_between_signed_duration_and_duration() {
    let source = Timing::new(SignedDuration::from_seconds(1.5), Duration::from_millis(250));
    let target = DurationControl::new();
    target.set_data_context(Some(source.clone()));

    // A time span is the duration of the same value.
    target.bind_binding(DurationControl::interval_property(), &Binding::with_path("Span"));
    assert_eq!(target.interval(), Duration::from_millis(1500));
    source.set_span(SignedDuration::from_seconds(2.0));
    assert_eq!(target.interval(), Duration::from_secs(2));

    // A duration is the time span of the same value, and the binding writes a time span
    // back as a duration.
    target.bind_binding(DurationControl::span_property(), &Binding::with_path_and_mode("Interval", BindingMode::TwoWay));
    assert_eq!(target.span(), SignedDuration::from_seconds(0.25));
    source.set_interval(Duration::from_secs(3));
    assert_eq!(target.span(), SignedDuration::from_seconds(3.0));
    target.set_value(DurationControl::span_property(), SignedDuration::from_seconds(4.0));
    assert_eq!(source.interval.get(), Duration::from_secs(4));
}

#[test]
fn a_negative_span_from_a_binding_is_a_conversion_error_and_keeps_the_value() {
    let source = Timing::new(SignedDuration::from_seconds(-1.0), Duration::ZERO);
    let target = DurationControl::new();
    let _root = TestRoot::with_child(&target);
    target.set_data_context(Some(source.clone()));

    // The binding reports the conversion error, and the property keeps its value: not the
    // negative span, not zero.
    let sink = TestLogSink::start(LogEventLevel::Warning);
    target.bind_binding(DurationControl::interval_property(), &Binding::with_path("Span"));
    let logs = sink.stop();
    assert_eq!(target.interval(), Duration::from_millis(500));
    assert_eq!(logs.len(), 1, "{logs:?}");
    assert!(
        logs[0].property_values.iter().any(|value| value.starts_with("Could not convert '-00:00:01'")),
        "{logs:?}"
    );

    // A span that is not negative is set; a negative one again is the unset value of a
    // binding that fails, as for any value it cannot convert: the property returns to the
    // value it has without the binding.
    source.set_span(SignedDuration::from_seconds(1.0));
    assert_eq!(target.interval(), Duration::from_secs(1));
    let sink = TestLogSink::start(LogEventLevel::Warning);
    source.set_span(SignedDuration::from_seconds(-2.0));
    assert!(!sink.stop().is_empty());
    assert_eq!(target.interval(), Duration::from_millis(500));

    // The fallback rules of the binding apply: its fallback value stands in for the value
    // it cannot convert.
    let target = DurationControl::new();
    let _root = TestRoot::with_child(&target);
    target.set_data_context(Some(source.clone()));
    let binding = Binding::with_path("Span");
    binding.set_fallback_value(Some(Rc::new(Duration::from_secs(9))));
    target.bind_binding(DurationControl::interval_property(), &binding);
    assert_eq!(target.interval(), Duration::from_secs(9));
}

// --- the Rust forms of a URI -------------------------------------------------------------
//
// Not from upstream (C5 of #423, section 3.5 of docs/idiomatic-rust-signatures.md): `Uri`,
// `UriRef` and `url::Url` are all `System.Uri`, and a binding converts between them with the
// casts of the untyped value conversions. A relative reference is not a URL: its conversion
// fails like any conversion a binding cannot make.

control_class!(
    /// A control with a property of each Rust form of a URI.
    UriControl: Control [link_property, source_property, address_property]
);

impl UriControl {
    ferro_property!(pub fn link_property() -> StyledProperty<Option<Uri>> {
        FerroProperty::register::<UriControl, _>("Link", None)
    });

    ferro_property!(pub fn source_property() -> StyledProperty<Option<UriRef>> {
        FerroProperty::register::<UriControl, _>("Source", None)
    });

    ferro_property!(pub fn address_property() -> StyledProperty<Option<url::Url>> {
        FerroProperty::register::<UriControl, _>("Address", None)
    });

    fn link(&self) -> Option<Uri> {
        self.get_value(Self::link_property())
    }

    fn source(&self) -> Option<UriRef> {
        self.get_value(Self::source_property())
    }

    fn address(&self) -> Option<url::Url> {
        self.get_value(Self::address_property())
    }
}

/// A view model with a URI of each Rust form.
struct Links {
    uri: RefCell<Uri>,
    reference: RefCell<UriRef>,
    url: RefCell<url::Url>,
    property_changed: Event<str>,
}

impl Links {
    fn new(uri: &str, reference: &str, url: &str) -> Rc<Self> {
        Model::new_model(Self {
            uri: RefCell::new(Uri::new(uri, UriKind::RelativeOrAbsolute).unwrap()),
            reference: RefCell::new(UriRef::parse(reference).unwrap()),
            url: RefCell::new(url::Url::parse(url).unwrap()),
            property_changed: Event::new(),
        })
    }

    fn set_uri(&self, value: Uri) {
        *self.uri.borrow_mut() = value;
        self.property_changed.raise("Uri");
    }

    fn set_reference(&self, value: UriRef) {
        *self.reference.borrow_mut() = value;
        self.property_changed.raise("Reference");
    }

    fn set_url(&self, value: url::Url) {
        *self.url.borrow_mut() = value;
        self.property_changed.raise("Url");
    }
}

impl INotifyPropertyChanged for Links {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Links, |b| b
    .notify_property_changed()
    .property::<Value<Uri>>("Uri", |o| o.uri.borrow().clone(), |o, v| o.set_uri(v))
    .property::<Value<UriRef>>("Reference", |o| o.reference.borrow().clone(), |o, v| o.set_reference(v))
    .property::<Value<url::Url>>("Url", |o| o.url.borrow().clone(), |o, v| o.set_url(v)));

#[test]
fn a_binding_casts_between_uri_uri_ref_and_url() {
    // The nullable form of the port, which the markup types of the base crate register.
    ValueTypes::register_nullable::<Uri>();
    let source = Links::new("https://ferroui.net/a", "Values.xaml", "fonts:SystemFonts");
    let control = || {
        let target = UriControl::new();
        target.set_data_context(Some(source.clone()));
        target
    };

    // A `Uri` is the reference of the same text, and an absolute one its URL.
    let target = control();
    target.bind_binding(UriControl::source_property(), &Binding::with_path("Uri"));
    assert_eq!(target.source(), UriRef::parse("https://ferroui.net/a").ok());
    let addressed = control();
    addressed.bind_binding(UriControl::address_property(), &Binding::with_path("Uri"));
    assert_eq!(addressed.address(), url::Url::parse("https://ferroui.net/a").ok());
    source.set_uri(Uri::new("/Styles/Base.xaml", UriKind::Relative).unwrap());
    assert_eq!(target.source(), Some(UriRef::Relative("/Styles/Base.xaml".into())));

    // A URL is the `Uri` and the reference of its text.
    let target = control();
    target.bind_binding(UriControl::link_property(), &Binding::with_path("Url"));
    assert_eq!(target.link(), Uri::absolute("fonts:SystemFonts").ok());
    let target = control();
    target.bind_binding(UriControl::source_property(), &Binding::with_path("Url"));
    assert_eq!(target.source(), UriRef::parse("fonts:SystemFonts").ok());

    // A reference is the `Uri` of its text, and the binding writes a `Uri` back as the
    // reference of its text.
    let target = control();
    target.bind_binding(UriControl::link_property(), &Binding::with_path_and_mode("Reference", BindingMode::TwoWay));
    assert_eq!(target.link(), Uri::new("Values.xaml", UriKind::RelativeOrAbsolute).ok());
    target.set_value(UriControl::link_property(), Uri::absolute("ferres://app/a.xaml").ok());
    assert_eq!(*source.reference.borrow(), UriRef::parse("ferres://app/a.xaml").unwrap());

    // An absolute reference is its URL, and the binding writes a URL back as a reference.
    let target = control();
    target.bind_binding(UriControl::address_property(), &Binding::with_path_and_mode("Reference", BindingMode::TwoWay));
    assert_eq!(target.address(), url::Url::parse("ferres://app/a.xaml").ok());
    target.set_value(UriControl::address_property(), url::Url::parse("https://ferroui.net/b").ok());
    assert_eq!(*source.reference.borrow(), UriRef::parse("https://ferroui.net/b").unwrap());
}

#[test]
fn a_relative_reference_from_a_binding_is_a_conversion_error_for_a_url() {
    let source = Links::new("https://ferroui.net/", "Values.xaml", "https://ferroui.net/");
    let target = UriControl::new();
    let _root = TestRoot::with_child(&target);
    target.set_data_context(Some(source.clone()));

    // The binding reports the conversion error, and the property keeps the value it has
    // without the binding.
    let sink = TestLogSink::start(LogEventLevel::Warning);
    target.bind_binding(UriControl::address_property(), &Binding::with_path("Reference"));
    let logs = sink.stop();
    assert_eq!(target.address(), None);
    assert_eq!(logs.len(), 1, "{logs:?}");
    assert!(
        logs[0].property_values.iter().any(|value| value.starts_with("Could not convert 'Values.xaml'")),
        "{logs:?}"
    );

    // An absolute reference is set; a relative one again returns the property to the value it
    // has without the binding.
    source.set_reference(UriRef::parse("fonts:SystemFonts").unwrap());
    assert_eq!(target.address(), url::Url::parse("fonts:SystemFonts").ok());
    let sink = TestLogSink::start(LogEventLevel::Warning);
    source.set_reference(UriRef::Relative("/Styles/Base.xaml".into()));
    assert!(!sink.stop().is_empty());
    assert_eq!(target.address(), None);
}
