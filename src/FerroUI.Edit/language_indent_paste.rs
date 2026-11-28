use crate::input_languages::{EditingRules, InputLanguages, InputToken, LanguageScope};
use crate::language_input::{
    bracket_enter, context_line, expand_enter_action, format_indentation, indentation_column,
    indentation_len, inherit_indentation, input_text,
};
use crate::text_editor::EditTarget;
use crate::{TextEditor, TextMateInstallation};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_edit_core::document::{DocumentEdit, DocumentLine, TextDocument, TextSnapshot};
use ferroui_edit_core::editing::{EditError, EditorOptions, Selection, SelectionSegment};
use ferroui_textmate::{IndentationMatch, StandardTokenType, StateStack};
use std::rc::Rc;

const LINES_PER_BATCH: usize = 32;

/// Optionally formats an ordinary clipboard paste before touching the document.
pub(crate) async fn paste(editor: &TextEditor, text: &str) -> Result<Option<String>, EditError> {
    let session = editor.session();
    let options = session.options();
    if !options.auto_indent_on_paste || text.trim().is_empty() {
        return Ok(None);
    }
    let selection = session.selection();
    let segment = selection.primary();
    if selection.is_rectangular()
        || selection.segments().len() != 1
        || segment.start_virtual_space() != 0
        || segment.end_virtual_space() != 0
    {
        return Ok(None);
    }
    let Some(installation) = editor.textmate_installation() else {
        return Ok(None);
    };
    let input = installation.input_languages();
    let max_line_length = installation.max_tokenized_line_length();
    let target = EditTarget::new(editor);
    let Some(strategy) = session
        .indentation_strategy()
        .filter(|strategy| input.has_indentation_rules() && strategy.allows_language_indentation())
    else {
        return Ok(None);
    };
    let is_current = || {
        let session = editor.session();
        target.is_current(editor)
            && session.selection() == selection
            && session.options() == options
            && session
                .indentation_strategy()
                .is_some_and(|current| Rc::ptr_eq(&current, &strategy))
            && editor
                .textmate_installation()
                .is_some_and(|current| Rc::ptr_eq(&current, &installation))
            && Rc::ptr_eq(&input, &installation.input_languages())
            && installation.max_tokenized_line_length() == max_line_length
    };

    let document = session.document();
    let source = document.create_snapshot();
    let start = source.get_line_by_offset(segment.start_offset())?;
    let end = source.get_line_by_offset(segment.end_offset())?;
    if segment.start_offset() > start.end_offset() || segment.end_offset() > end.end_offset() {
        return Ok(None);
    }
    let prefix = source.get_text(start.offset, segment.start_offset() - start.offset)?;
    let suffix = source.get_text(
        segment.end_offset(),
        end.end_offset() - segment.end_offset(),
    )?;
    let Some(state) = installation.state_before_input_line(document, start.line_number) else {
        return Ok(None);
    };
    let preview = TextSnapshot::from_text(&format!("{prefix}{text}{suffix}"));
    let paste_start = segment.start_offset() - start.offset;
    let paste_end = paste_start + text.encode_utf16().count();
    let planner = PasteIndentation {
        editor,
        document,
        source: &source,
        installation: &installation,
        input: &input,
        options: &options,
        size: options.indentation_size.clamp(1, 256),
    };
    let Some(lines) = planner
        .scan(&preview, state, paste_start, paste_end, &is_current)
        .await?
    else {
        return Ok(None);
    };
    let Some(edit) =
        planner.replacement(&preview, &lines, start, paste_start, paste_end, segment)?
    else {
        return Ok(None);
    };
    // The optional plan can reach into leading whitespace before the selection.
    // If that prefix is protected, leave the regular clipboard replacement in charge.
    let validation = session.validate_document_edits(std::slice::from_ref(&edit));
    if !is_current() {
        return Err(EditError::StaleEditPlan);
    }
    if matches!(validation, Err(EditError::ReadOnly)) {
        return Ok(None);
    }
    validation?;
    let caret = edit.offset + edit.text.encode_utf16().count();
    session.apply_document_edits_and_select(
        std::slice::from_ref(&edit),
        Selection::from_segments([SelectionSegment::empty(caret)], 0),
        is_current,
    )?;
    editor.selection_changed();
    Ok(Some(edit.text))
}

struct PasteIndentation<'a> {
    editor: &'a TextEditor,
    document: &'a TextDocument,
    source: &'a TextSnapshot,
    installation: &'a TextMateInstallation,
    input: &'a InputLanguages,
    options: &'a EditorOptions,
    size: usize,
}

struct PasteLine {
    line: DocumentLine,
    language: u8,
    prefix: usize,
    column: usize,
    text: String,
    blank: bool,
    starts_in_string: bool,
}

impl PasteIndentation<'_> {
    async fn scan(
        &self,
        preview: &TextSnapshot,
        mut state: Option<StateStack>,
        start: usize,
        end: usize,
        is_current: &impl Fn() -> bool,
    ) -> Result<Option<Vec<PasteLine>>, EditError> {
        let grammar = self.installation.grammar().clone();
        let tokenize_options = self.installation.tokenize_options();
        let mut lines = Vec::with_capacity(preview.line_count());
        for number in 1..=preview.line_count() {
            if (number - 1) % LINES_PER_BATCH == 0 && number != 1 {
                Dispatcher::yield_with_priority(DispatcherPriority::BACKGROUND).await;
                if !is_current() {
                    return Err(EditError::StaleEditPlan);
                }
            }
            let line = preview.get_line_by_number(number)?;
            if line.length > self.installation.max_tokenized_line_length() {
                return Ok(None);
            }
            let raw = preview.get_text(line.offset, line.length)?;
            let result = match grammar.tokenize_line_with_options(
                &raw,
                state.as_ref(),
                tokenize_options,
                None,
            ) {
                Ok(result) if !result.stopped_early => result,
                Ok(_) => return Ok(None),
                Err(error) => {
                    self.editor.report_error(error.to_string());
                    return Ok(None);
                }
            };
            state = Some(result.rule_stack);
            let tokens = self.input.tokens(&result.tokens);
            if !self.options.auto_indent_on_paste_within_string
                && [start, end].into_iter().any(|offset| {
                    line.offset <= offset
                        && offset <= line.end_offset()
                        && token_kind(&tokens, offset - line.offset) == StandardTokenType::String
                })
            {
                return Ok(None);
            }
            lines.push(self.line_context(preview, line, &raw, &tokens)?);
        }
        Ok(Some(lines))
    }

    fn line_context(
        &self,
        snapshot: &TextSnapshot,
        line: DocumentLine,
        raw: &str,
        tokens: &[InputToken],
    ) -> Result<PasteLine, EditError> {
        let prefix = indentation_len(raw);
        let column = indentation_column(&raw[..prefix], self.size);
        let scope = LanguageScope::at(
            tokens,
            self.input.grammar.initial_language,
            prefix,
            line.length,
        );
        let content_start = scope.start.max(prefix);
        let starts_in_string = token_kind(tokens, 0) == StandardTokenType::String;
        let blank = raw.trim().is_empty();
        let mut text = raw[..prefix].to_owned();
        if content_start < scope.end {
            text.push_str(&input_text(
                self.input,
                snapshot,
                line,
                content_start..scope.end,
                tokens,
            )?);
        }
        Ok(PasteLine {
            line,
            language: scope.id,
            prefix,
            column,
            text,
            blank,
            starts_in_string,
        })
    }

    fn replacement(
        &self,
        preview: &TextSnapshot,
        lines: &[PasteLine],
        destination: DocumentLine,
        paste_start: usize,
        paste_end: usize,
        selection: SelectionSegment,
    ) -> Result<Option<DocumentEdit>, EditError> {
        let mut populated = lines
            .iter()
            .enumerate()
            .filter(|(_, row)| !row.blank && row.line.offset + row.prefix < paste_end);
        let Some((_, first)) = populated.next() else {
            return Ok(None);
        };
        let Some((rules, kind)) = self.classify(first) else {
            return Ok(None);
        };
        let mut edits = Vec::new();
        let mut first_column = first.column;
        let editable_prefix = first.line.offset + first.prefix >= paste_start;
        if editable_prefix && !self.keep_literal(first) {
            if let Some(target) =
                self.first_column(destination.line_number, first.language, kind.decrease)?
            {
                first_column = target;
                if first_column == first.column
                    && !kind.increase
                    && !kind.decrease
                    && !kind.next_line
                {
                    return Ok(None);
                }
                self.change_prefix(&mut edits, first, target, paste_end);
            }
        }

        if let Some((index, second)) = populated.find(|(_, row)| !self.keep_literal(row)) {
            if let Some((_, second_kind)) = self.classify(second) {
                let previous = if first_column == first.column {
                    first.text.clone()
                } else {
                    format!(
                        "{}{}",
                        format_indentation(first_column, self.options, self.size),
                        &first.text[first.prefix..]
                    )
                };
                let updated = self.classify_text(rules, &previous);
                if let Some(base) = updated.and_then(|kind| {
                    self.column_after_line(
                        rules,
                        &previous,
                        first_column,
                        kind,
                        second_kind.decrease,
                    )
                }) {
                    for row in &lines[index..] {
                        if !self.keep_literal(row) {
                            let shifted = (row.column + base).saturating_sub(second.column);
                            self.change_prefix(&mut edits, row, shifted, paste_end);
                        }
                    }
                }
            }
        }

        let Some(first_edit) = edits.first() else {
            return Ok(None);
        };
        let replacement_start = first_edit.offset.min(paste_start);
        let mut text = String::new();
        let mut offset = replacement_start;
        for edit in edits {
            text.push_str(&preview.get_text(offset, edit.offset - offset)?);
            text.push_str(&edit.text);
            offset = edit.offset + edit.length;
        }
        text.push_str(&preview.get_text(offset, paste_end - offset)?);
        let absolute_start = destination.offset + replacement_start;
        Ok(Some(DocumentEdit::new(
            absolute_start,
            selection.end_offset() - absolute_start,
            text,
        )))
    }

    fn change_prefix(
        &self,
        edits: &mut Vec<DocumentEdit>,
        line: &PasteLine,
        column: usize,
        paste_end: usize,
    ) {
        if line.column != column
            && line.line.offset < paste_end
            && line.line.offset + line.prefix <= paste_end
        {
            edits.push(DocumentEdit::new(
                line.line.offset,
                line.prefix,
                format_indentation(column, self.options, self.size),
            ));
        }
    }

    fn keep_literal(&self, line: &PasteLine) -> bool {
        !self.options.auto_indent_on_paste_within_string && line.starts_in_string
    }

    fn classify(&self, line: &PasteLine) -> Option<(&EditingRules, IndentationMatch)> {
        let rules = self.input.get(line.language)?;
        Some((rules, self.classify_text(rules, &line.text)?))
    }

    fn classify_text(&self, rules: &EditingRules, text: &str) -> Option<IndentationMatch> {
        rules
            .indentation
            .as_ref()?
            .classify(text)
            .map_err(|error| self.editor.report_error(error.to_string()))
            .ok()
    }

    fn first_column(
        &self,
        number: usize,
        language: u8,
        decreases: bool,
    ) -> Result<Option<usize>, EditError> {
        if number == 1 {
            return Ok(Some(0));
        }
        let read = |number| -> Result<Option<PasteLine>, EditError> {
            let line = self.source.get_line_by_number(number)?;
            let raw = self.source.get_text(line.offset, line.length)?;
            self.installation
                .input_tokens(self.document, number, self.input)
                .map(|tokens| self.line_context(self.source, line, &raw, &tokens))
                .transpose()
        };
        let Some(previous) = read(number - 1)? else {
            return Ok(None);
        };
        if previous.language != language && previous.language != self.input.grammar.initial_language
        {
            return Ok(None);
        }
        let Some((rules, kind)) = self.classify(&previous) else {
            return Ok(None);
        };
        if previous.language != language && !kind.increase {
            return Ok(None);
        }
        if !previous.blank && !kind.ignore {
            let Some(next) =
                self.column_after_line(rules, &previous.text, previous.column, kind, decreases)
            else {
                return Ok(None);
            };
            if kind.increase
                || kind.next_line
                || kind.decrease
                || next != self.adjust(previous.column, false, decreases)
            {
                return Ok(Some(next));
            }
        }

        let current = context_line(&previous.text, previous.column, |_| Some(kind)).unwrap();
        let (base, increase) = inherit_indentation(number - 1, current, |number| {
            let Some(row) = read(number)? else {
                return Ok(None);
            };
            if row.language != language {
                return Ok(None);
            }
            let Some((_, kind)) = self.classify(&row) else {
                return Ok(None);
            };
            Ok(context_line(&row.text, row.column, |_| Some(kind)))
        })?;
        Ok(Some(self.adjust(base, increase, decreases)))
    }

    fn column_after_line(
        &self,
        rules: &EditingRules,
        text: &str,
        column: usize,
        kind: IndentationMatch,
        decreases: bool,
    ) -> Option<usize> {
        let explicit = rules
            .enter
            .action(text, "", "")
            .map_err(|error| self.editor.report_error(error.to_string()))
            .ok()?;
        let bracket = bracket_enter(&rules.enter_brackets, text, "");
        if let Some(action) = explicit.or(bracket.as_ref()) {
            let result = expand_enter_action(action, text, "", self.options);
            let indentation = &result.before_caret[..indentation_len(&result.before_caret)];
            return Some(self.adjust(indentation_column(indentation, self.size), false, decreases));
        }
        Some(self.adjust(column, kind.increase || kind.next_line, decreases))
    }

    fn adjust(&self, column: usize, increase: bool, decrease: bool) -> usize {
        let column = if increase {
            (column / self.size + 1) * self.size
        } else {
            column
        };
        if decrease {
            column.saturating_sub(1) / self.size * self.size
        } else {
            column
        }
    }
}

fn token_kind(tokens: &[InputToken], column: usize) -> StandardTokenType {
    let index = tokens
        .partition_point(|token| token.start <= column)
        .saturating_sub(1);
    tokens
        .get(index)
        .map_or(StandardTokenType::Other, |token| token.kind)
}
