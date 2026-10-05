//! Port of the upstream `BindingExpressionTests.Negation` tests.

use super::binding_test_support::*;
use crate::data::BindingMode;

fn bool_path() -> Path {
    Path::of(ViewModel::bool_step())
}

fn next_bool_path() -> Path {
    Path::of(ViewModel::next_step()).then(ViewModel::bool_step())
}

binding_tests! {
    fn should_negate_boolean_value(f) {
        for value in [true, false] {
            let data = ViewModel::with_bool(value);
            let target = create_target_with_source(f, src(&data), &bool_path().not(), Opts::default());

            assert_eq!(target.bool(), !value);
        }
    }

    fn should_negate_boolean_value_in_path(f) {
        for value in [true, false] {
            let data = ViewModel::with_next(ViewModel::with_bool(value));
            let target = create_target_with_source(f, src(&data), &next_bool_path().not(), Opts::default());

            assert_eq!(target.bool(), !value);
        }
    }

    fn should_double_negate_boolean_value(f) {
        for value in [true, false] {
            let data = ViewModel::with_bool(value);
            let target = create_target_with_source(f, src(&data), &bool_path().not().not(), Opts::default());

            assert_eq!(target.bool(), value);
        }
    }

    fn should_double_negate_boolean_value_in_path(f) {
        for value in [true, false] {
            let data = ViewModel::with_next(ViewModel::with_bool(value));
            let target = create_target_with_source(f, src(&data), &next_bool_path().not().not(), Opts::default());

            assert_eq!(target.bool(), value);
        }
    }

    fn can_set_negated_value(f) {
        let data = ViewModel::with_bool(true);
        let target =
            create_target_with_source(f, src(&data), &bool_path().not(), Opts::mode(BindingMode::TwoWay));

        target.set_bool(true);

        assert!(!data.bool_value());
    }

    fn can_set_negated_value_in_path(f) {
        let data = ViewModel::with_next(ViewModel::with_bool(true));
        let target =
            create_target_with_source(f, src(&data), &next_bool_path().not(), Opts::mode(BindingMode::TwoWay));

        target.set_bool(true);

        assert!(!data.next().unwrap().bool_value());
    }

    fn can_set_double_negated_value(f) {
        let data = ViewModel::with_bool(true);
        let target =
            create_target_with_source(f, src(&data), &bool_path().not().not(), Opts::mode(BindingMode::TwoWay));

        target.set_bool(false);

        assert!(!data.bool_value());
    }

    fn can_set_double_negated_value_in_path(f) {
        let data = ViewModel::with_next(ViewModel::with_bool(true));
        let target = create_target_with_source(
            f,
            src(&data),
            &next_bool_path().not().not(),
            Opts::mode(BindingMode::TwoWay),
        );

        target.set_bool(false);

        assert!(!data.next().unwrap().bool_value());
    }
}
