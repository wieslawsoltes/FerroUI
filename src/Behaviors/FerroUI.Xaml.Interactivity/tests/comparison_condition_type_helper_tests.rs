//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/ComparisonConditionTypeHelperTests.cs`
//! of Xaml.Behaviors.
//!
//! Adapted: the theories run their cases in a loop.

use crate::helpers::ComparisonConditionTypeHelper;
use crate::ComparisonConditionType;
use ferroui_base::BoxedValue;
use std::rc::Rc;

fn number(value: i32) -> BoxedValue {
    Rc::new(value)
}

fn text(value: &str) -> BoxedValue {
    Rc::new(value.to_string())
}

#[test]
fn compare_non_convertible_string_uses_non_equal_semantics() {
    let cases = [
        (ComparisonConditionType::Equal, false),
        (ComparisonConditionType::NotEqual, true),
        (ComparisonConditionType::LessThan, false),
        (ComparisonConditionType::LessThanOrEqual, false),
        (ComparisonConditionType::GreaterThan, false),
        (ComparisonConditionType::GreaterThanOrEqual, false),
    ];

    for (operator_type, expected) in cases {
        let result =
            ComparisonConditionTypeHelper::compare(Some(&number(42)), operator_type, Some(&text("not-an-integer")));

        assert_eq!(Ok(expected), result, "{operator_type}");
    }
}

#[test]
fn compare_overflowing_string_uses_non_equal_semantics() {
    let result = ComparisonConditionTypeHelper::compare(
        Some(&number(42)),
        ComparisonConditionType::Equal,
        Some(&text("999999999999999999999999999999")),
    );

    assert_eq!(Ok(false), result);
}

// Not ported: Compare_IncompatibleComparable_UsesNonEqualSemantics - its operands are a comparable
// class whose CompareTo throws; values of the port are compared by the comparison of the untyped
// values (numbers, text, enumerations, registered comparers), which has no comparer that fails.
