//! Port of the upstream `BindingExpressionTests.UpdateSourceTrigger` tests.

use super::binding_test_support::*;
use super::*;
use crate::data::{BindingExpressionBase, BindingMode, UpdateSourceTrigger};

binding_tests! {
    fn two_way_property_changed_should_update_source_on_property_changed(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::TwoWay),
        );

        assert_eq!(target.string(), Some(s("foo")));
        assert_eq!(data.string_value(), Some(s("foo")));

        target.set_string(Some("bar"));

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("bar")));
    }

    fn two_way_lost_focus_should_update_source_on_lost_focus(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::TwoWay).with_update_source_trigger(UpdateSourceTrigger::LostFocus),
        );
        let root = TestRoot::new(&target);

        assert!(target.focus());

        assert_eq!(target.string(), Some(s("foo")));
        assert_eq!(data.string_value(), Some(s("foo")));

        target.set_string(Some("bar"));

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("foo")));

        assert!(root.root.focus());

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("bar")));
    }

    fn one_way_to_source_lost_focus_should_update_source_on_lost_focus(f) {
        let data = ViewModel::with_string("foo");
        let target = create_target_with_source(
            f,
            src(&data),
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::OneWayToSource).with_update_source_trigger(UpdateSourceTrigger::LostFocus),
        );
        let root = TestRoot::new(&target);

        assert!(target.focus());

        assert_eq!(target.string(), None);
        assert_eq!(data.string_value(), Some(s("foo")));

        target.set_string(Some("bar"));

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("foo")));

        assert!(root.root.focus());

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("bar")));
    }

    fn two_way_explicit_should_update_source_on_call_to_update_source(f) {
        let data = ViewModel::with_string("foo");
        let (target, expression) = create_target_and_expression(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::TwoWay)
                .with_source(src(&data))
                .with_update_source_trigger(UpdateSourceTrigger::Explicit),
        );
        let root = TestRoot::new(&target);

        assert!(target.focus());

        assert_eq!(target.string(), Some(s("foo")));
        assert_eq!(data.string_value(), Some(s("foo")));

        target.set_string(Some("bar"));

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("foo")));

        assert!(root.root.focus());

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("foo")));

        expression.update_source();

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("bar")));
    }

    fn two_way_explicit_should_update_target_on_call_to_update_target(f) {
        let data = ViewModel::with_string("foo");
        let (target, expression) = create_target_and_expression(
            f,
            &Path::of(ViewModel::string_step()),
            Opts::mode(BindingMode::TwoWay)
                .with_source(src(&data))
                .with_update_source_trigger(UpdateSourceTrigger::Explicit),
        );

        assert_eq!(target.string(), Some(s("foo")));
        assert_eq!(data.string_value(), Some(s("foo")));

        target.set_string(Some("bar"));

        assert_eq!(target.string(), Some(s("bar")));
        assert_eq!(data.string_value(), Some(s("foo")));

        // Updating the target forces a transfer from source to target,
        // discarding the value set on the target.
        expression.update_target();

        assert_eq!(target.string(), Some(s("foo")));
        assert_eq!(data.string_value(), Some(s("foo")));
    }
}
