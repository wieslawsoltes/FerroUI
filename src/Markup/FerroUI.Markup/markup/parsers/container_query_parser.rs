//! Port of `Markup/Parsers/ContainerQueryParser.cs`: creates a container
//! query from its text at run time.

use super::{ContainerQueryGrammar, ContainerQuerySyntax};
use ferroui_base::data::core::ExpressionParseException;
use ferroui_base::styling::{StyleQueries, StyleQuery};
use std::fmt;

/// What [`ContainerQueryParser::parse`] fails with: the exceptions the
/// managed original throws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerQueryParserError {
    /// The text is not a container query (`ExpressionParseException`).
    Parse(ExpressionParseException),
    /// The query cannot be created from this syntax
    /// (`NotSupportedException`).
    NotSupported(String),
}

impl fmt::Display for ContainerQueryParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContainerQueryParserError::Parse(error) => error.fmt(f),
            ContainerQueryParserError::NotSupported(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ContainerQueryParserError {}

impl From<ExpressionParseException> for ContainerQueryParserError {
    fn from(error: ExpressionParseException) -> Self {
        ContainerQueryParserError::Parse(error)
    }
}

/// Parses a [`StyleQuery`] from text.
#[derive(Clone, Copy, Debug, Default)]
pub struct ContainerQueryParser;

impl ContainerQueryParser {
    /// Creates a parser.
    pub fn new() -> Self {
        Self
    }

    /// Parses a [`StyleQuery`] from text. `None` for an empty query.
    pub fn parse(&self, s: &str) -> Result<Option<StyleQuery>, ContainerQueryParserError> {
        let syntax = ContainerQueryGrammar::parse(s)?;
        self.create(&syntax)
    }

    fn create(&self, syntax: &[ContainerQuerySyntax]) -> Result<Option<StyleQuery>, ContainerQueryParserError> {
        let mut result: Option<StyleQuery> = None;
        let mut results: Option<Vec<StyleQuery>> = None;

        for i in syntax {
            match i {
                ContainerQuerySyntax::Width { value, operator } => {
                    result = Some(StyleQueries::width(result, *operator, *value));
                }
                ContainerQuerySyntax::Height { value, operator } => {
                    result = Some(StyleQueries::height(result, *operator, *value));
                }
                ContainerQuerySyntax::Or | ContainerQuerySyntax::And => {
                    let query = result
                        .take()
                        .ok_or_else(|| ContainerQueryParserError::NotSupported("Invalid query!".to_string()))?;
                    results.get_or_insert_with(Vec::new).push(query);
                }
            }
        }

        if let Some(mut results) = results {
            if let Some(result) = result.take() {
                results.push(result);
            }

            result = if results.len() > 1 { Some(StyleQueries::or(results)) } else { results.into_iter().next() };
        }

        Ok(result)
    }
}
