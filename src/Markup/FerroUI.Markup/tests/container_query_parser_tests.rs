//! Not from upstream, which has no tests of `ContainerQueryParser`: the
//! parser creates a query for each alternative of the text.

use crate::markup::parsers::ContainerQueryParser;

#[test]
fn parses_a_width_query() {
    let result = ContainerQueryParser::new().parse("min-width:100");

    assert!(matches!(result, Ok(Some(_))), "{:?}", result.is_ok());
}

#[test]
fn parses_an_empty_query_as_none() {
    let result = ContainerQueryParser::new().parse("");

    assert!(matches!(result, Ok(None)), "{:?}", result.is_ok());
}
