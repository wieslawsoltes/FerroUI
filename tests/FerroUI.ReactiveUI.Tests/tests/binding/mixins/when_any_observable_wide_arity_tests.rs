//! `WhenAnyObservableWideArityTests` (`ReactiveUI.Binding.Tests/Mixins/WhenAnyObservableWideArityTests.cs`).
//!
//! Upstream calls the run-time overloads of every arity, 1 to 12 expressions; the port passes the
//! paths as one argument, one path or a tuple, and the selector takes the latest values as one
//! tuple.

use crate::mocks::collect_items;
use crate::tests::binding::test_models::WideArityObservableFixture;
use ferroui_reactiveui::*;
use ferroui_system::prop;

/// The value each stream is driven with.
const EMITTED_VALUE: &str = "a";

/// The first and last stream's values read together (`EmittedValue + EmittedValue`), which is
/// what the selectors project.
const BOTH_ENDS: &str = "aa";

/// The paths of the properties `stream1` to `streamN` of [`WideArityObservableFixture`], as one
/// tuple.
macro_rules! paths {
    ($($field:ident),+) => {
        ($(prop!(WideArityObservableFixture => $field),)+)
    };
}

/// Fills the streams of a new fixture, subscribes to what `$observe` makes of it, drives every
/// stream with [`EMITTED_VALUE`] and checks the last value seen.
macro_rules! observation_test {
    ($name:ident, $expected:expr, |$fixture:ident| $observe:expr) => {
        #[test]
        fn $name() {
            let $fixture = WideArityObservableFixture::new();
            let streams = $fixture.fill_streams();
            let seen = collect_items(&$observe);

            for stream in &streams {
                // Only the streams this arity observes were subscribed to, so the rest have no observer.
                stream.on_next(EMITTED_VALUE.to_string());
            }

            assert_eq!(seen.borrow().last().cloned().expect("a value was observed"), $expected);
        }
    };
}

observation_test!(
    when_any_observable_following_one_stream_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(prop!(WideArityObservableFixture => stream1))
);
observation_test!(
    when_any_observable_following_two_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2))
);
observation_test!(
    when_any_observable_following_three_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3))
);
observation_test!(
    when_any_observable_following_four_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4))
);
observation_test!(
    when_any_observable_following_five_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5))
);
observation_test!(
    when_any_observable_following_six_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5, stream6))
);
observation_test!(
    when_any_observable_following_seven_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7))
);
observation_test!(
    when_any_observable_following_eight_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8))
);
observation_test!(
    when_any_observable_following_nine_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9))
);
observation_test!(
    when_any_observable_following_ten_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9, stream10))
);
observation_test!(
    when_any_observable_following_eleven_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9, stream10, stream11))
);
observation_test!(
    when_any_observable_following_twelve_streams_reports_the_emitted_value,
    EMITTED_VALUE,
    |fixture| fixture.when_any_observable(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9, stream10, stream11, stream12))
);
observation_test!(
    when_any_observable_combining_two_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2), |v| format!("{}{}", v.0, v.1))
);
observation_test!(
    when_any_observable_combining_three_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3), |v| format!("{}{}", v.0, v.2))
);
observation_test!(
    when_any_observable_combining_four_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4), |v| format!("{}{}", v.0, v.3))
);
observation_test!(
    when_any_observable_combining_five_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5), |v| format!("{}{}", v.0, v.4))
);
observation_test!(
    when_any_observable_combining_six_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5, stream6), |v| format!("{}{}", v.0, v.5))
);
observation_test!(
    when_any_observable_combining_seven_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7), |v| format!("{}{}", v.0, v.6))
);
observation_test!(
    when_any_observable_combining_eight_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8), |v| format!("{}{}", v.0, v.7))
);
observation_test!(
    when_any_observable_combining_nine_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9), |v| format!("{}{}", v.0, v.8))
);
observation_test!(
    when_any_observable_combining_ten_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9, stream10), |v| format!("{}{}", v.0, v.9))
);
observation_test!(
    when_any_observable_combining_eleven_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9, stream10, stream11), |v| format!("{}{}", v.0, v.10))
);
observation_test!(
    when_any_observable_combining_twelve_streams_reports_the_selector_result,
    BOTH_ENDS,
    |fixture| fixture.when_any_observable_select(paths!(stream1, stream2, stream3, stream4, stream5, stream6, stream7, stream8, stream9, stream10, stream11, stream12), |v| format!("{}{}", v.0, v.11))
);
