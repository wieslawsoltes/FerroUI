//! Port of the upstream `BindingExpressionTests` tests of bindings to
//! registered properties.

use super::binding_test_support::*;
use super::*;

fn source_with_string(value: &str) -> crate::Ref<SourceControl> {
    let data = SourceControl::new();
    data.set_string_value(Some(s(value)));
    data
}

binding_tests! {
    fn should_get_simple_ferro_property_value(f) {
        let data = source_with_string("foo");
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &Path::of(SourceControl::string_step()),
            Opts::default(),
        );

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn should_get_simple_clr_property_value(f) {
        let data = SourceControl::new();
        data.set_clr_property(Some(s("foo")));
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &Path::of(SourceControl::clr_step()),
            Opts::default(),
        );

        assert_eq!(target.string(), Some(s("foo")));
    }

    fn should_track_simple_ferro_property_value(f) {
        let data = source_with_string("foo");
        let target = create_target_with_source(
            f,
            Some(boxed(data.clone())),
            &Path::of(SourceControl::string_step()),
            Opts::default(),
        );

        assert_eq!(target.string(), Some(s("foo")));

        data.set_string_value(Some(s("bar")));

        assert_eq!(target.string(), Some(s("bar")));
    }

    fn should_unsubscribe_from_ferro_property_source(f) {
        let data = source_with_string("foo");
        let (_target, expression) = create_target_and_expression(
            f,
            &Path::of(SourceControl::string_step()),
            Opts::property(TargetClass::string_property()).with_source(Some(boxed(data.clone()))),
        );

        assert!(data.property_changed_subscriber_count() > 0);

        expression.dispose();

        assert_eq!(data.property_changed_subscriber_count(), 0);
    }

    fn should_not_keep_ferro_property_source_alive(f) {
        let run = || {
            let source = SourceControl::new();
            let target = create_target_with_source(
                f,
                Some(boxed(source.clone())),
                &Path::of(SourceControl::string_step()),
                Opts::property(TargetClass::string_property()),
            );
            (target, source.downgrade())
        };

        let result = run();

        assert!(result.1.upgrade().is_none());
    }
}
