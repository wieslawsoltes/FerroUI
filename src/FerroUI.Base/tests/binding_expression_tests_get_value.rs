//! Port of the upstream `BindingExpressionTests.GetValue` tests.

use super::binding_test_support::*;
use super::*;
use crate::data::core::{BindingExpression, BindingExpressionOptions, TargetTypeConverter, UntypedBindingExpression};
use crate::data::{BindingExpressionBase, BindingMode};
use crate::FerroProperty;

binding_tests! {
    fn should_get_source_value(f) {
        let data = boxed(s("foo"));
        let target = create_target_with_source(f, Some(data), &Path::empty(Out::String), Opts::default());

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn should_convert_string_to_double(f) {
        let data = ViewModel::with_string("5.6");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            Opts::property(TargetClass::double_property()),
        );

        assert_eq!(target.double(), 5.6);
    }

    fn should_convert_double_to_string(f) {
        let data = ViewModel::with_double(5.6);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            Opts::property(TargetClass::string_property()),
        );

        assert_eq!(target.string(), Some(s("5.6")));
    }

    fn should_use_fallback_value_for_non_convertible_target_value(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            Opts::property(TargetClass::int_property()).with_fallback_value(boxed(42)),
        );

        assert_eq!(target.int(), 42);
    }

    fn should_pass_converter_parameter_to_converter(f) {
        let data = ViewModel::with_double(5.6);
        let converter = PrefixConverter::new(None);
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::double_step()),
            Opts::property(TargetClass::string_property())
                .with_converter(converter)
                .with_converter_parameter("foo"),
        );

        assert_eq!(target.string(), Some(s("foo5.6")));
    }

    fn target_null_value_should_be_used_when_source_string_is_null(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            Opts::default().with_target_null_value(boxed(s("bar"))),
        );

        assert_eq!(target.string(), Some(s("foo")));

        data.set_string_value(None);

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn target_null_value_should_be_used_when_source_is_data_context_and_null(f) {
        let target =
            create_target(f, &Path::empty(Out::String), Opts::default().with_target_null_value(boxed(s("bar"))));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn can_use_update_target_to_update_from_non_inpc_data(f) {
        let data = PodViewModel::new(Some("foo"));
        let (target, expression) = create_target_and_expression(
            f,
            &Path::of(PodViewModel::string_step()),
            Opts::default().with_source(src(&data)),
        );

        assert_eq!(target.string(), Some(s("foo")));

        data.set_string_value(Some(s("bar")));
        assert_eq!(target.string(), Some(s("foo")));

        expression.update_target();
        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_use_converter_for_relative_source_self_binding_with_no_path(f) {
        let converter = PrefixConverter::new(None);
        let target = create_target(
            f,
            &Path::empty(Out::Object),
            Opts {
                relative_source_self: true,
                ..Opts::property(TargetClass::string_property())
                    .with_converter(converter)
                    .with_converter_parameter("foo")
            },
        );

        assert_eq!(target.string(), Some(s("fooTargetClass")));
    }

    fn should_not_pass_unset_value_to_converter_until_first_value_produced(f) {
        let data = ViewModel::with_string("Bar");
        let converter = PrefixConverter::new(None);
        let target = create_target(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::default().with_converter(converter).with_converter_parameter("foo"),
        );

        assert_eq!(target.string(), None);

        target.set_data_context(src(&data));

        assert_eq!(target.string(), Some(s("fooBar")));
    }

    fn should_use_converter_for_null_data_context_without_path(f) {
        let converter = PrefixConverter::new(None);
        let target = create_target(
            f,
            &Path::empty(Out::String),
            Opts::default().with_converter(converter).with_converter_parameter("foo"),
        );

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn leaf_node_should_be_null_when_nodes_list_is_empty(f) {
        let _ = f;
        // Reproduces issue #20441: a binding expression with no nodes (a
        // binding with a source and a converter but no path).
        let binding_expression = BindingExpression::new(
            Some(boxed(s("Elements"))),
            Vec::new(),
            BindingExpressionOptions {
                fallback_value: Some(FerroProperty::unset_value()),
                converter: Some(PrefixConverter::new(Some("Prefix"))),
                mode: BindingMode::OneWay,
                target_property: Some(TargetClass::string_property()),
                target_type_converter: Some(TargetTypeConverter::get_reflection_converter()),
                ..BindingExpressionOptions::default()
            },
        );

        // These should not panic.
        let leaf_node = binding_expression.leaf_node();
        let _description = binding_expression.description();

        // The leaf node is null when there are no nodes.
        assert!(leaf_node.is_none());
    }
}
