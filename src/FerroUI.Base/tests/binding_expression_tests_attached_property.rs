//! Port of the upstream `BindingExpressionTests.AttachedProperty` tests.

use super::binding_test_support::*;
use super::*;

fn attached_path() -> Path {
    Path::of(AttachedProperties::attached_string_step())
}

fn chained_attached_path() -> Path {
    Path::of(SourceControl::next_step()).then(AttachedProperties::attached_string_step())
}

fn source_with_attached(value: &str) -> Ref<SourceControl> {
    let data = SourceControl::new();
    data.set_value(AttachedProperties::attached_string_property(), Some(s(value)));
    data
}

fn source_with_next_attached(value: &str) -> Ref<SourceControl> {
    let data = SourceControl::new();
    data.set_next(Some(source_with_attached(value)));
    data
}

use crate::Ref;

binding_tests! {
    fn should_get_attached_property_value(f) {
        let data = source_with_attached("foo");
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &attached_path(),
            Opts::property(TargetClass::string_property()),
        );

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn should_get_chained_attached_property_value(f) {
        let data = source_with_next_attached("foo");
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &chained_attached_path(),
            Opts::property(TargetClass::string_property()),
        );

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn should_track_simple_attached_value(f) {
        let data = source_with_attached("foo");
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &attached_path(),
            Opts::property(TargetClass::string_property()),
        );

        assert_eq!(target.string(), Some(s("foo")));

        data.set_value(AttachedProperties::attached_string_property(), Some(s("bar")));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_track_chained_attached_value(f) {
        let data = source_with_next_attached("foo");
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &chained_attached_path(),
            Opts::property(TargetClass::string_property()),
        );

        assert_eq!(target.string(), Some(s("foo")));

        data.next().unwrap().set_value(AttachedProperties::attached_string_property(), Some(s("bar")));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_unsubscribe_from_attached_property_source(f) {
        let data = source_with_attached("foo");
        let (_target, expression) = create_target_and_expression(
            f,
            &attached_path(),
            Opts::property(TargetClass::string_property()).with_source(Some(boxed(data.clone()))),
        );

        assert!(data.property_changed_subscriber_count() > 0);

        expression.dispose();

        assert_eq!(data.property_changed_subscriber_count(), 0);
    }

    fn should_unsubscribe_from_chained_source(f) {
        let data = source_with_next_attached("foo");
        let (_target, expression) = create_target_and_expression(
            f,
            &chained_attached_path(),
            Opts::property(TargetClass::string_property()).with_source(Some(boxed(data.clone()))),
        );

        assert!(data.next().unwrap().property_changed_subscriber_count() > 0);

        expression.dispose();

        assert_eq!(data.next().unwrap().property_changed_subscriber_count(), 0);
    }

    fn should_not_keep_attached_property_source_alive(f) {
        let run = || {
            let source = SourceControl::new();
            let target = create_target_with_source(
                f,
                Some(boxed(source.clone())),
                &chained_attached_path(),
                Opts::property(TargetClass::string_property()),
            );
            (target, source.downgrade())
        };

        let result = run();

        assert!(result.1.upgrade().is_none());
    }
}
