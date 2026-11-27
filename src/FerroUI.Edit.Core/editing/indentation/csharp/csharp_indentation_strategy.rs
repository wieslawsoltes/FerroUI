use super::super::{DefaultIndentationStrategy, IIndentationStrategy, IndentationError};
use super::indentation_reformatter::{IndentationReformatter, IndentationSettings};
use super::literal_scanner::LiteralScanner;
use super::{IDocumentAccessor, TextDocumentAccessor};
use crate::document::{DocumentEdit, TextDocument, TextSnapshot};
use crate::editing::EditorOptions;

/// Stateful C# indentation for braces, switch labels, bracket alignment,
/// continuations and unbraced statements. The lexical scan preserves raw and
/// interpolated string content, including nested strings and multiline holes.
/// Each plan starts a fresh scan, so the strategy itself can be shared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CSharpIndentationStrategy {
    indentation_string: String,
}

impl Default for CSharpIndentationStrategy {
    fn default() -> Self {
        Self {
            indentation_string: "\t".into(),
        }
    }
}

impl CSharpIndentationStrategy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_options(options: &EditorOptions) -> Self {
        Self {
            indentation_string: options.indentation_string(),
        }
    }

    pub fn with_indentation_string(
        indentation: impl Into<String>,
    ) -> Result<Self, IndentationError> {
        let mut strategy = Self::new();
        strategy.set_indentation_string(indentation)?;
        Ok(strategy)
    }

    pub fn indentation_string(&self) -> &str {
        &self.indentation_string
    }

    /// Indentation must contain spaces or tabs, with at least one character.
    /// Rejecting delimiters ensures a plan cannot create additional lines.
    pub fn set_indentation_string(
        &mut self,
        indentation: impl Into<String>,
    ) -> Result<(), IndentationError> {
        let indentation = indentation.into();
        if indentation.is_empty() || !indentation.bytes().all(|c| matches!(c, b' ' | b'\t')) {
            return Err(IndentationError::InvalidIndentationString);
        }
        self.indentation_string = indentation;
        Ok(())
    }

    /// Reformats a streaming accessor. `keep_empty_lines` leaves truly empty
    /// lines empty, while whitespace-only lines receive their block indentation.
    pub fn indent(
        &self,
        document: &mut dyn IDocumentAccessor,
        keep_empty_lines: bool,
    ) -> Result<(), IndentationError> {
        self.reformat(document, keep_empty_lines).map(|_| ())
    }

    fn reformat(
        &self,
        document: &mut dyn IDocumentAccessor,
        keep_empty_lines: bool,
    ) -> Result<IndentationReformatter, IndentationError> {
        let settings = IndentationSettings {
            indent_string: &self.indentation_string,
            leave_empty_lines: keep_empty_lines,
        };
        let mut formatter = IndentationReformatter::default();
        formatter.reformat(document, &settings)?;
        Ok(formatter)
    }
}

impl IIndentationStrategy for CSharpIndentationStrategy {
    fn preserve_new_line_whitespace(
        &self,
        source: &TextSnapshot,
        offset: usize,
    ) -> Result<bool, IndentationError> {
        let current = source.get_line_by_offset(offset)?;
        let mut scanner = LiteralScanner::default();
        let mut units = Vec::new();
        for number in 1..current.line_number {
            let line = source.get_line_by_number(number)?;
            units.clear();
            units.extend(source.get_text(line.offset, line.length)?.encode_utf16());
            scanner.mask(&mut units);
        }
        let prefix_length = (offset - current.offset).min(current.length);
        units.clear();
        units.extend(
            source
                .get_text(current.offset, prefix_length)?
                .encode_utf16(),
        );
        scanner.scan(&mut units);
        Ok(scanner.in_string())
    }

    fn indent_line(
        &self,
        document: &TextDocument,
        line_number: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError> {
        let mut accessor = TextDocumentAccessor::with_range(document, line_number, line_number)?;
        let formatter = self.reformat(&mut accessor, false)?;
        if accessor.text().is_empty() && !formatter.in_raw_string() {
            // New empty lines inside comments or verbatim strings inherit the
            // previous line's indentation when the state machine leaves them.
            DefaultIndentationStrategy.indent_line(document, line_number)
        } else {
            Ok(accessor.into_edits())
        }
    }

    fn indent_lines(
        &self,
        document: &TextDocument,
        begin_line: usize,
        end_line: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError> {
        let mut accessor = TextDocumentAccessor::with_range(document, begin_line, end_line)?;
        self.indent(&mut accessor, true)?;
        Ok(accessor.into_edits())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(input: &str) -> String {
        let document = TextDocument::from_text(input);
        let edits = CSharpIndentationStrategy::new()
            .indent_lines(&document, 1, document.line_count())
            .unwrap();
        assert_eq!(
            document.text(),
            input,
            "planning must not mutate the source"
        );
        document.apply_edits(&edits).unwrap();
        document.text()
    }

    #[test]
    fn a_long_line_of_brackets_keeps_its_text_and_bounded_indentation() {
        let line = "(".repeat(200_000);
        assert!(format(&line).ends_with(&line));
    }

    #[test]
    fn raw_string_content_and_the_closing_column_survive_reindentation() {
        let input = concat!(
            "class C\r\n{\r\n",
            "string Raw()\r\n{\r\n",
            "var value = \"\"\"\"\r\n",
            "  } // text 😀  \r\n",
            "      \"\"\" is part of the value\r\n",
            "       \r\n",
            "  \"\"\"\";\r\n",
            "return value;\r\n}\r\n}\r\n",
        );
        let expected = input
            .replace("string Raw()\r\n{", "\tstring Raw()\r\n\t{")
            .replace("var value", "\t\tvar value")
            .replace("return value;\r\n}", "\t\treturn value;\r\n\t}");
        assert_eq!(format(input), expected);

        let document = TextDocument::from_text(
            &expected
                .replace("       \r\n", "\r\n")
                .replace("\t\treturn value;", "return value;"),
        );
        let strategy = CSharpIndentationStrategy::new();
        assert!(strategy.indent_line(&document, 8).unwrap().is_empty());
        let edits = strategy.indent_lines(&document, 6, 10).unwrap();
        assert_eq!(edits.len(), 1);
        let line = document.get_line_by_number(10).unwrap();
        assert_eq!(edits[0], DocumentEdit::new(line.offset, 0, "\t\t"));
    }

    #[test]
    fn interpolation_holes_keep_nested_literals_and_format_text_out_of_the_outer_blocks() {
        let input = concat!(
            "class C\n{\n",
            "var regular = $\"{{{new { Text = \"} //\", Nested = $\"{Format(\")]\")}\" }.Text,4:00}}}\";\n",
            "var verbatim = @$\"{Format(\"} /*\")}\n",
            " existing \"\"quotes\"\" {{ brace  \n",
            " \";\n",
            "var raw = $$\"\"\"\n",
            "  { \"json\": {{{Format(global::X.Name) /* } */ :00}}}  }  \n",
            "  \"\"\";\n",
            "void After() {\nRun();\n}\n}",
        );
        let expected = input.replace("\nvar ", "\n\tvar ").replace(
            "\nvoid After() {\nRun();\n}",
            "\n\tvoid After() {\n\t\tRun();\n\t}",
        );
        assert_eq!(format(input), expected);
    }

    #[test]
    fn multiline_regular_interpolation_and_incomplete_single_line_raw_string_recover() {
        let input = concat!(
            "class C\n{\n",
            "var value = $\"{Format(\n",
            "    \"} // ignored\", // }\n",
            "    \"\"\" raw } \"\"\")}\";\n",
            "var unfinished = \"\"\" } //\n",
            ";\n",
            "void After() {\nRun();\n}\n}",
        );
        let expected = input
            .replace("\nvar ", "\n\tvar ")
            .replace("\n;\n", "\n\t\t;\n")
            .replace(
                "\nvoid After() {\nRun();\n}",
                "\n\tvoid After() {\n\t\tRun();\n\t}",
            );
        assert_eq!(format(input), expected);
    }

    #[test]
    fn nested_braces_follow_block_indentation_and_preserve_delimiters() {
        assert_eq!(
            format(concat!(
                "namespace N\r\n{\r\nclass C\r\n{\r\nvoid M()\r\n{\r\n",
                "Call();\r\n}\r\n}\r\n}\r\n",
            )),
            concat!(
                "namespace N\r\n{\r\n\tclass C\r\n\t{\r\n\t\tvoid M()\r\n\t\t{\r\n",
                "\t\t\tCall();\r\n\t\t}\r\n\t}\r\n}\r\n",
            )
        );
    }

    #[test]
    fn switch_labels_have_one_less_level_than_their_statements() {
        assert_eq!(
            format(concat!(
                "switch (value)\n{\ncase 1:\nOne();\nbreak;\n",
                "case 2:\ncase 3:\nThree();\nbreak;\ndefault:\nOther();\nbreak;\n}\n",
            )),
            concat!(
                "switch (value)\n{\n\tcase 1:\n\t\tOne();\n\t\tbreak;\n",
                "\tcase 2:\n\tcase 3:\n\t\tThree();\n\t\tbreak;\n",
                "\tdefault:\n\t\tOther();\n\t\tbreak;\n}\n",
            )
        );
    }

    #[test]
    fn nested_unbraced_statements_restore_else_depth() {
        assert_eq!(
            format("if (a)\nif (b)\nA();\nelse\nB();\nAfter();"),
            "if (a)\n\tif (b)\n\t\tA();\n\telse\n\t\tB();\nAfter();"
        );
        for keyword in ["if", "for", "while", "foreach", "using", "lock"] {
            let input = format!("{keyword} (condition)\nWork();\nAfter();");
            assert_eq!(
                format(&input),
                format!("{keyword} (condition)\n\tWork();\nAfter();")
            );
        }
        assert_eq!(
            format("do\nWork();\nwhile (condition);\nAfter();"),
            "do\n\tWork();\nwhile (condition);\nAfter();"
        );
    }

    #[test]
    fn continuations_reset_after_finished_statements_and_labels_outdent() {
        assert_eq!(
            format("{\nvar value =\nA +\nB;\nlabel:\nCall();\n}"),
            "{\n\tvar value =\n\t\tA +\n\t\tB;\nlabel:\n\tCall();\n}"
        );
        assert_eq!(format("class C\n: Base\n{\n}"), "class C\n\t: Base\n{\n}");
    }

    #[test]
    fn parentheses_and_brackets_align_to_opening_columns() {
        assert_eq!(
            format("{\nCall(first,\nsecond,\nthird);\n}"),
            "{\n\tCall(first,\n\t     second,\n\t     third);\n}"
        );
        assert_eq!(
            format("{\nCall(\nvalue\n);\nvar value = data[\nindex\n];\n}"),
            "{\n\tCall(\n\t\tvalue\n\t);\n\tvar value = data[\n\t\tindex\n\t];\n}"
        );
    }

    #[test]
    fn alignment_counts_utf16_columns_through_supplementary_text() {
        assert_eq!(
            format("{\nF(\"😀\", G(first,\nsecond));\n}"),
            "{\n\tF(\"😀\", G(first,\n\t          second));\n}"
        );
    }

    #[test]
    fn escaped_strings_characters_and_line_comments_hide_braces() {
        assert_eq!(
            format(concat!(
                "{\nvar s = \"} // \\\" {\";\nvar c = '}';\n",
                "var quote = '\\\'';\n// } {\nCall();\n}\n",
            )),
            concat!(
                "{\n\tvar s = \"} // \\\" {\";\n\tvar c = '}';\n",
                "\tvar quote = '\\\'';\n\t// } {\n\tCall();\n}\n",
            )
        );
    }

    #[test]
    fn block_comment_stars_align_and_other_comment_content_keeps_indentation() {
        assert_eq!(
            format("{\n/* comment {\n* body }\n    literal {\n*/\nCall();\n}"),
            "{\n\t/* comment {\n\t * body }\n    literal {\n\t */\n\tCall();\n}"
        );
        assert_eq!(
            format("{\n//\tcommented code\n//\nCall();\n}"),
            "{\n//\tcommented code\n//\n\tCall();\n}"
        );
    }

    #[test]
    fn multiline_verbatim_strings_and_doubled_quotes_keep_content_indentation() {
        assert_eq!(
            format("{\nvar text = @\"first {\n  literal \"\"quoted\"\" }\nmore\";\nAfter();\n}"),
            "{\n\tvar text = @\"first {\n  literal \"\"quoted\"\" }\nmore\";\n\tAfter();\n}"
        );
    }

    #[test]
    fn directives_do_not_change_the_block_stack() {
        assert_eq!(
            format("{\n#if DEBUG // {\nCall();\n#else\nOther();\n#endif\n}"),
            "{\n\t#if DEBUG // {\n\tCall();\n\t#else\n\tOther();\n\t#endif\n}"
        );
    }

    #[test]
    fn range_preserves_empty_lines_but_indents_whitespace_only_lines() {
        assert_eq!(format("{\n\n    \nWork();\n}\n"), "{\n\n\t\n\tWork();\n}\n");
        let document = TextDocument::from_text("{\r\n");
        let edits = CSharpIndentationStrategy::new()
            .indent_line(&document, 2)
            .unwrap();
        assert_eq!(edits, vec![DocumentEdit::new(3, 0, "\t")]);
        document.apply_edits(&edits).unwrap();
        assert_eq!(document.text(), "{\r\n\t");
    }

    #[test]
    fn empty_lines_inside_comments_and_verbatim_strings_inherit_previous_padding() {
        for text in ["  /* comment\n", "  var text = @\"literal\n"] {
            let document = TextDocument::from_text(text);
            let edits = CSharpIndentationStrategy::new()
                .indent_line(&document, 2)
                .unwrap();
            document.apply_edits(&edits).unwrap();
            assert_eq!(document.text(), format!("{text}  "));
        }
    }

    #[test]
    fn partial_range_accepts_preceding_indentation_without_changing_context() {
        let text = "{\n   Existing();  \nnext();\n   untouched();  \n}";
        let document = TextDocument::from_text(text);
        let edits = CSharpIndentationStrategy::new()
            .indent_lines(&document, 3, 3)
            .unwrap();
        document.apply_edits(&edits).unwrap();
        assert_eq!(
            document.text(),
            "{\n   Existing();  \n   next();\n   untouched();  \n}"
        );
    }

    #[test]
    fn trailing_padding_is_trimmed_but_empty_comment_space_is_retained() {
        assert_eq!(
            format("{\nCall();  \n// \n/* \n* comment  \n*/\n}"),
            "{\n\tCall();\n\t// \n\t/* \n\t * comment\n\t */\n}"
        );
    }

    #[test]
    fn settings_support_options_and_reject_invalid_input_without_changes() {
        let mut strategy = CSharpIndentationStrategy::new();
        assert_eq!(strategy.indentation_string(), "\t");
        strategy.set_indentation_string("  ").unwrap();
        for invalid in ["", "\n", "\r", "\r\n", "x", "  \n", "\u{2003}"] {
            assert_eq!(
                strategy.set_indentation_string(invalid),
                Err(IndentationError::InvalidIndentationString)
            );
            assert_eq!(strategy.indentation_string(), "  ");
        }
        let options = EditorOptions {
            indentation_size: 3,
            convert_tabs_to_spaces: true,
            ..EditorOptions::default()
        };
        assert_eq!(
            CSharpIndentationStrategy::from_options(&options).indentation_string(),
            "   "
        );
        let document = TextDocument::from_text("{\nWork();\n}");
        document
            .apply_edits(&strategy.indent_lines(&document, 1, 3).unwrap())
            .unwrap();
        assert_eq!(document.text(), "{\n  Work();\n}");
        for (begin, end) in [(0, 1), (3, 2), (1, 4), (usize::MAX, usize::MAX)] {
            assert!(matches!(
                strategy.indent_lines(&document, begin, end),
                Err(IndentationError::InvalidLineRange { .. })
            ));
        }
    }

    #[test]
    fn formatting_is_one_undo_step_preserves_content_anchors_and_is_idempotent() {
        let text = "{\nWork(\"😀\");  \n}";
        let document = TextDocument::from_text(text);
        let anchor = document.create_anchor(5).unwrap();
        let strategy = CSharpIndentationStrategy::with_indentation_string("    ").unwrap();
        let edits = strategy.indent_lines(&document, 1, 3).unwrap();
        assert_eq!(document.text(), text);
        document.apply_edits(&edits).unwrap();
        assert_eq!(document.text(), "{\n    Work(\"😀\");\n}");
        assert_eq!(anchor.offset().unwrap(), 9);
        assert!(strategy.indent_lines(&document, 1, 3).unwrap().is_empty());
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), text);
        assert_eq!(anchor.offset().unwrap(), 5);
        assert!(!document.undo().unwrap());
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), "{\n    Work(\"😀\");\n}");
    }

    #[test]
    fn incomplete_delimiters_return_valid_plans_without_panicking() {
        for text in ["}", ")", "]", "{\nCall(\n}", "if (\n", "/*\n*", "@\"\n"] {
            let document = TextDocument::from_text(text);
            let edits = CSharpIndentationStrategy::new()
                .indent_lines(&document, 1, document.line_count())
                .unwrap();
            document.apply_edits(&edits).unwrap();
            assert_eq!(
                document.line_count(),
                text.bytes().filter(|&c| c == b'\n').count() + 1
            );
        }
    }
}
