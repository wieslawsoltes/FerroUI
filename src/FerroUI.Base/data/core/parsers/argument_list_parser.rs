//! Argument list parsing (`[a, b]`) on top of [`CharacterReader`].

use crate::data::core::ExpressionParseException;
use crate::utilities::character_reader::{is_white_space, CharacterReader};

impl CharacterReader<'_> {
    /// Parses a list of arguments enclosed in `open`/`close` and separated by
    /// `,`. The reader must be positioned on `open`.
    pub fn parse_arguments(
        &mut self,
        open: char,
        close: char,
    ) -> Result<Vec<String>, ExpressionParseException> {
        self.parse_arguments_with_delimiter(open, close, ',')
    }

    /// Parses a list of arguments enclosed in `open`/`close` and separated by
    /// `delimiter`. The reader must be positioned on `open`.
    pub fn parse_arguments_with_delimiter(
        &mut self,
        open: char,
        close: char,
        delimiter: char,
    ) -> Result<Vec<String>, ExpressionParseException> {
        let r = self;

        if r.peek() == Some(open) {
            let mut result = Vec::new();

            r.take();

            while !r.end() {
                let argument =
                    r.take_while(|c| c != delimiter && c != close && !is_white_space(c));
                if argument.is_empty() {
                    return Err(ExpressionParseException::new(
                        r.position(),
                        "Expected indexer argument.",
                    ));
                }

                result.push(argument.to_string());

                r.skip_whitespace();

                if r.end() {
                    return Err(ExpressionParseException::new(
                        r.position(),
                        format!("Expected '{delimiter}'."),
                    ));
                } else if r.take_if(close) {
                    return Ok(result);
                } else {
                    if r.take() != Some(delimiter) {
                        return Err(ExpressionParseException::new(
                            r.position(),
                            format!("Expected '{delimiter}'."),
                        ));
                    }

                    r.skip_whitespace();
                }
            }

            return Err(ExpressionParseException::new(
                r.position(),
                format!("Expected '{close}'."),
            ));
        }

        Err(ExpressionParseException::new(
            r.position(),
            format!("Expected '{open}'."),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<Vec<String>, ExpressionParseException> {
        CharacterReader::new(s).parse_arguments('[', ']')
    }

    fn error(s: &str) -> (i32, String) {
        let e = parse(s).unwrap_err();
        (e.column(), e.message().to_string())
    }

    #[test]
    fn parses_single_argument() {
        assert_eq!(parse("[15]").unwrap(), ["15"]);
    }

    #[test]
    fn parses_multiple_arguments_with_whitespace() {
        assert_eq!(parse("[5, 16 ,key]").unwrap(), ["5", "16", "key"]);
    }

    #[test]
    fn leaves_reader_after_close() {
        let mut r = CharacterReader::new("[1,2].Foo");
        assert_eq!(r.parse_arguments('[', ']').unwrap(), ["1", "2"]);
        assert_eq!(r.position(), 5);
        assert_eq!(r.peek(), Some('.'));
    }

    #[test]
    fn parses_custom_brackets_and_delimiter() {
        let mut r = CharacterReader::new("(a:B; 2)");
        assert_eq!(
            r.parse_arguments_with_delimiter('(', ')', ';').unwrap(),
            ["a:B", "2"]
        );
        assert!(r.end());
    }

    #[test]
    fn fails_without_open() {
        assert_eq!(error("15]"), (0, "Expected '['.".to_string()));
        assert_eq!(error(""), (0, "Expected '['.".to_string()));
    }

    #[test]
    fn fails_on_empty_argument() {
        assert_eq!(error("[]"), (1, "Expected indexer argument.".to_string()));
        assert_eq!(error("[,3]"), (1, "Expected indexer argument.".to_string()));
        assert_eq!(error("[3,,4]"), (3, "Expected indexer argument.".to_string()));
        assert_eq!(error("[3,4,]"), (5, "Expected indexer argument.".to_string()));
        assert_eq!(error("[ 3]"), (1, "Expected indexer argument.".to_string()));
    }

    #[test]
    fn fails_on_missing_delimiter() {
        assert_eq!(error("[3 4]"), (4, "Expected ','.".to_string()));
        assert_eq!(error("[3"), (2, "Expected ','.".to_string()));
        assert_eq!(error("[3 "), (3, "Expected ','.".to_string()));
    }

    #[test]
    fn fails_on_missing_close() {
        assert_eq!(error("["), (1, "Expected ']'.".to_string()));
        assert_eq!(error("[3,"), (3, "Expected ']'.".to_string()));
        assert_eq!(error("[3, "), (4, "Expected ']'.".to_string()));
    }
}
