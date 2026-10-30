//! Tests of the expressions of WinUI's NumberBox (`number_expression`), with `f64` and with
//! `Decimal` (docs/porting/winui-controls.md, Input).

use super::number_expression::{evaluate_number_expression, NumberExpressionErrorKind as Kind};
use ferroui_base::utilities::Decimal;

fn double(input: &str) -> Result<f64, Kind> {
    evaluate_number_expression(input, |text| text.parse::<f64>().ok()).map_err(|error| error.kind)
}

fn decimal(input: &str) -> Result<Decimal, Kind> {
    evaluate_number_expression(input, Decimal::try_parse).map_err(|error| error.kind)
}

fn d(text: &str) -> Decimal {
    Decimal::try_parse(text).expect("a decimal number")
}

#[test]
fn the_operators_have_the_precedence_of_number_box_and_associate_from_the_left() {
    assert_eq!(Ok(11.0), double("2 + 3 * (4 - 1)"));
    assert_eq!(Ok(64.0), double("2 ^ 3 ^ 2"));
    assert_eq!(Ok(1.0), double("8 / 4 / 2"));
    assert_eq!(Ok(-1.0), double("1 - 1 - 1"));
    assert_eq!(Ok(6.0), double("-2 * -3"));
    assert_eq!(Ok(18.0), double("2 * 3 ^ 2"));
    assert_eq!(Ok(5.0), double("((5))"));
}

#[test]
fn malformed_expressions_are_rejected_where_they_are_wrong() {
    assert_eq!(Err(Kind::ExpectedNumber), double(""));
    assert_eq!(Err(Kind::ExpectedNumber), double("   "));
    assert_eq!(Err(Kind::UnmatchedOpeningParenthesis), double("(1"));
    assert_eq!(Err(Kind::UnmatchedClosingParenthesis), double("1)"));
    assert_eq!(Err(Kind::ExpectedNumber), double("1 +"));
    assert_eq!(Err(Kind::ExpectedNumber), double("()"));
    // A minus sign before a parenthesis is not part of the grammar.
    assert_eq!(Err(Kind::ExpectedNumber), double("-(2)"));
    assert_eq!(Err(Kind::ExpectedOperator), double("1 2"));
    assert_eq!(Err(Kind::InvalidNumber), double("1 + apple"));
    // Only spaces separate tokens.
    assert_eq!(Err(Kind::ExpectedOperator), double("1\t+2"));
    let error = evaluate_number_expression("12 + x", |text| text.parse::<f64>().ok()).unwrap_err();
    assert_eq!(5, error.offset);
}

#[test]
fn the_input_is_bounded_and_deep_nesting_does_not_recurse() {
    assert_eq!(Err(Kind::TooLong), double(&"(".repeat(65_537)));
    assert_eq!(Err(Kind::TooManyTokens), double(&format!("{}1{}", "(".repeat(4096), ")".repeat(4096))));
    assert_eq!(Ok(1.0), double(&format!("{}1{}", "(".repeat(2000), ")".repeat(2000))));
}

#[test]
fn a_division_by_zero_is_reported_and_doubles_overflow_to_infinity() {
    let error = evaluate_number_expression("4 + 1 / 0", |text| text.parse::<f64>().ok()).unwrap_err();
    assert_eq!(Kind::DivisionByZero, error.kind);
    assert_eq!(6, error.offset);
    assert_eq!(Ok(f64::INFINITY), double("1e308 * 10"));
}

#[test]
fn decimals_are_exact_and_report_overflow() {
    assert_eq!(Ok(d("0.3")), decimal("0.1 + 0.2"));
    assert_eq!(Ok(d("2.20")), decimal("1.10 * 2"));
    assert_eq!(Ok(d("0.25")), decimal("2 ^ -2"));
    assert_eq!(Ok(d("1024")), decimal("2 ^ 10"));
    assert_eq!(Ok(Decimal::ONE), decimal("7 ^ 0"));
    assert_eq!(Ok(d("3")), decimal("9 ^ 0.5"));
    assert_eq!(Err(Kind::DivisionByZero), decimal("1 / 0"));
    assert_eq!(Err(Kind::Overflow), decimal("0 ^ -1"));
    assert_eq!(Err(Kind::Overflow), decimal("10 ^ 40"));
    assert_eq!(Err(Kind::Overflow), decimal("79228162514264337593543950335 + 1"));
}
