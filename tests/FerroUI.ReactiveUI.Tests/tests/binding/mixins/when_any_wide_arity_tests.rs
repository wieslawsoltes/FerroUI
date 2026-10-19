//! `WhenAnyWideArityTests` (`ReactiveUI.Binding.Tests/Mixins/WhenAnyWideArityTests.cs`).
//!
//! Upstream calls the run-time overloads of every arity, 1 to 12 expressions; the port passes the
//! paths as one argument, one path or a tuple, and the selector takes the changes as one tuple.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::WideArityFixture;
use ferroui_reactiveui::*;
use ferroui_system::prop;

/// The value every observed property starts out holding.
const INITIAL_VALUE: &str = "a";

/// The first and last observed values read together (`InitialValue + InitialValue`), which is
/// what each test projects.
const BOTH_ENDS: &str = "aa";

/// The paths of the properties `value1` to `valueN` of [`WideArityFixture`], as one tuple.
macro_rules! paths {
    ($($field:ident),+) => {
        ($(prop!(WideArityFixture => $field),)+)
    };
}

/// Subscribes to what `$observe` makes of a new fixture and checks the first value seen.
macro_rules! observation_test {
    ($name:ident, $expected:expr, |$fixture:ident| $observe:expr) => {
        #[test]
        fn $name() {
            let $fixture = WideArityFixture::new();
            let seen = collect_items(&$observe);

            assert_eq!(seen.borrow()[0], $expected);
        }
    };
}

observation_test!(
    when_any_on_one_property_reports_the_selector_result,
    INITIAL_VALUE,
    |fixture| fixture.when_any(prop!(WideArityFixture => value1), |c1| c1.value)
);
observation_test!(
    when_any_on_two_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2), |c| format!("{}{}", c.0.value, c.1.value))
);
observation_test!(
    when_any_on_three_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3), |c| format!("{}{}", c.0.value, c.2.value))
);
observation_test!(
    when_any_on_four_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4), |c| format!("{}{}", c.0.value, c.3.value))
);
observation_test!(
    when_any_on_five_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5), |c| format!("{}{}", c.0.value, c.4.value))
);
observation_test!(
    when_any_on_six_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5, value6), |c| format!("{}{}", c.0.value, c.5.value))
);
observation_test!(
    when_any_on_seven_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5, value6, value7), |c| format!("{}{}", c.0.value, c.6.value))
);
observation_test!(
    when_any_on_eight_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5, value6, value7, value8), |c| format!("{}{}", c.0.value, c.7.value))
);
observation_test!(
    when_any_on_nine_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9), |c| format!("{}{}", c.0.value, c.8.value))
);
observation_test!(
    when_any_on_ten_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10), |c| format!("{}{}", c.0.value, c.9.value))
);
observation_test!(
    when_any_on_eleven_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11), |c| format!("{}{}", c.0.value, c.10.value))
);
observation_test!(
    when_any_on_twelve_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11, value12), |c| format!("{}{}", c.0.value, c.11.value))
);
