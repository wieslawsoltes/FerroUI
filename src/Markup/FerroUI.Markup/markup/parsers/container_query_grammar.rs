//! The grammar of container queries (`min-width:400 and max-height:300, width:100`).

use ferroui_base::data::core::ExpressionParseException;
use ferroui_base::styling::StyleQueryComparisonOperator;
use ferroui_base::utilities::character_reader::{is_letter, is_white_space};
use ferroui_base::utilities::CharacterReader;

const MIN_WIDTH_KEYWORD: &str = "min-width";
const MIN_HEIGHT_KEYWORD: &str = "min-height";
const MAX_WIDTH_KEYWORD: &str = "max-width";
const MAX_HEIGHT_KEYWORD: &str = "max-height";
const WIDTH_KEYWORD: &str = "width";
const HEIGHT_KEYWORD: &str = "height";
const KEYWORDS: [&str; 6] = [
    MIN_WIDTH_KEYWORD,
    MIN_HEIGHT_KEYWORD,
    MAX_WIDTH_KEYWORD,
    MAX_HEIGHT_KEYWORD,
    WIDTH_KEYWORD,
    HEIGHT_KEYWORD,
];

/// A syntax element of a parsed container query.
#[derive(Clone, Debug, PartialEq)]
pub enum ContainerQuerySyntax {
    /// Separator of alternatives (`,`).
    Or,
    /// Conjunction of features (`and`).
    And,
    /// A width feature (`width`, `min-width`, `max-width`).
    Width {
        value: f64,
        operator: StyleQueryComparisonOperator,
    },
    /// A height feature (`height`, `min-height`, `max-height`).
    Height {
        value: f64,
        operator: StyleQueryComparisonOperator,
    },
}

/// Parser of container queries.
pub struct ContainerQueryGrammar;

impl ContainerQueryGrammar {
    /// Parses `s` into a list of syntax elements.
    pub fn parse(s: &str) -> Result<Vec<ContainerQuerySyntax>, ExpressionParseException> {
        let mut r = CharacterReader::new(s);
        parse(&mut r, None)
    }
}

type ParseResult<T> = Result<T, ExpressionParseException>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Start,
    Middle,
    End,
}

fn parse(
    r: &mut CharacterReader<'_>,
    end: Option<char>,
) -> ParseResult<Vec<ContainerQuerySyntax>> {
    let mut state = State::Start;
    let mut selector = Vec::new();
    while !r.end() && state != State::End {
        let syntax;
        (state, syntax) = match state {
            State::Start => parse_start(r)?,
            State::Middle => parse_middle(r, end)?,
            State::End => (State::End, None),
        };
        if let Some(syntax) = syntax {
            selector.push(syntax);
        }
    }

    Ok(selector)
}

fn parse_feature(r: &mut CharacterReader<'_>) -> ParseResult<Option<ContainerQuerySyntax>> {
    r.skip_whitespace();

    let identifier = r.parse_style_class();

    if identifier.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Expected query feature name.",
        ));
    }

    if !KEYWORDS.contains(&identifier) {
        return Err(ExpressionParseException::new(
            r.position(),
            format!("Unknown feature name found: {identifier}"),
        ));
    }

    if identifier == MIN_WIDTH_KEYWORD
        || identifier == MAX_WIDTH_KEYWORD
        || identifier == WIDTH_KEYWORD
    {
        if !r.take_if(':') {
            return Err(ExpressionParseException::new(
                r.position(),
                format!("Expected ':' after '{identifier}'."),
            ));
        }
        let val = parse_decimal(r)?;

        let syntax = ContainerQuerySyntax::Width {
            value: val,
            operator: if identifier == WIDTH_KEYWORD {
                StyleQueryComparisonOperator::Equals
            } else if identifier == MIN_WIDTH_KEYWORD {
                StyleQueryComparisonOperator::GreaterThanOrEquals
            } else {
                StyleQueryComparisonOperator::LessThanOrEquals
            },
        };

        return Ok(Some(syntax));
    }

    if identifier == MIN_HEIGHT_KEYWORD
        || identifier == MAX_HEIGHT_KEYWORD
        || identifier == HEIGHT_KEYWORD
    {
        if !r.take_if(':') {
            return Err(ExpressionParseException::new(
                r.position(),
                format!("Expected ':' after '{identifier}'."),
            ));
        }
        let val = parse_decimal(r)?;

        let syntax = ContainerQuerySyntax::Height {
            value: val,
            // As upstream, the exact comparison is selected by the *width*
            // keyword, so a plain `height` feature yields `LessThanOrEquals`.
            operator: if identifier == WIDTH_KEYWORD {
                StyleQueryComparisonOperator::Equals
            } else if identifier == MIN_HEIGHT_KEYWORD {
                StyleQueryComparisonOperator::GreaterThanOrEquals
            } else {
                StyleQueryComparisonOperator::LessThanOrEquals
            },
        };

        return Ok(Some(syntax));
    }

    Ok(None)
}

fn parse_start(
    r: &mut CharacterReader<'_>,
) -> ParseResult<(State, Option<ContainerQuerySyntax>)> {
    r.skip_whitespace();
    if r.end() {
        return Ok((State::End, None));
    }

    if r.peek().is_some_and(is_letter) {
        return Ok((State::Middle, parse_feature(r)?));
    }

    Err(ExpressionParseException::new(
        r.position(),
        "Invalid syntax found",
    ))
}

fn parse_middle(
    r: &mut CharacterReader<'_>,
    end: Option<char>,
) -> ParseResult<(State, Option<ContainerQuerySyntax>)> {
    r.skip_whitespace();

    if r.take_if(',') {
        return Ok((State::Start, Some(ContainerQuerySyntax::Or)));
    } else if end.is_some() && !r.end() && r.peek() == end {
        return Ok((State::End, None));
    } else {
        let identifier = r.take_while(|c| !is_white_space(c));

        if identifier == "and" {
            return Ok((State::Start, Some(ContainerQuerySyntax::And)));
        }
    }
    Err(ExpressionParseException::new(
        r.position(),
        "Invalid syntax found",
    ))
}

fn parse_decimal(r: &mut CharacterReader<'_>) -> ParseResult<f64> {
    r.skip_whitespace();
    let number = r.parse_number();
    if number.is_empty() {
        return Err(ExpressionParseException::new(
            r.position(),
            "Expected a number after.",
        ));
    }

    // Only ASCII digits form a number; other decimal digits are rejected.
    number.parse::<f64>().map_err(|_| {
        ExpressionParseException::new(
            r.position(),
            format!("The input string '{number}' was not in a correct format."),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use StyleQueryComparisonOperator::{Equals, GreaterThanOrEquals, LessThanOrEquals};

    fn parse(s: &str) -> Vec<ContainerQuerySyntax> {
        ContainerQueryGrammar::parse(s).unwrap()
    }

    fn error(s: &str) -> String {
        ContainerQueryGrammar::parse(s).unwrap_err().message().to_string()
    }

    fn width(value: f64, operator: StyleQueryComparisonOperator) -> ContainerQuerySyntax {
        ContainerQuerySyntax::Width { value, operator }
    }

    fn height(value: f64, operator: StyleQueryComparisonOperator) -> ContainerQuerySyntax {
        ContainerQuerySyntax::Height { value, operator }
    }

    #[test]
    fn empty_query() {
        assert_eq!(parse(""), []);
        assert_eq!(parse("  "), []);
    }

    #[test]
    fn width_features() {
        assert_eq!(parse("width:100"), [width(100.0, Equals)]);
        assert_eq!(parse("min-width:400"), [width(400.0, GreaterThanOrEquals)]);
        assert_eq!(parse("max-width:400"), [width(400.0, LessThanOrEquals)]);
    }

    #[test]
    fn height_features() {
        assert_eq!(parse("min-height:300"), [height(300.0, GreaterThanOrEquals)]);
        assert_eq!(parse("max-height:300"), [height(300.0, LessThanOrEquals)]);
        // Upstream behaviour: `height` does not map to `Equals`.
        assert_eq!(parse("height:300"), [height(300.0, LessThanOrEquals)]);
    }

    #[test]
    fn whitespace_before_feature_and_value() {
        assert_eq!(parse("  min-width: 400"), [width(400.0, GreaterThanOrEquals)]);
    }

    #[test]
    fn and_combination() {
        assert_eq!(
            parse("min-width:400 and max-height:300"),
            [
                width(400.0, GreaterThanOrEquals),
                ContainerQuerySyntax::And,
                height(300.0, LessThanOrEquals)
            ]
        );
    }

    #[test]
    fn and_needs_no_leading_whitespace() {
        // As upstream: white space before `and` is optional.
        assert_eq!(
            parse("width:1and width:2"),
            [width(1.0, Equals), ContainerQuerySyntax::And, width(2.0, Equals)]
        );
    }

    #[test]
    fn or_combination() {
        assert_eq!(
            parse("min-width:400, max-width:200 , width:1"),
            [
                width(400.0, GreaterThanOrEquals),
                ContainerQuerySyntax::Or,
                width(200.0, LessThanOrEquals),
                ContainerQuerySyntax::Or,
                width(1.0, Equals)
            ]
        );
    }

    #[test]
    fn trailing_separator_is_accepted() {
        // As upstream: the loop stops at the end of the input.
        assert_eq!(
            parse("width:1,"),
            [width(1.0, Equals), ContainerQuerySyntax::Or]
        );
        assert_eq!(
            parse("width:1 and"),
            [width(1.0, Equals), ContainerQuerySyntax::And]
        );
    }

    #[test]
    fn invalid_start() {
        assert_eq!(error("1"), "Invalid syntax found");
        assert_eq!(error("(width:1)"), "Invalid syntax found");
        assert_eq!(error("width:1,,"), "Invalid syntax found");
    }

    #[test]
    fn unknown_feature() {
        assert_eq!(error("depth:1"), "Unknown feature name found: depth");
        assert_eq!(error("Width:1"), "Unknown feature name found: Width");
    }

    #[test]
    fn missing_colon() {
        assert_eq!(error("width 1"), "Expected ':' after 'width'.");
        assert_eq!(error("min-height=1"), "Expected ':' after 'min-height'.");
        assert_eq!(error("max-width"), "Expected ':' after 'max-width'.");
    }

    #[test]
    fn missing_number() {
        assert_eq!(error("width:"), "Expected a number after.");
        assert_eq!(error("width:abc"), "Expected a number after.");
        assert_eq!(error("height:-1"), "Expected a number after.");
    }

    #[test]
    fn non_ascii_digits_are_rejected() {
        assert_eq!(
            error("width:٣"),
            "The input string '٣' was not in a correct format."
        );
    }

    #[test]
    fn invalid_middle() {
        assert_eq!(error("width:1 or width:2"), "Invalid syntax found");
        assert_eq!(error("width:1.5"), "Invalid syntax found");
        // Trailing white space fails here; upstream fails with an
        // index-out-of-range error on the same input.
        assert_eq!(error("width:1 "), "Invalid syntax found");
    }

    #[test]
    fn error_column_is_reported() {
        assert_eq!(ContainerQueryGrammar::parse("width 1").unwrap_err().column(), 5);
    }
}
