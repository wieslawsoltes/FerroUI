//! Port of the upstream `BindingExpressionTests.SetValue` tests.
//!
//! Not ported here: `Should_Write_Indexed_Value_To_Source` (indexers are
//! covered by the indexer suite).

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
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(to_text(value).map(|v| boxed(v.to_uppercase())))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(to_text(value).map(|v| boxed(v.to_lowercase())))
    }
}

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
