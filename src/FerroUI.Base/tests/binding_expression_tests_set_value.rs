//! Port of the upstream `BindingExpressionTests.SetValue` tests.
//!
//! `Should_Write_Indexed_Value_To_Source` is in `binding_expression_tests_indexer.rs`, which has the data with an
//! array and its paths.

use crate::utilities::CultureInfo;
use super::binding_test_support::*;
use super::*;
use crate::data::converters::IValueConverter;
use crate::data::core::{Value, ValueType};
use crate::data::model::{Event, INotifyPropertyChanged, Model};
use crate::data::{BindingError, BindingMode};
use crate::{ferro_model, FerroObject};

struct Cat {
    whisker_count: Cell<i32>,
    lives: Cell<i32>,
    property_changed: Event<str>,
}

impl Cat {
    fn new() -> Rc<Self> {
        Model::new_model(Self { whisker_count: Cell::new(4), lives: Cell::new(9), property_changed: Event::new() })
    }

    fn whisker_count(&self) -> i32 {
        self.whisker_count.get()
    }

    fn set_whisker_count(&self, value: i32) {
        self.whisker_count.set(value);
        self.property_changed.raise("WhiskerCount");
        self.lives.set(self.lives.get() - 1);
    }

    fn whisker_count_step() -> Step {
        inpc_prop::<Cat, Value<i32>>("WhiskerCount", Out::Int, |o| o.whisker_count(), |o, v| o.set_whisker_count(v))
    }
}

impl INotifyPropertyChanged for Cat {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

ferro_model!(Cat, |b| b
    .notify_property_changed()
    .property::<Value<i32>>("WhiskerCount", |o| o.whisker_count(), |o, v| o.set_whisker_count(v)));

struct CaseConverter;

impl IValueConverter for CaseConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(to_text(value).map(|v| boxed(v.to_uppercase())))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(to_text(value).map(|v| boxed(v.to_lowercase())))
    }
}

/// Whether the value equals the parameter; not reversible, and it counts the attempts.
#[derive(Default)]
struct EqualsParameterConverter {
    convert_back_count: Cell<i32>,
    unset_for_null: bool,
}

impl IValueConverter for EqualsParameterConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let text = to_text(value);
        if text.is_none() && self.unset_for_null {
            return Ok(Some(crate::FerroProperty::unset_value()));
        }
        Ok(Some(boxed(text == to_text(parameter))))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.convert_back_count.set(self.convert_back_count.get() + 1);
        let is_true = value.and_then(|v| v.downcast_ref::<bool>()).copied().unwrap_or(false);
        Ok(if is_true { parameter.cloned() } else { None })
    }
}

crate::ferro_property!(fn coerced_int_property() -> crate::StyledProperty<i32> {
    crate::FerroProperty::register_with::<TargetClass, i32>(
        "CoercedInt",
        crate::StyledPropertyOptions::new(0).coerce(|_, value: i32| value.min(10)),
    )
});

fn two_way() -> Opts {
    Opts::mode(BindingMode::TwoWay)
}

binding_tests! {
    fn should_write_value_to_source(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(f, src(&data), &Path::of(ViewModel::string_step()), two_way());

        target.set_string(Some("bar"));

        assert_eq!(data.string_value(), Some(s("bar")));
    }

    fn should_write_value_to_attached_property_on_source(f) {
        let data = FerroObject::new();
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &Path::of(DockPanel::dock_step()),
            two_way().with_property(TargetClass::tag_property()),
        );

        target.set_tag(Some(boxed(Dock::Right)));

        assert_eq!(data.get_value(DockPanel::dock_property()), Dock::Right);
    }

    fn should_write_value_to_source_on_simple_property_chain(f) {
        let data = ViewModel::with_next(ViewModel::with_string("foo"));
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::next_step()).then(ViewModel::string_step()),
            two_way(),
        );

        target.set_string(Some("bar"));

        assert_eq!(data.next().unwrap().string_value(), Some(s("bar")));
    }

    fn target_value_can_be_set_on_broken_chain(f) {
        let data = ViewModel::with_next(ViewModel::with_string("foo"));
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::next_step()).then(ViewModel::string_step()),
            two_way(),
        );

        data.set_next(None);
        target.set_string(Some("bar"));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_use_converter_when_writing_to_source(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            two_way().with_converter(Rc::new(CaseConverter)),
        );

        target.set_string(Some("BaR"));

        assert_eq!(data.string_value(), Some(s("bar")));
    }

    fn two_way_binding_should_not_write_unchanged_value_back_to_property_with_converter(f) {
        let data = Cat::new();
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(Cat::whisker_count_step()),
            two_way().with_converter(Rc::new(CaseConverter)),
        );

        assert_eq!(target.int(), 4);
        assert_eq!(data.lives.get(), 9);

        data.set_whisker_count(3);

        assert_eq!(target.int(), 3);
        assert_eq!(data.lives.get(), 8);
    }

    fn two_way_binding_should_not_write_source_value_back_with_non_reversible_converter(f) {
        let data = ViewModel::with_string("foo");
        let converter = Rc::new(EqualsParameterConverter::default());
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            two_way()
                .with_property(TargetClass::bool_property())
                .with_converter(converter.clone())
                .with_converter_parameter("foo"),
        );
        assert!(target.bool());

        data.set_string_value(Some(s("bar")));

        assert!(!target.bool());
        assert_eq!(data.string_value(), Some(s("bar")));
        assert_eq!(converter.convert_back_count.get(), 0);
    }

    fn two_way_binding_should_not_write_default_value_back_when_converter_returns_unset(f) {
        let data = ViewModel::with_string("foo");
        let converter = Rc::new(EqualsParameterConverter { unset_for_null: true, ..Default::default() });
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            two_way()
                .with_property(TargetClass::bool_property())
                .with_converter(converter.clone())
                .with_converter_parameter("foo"),
        );
        assert!(target.bool());

        data.set_string_value(None);

        assert!(!target.bool());
        assert_eq!(data.string_value(), None);
        assert_eq!(converter.convert_back_count.get(), 0);
    }

    fn two_way_binding_should_write_coerced_value_back_to_source(f) {
        let data = ViewModel::with_int(5);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::int_step()),
            two_way().with_property(coerced_int_property()),
        );

        data.set_int_value(15);

        assert_eq!(target.get_value(coerced_int_property()), 10);
        assert_eq!(data.int_value(), 10);
    }

    fn setter_should_convert_double_to_string(f) {
        let data = ViewModel::with_string("5.6");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            two_way().with_property(TargetClass::double_property()),
        );

        target.set_double(6.7);

        assert_eq!(data.string_value(), Some(s("6.7")));
    }

    fn setter_should_convert_string_to_double(f) {
        let data = ViewModel::with_double(5.6);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            two_way().with_property(TargetClass::string_property()),
        );

        target.set_string(Some("6.7"));

        assert_eq!(data.double_value(), 6.7);
    }

    fn setting_invalid_double_string_should_not_change_target(f) {
        let data = ViewModel::with_double(5.6);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            two_way().with_property(TargetClass::string_property()),
        );

        target.set_string(Some("foo"));

        assert_eq!(data.double_value(), 5.6);
    }

    fn setting_invalid_double_string_should_use_fallback_value(f) {
        let data = ViewModel::with_double(5.6);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            two_way().with_fallback_value(boxed(9.8)).with_property(TargetClass::string_property()),
        );

        target.set_string(Some("foo"));

        assert_eq!(data.double_value(), 9.8);
    }

    fn should_pass_converter_parameter_to_converter_convert_back(f) {
        let data = ViewModel::with_string("Initial");
        let converter = PrefixConverter::new(None);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            two_way().with_converter(converter).with_converter_parameter("foo"),
        );

        target.set_string(Some("fooBar"));

        assert_eq!(data.string_value(), Some(s("Bar")));
    }
}
