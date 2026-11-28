use crate::input_languages::{InputLanguages, LanguageScope};
use crate::language_input::{format_indentation, indentation_column, indentation_len, input_text};
use crate::{TextEditor, TextMateInstallation};
use ferroui_edit_core::document::{DocumentEdit, TextDocument, TextSnapshot};
use ferroui_edit_core::editing::{EditError, EditorOptions, Selection, SelectionSegment};
use ferroui_textmate::{IndentationMatch, IndentationRules, StandardTokenType};

const LOOKBACK: usize = 128;

pub(crate) struct TypingIndentation<'a> {
    pub(crate) installation: &'a TextMateInstallation,
    pub(crate) input: &'a InputLanguages,
    pub(crate) document: &'a TextDocument,
    pub(crate) snapshot: &'a TextSnapshot,
    pub(crate) options: &'a EditorOptions,
}

impl TypingIndentation<'_> {
    pub(crate) fn plan(
        &self,
        editor: &TextEditor,
        selection: &Selection,
        typed: &str,
    ) -> Result<Option<Vec<DocumentEdit>>, EditError> {
        let mut edits = Vec::with_capacity(selection.segments().len());
        let mut previous_end = None;
        for &segment in selection.segments() {
            let Some(edit) = self.plan_prefix(editor, segment, typed)? else {
                return Ok(None);
            };
            if previous_end.is_some_and(|end| edit.offset <= end) {
                // Multiple carets on one line must not rewrite each other's prefix.
                return Ok(None);
            }
            edits.push(edit);
            previous_end = Some(segment.end_offset());
        }
        Ok(Some(edits))
    }

    fn plan_prefix(
        &self,
        editor: &TextEditor,
        segment: SelectionSegment,
        typed: &str,
    ) -> Result<Option<DocumentEdit>, EditError> {
        let start = self.snapshot.get_line_by_offset(segment.start_offset())?;
        let end = self.snapshot.get_line_by_offset(segment.end_offset())?;
        let start_column = segment.start_offset() - start.offset;
        let end_column = segment.end_offset() - end.offset;
        if start_column > start.length || end_column > end.length {
            return Ok(None);
        }
        let Some(start_tokens) =
            self.installation
                .input_tokens(self.document, start.line_number, self.input)
        else {
            return Ok(None);
        };
        let root = self.input.grammar.initial_language;
        let start_scope = LanguageScope::at(&start_tokens, root, start_column, start.length);
        if start_scope.start != 0 {
            return Ok(None);
        }
        let Some(rules) = self.input.get(start_scope.id) else {
            return Ok(None);
        };
        let Some(indentation) = &rules.indentation else {
            return Ok(None);
        };
        let end_tokens;
        let suffix_tokens = if start.line_number == end.line_number {
            &start_tokens
        } else {
            let Some(tokens) =
                self.installation
                    .input_tokens(self.document, end.line_number, self.input)
            else {
                return Ok(None);
            };
            end_tokens = tokens;
            &end_tokens
        };
        let end_scope = LanguageScope::at(suffix_tokens, root, end_column, end.length);
        if start_scope.id != end_scope.id {
            return Ok(None);
        }

        let before = input_text(
            self.input,
            self.snapshot,
            start,
            0..start_column,
            &start_tokens,
        )?;
        let after = input_text(
            self.input,
            self.snapshot,
            end,
            end_column..end_scope.end,
            suffix_tokens,
        )?;
        let prospective = format!("{before}{typed}{after}");
        let Some((decreases, next_line_opener)) = report(
            editor,
            (|| {
                let decreases = indentation.should_decrease(&prospective)?
                    && !indentation.should_decrease(&format!("{before}{after}"))?;
                let next_line_opener = !decreases
                    && segment.is_empty()
                    && before.trim().is_empty()
                    && after.trim().is_empty()
                    && rules
                        .pairs
                        .auto_closing
                        .iter()
                        .any(|pair| pair.open.ends_with(typed))
                    && indentation.should_increase(&prospective)?;
                Ok((decreases, next_line_opener))
            })(),
        ) else {
            return Ok(None);
        };
        if !decreases && !next_line_opener {
            return Ok(None);
        }

        let raw_before = self.snapshot.get_text(start.offset, start_column)?;
        let raw_after = self
            .snapshot
            .get_text(segment.end_offset(), end.length - end_column)?;
        let hypothetical = format!("{raw_before}{typed}{raw_after}");
        if self.installation.prospective_token_type(
            self.document,
            start.line_number,
            &hypothetical,
            start_column,
        ) != Some(StandardTokenType::Other)
        {
            return Ok(None);
        }
        let size = self.options.indentation_size.clamp(1, 256);
        let current = &raw_before[..indentation_len(&raw_before)];
        let read_line =
            |number| self.previous_line(editor, number, start_scope.id, indentation, size);
        let Some((base, shift)) = inherited_indentation(start.line_number, &read_line)? else {
            return Ok(None);
        };
        let column = if decreases {
            if shift {
                base
            } else {
                base.saturating_sub(1) / size * size
            }
        } else if start.line_number > 1
            && shift
            && read_line(start.line_number - 1)?.is_some_and(|(_, rules)| rules.next_line)
            && current == format_indentation((base / size + 1) * size, self.options, size)
        {
            base
        } else {
            return Ok(None);
        };
        if column == indentation_column(current, size) {
            return Ok(None);
        }
        Ok(Some(DocumentEdit::new(
            start.offset,
            current.len(),
            format_indentation(column, self.options, size),
        )))
    }

    fn previous_line(
        &self,
        editor: &TextEditor,
        number: usize,
        language: u8,
        rules: &IndentationRules,
        size: usize,
    ) -> Result<Option<(usize, IndentationMatch)>, EditError> {
        let line = self.snapshot.get_line_by_number(number)?;
        let Some(tokens) = self
            .installation
            .input_tokens(self.document, number, self.input)
        else {
            return Ok(None);
        };
        let scope = LanguageScope::at(&tokens, self.input.grammar.initial_language, 0, line.length);
        if scope.id != language {
            return Ok(None);
        }
        let text = input_text(self.input, self.snapshot, line, 0..scope.end, &tokens)?;
        let column = indentation_column(&text[..indentation_len(&text)], size);
        if text.trim().is_empty()
            || tokens
                .first()
                .is_some_and(|token| token.kind == StandardTokenType::String)
        {
            return Ok(Some((
                column,
                IndentationMatch {
                    ignore: true,
                    ..IndentationMatch::default()
                },
            )));
        }
        Ok(report(editor, rules.classify(&text)).map(|rule| (column, rule)))
    }
}

fn report<T>(editor: &TextEditor, result: ferroui_textmate::Result<T>) -> Option<T> {
    result
        .map_err(|error| editor.report_error(error.to_string()))
        .ok()
}

fn inherited_indentation(
    number: usize,
    context: &impl Fn(usize) -> Result<Option<(usize, IndentationMatch)>, EditError>,
) -> Result<Option<(usize, bool)>, EditError> {
    if number == 1 {
        return Ok(Some((0, false)));
    }
    let first = number.saturating_sub(LOOKBACK).max(1);
    let mut baseline = None;
    let mut temporary = None;
    for number in (first..number).rev() {
        let Some((column, rules)) = context(number)? else {
            break;
        };
        let next_only = rules.next_line && !rules.increase && !rules.decrease;
        if temporary.is_some() && !next_only {
            break;
        }
        if rules.ignore {
            continue;
        }
        if rules.increase || (rules.next_line && baseline.is_none()) {
            return Ok(Some((column, true)));
        }
        if rules.decrease {
            return Ok(Some((column, false)));
        }
        if next_only {
            temporary = Some(column);
        }
        baseline = Some(column);
    }
    Ok(temporary.or(baseline).map(|column| (column, false)))
}
