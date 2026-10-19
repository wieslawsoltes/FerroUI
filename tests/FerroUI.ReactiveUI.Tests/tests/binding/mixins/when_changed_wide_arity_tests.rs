//! `WhenChangedWideArityTests` (`ReactiveUI.Binding.Tests/Mixins/WhenChangedWideArityTests.cs`).
//!
//! Upstream calls the run-time overloads of every arity, 1 to 16 expressions. The port passes the
//! paths as one argument, one path or a tuple of 2 to 12, so the arities 13 to 16 are not ported.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::WideArityFixture;
use ferroui_reactive::ObservableExt;
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

            assert_eq!(seen.borrow()[0].clone(), $expected);
        }
    };
}

observation_test!(
    when_changed_on_one_property_reports_the_observed_values,
    INITIAL_VALUE,
    |fixture| fixture.when_changed(prop!(WideArityFixture => value1))
);
observation_test!(
    when_changed_on_two_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2)).select(|v| format!("{}{}", v.0, v.1))
);
observation_test!(
    when_changed_on_three_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3)).select(|v| format!("{}{}", v.0, v.2))
);
observation_test!(
    when_changed_on_four_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4)).select(|v| format!("{}{}", v.0, v.3))
);
observation_test!(
    when_changed_on_five_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5)).select(|v| format!("{}{}", v.0, v.4))
);
observation_test!(
    when_changed_on_six_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5, value6)).select(|v| format!("{}{}", v.0, v.5))
);
observation_test!(
    when_changed_on_seven_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5, value6, value7)).select(|v| format!("{}{}", v.0, v.6))
);
observation_test!(
    when_changed_on_eight_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5, value6, value7, value8)).select(|v| format!("{}{}", v.0, v.7))
);
observation_test!(
    when_changed_on_nine_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9)).select(|v| format!("{}{}", v.0, v.8))
);
observation_test!(
    when_changed_on_ten_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10)).select(|v| format!("{}{}", v.0, v.9))
);
observation_test!(
    when_changed_on_eleven_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11)).select(|v| format!("{}{}", v.0, v.10))
);
observation_test!(
    when_changed_on_twelve_properties_reports_the_observed_values,
    BOTH_ENDS,
    |fixture| fixture.when_changed(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11, value12)).select(|v| format!("{}{}", v.0, v.11))
);
// Not ported: WhenChanged_OnThirteenProperties_ReportsTheObservedValues - needs arity 13 to 16 (the tuple traits stop at 12)
// Not ported: WhenChanged_OnFourteenProperties_ReportsTheObservedValues - needs arity 13 to 16 (the tuple traits stop at 12)
// Not ported: WhenChanged_OnFifteenProperties_ReportsTheObservedValues - needs arity 13 to 16 (the tuple traits stop at 12)
// Not ported: WhenChanged_OnSixteenProperties_ReportsTheObservedValues - needs arity 13 to 16 (the tuple traits stop at 12)
observation_test!(
    when_changed_projecting_two_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2), |v| format!("{}{}", v.0, v.1))
);
observation_test!(
    when_changed_projecting_three_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3), |v| format!("{}{}", v.0, v.2))
);
observation_test!(
    when_changed_projecting_four_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4), |v| format!("{}{}", v.0, v.3))
);
observation_test!(
    when_changed_projecting_five_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5), |v| format!("{}{}", v.0, v.4))
);
observation_test!(
    when_changed_projecting_six_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5, value6), |v| format!("{}{}", v.0, v.5))
);
observation_test!(
    when_changed_projecting_seven_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5, value6, value7), |v| format!("{}{}", v.0, v.6))
);
observation_test!(
    when_changed_projecting_eight_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5, value6, value7, value8), |v| format!("{}{}", v.0, v.7))
);
observation_test!(
    when_changed_projecting_nine_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9), |v| format!("{}{}", v.0, v.8))
);
observation_test!(
    when_changed_projecting_ten_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10), |v| format!("{}{}", v.0, v.9))
);
observation_test!(
    when_changed_projecting_eleven_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11), |v| format!("{}{}", v.0, v.10))
);
observation_test!(
    when_changed_projecting_twelve_properties_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_changed_select(paths!(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11, value12), |v| format!("{}{}", v.0, v.11))
);
// Not ported: WhenChanged_ProjectingThirteenProperties_ReportsTheSelectorResult - needs arity 13 to 16 (the tuple traits stop at 12)
// Not ported: WhenChanged_ProjectingFourteenProperties_ReportsTheSelectorResult - needs arity 13 to 16 (the tuple traits stop at 12)
// Not ported: WhenChanged_ProjectingFifteenProperties_ReportsTheSelectorResult - needs arity 13 to 16 (the tuple traits stop at 12)
// Not ported: WhenChanged_ProjectingSixteenProperties_ReportsTheSelectorResult - needs arity 13 to 16 (the tuple traits stop at 12)
