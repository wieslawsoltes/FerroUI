//! Port of the upstream `BindingExpressionTests.Mode` tests.

use super::binding_test_support::*;
use super::*;
use crate::data::core::{Untyped, Value};
use crate::data::model::Model;
use crate::data::BindingMode;
use crate::ferro_model;

/// `new { Baz = "baz" }`.
struct BazData {
    baz: String,
}

ferro_model!(BazData, |b| b.read_only::<Value<String>>("Baz", |o| o.baz.clone()));

/// `new { DoubleValue = new object() }`.
struct ObjectDoubleData {
    double_value: Option<BoxedValue>,
}

ferro_model!(ObjectDoubleData, |b| b.read_only::<Untyped>("DoubleValue", |o| o.double_value.clone()));

fn next_string_path() -> Path {
    Path::of(ViewModel::next_step()).then(ViewModel::string_step())
}

binding_tests! {
    fn one_time_binding_sets_target_only_once_if_data_context_does_not_change(f) {
        let data = ViewModel::with_next(ViewModel::with_string("foo"));
        let target = create_target(f, &next_string_path(), Opts::mode(BindingMode::OneTime));
        target.set_data_context(src(&data));

        assert_eq!(target.string(), Some(s("foo")));

        data.next().unwrap().set_string_value(Some(s("bar")));
        assert_eq!(target.string(), Some(s("foo")));

        data.set_next(Some(ViewModel::with_string("baz")));
        assert_eq!(target.string(), Some(s("foo")));
    }

    fn one_time_binding_with_simple_path_sets_target_when_data_context_changes(f) {
        let data1 = ViewModel::with_string("foo");
        let target = create_target(f, &Path::of(ViewModel::string_step()), Opts::mode(BindingMode::OneTime));
        target.set_data_context(src(&data1));

        assert_eq!(target.string(), Some(s("foo")));

        let data2 = ViewModel::with_string("bar");
        target.set_data_context(src(&data2));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn one_time_binding_with_complex_path_sets_target_when_data_context_changes(f) {
        let data1 = ViewModel::with_next(ViewModel::with_string("foo"));
        let target = create_target(f, &next_string_path(), Opts::mode(BindingMode::OneTime));
        target.set_data_context(src(&data1));

        assert_eq!(target.string(), Some(s("foo")));

        let data2 = ViewModel::with_next(ViewModel::with_string("bar"));
        target.set_data_context(src(&data2));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn one_time_binding_without_path_sets_target_when_data_context_changes(f) {
        let target = create_target(f, &Path::empty(Out::String), Opts::mode(BindingMode::OneTime));
        target.set_data_context(Some(boxed(s("foo"))));

        assert_eq!(target.string(), Some(s("foo")));

        target.set_data_context(Some(boxed(s("bar"))));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn one_time_binding_waits_for_data_context(f) {
        let target = create_target(f, &Path::of(ViewModel::string_step()), Opts::mode(BindingMode::OneTime));

        assert_eq!(target.string(), None);
    }

    fn one_time_binding_waits_for_data_context_with_matching_property_name(f) {
        let data1 = Model::new_model(BazData { baz: s("baz") });
        let data2 = ViewModel::with_string("foo");
        let target = create_target(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::OneTime).with_data_context(src(&data1)),
        );

        assert_eq!(target.string(), None);

        target.set_data_context(src(&data2));
        assert_eq!(target.string(), Some(s("foo")));

        data2.set_string_value(Some(s("bar")));
        assert_eq!(target.string(), Some(s("foo")));
    }

    fn one_time_binding_waits_for_data_context_with_matching_property_type(f) {
        let data1 = Model::new_model(ObjectDoubleData { double_value: Some(boxed(())) });
        let data2 = ViewModel::with_double(0.5);
        let target = create_target(
            f,
            &Path::of(ViewModel::double_step()),
            Opts::mode(BindingMode::OneTime).with_data_context(src(&data1)),
        );

        assert_eq!(target.double(), 0.0);

        target.set_data_context(src(&data2));
        assert_eq!(target.double(), 0.5);

        data2.set_double_value(0.2);
        assert_eq!(target.double(), 0.5);
    }

    fn one_time_binding_waits_for_data_context_without_property_path(f) {
        let target = create_target(f, &Path::empty(Out::String), Opts::mode(BindingMode::OneTime));

        target.set_data_context(Some(boxed(s("foo"))));

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn one_time_binding_waits_for_data_context_without_property_path_with_string_format(f) {
        let target = create_target(
            f,
            &Path::empty(Out::String),
            Opts::mode(BindingMode::OneTime).with_string_format("bar: {0}"),
        );

        target.set_data_context(Some(boxed(s("foo"))));

        assert_eq!(target.string(), Some(s("bar: foo")));
    }

    fn one_way_to_source_binding_updates_source_when_target_changes(f) {
        let data = ViewModel::new();
        let target = create_target(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::OneWayToSource).with_data_context(src(&data)),
        );

        assert_eq!(data.string_value(), None);

        target.set_string(Some("foo"));

        assert_eq!(data.string_value(), Some(s("foo")));
    }

    fn one_way_to_source_binding_does_not_update_target_when_source_changes(f) {
        let data = ViewModel::new();
        let target = create_target(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::OneWayToSource).with_data_context(src(&data)),
        );

        target.set_string(Some("foo"));
        assert_eq!(data.string_value(), Some(s("foo")));

        data.set_string_value(Some(s("bar")));
        assert_eq!(target.string(), Some(s("foo")));
    }

    fn one_way_to_source_binding_updates_source_when_data_context_changes(f) {
        let data1 = ViewModel::new();
        let data2 = ViewModel::new();
        let target = create_target(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::OneWayToSource).with_data_context(src(&data1)),
        );

        target.set_string(Some("foo"));
        assert_eq!(data1.string_value(), Some(s("foo")));

        target.set_data_context(src(&data2));
        assert_eq!(data2.string_value(), Some(s("foo")));
    }

    fn can_bind_readonly_property_one_way_to_source(f) {
        let data = ViewModel::new();
        let target = create_target(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::OneWayToSource)
                .with_data_context(src(&data))
                .with_property(TargetClass::read_only_string_property()),
        );

        assert_eq!(data.string_value(), Some(s("readonly")));

        target.set_read_only_string(Some("foo"));

        assert_eq!(data.string_value(), Some(s("foo")));
    }

    fn one_way_binding_updates_target_when_changes_and_source_raises_property_changed(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::OneWay).with_data_context(src(&data)),
        );

        assert_eq!(target.string(), Some(s("foo")));

        target.set_current_value(TargetClass::string_property(), Some(s("bar")));
        assert_eq!(target.string(), Some(s("bar")));

        data.raise_property_changed("StringValue");
        assert_eq!(target.string(), Some(s("foo")));
    }
}
