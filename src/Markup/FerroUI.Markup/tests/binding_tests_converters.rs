//! Ported from the upstream `BindingTests_Converters`.

use super::test_support::*;
use crate::data::Binding;
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::converters::{IValueConverter, StringConverters};
use ferroui_base::data::core::{BindingExpression, Value, ValueType};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::{BindingBase, BindingError, BindingMode};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::utilities::CultureInfo;
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

// --- the two Rust forms of a culture --------------------------------------------------
//
// Not from upstream (C5 of #423, docs/idiomatic-rust-signatures.md section 3.4): `CultureInfo`
// and `Locale` are both `System.Globalization.CultureInfo`, and a binding converts between
// them with the casts of the untyped value conversions. A culture whose name is no BCP-47 tag
// is not a locale: its conversion fails like any conversion a binding cannot make.

control_class!(
    /// A control with a property of each Rust form of a culture.
    LocaleControl: Control [language_property, culture_property]
);

impl LocaleControl {
    ferro_property!(pub fn language_property() -> StyledProperty<ferroui_base::utilities::Locale> {
        FerroProperty::register::<LocaleControl, _>("Language", ferroui_base::utilities::locale!("en-US"))
    });

    ferro_property!(pub fn culture_property() -> StyledProperty<CultureInfo> {
        FerroProperty::register::<LocaleControl, _>("Culture", CultureInfo::invariant_culture())
    });

    fn language(&self) -> ferroui_base::utilities::Locale {
        self.get_value(Self::language_property())
    }

    fn culture(&self) -> CultureInfo {
        self.get_value(Self::culture_property())
    }
}

/// A view model with a culture and a locale.
struct Cultures {
    culture: RefCell<CultureInfo>,
    language: RefCell<ferroui_base::utilities::Locale>,
    property_changed: Event<str>,
}

impl Cultures {
    fn new(culture: CultureInfo, language: ferroui_base::utilities::Locale) -> Rc<Self> {
        Model::new_model(Self { culture: RefCell::new(culture), language: RefCell::new(language), property_changed: Event::new() })
    }

    fn set_culture(&self, value: CultureInfo) {
        *self.culture.borrow_mut() = value;
        self.property_changed.raise("Culture");
    }

    fn set_language(&self, value: ferroui_base::utilities::Locale) {
        *self.language.borrow_mut() = value;
        self.property_changed.raise("Language");
    }
}

impl INotifyPropertyChanged for Cultures {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Cultures, |b| b
    .notify_property_changed()
    .property::<Value<CultureInfo>>("Culture", |o| o.culture.borrow().clone(), |o, v| o.set_culture(v))
    .property::<Value<ferroui_base::utilities::Locale>>("Language", |o| o.language.borrow().clone(), |o, v| o.set_language(v)));

#[test]
fn a_binding_casts_between_culture_info_and_locale() {
    use ferroui_base::utilities::locale;

    let source = Cultures::new(CultureInfo::get_culture_info("ja-JP"), locale!("de-DE"));
    let target = LocaleControl::new();
    target.set_data_context(Some(source.clone()));

    // A culture is its locale.
    target.bind_binding(LocaleControl::language_property(), &Binding::with_path("Culture"));
    assert_eq!(target.language(), locale!("ja-JP"));
    source.set_culture(CultureInfo::invariant_culture());
    assert_eq!(target.language(), ferroui_base::utilities::Locale::UNKNOWN);

    // A locale is the culture of its name, and the binding writes a culture back as a
    // locale.
    target.bind_binding(LocaleControl::culture_property(), &Binding::with_path_and_mode("Language", BindingMode::TwoWay));
    assert_eq!(target.culture(), CultureInfo::get_culture_info("de-DE"));
    source.set_language(locale!("ko-KR"));
    assert_eq!(target.culture(), CultureInfo::get_culture_info("ko-KR"));
    target.set_value(LocaleControl::culture_property(), CultureInfo::get_culture_info("fr-FR"));
    assert_eq!(*source.language.borrow(), locale!("fr-FR"));
}

#[test]
fn a_culture_that_is_no_locale_from_a_binding_is_a_conversion_error_and_keeps_the_value() {
    let source = Cultures::new(CultureInfo::get_culture_info("es-ES_tradnl"), ferroui_base::utilities::Locale::UNKNOWN);
    let target = LocaleControl::new();
    let _root = TestRoot::with_child(&target);
    target.set_data_context(Some(source.clone()));

    // The binding reports the conversion error, and the property keeps its value.
    let sink = TestLogSink::start(LogEventLevel::Warning);
    target.bind_binding(LocaleControl::language_property(), &Binding::with_path("Culture"));
    let logs = sink.stop();
    assert_eq!(target.language(), ferroui_base::utilities::locale!("en-US"));
    assert_eq!(logs.len(), 1, "{logs:?}");

    // A culture that is a locale is set.
    source.set_culture(CultureInfo::get_culture_info("pl-PL"));
    assert_eq!(target.language(), ferroui_base::utilities::locale!("pl-PL"));
}
