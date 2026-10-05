//! The parser of composition expressions.

use super::expression::{
    BinaryExpression, ConditionalExpression, ConstantExpression, Expression, ExpressionKeyword, ExpressionKeywords,
    ExpressionType, FunctionCallExpression, KeywordExpression, MemberAccessExpression, ParameterExpression,
    UnaryExpression,
};
use super::expression_parse_exception::ExpressionParseException;
use super::token_parser::TokenParser;

/// The parser of composition expressions.
pub struct ExpressionParser;

impl ExpressionParser {
    /// Parses an expression.
    pub fn parse(s: &str) -> Result<Expression, ExpressionParseException> {
        let mut p = TokenParser::new(s);
        let (parsed, _) = parse_till_terminator(&mut p, "", false, false)?;
        p.skip_whitespace();
        if p.length() != 0 {
            return Err(ExpressionParseException::new("Unexpected data ", p.position()));
        }
        Ok(parsed)
    }
}

fn try_parse_atomic(parser: &mut TokenParser<'_>) -> Option<Expression> {
    // We can parse keywords, parameter names and constants
    let expr: Option<Expression> = if parser.try_parse_keyword_lower_case(ExpressionKeywords::STARTING_VALUE) {
        Some(KeywordExpression::new(ExpressionKeyword::StartingValue).into())
    } else if parser.try_parse_keyword_lower_case(ExpressionKeywords::CURRENT_VALUE) {
        Some(KeywordExpression::new(ExpressionKeyword::CurrentValue).into())
    } else if parser.try_parse_keyword_lower_case(ExpressionKeywords::FINAL_VALUE) {
        Some(KeywordExpression::new(ExpressionKeyword::FinalValue).into())
    } else if parser.try_parse_keyword_lower_case(ExpressionKeywords::PI) {
        Some(KeywordExpression::new(ExpressionKeyword::Pi).into())
    } else if parser.try_parse_keyword_lower_case(ExpressionKeywords::TRUE) {
        Some(KeywordExpression::new(ExpressionKeyword::True).into())
    } else if parser.try_parse_keyword_lower_case(ExpressionKeywords::FALSE) {
        Some(KeywordExpression::new(ExpressionKeyword::False).into())
    } else if parser.try_parse_keyword_lower_case(ExpressionKeywords::TARGET) {
        Some(KeywordExpression::new(ExpressionKeyword::Target).into())
    } else if parser.try_parse_keyword_lower_case("relativeunit.relative") {
        Some(FunctionCallExpression::new("RelativeUnit.Relative", Vec::new()).into())
    } else if parser.try_parse_keyword_lower_case("relativeunit.absolute") {
        Some(FunctionCallExpression::new("RelativeUnit.Absolute", Vec::new()).into())
    } else {
        None
    };
    if expr.is_some() {
        return expr;
    }

    if let Some(identifier) = parser.try_parse_identifier() {
        return Some(ParameterExpression::new(identifier).into());
    }

    if let Some(scalar) = parser.try_parse_float() {
        return Some(ConstantExpression::new(scalar).into());
    }

    None
}

fn try_parse_operator(parser: &mut TokenParser<'_>) -> Option<ExpressionType> {
    if parser.try_consume_str("||") {
        Some(ExpressionType::LogicalOr)
    } else if parser.try_consume_str("&&") {
        Some(ExpressionType::LogicalAnd)
    } else if parser.try_consume_str(">=") {
        Some(ExpressionType::MoreThanOrEqual)
    } else if parser.try_consume_str("<=") {
        Some(ExpressionType::LessThanOrEqual)
    } else if parser.try_consume_str("==") {
        Some(ExpressionType::Equals)
    } else if parser.try_consume_str("!=") {
        Some(ExpressionType::NotEquals)
    } else {
        Some(match parser.try_consume_any("+-/*><%")? {
            '+' => ExpressionType::Add,
            '-' => ExpressionType::Subtract,
            '/' => ExpressionType::Divide,
            '*' => ExpressionType::Multiply,
            '<' => ExpressionType::LessThan,
            '>' => ExpressionType::MoreThan,
            _ => ExpressionType::Remainder,
        })
    }
}

/// A flat sequence of operands separated by binary operators, turned into a
/// tree according to operator precedence.
#[derive(Default)]
struct ExpressionOperatorGroup {
    expressions: Option<Vec<Option<Expression>>>,
    operators: Vec<ExpressionType>,
    first: Option<Expression>,
}

// https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/operators/
/// The operator precedence groups, from the lowest precedence to the highest.
const OPERATOR_PRECEDENCE_GROUPS_REVERSED: [&[ExpressionType]; 6] = [
    // conditional OR
    &[ExpressionType::LogicalOr],
    // conditional AND
    &[ExpressionType::LogicalAnd],
    // equality
    &[ExpressionType::Equals, ExpressionType::NotEquals],
    // relational
    &[
        ExpressionType::MoreThan,
        ExpressionType::MoreThanOrEqual,
        ExpressionType::LessThan,
        ExpressionType::LessThanOrEqual,
    ],
    // additive
    &[ExpressionType::Add, ExpressionType::Subtract],
    // multiplicative
    &[ExpressionType::Multiply, ExpressionType::Divide, ExpressionType::Remainder],
];

impl ExpressionOperatorGroup {
    fn not_empty(&self) -> bool {
        !self.empty()
    }

    fn empty(&self) -> bool {
        self.expressions.is_none() && self.first.is_none()
    }

    fn append_first(&mut self, expr: Expression) {
        assert!(!self.not_empty(), "the operator group already has an expression");
        self.first = Some(expr);
    }

    fn append_with_operator(&mut self, expr: Expression, op: ExpressionType) {
        if self.expressions.is_none() {
            let first = self.first.take().expect("the operator group has no first expression");
            self.expressions = Some(vec![Some(first)]);
            self.operators = Vec::new();
        }
        if let Some(expressions) = &mut self.expressions {
            expressions.push(Some(expr));
        }
        self.operators.push(op);
    }

    // a*b+c [a,b,c] [*,+], call with (0, 2)
    // ToExpression(a*b) + ToExpression(c)
    // a+b*c -> ToExpression(a) + ToExpression(b*c)
    //
    // The sequence is divided at the leftmost operator of the lowest
    // precedence group present, as in the reference implementation; a run of
    // operators of the same precedence therefore groups to the right
    // (`a-b-c` is `a-(b-c)`).
    fn range_to_expression(
        expressions: &mut [Option<Expression>],
        operators: &[ExpressionType],
        from: usize,
        to: usize,
    ) -> Result<Expression, ExpressionParseException> {
        let mut take = |index: usize| -> Result<Expression, ExpressionParseException> {
            expressions[index]
                .take()
                .ok_or_else(|| ExpressionParseException::new("Expression parsing algorithm bug in ToExpression", 0))
        };

        if to - from == 0 {
            return take(from);
        }

        if to - from == 1 {
            let left = take(from)?;
            let right = take(to)?;
            return Ok(BinaryExpression::new(left, right, operators[from]).into());
        }

        for grp in OPERATOR_PRECEDENCE_GROUPS_REVERSED {
            for c in from..to {
                let current_operator = operators[c];
                for &operator_from_group in grp {
                    if current_operator == operator_from_group {
                        // We are dividing the expression right here
                        let left = Self::range_to_expression(expressions, operators, from, c)?;
                        let right = Self::range_to_expression(expressions, operators, c + 1, to)?;
                        return Ok(BinaryExpression::new(left, right, current_operator).into());
                    }
                }
            }
        }

        // We shouldn't ever get here, if we are, there is something wrong in the code
        Err(ExpressionParseException::new(
            "Expression parsing algorithm bug in ToExpression",
            0,
        ))
    }

    fn into_expression(mut self) -> Result<Expression, ExpressionParseException> {
        match &mut self.expressions {
            None => Ok(self.first.take().expect("the operator group is empty")),
            Some(expressions) => {
                let to = expressions.len() - 1;
                Self::range_to_expression(expressions, &self.operators, 0, to)
            }
        }
    }
}

/// Parses an expression up to one of the terminator characters (which is
/// consumed and returned) or up to the end of the text.
fn parse_till_terminator(
    parser: &mut TokenParser<'_>,
    terminator_chars: &str,
    throw_on_terminator: bool,
    throw_on_end: bool,
) -> Result<(Expression, Option<char>), ExpressionParseException> {
    let mut left = ExpressionOperatorGroup::default();
    loop {
        if let Some(consumed_token) = parser.try_consume_any(terminator_chars) {
            if throw_on_terminator || left.empty() {
                // The reference implementation formats the (not yet assigned)
                // output token here, so the message never names the character.
                return Err(ExpressionParseException::new("Unexpected ''", parser.position() - 1));
            }
            return Ok((left.into_expression()?, Some(consumed_token)));
        }
        parser.skip_whitespace();
        if parser.length() == 0 {
            if throw_on_end || left.empty() {
                return Err(ExpressionParseException::new(
                    "Unexpected end of  expression",
                    parser.position(),
                ));
            }
            return Ok((left.into_expression()?, None));
        }

        let mut op: Option<ExpressionType> = None;
        if left.not_empty() {
            if parser.try_consume('?') {
                let (true_part, _) = parse_till_terminator(parser, ":", false, true)?;
                // pass through the current parsing rules to consume the rest
                let (false_part, token) =
                    parse_till_terminator(parser, terminator_chars, throw_on_terminator, throw_on_end)?;

                return Ok((
                    ConditionalExpression::new(left.into_expression()?, true_part, false_part).into(),
                    token,
                ));
            }

            // We expect a binary operator here
            match try_parse_operator(parser) {
                Some(sop) => op = Some(sop),
                None => return Err(ExpressionParseException::new("Unexpected token", parser.position())),
            }
        }

        // We expect an expression to be parsed (either due to expecting a binary operator or parsing the first part
        let mut apply_negation = false;
        while parser.try_consume('!') {
            apply_negation = !apply_negation;
        }

        let mut apply_unary_minus = false;
        while parser.try_consume('-') {
            apply_unary_minus = !apply_unary_minus;
        }

        let mut parsed: Expression;

        if parser.try_consume('(') {
            parsed = parse_till_terminator(parser, ")", false, true)?.0;
        } else if let Some(function_name) = parser.try_parse_call() {
            let mut parameter_list = Vec::new();
            while !parser.try_consume(')') {
                let (parameter, closing_token) = parse_till_terminator(parser, ",)", false, true)?;
                parameter_list.push(parameter);
                if closing_token == Some(')') {
                    break;
                }
                if closing_token != Some(',') {
                    return Err(ExpressionParseException::new(
                        "Unexpected end of the expression",
                        parser.position(),
                    ));
                }
            }

            parsed = FunctionCallExpression::new(function_name, parameter_list).into();
        } else if let Some(atomic) = try_parse_atomic(parser) {
            parsed = atomic;
        } else {
            return Err(ExpressionParseException::new("Unexpected token", parser.position()));
        }

        // Parse any following member accesses
        while parser.try_consume('.') {
            match parser.try_parse_identifier() {
                Some(member_name) => parsed = MemberAccessExpression::new(parsed, member_name).into(),
                None => return Err(ExpressionParseException::new("Unexpected token", parser.position())),
            }
        }

        // Apply ! operator
        if apply_negation {
            parsed = UnaryExpression::new(parsed, ExpressionType::Not).into();
        }

        if apply_unary_minus {
            parsed = match parsed {
                Expression::Constant(constexpr) => ConstantExpression::new(-constexpr.constant).into(),
                other => UnaryExpression::new(other, ExpressionType::UnaryMinus).into(),
            };
        }

        match op {
            None => left.append_first(parsed),
            Some(op) => left.append_with_operator(parsed, op),
        }
    }
}
