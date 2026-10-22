//! The bounded, iterative expression evaluator of the WinUI number editor.
//! Operator precedence and left-associative exponentiation follow NumberBoxParser.cpp.

use std::fmt;

/// A rejected numeric expression and its UTF-8 byte offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberBoxExpressionError {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for NumberBoxExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at byte {}", self.message, self.offset)
    }
}

impl std::error::Error for NumberBoxExpressionError {}

#[derive(Clone, Copy)]
enum Token {
    Number(f64),
    Operator(u8),
}

fn precedence(operator: u8) -> u8 {
    match operator {
        b'^' => 2,
        b'*' | b'/' => 1,
        _ => 0,
    }
}

/// Evaluates the WinUI NumberBox expression grammar with a caller-supplied number parser.
///
/// All operators, including `^`, associate left to right. A minus sign may prefix a
/// numeric token; unary operators before parentheses are not part of this grammar.
/// Division by zero produces NaN, as the pinned implementation does. The input is
/// bounded to 64 KiB and 4096 tokens, and parsing never consumes the native stack.
pub fn evaluate_number_expression(
    input: &str,
    parse_number: impl Fn(&str) -> Option<f64>,
) -> Result<f64, NumberBoxExpressionError> {
    let error = |offset, message| NumberBoxExpressionError { offset, message };
    if input.len() > 65_536 {
        return Err(error(65_536, "Expression is longer than 64 KiB"));
    }
    let bytes = input.as_bytes();
    let mut output = Vec::new();
    let mut operators: Vec<(u8, usize)> = Vec::new();
    let mut index = 0;
    let mut tokens = 0;
    let mut expect_number = true;
    while index < bytes.len() {
        if bytes[index] == b' ' {
            index += 1;
            continue;
        }
        tokens += 1;
        if tokens > 4096 {
            return Err(error(index, "Expression has more than 4096 tokens"));
        }
        let c = bytes[index];
        if expect_number {
            if c == b'(' {
                operators.push((c, index));
                index += 1;
                continue;
            }
            let start = index;
            if c == b'-' {
                index += 1;
            }
            let digits_start = index;
            while index < bytes.len()
                && !matches!(bytes[index], b'+' | b'-' | b'*' | b'/' | b'^' | b'(' | b')')
                && !bytes[index].is_ascii_whitespace()
            {
                index += 1;
            }
            if index == digits_start {
                return Err(error(start, "Expected a number"));
            }
            let number =
                parse_number(&input[start..index]).ok_or_else(|| error(start, "Invalid number"))?;
            output.push(Token::Number(number));
            expect_number = false;
        } else if matches!(c, b'+' | b'-' | b'*' | b'/' | b'^') {
            while let Some(&(operator, _)) = operators.last() {
                if operator == b'(' || precedence(operator) < precedence(c) {
                    break;
                }
                operators.pop();
                output.push(Token::Operator(operator));
            }
            operators.push((c, index));
            expect_number = true;
            index += 1;
        } else if c == b')' {
            let mut matched = false;
            while let Some((operator, _)) = operators.pop() {
                if operator == b'(' {
                    matched = true;
                    break;
                }
                output.push(Token::Operator(operator));
            }
            if !matched {
                return Err(error(index, "Unmatched closing parenthesis"));
            }
            index += 1;
        } else {
            return Err(error(index, "Expected an operator"));
        }
    }
    if expect_number {
        return Err(error(index, "Expected a number"));
    }
    while let Some((operator, offset)) = operators.pop() {
        if operator == b'(' {
            return Err(error(offset, "Unmatched opening parenthesis"));
        }
        output.push(Token::Operator(operator));
    }
    let mut values = Vec::new();
    for token in output {
        match token {
            Token::Number(value) => values.push(value),
            Token::Operator(operator) => {
                let right = values
                    .pop()
                    .ok_or_else(|| error(index, "Missing operand"))?;
                let left = values
                    .pop()
                    .ok_or_else(|| error(index, "Missing operand"))?;
                let result = match operator {
                    b'+' => left + right,
                    b'-' => left - right,
                    b'*' => left * right,
                    b'/' if right == 0.0 => return Ok(f64::NAN),
                    b'/' => left / right,
                    b'^' => left.powf(right),
                    _ => unreachable!("operators are validated before evaluation"),
                };
                values.push(result);
            }
        }
    }
    if values.len() != 1 {
        return Err(error(index, "Expression did not produce one value"));
    }
    Ok(values[0])
}
