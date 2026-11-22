use crate::input_languages::{InputLanguages, InputToken, LanguageScope};
use crate::text_editor::EditTarget;
use crate::{TextEditor, TextMateInstallation};
use ferroui_edit_core::document::{DocumentLine, TextSnapshot};
use ferroui_edit_core::editing::{
    EditError, EditorOptions, LineBreakInsertion, LineBreakPlan, SelectionSegment,
};
use ferroui_textmate::{EnterAction, IndentAction, IndentationMatch, StandardTokenType};
use std::ops::Range;

const INDENTATION_LOOKBACK: usize = 128;

pub(crate) fn insert_new_line(editor: &TextEditor) -> Result<(), EditError> {
    let target = EditTarget::new(editor);
    let session = editor.session();
    let is_current = || target.is_current(editor);
    let configured = editor.textmate_installation().and_then(|installation| {
        let input = installation.input_languages();
        input.has_enter_rules().then_some((installation, input))
    });
    let Some((installation, input)) = configured else {
        return session.insert_document_new_line_with_plan_checked(|_, _, _| Ok(None), is_current);
    };
    if let Some(last) = session.selection().segments().last() {
        let line = session.document().get_line_by_offset(last.end_offset())?;
        installation.advance_to(session.document(), line.line_number, INDENTATION_LOOKBACK);
    }
    let options = editor.options();
    session.insert_document_new_line_with_plan_checked(
        |snapshot, selection, newline| {
            plan_enter(editor, &installation, &input, snapshot, selection, newline, &options)
        },
        is_current,
    )
}

fn plan_enter(
    editor: &TextEditor,
    installation: &TextMateInstallation,
    input: &InputLanguages,
    snapshot: &TextSnapshot,
    selection: SelectionSegment,
    newline: &str,
    options: &EditorOptions,
) -> Result<Option<LineBreakPlan>, EditError> {
    let start_line = snapshot.get_line_by_offset(selection.start_offset())?;
    let end_line = snapshot.get_line_by_offset(selection.end_offset())?;
    let start_column = (selection.start_offset() - start_line.offset).min(start_line.length);
    let end_column = (selection.end_offset() - end_line.offset).min(end_line.length);
    let document = editor.document();
    let tokens_at = |line| installation.input_tokens(&document, line, input);
    let start_tokens = tokens_at(start_line.line_number);
    if start_tokens.is_none() && !input.grammar.embedded_languages.is_empty() {
        return Ok(None);
    }
    let start_is_tokenized = start_tokens.is_some();
    let start_tokens = start_tokens.unwrap_or_default();
    let root = input.grammar.initial_language;
    let start = LanguageScope::at(&start_tokens, root, start_column, start_line.length);
    let Some(rules) = input.get(start.id) else {
        return Ok(None);
    };
    let before = input_text(
        input,
        snapshot,
        start_line,
        start.start..start_column,
        &start_tokens,
    )?;
    let separate_end_tokens = if end_line.line_number == start_line.line_number {
        None
    } else {
        match tokens_at(end_line.line_number) {
            Some(tokens) => Some(tokens),
            None if input.grammar.embedded_languages.is_empty() => Some(Vec::new()),
            None => return Ok(None),
        }
    };
    let end_tokens = separate_end_tokens.as_deref().unwrap_or(&start_tokens);
    let end = LanguageScope::at(end_tokens, root, end_column, end_line.length);
    let after = if end.id == start.id {
        input_text(input, snapshot, end_line, end_column..end.end, end_tokens)?
    } else {
        String::new()
    };
    let previous = if !rules.enter.is_empty() && start.start == 0 && start_line.line_number > 1 {
        let line = snapshot.get_line_by_number(start_line.line_number - 1)?;
        let tokens = tokens_at(line.line_number).unwrap_or_default();
        let scope = LanguageScope::at(&tokens, root, line.length, line.length);
        if scope.id == start.id {
            input_text(input, snapshot, line, scope.start..scope.end, &tokens)?
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let explicit = match rules.enter.action(&before, &after, &previous) {
        Ok(action) => action,
        Err(error) => {
            // A malformed package expression should not prevent ordinary Enter.
            editor.report_error(error.to_string());
            return Ok(None);
        }
    };
    let raw_prefix = snapshot.get_text(start_line.offset, start_column)?;
    if let Some(action) = explicit {
        return Ok(Some(LineBreakPlan::Insert(expand_enter_action(
            action,
            &raw_prefix,
            newline,
            options,
        ))));
    }
    if !start_is_tokenized
        || !editor
            .session()
            .indentation_strategy()
            .is_some_and(|strategy| strategy.allows_language_indentation())
    {
        return Ok(None);
    }
    if let Some(action) = bracket_enter(&rules.enter_brackets, &before, &after) {
        return Ok(Some(LineBreakPlan::Insert(expand_enter_action(
            &action,
            &raw_prefix,
            newline,
            options,
        ))));
    }
    let Some(indentation) = &rules.indentation else {
        return Ok(None);
    };
    let classify = |text: &str| {
        indentation
            .classify(text)
            .map_err(|error| editor.report_error(error.to_string()))
            .ok()
    };
    let size = options.indentation_size.clamp(1, 256);
    let column = |text: &str| indentation_column(&text[..indentation_len(text)], size);
    let read_line = |number| {
        let line = snapshot.get_line_by_number(number)?;
        let Some(tokens) = tokens_at(number) else {
            return Ok(None);
        };
        let scope = LanguageScope::at(&tokens, root, line.length, line.length);
        if scope.id != start.id {
            return Ok(None);
        }
        let raw = snapshot.get_text(line.offset, line.length)?;
        let context = input_text(input, snapshot, line, scope.start..scope.end, &tokens)?;
        Ok(context_line(&context, column(&raw), &classify))
    };
    let Some(current) = context_line(&before, column(&raw_prefix), &classify) else {
        return Ok(None);
    };
    let (base, increase) = if start.start == 0 {
        inherit_indentation(start_line.line_number, current, read_line)?
    } else {
        (current.column, current.increases())
    };
    let suffix = if end.id == start.id {
        classify(&after)
    } else {
        Some(IndentationMatch::default())
    };
    let Some(suffix) = suffix else {
        return Ok(None);
    };
    let target = if increase {
        (base / size + 1) * size
    } else {
        base
    };
    let target = if suffix.decrease {
        target.saturating_sub(1) / size * size
    } else {
        target
    };
    let indent = format_indentation(target, options, size);
    let end_text = snapshot.get_text(end_line.offset, end_line.length)?;
    let existing_indent = indentation_len(&end_text);
    let caret = if existing_indent < end_text.len() && end_column <= existing_indent {
        let visual_column = column(&end_text[..end_column]);
        let characters = if options.convert_tabs_to_spaces {
            visual_column
        } else {
            visual_column.div_ceil(size)
        };
        characters.min(indent.len())
    } else {
        indent.len()
    };
    let (before_caret, after_caret) = indent.split_at(caret);
    Ok(Some(LineBreakPlan::ReplaceIndentation(
        LineBreakInsertion {
            before_caret: format!("{newline}{before_caret}"),
            after_caret: after_caret.to_owned(),
        },
    )))
}

pub(crate) fn bracket_enter(pairs: &[(String, String)], before: &str, after: &str) -> Option<EnterAction> {
    let word = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
    let before = before.trim_end();
    let after = after.trim_start();
    let mut openings = pairs
        .iter()
        .filter(|(open, _)| {
            before
                .strip_suffix(open)
                .is_some_and(|prefix| !open.starts_with(word) || !prefix.ends_with(word))
        })
        .peekable();
    openings.peek()?;
    let indent = if openings.any(|(_, close)| {
        after
            .strip_prefix(close)
            .is_some_and(|suffix| !close.ends_with(word) || !suffix.starts_with(word))
    }) {
        IndentAction::IndentOutdent
    } else {
        IndentAction::Indent
    };
    Some(EnterAction {
        indent,
        ..EnterAction::default()
    })
}

#[derive(Clone, Copy)]
pub(crate) struct IndentContextLine {
    column: usize,
    kind: Option<IndentationMatch>,
}

impl IndentContextLine {
    fn increases(self) -> bool {
        self.kind
            .is_some_and(|kind| kind.increase || kind.next_line)
    }

    fn temporary(self) -> bool {
        self.kind
            .is_some_and(|kind| kind.next_line && !kind.increase && !kind.decrease)
    }
}

pub(crate) fn context_line(
    text: &str,
    column: usize,
    classify: impl Fn(&str) -> Option<IndentationMatch>,
) -> Option<IndentContextLine> {
    let kind = if text.trim().is_empty() {
        None
    } else {
        let kind = classify(text)?;
        (!kind.ignore).then_some(kind)
    };
    Some(IndentContextLine { column, kind })
}

pub(crate) fn inherit_indentation(
    mut number: usize,
    current: IndentContextLine,
    context: impl Fn(usize) -> Result<Option<IndentContextLine>, EditError>,
) -> Result<(usize, bool), EditError> {
    let first = number.saturating_sub(INDENTATION_LOOKBACK - 1).max(1);
    let mut line = current;
    while line.kind.is_none() && number > first {
        number -= 1;
        let Some(previous) = context(number)? else {
            return Ok((current.column, false));
        };
        line = previous;
    }
    if line.increases() || line.kind.is_none_or(|kind| kind.decrease) {
        return Ok((line.column, line.increases()));
    }
    // Consecutive indent-next rules can defer that one-line indentation.
    // After the following ordinary line, return to the first rule's column.
    while number > first {
        number -= 1;
        let Some(previous) = context(number)? else {
            break;
        };
        if !previous.temporary() {
            break;
        }
        line = previous;
    }
    Ok((line.column, false))
}

pub(crate) fn input_text(
    input: &InputLanguages,
    snapshot: &TextSnapshot,
    line: DocumentLine,
    range: Range<usize>,
    tokens: &[InputToken],
) -> Result<String, EditError> {
    let Some(brackets) = &input.brackets else {
        return Ok(snapshot.get_text(line.offset + range.start, range.len())?);
    };
    let mut output = String::new();
    let mut cursor = range.start;
    for token in tokens
        .iter()
        .filter(|token| token.kind != StandardTokenType::Other)
    {
        let start = token.start.max(cursor);
        let end = token.end.min(range.end);
        if start < end {
            output.push_str(&snapshot.get_text(line.offset + cursor, start - cursor)?);
            let text = snapshot.get_text(line.offset + start, end - start)?;
            output.push_str(&brackets.remove_delimiters(token.id, &text).unwrap_or(text));
            cursor = end;
        }
    }
    output.push_str(&snapshot.get_text(line.offset + cursor, range.end - cursor)?);
    Ok(output)
}

pub(crate) fn expand_enter_action(
    action: &EnterAction,
    before: &str,
    newline: &str,
    options: &EditorOptions,
) -> LineBreakInsertion {
    let indentation = &before[..indentation_len(before)];
    let indentation = &indentation[..indentation
        .len()
        .saturating_sub(action.remove_text.unwrap_or(0))];
    let size = options.indentation_size.clamp(1, 256);
    let mut leading = indentation.to_owned();
    let append = action
        .append_text
        .as_deref()
        .filter(|text| !text.is_empty());
    match action.indent {
        IndentAction::None => leading.push_str(append.unwrap_or("")),
        IndentAction::Indent => {
            leading.push('\t');
            leading.push_str(append.unwrap_or(""));
        }
        IndentAction::Outdent => {
            let column = indentation_column(indentation, size);
            leading = format_indentation(column.saturating_sub(1) / size * size, options, size);
            leading.push_str(append.unwrap_or(""));
        }
        IndentAction::IndentOutdent => leading.push_str(append.unwrap_or("\t")),
    }
    let split = indentation_len(&leading);
    let mut before_caret = newline.to_owned();
    before_caret.push_str(&format_indentation(
        indentation_column(&leading[..split], size),
        options,
        size,
    ));
    before_caret.push_str(&leading[split..]);
    let after_caret = if action.indent == IndentAction::IndentOutdent {
        format!(
            "{newline}{}",
            format_indentation(indentation_column(indentation, size), options, size)
        )
    } else {
        String::new()
    };
    LineBreakInsertion {
        before_caret,
        after_caret,
    }
}

pub(crate) fn indentation_len(text: &str) -> usize {
    text.bytes()
        .take_while(|&byte| matches!(byte, b' ' | b'\t'))
        .count()
}

pub(crate) fn indentation_column(text: &str, size: usize) -> usize {
    text.bytes().fold(0, |column, byte| {
        if byte == b'\t' {
            column + (size - column % size)
        } else {
            column + 1
        }
    })
}

pub(crate) fn format_indentation(column: usize, options: &EditorOptions, size: usize) -> String {
    if options.convert_tabs_to_spaces {
        " ".repeat(column)
    } else {
        "\t".repeat(column / size) + &" ".repeat(column % size)
    }
}
