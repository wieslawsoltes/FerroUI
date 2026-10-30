//! The arithmetic expressions of WinUI's NumberBox (`NumberBoxParser`,
//! `controls/dev/NumberBox/NumberBoxParser.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md), shared by the number editors of the crate: [`NumberBox`](crate::NumberBox)
//! evaluates them with `f64`, and the evaluator is generic over the number type so that an
//! editor of [`Decimal`] values evaluates them exactly.
//!
//! The grammar is WinUI's: numbers, the binary operators `+`, `-`, `*`, `/` and `^`, and
//! parentheses. `^` binds tighter than `*` and `/`, which bind tighter than `+` and `-`, and every
//! operator, `^` included, associates from left to right (`2 ^ 3 ^ 2` is 64). A number is the
//! longest run of characters that are not operators, parentheses or white space, optionally after
//! a minus sign, and the caller's parser reads it (the number formatter of the editor); a minus
//! sign before a parenthesis is not part of the grammar. Spaces separate tokens; other white space
//! is an error.
//!
//! Deviation (docs/porting/winui-controls.md, Input): WinUI's parser has no bounds and returns
//! `null` for every error; this one rejects an input longer than 64 KiB or with more than 4096
//! tokens, never recurses, and reports what is wrong and where.

use ferroui_base::utilities::Decimal;
use std::fmt;

/// The longest input, in bytes.
const MAX_INPUT_LENGTH: usize = 65_536;

/// The most tokens (numbers, operators and parentheses) of an input.
const MAX_TOKENS: usize = 4096;

/// A number type the expressions are evaluated with: its arithmetic, `None` where a result is not
/// a value of the type.
pub trait ExpressionNumber: Copy {
    /// Whether the number is zero (a divisor that is zero is a division by zero).
    fn is_zero(&self) -> bool;
    /// The sum.
    fn add(self, other: Self) -> Option<Self>;
    /// The difference.
    fn sub(self, other: Self) -> Option<Self>;
    /// The product.
    fn mul(self, other: Self) -> Option<Self>;
    /// The quotient; `other` is not zero.
    fn div(self, other: Self) -> Option<Self>;
    /// `self` raised to the power `exponent`.
    fn pow(self, exponent: Self) -> Option<Self>;
}

/// Floating-point numbers, as WinUI's NumberBox computes: the results of IEEE 754 arithmetic,
/// infinities and NaN included.
impl ExpressionNumber for f64 {
    fn is_zero(&self) -> bool {
        *self == 0.0
    }

    fn add(self, other: Self) -> Option<Self> {
        Some(self + other)
    }

    fn sub(self, other: Self) -> Option<Self> {
        Some(self - other)
    }

    fn mul(self, other: Self) -> Option<Self> {
        Some(self * other)
    }

    fn div(self, other: Self) -> Option<Self> {
        Some(self / other)
    }

    fn pow(self, exponent: Self) -> Option<Self> {
        Some(self.powf(exponent))
    }
}

/// Exact decimal numbers: a result outside the range of [`Decimal`] is `None`. A power with an
/// integral exponent is exact; a power with a fractional exponent is computed with `f64` and is
/// `None` when that result is not a decimal number.
impl ExpressionNumber for Decimal {
    fn is_zero(&self) -> bool {
        Decimal::is_zero(self)
    }

    fn add(self, other: Self) -> Option<Self> {
        self.checked_add(other)
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.checked_sub(other)
    }

    fn mul(self, other: Self) -> Option<Self> {
        self.checked_mul(other)
    }

    fn div(self, other: Self) -> Option<Self> {
        if other.is_zero() {
            return None;
        }
        self.checked_div(other)
    }

    fn pow(self, exponent: Self) -> Option<Self> {
        let integral = i64::try_from(exponent.round_to_integer()).ok().filter(|n| Decimal::from(*n) == exponent);
        match integral {
            Some(n) => {
                // Exponentiation by squaring: at most 64 steps.
                let mut result = Decimal::ONE;
                let mut base = self;
                let mut remaining = n.unsigned_abs();
                while remaining > 0 {
                    if remaining & 1 == 1 {
                        result = result.checked_mul(base)?;
                    }
                    remaining >>= 1;
                    if remaining > 0 {
                        base = base.checked_mul(base)?;
                    }
                }
                if n < 0 {
                    if result.is_zero() {
                        return None;
                    }
                    result = Decimal::ONE.checked_div(result)?;
                }
                Some(result)
            }
            None => Decimal::try_from_f64(self.to_f64().powf(exponent.to_f64())),
        }
    }
}

/// What is wrong with an expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NumberExpressionErrorKind {
    /// The input is longer than 64 KiB.
    TooLong,
    /// The input has more than 4096 tokens.
    TooManyTokens,
    /// A number (or an opening parenthesis) was expected.
    ExpectedNumber,
    /// The parser of the caller does not read the text as a number.
    InvalidNumber,
    /// An operator (or a closing parenthesis) was expected.
    ExpectedOperator,
    /// A closing parenthesis has no opening one.
    UnmatchedClosingParenthesis,
    /// An opening parenthesis is not closed.
    UnmatchedOpeningParenthesis,
    /// A division by zero. WinUI's NumberBox gives the expression the value NaN.
    DivisionByZero,
    /// A result is not a value of the number type.
    Overflow,
}

impl NumberExpressionErrorKind {
    fn message(self) -> &'static str {
        match self {
            Self::TooLong => "the expression is longer than 64 KiB",
            Self::TooManyTokens => "the expression has more than 4096 tokens",
            Self::ExpectedNumber => "a number was expected",
            Self::InvalidNumber => "the text is not a number",
            Self::ExpectedOperator => "an operator was expected",
            Self::UnmatchedClosingParenthesis => "the closing parenthesis has no opening one",
            Self::UnmatchedOpeningParenthesis => "the opening parenthesis is not closed",
            Self::DivisionByZero => "division by zero",
            Self::Overflow => "the result is out of the range of the number type",
        }
    }
}

/// A rejected expression: what is wrong, and the byte offset in the input where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberExpressionError {
    /// The byte offset in the input.
    pub offset: usize,
    /// What is wrong.
    pub kind: NumberExpressionErrorKind,
}

impl fmt::Display for NumberExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.kind.message(), self.offset)
    }
}

impl std::error::Error for NumberExpressionError {}

#[derive(Clone, Copy)]
enum Token<T> {
    Number(T),
    /// An operator and its byte offset.
    Operator(u8, usize),
}

/// NumberBoxParser::GetPrecedenceValue.
fn precedence(operator: u8) -> u8 {
    match operator {
        b'^' => 2,
        b'*' | b'/' => 1,
        _ => 0,
    }
}

fn is_operator(c: u8) -> bool {
    matches!(c, b'+' | b'-' | b'*' | b'/' | b'^')
}

/// Evaluates an arithmetic expression of WinUI's NumberBox (see the [module](self)), reading its
/// numbers with `parse_number` (NumberBoxParser::Compute: GetTokens, ConvertInfixToPostfix and
/// ComputePostfixExpression in one pass, with the shunting-yard algorithm and no recursion).
///
/// ```
/// use ferroui_controls::utils::number_expression::evaluate_number_expression;
///
/// assert_eq!(Ok(11.0), evaluate_number_expression("2 + 3 * (4 - 1)", |text| text.parse::<f64>().ok()));
/// assert_eq!(Ok(64.0), evaluate_number_expression("2 ^ 3 ^ 2", |text| text.parse::<f64>().ok()));
/// assert!(evaluate_number_expression("1 +", |text| text.parse::<f64>().ok()).is_err());
/// ```
pub fn evaluate_number_expression<T: ExpressionNumber>(
    input: &str,
    parse_number: impl Fn(&str) -> Option<T>,
) -> Result<T, NumberExpressionError> {
    let error = |offset, kind| NumberExpressionError { offset, kind };
    if input.len() > MAX_INPUT_LENGTH {
        return Err(error(MAX_INPUT_LENGTH, NumberExpressionErrorKind::TooLong));
    }
    let bytes = input.as_bytes();
    let mut output: Vec<Token<T>> = Vec::new();
    // The operators and opening parentheses not yet output, with their offsets.
    let mut operators: Vec<(u8, usize)> = Vec::new();
    let mut index = 0;
    let mut tokens = 0;
    let mut expect_number = true;
    while index < bytes.len() {
        let c = bytes[index];
        if c == b' ' {
            index += 1;
            continue;
        }
        tokens += 1;
        if tokens > MAX_TOKENS {
            return Err(error(index, NumberExpressionErrorKind::TooManyTokens));
        }
        if expect_number {
            if c == b'(' {
                // An opening parenthesis does not change the kind of the next token.
                operators.push((c, index));
                index += 1;
                continue;
            }
            // NumberBoxParser::GetNextNumber: `^-?([^-+/*\(\)\^\s]+)`.
            let start = index;
            if c == b'-' {
                index += 1;
            }
            let digits = index;
            while index < bytes.len()
                && !is_operator(bytes[index])
                && !matches!(bytes[index], b'(' | b')')
                && !bytes[index].is_ascii_whitespace()
            {
                index += 1;
            }
            if index == digits {
                return Err(error(start, NumberExpressionErrorKind::ExpectedNumber));
            }
            let number =
                parse_number(&input[start..index]).ok_or_else(|| error(start, NumberExpressionErrorKind::InvalidNumber))?;
            output.push(Token::Number(number));
            expect_number = false;
        } else if is_operator(c) {
            while let Some(&(operator, offset)) = operators.last() {
                if operator == b'(' || precedence(operator) < precedence(c) {
                    break;
                }
                operators.pop();
                output.push(Token::Operator(operator, offset));
            }
            operators.push((c, index));
            expect_number = true;
            index += 1;
        } else if c == b')' {
            // A closing parenthesis does not change the kind of the next token either.
            let mut matched = false;
            while let Some((operator, offset)) = operators.pop() {
                if operator == b'(' {
                    matched = true;
                    break;
                }
                output.push(Token::Operator(operator, offset));
            }
            if !matched {
                return Err(error(index, NumberExpressionErrorKind::UnmatchedClosingParenthesis));
            }
            index += 1;
        } else {
            return Err(error(index, NumberExpressionErrorKind::ExpectedOperator));
        }
    }
    if expect_number {
        return Err(error(index, NumberExpressionErrorKind::ExpectedNumber));
    }
    while let Some((operator, offset)) = operators.pop() {
        if operator == b'(' {
            return Err(error(offset, NumberExpressionErrorKind::UnmatchedOpeningParenthesis));
        }
        output.push(Token::Operator(operator, offset));
    }

    // NumberBoxParser::ComputePostfixExpression.
    let mut values: Vec<T> = Vec::new();
    for token in output {
        match token {
            Token::Number(value) => values.push(value),
            Token::Operator(operator, offset) => {
                // The tokens alternate, so every operator has its two operands.
                let (Some(right), Some(left)) = (values.pop(), values.pop()) else {
                    return Err(error(offset, NumberExpressionErrorKind::ExpectedNumber));
                };
                let result = match operator {
                    b'+' => left.add(right),
                    b'-' => left.sub(right),
                    b'*' => left.mul(right),
                    b'/' if right.is_zero() => return Err(error(offset, NumberExpressionErrorKind::DivisionByZero)),
                    b'/' => left.div(right),
                    _ => left.pow(right),
                };
                values.push(result.ok_or_else(|| error(offset, NumberExpressionErrorKind::Overflow))?);
            }
        }
    }
    match values.as_slice() {
        [value] => Ok(*value),
        _ => Err(error(index, NumberExpressionErrorKind::ExpectedOperator)),
    }
}
