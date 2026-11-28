use crate::input_languages::InputLanguages;
use crate::language_indent_type::TypingIndentation;
use crate::{text_editor::EditTarget, TextEditor, TextMateInstallation};
use ferroui_edit_core::document::{
    AnchorSegment, DocumentEdit, DocumentLine, SimpleSegment, TextDocument, TextSnapshot,
};
use ferroui_edit_core::editing::{EditError, EditorSession, Selection, SelectionSegment};
use ferroui_textmate::{AutoPair, LanguagePairs};
use std::rc::Rc;

pub(crate) struct GeneratedPair {
    open: AnchorSegment,
    close: AnchorSegment,
    open_text: String,
    close_text: String,
}

impl GeneratedPair {
    fn new(document: &TextDocument, caret: usize, open: &str, close: &str) -> Option<Self> {
        let open_length = open.encode_utf16().count();
        let close_length = close.encode_utf16().count();
        Some(Self {
            open: AnchorSegment::new(document, caret.checked_sub(open_length)?, open_length)
                .ok()?,
            close: AnchorSegment::new(document, caret, close_length).ok()?,
            open_text: open.to_owned(),
            close_text: close.to_owned(),
        })
    }

    fn ranges(&self, document: &TextDocument) -> Option<(SimpleSegment, SimpleSegment)> {
        if self.open.start.document() != *document {
            return None;
        }
        let open = self.open.segment().ok()?;
        let close = self.close.segment().ok()?;
        let unchanged = |range: SimpleSegment, text: &str| {
            range.length == text.encode_utf16().count()
                && document
                    .get_text(range.offset, range.length)
                    .is_ok_and(|actual| actual == text)
        };
        (open.end_offset() <= close.offset
            && unchanged(open, &self.open_text)
            && unchanged(close, &self.close_text))
        .then_some((open, close))
    }
}

pub(crate) fn retain_active_pairs(
    installation: &TextMateInstallation,
    document: &TextDocument,
    selection: &Selection,
) {
    installation.generated_pairs.borrow_mut().retain(|pair| {
        pair.ranges(document).is_some_and(|(open, close)| {
            selection.segments().iter().any(|segment| {
                open.end_offset() <= segment.start_offset() && segment.end_offset() <= close.offset
            })
        })
    });
}

pub(crate) fn try_type(editor: &TextEditor, text: &str) -> Result<bool, EditError> {
    let mut characters = text.chars();
    let Some(character) = characters.next().filter(|_| characters.next().is_none()) else {
        return Ok(false);
    };
    let Some(installation) = editor.textmate_installation() else {
        return Ok(false);
    };
    let input = installation.input_languages();
    let target = EditTarget::new(editor);
    let session = editor.session();
    let document = session.document();
    let selection = session.selection();
    if session.is_read_only() || document.is_read_only() || !is_physical(&selection) {
        return Ok(false);
    }
    if selection.is_empty() && input.has_pairs() {
        if let Some(carets) = typed_closers(&installation, document, &selection, text) {
            session.set_selection(Selection::from_segments(carets, selection.primary_index()))?;
            return Ok(true);
        }
    }
    let may_pair = if selection.is_empty() {
        input.can_open_pair(character)
    } else {
        input.can_surround(text)
    };
    let strategy = session
        .indentation_strategy()
        .filter(|strategy| input.has_indentation_rules() && strategy.allows_language_indentation());
    if !may_pair && strategy.is_none() {
        return Ok(false);
    }
    let snapshot = document.create_snapshot();
    if strategy.is_some() || !input.grammar.embedded_languages.is_empty() {
        let last = selection
            .segments()
            .last()
            .expect("a selection owns a caret");
        let line = snapshot.get_line_by_offset(last.end_offset())?;
        installation.advance_to(document, line.line_number, 128);
    }
    let same_rules = || {
        editor
            .textmate_installation()
            .is_some_and(|current| Rc::ptr_eq(&current, &installation))
            && Rc::ptr_eq(&installation.input_languages(), &input)
    };
    let options = session.options();
    let indentation_is_current = || {
        let session = editor.session();
        session.options() == options
            && session
                .indentation_strategy()
                .zip(strategy.as_ref())
                .is_some_and(|(current, original)| Rc::ptr_eq(&current, original))
    };
    let mut prefixes = if strategy.is_some() {
        TypingIndentation {
            installation: &installation,
            input: &input,
            document,
            snapshot: &snapshot,
            options: &options,
        }
        .plan(editor, &selection, text)?
        .unwrap_or_default()
    } else {
        Vec::new()
    };
    if !prefixes.is_empty() {
        // Indentation is optional. A protected prefix must not consume typing
        // that the ordinary edit path can apply to the writable selection.
        let mut ranges = prefixes.clone();
        ranges.extend(
            selection
                .segments()
                .iter()
                .map(|segment| DocumentEdit::new(segment.start_offset(), segment.length(), text)),
        );
        let allowed = session.validate_document_edits(&ranges);
        if !target.is_current(editor)
            || !same_rules()
            || !indentation_is_current()
            || session.selection() != selection
        {
            return Err(EditError::StaleEditPlan);
        }
        match allowed {
            Err(EditError::ReadOnly) => prefixes.clear(),
            result => result?,
        }
    }

    let current = || {
        target.is_current(editor)
            && same_rules()
            && (prefixes.is_empty() || indentation_is_current())
    };
    let contexts: Option<Vec<_>> = if may_pair && (selection.is_empty() || prefixes.is_empty()) {
        selection
            .segments()
            .iter()
            .map(|segment| {
                InputContext::new(
                    &snapshot,
                    segment.start_offset(),
                    &installation,
                    document,
                    &input,
                )
            })
            .collect::<Result<_, _>>()?
    } else {
        None
    };
    let opening = match contexts.as_deref() {
        Some(contexts) if !selection.is_empty() => {
            return surround(editor, contexts, &selection, text)
        }
        Some(contexts) => match_opening_pair(editor, &installation, document, contexts, text),
        None => None,
    };
    let Some((pair, closing)) = opening else {
        if prefixes.is_empty() {
            return Ok(false);
        }
        replace_at_carets(
            &session,
            &selection,
            &prefixes,
            text,
            character.len_utf16(),
            current,
        )?;
        return Ok(true);
    };

    let (intended_selection, expected_changes) = if pair.open != text && !document.is_in_update() {
        let (after_opener, changes) = replace_at_carets(
            &session,
            &selection,
            &prefixes,
            text,
            character.len_utf16(),
            || {
                target.is_current(editor)
                    && (prefixes.is_empty() || same_rules() && indentation_is_current())
            },
        )?;
        // Two undo groups keep the typed opener and its indentation when the
        // generated text is undone. A host can take over after the first group.
        let can_close = || {
            target.is_current_after(editor, changes)
                && same_rules()
                && editor.selection() == after_opener
        };
        if !can_close() {
            return Ok(true);
        }
        match replace_at_carets(&session, &after_opener, &[], closing, 0, can_close) {
            Ok((selection, followup)) => (selection, changes + followup),
            Err(EditError::ReadOnly | EditError::StaleEditPlan) => return Ok(true),
            Err(error) => return Err(error),
        }
    } else {
        replace_at_carets(
            &session,
            &selection,
            &prefixes,
            &format!("{text}{closing}"),
            character.len_utf16(),
            current,
        )?
    };
    if target.is_current_after(editor, expected_changes)
        && editor.selection() == intended_selection
        && same_rules()
    {
        let mut generated = installation.generated_pairs.borrow_mut();
        generated.extend(intended_selection.segments().iter().filter_map(|caret| {
            GeneratedPair::new(document, caret.caret, &pair.open, closing)
                .filter(|pair| pair.ranges(document).is_some())
        }));
    }
    Ok(true)
}

fn match_opening_pair<'a>(
    editor: &TextEditor,
    installation: &TextMateInstallation,
    document: &TextDocument,
    contexts: &[InputContext<'a>],
    text: &str,
) -> Option<(&'a AutoPair, &'a str)> {
    let matched = match_openers(contexts, text)
        .map_err(|error| editor.report_error(error.to_string()))
        .ok()??;
    let pair = matched[0];
    if contexts
        .iter()
        .zip(&matched)
        .any(|(context, pair)| !context.allows_pair(installation, document, pair, text))
    {
        return None;
    }
    if !is_quote(text)
        && installation.has_unmatched_closer_at_carets(
            document,
            &pair.open,
            contexts
                .iter()
                .map(|context| (context.language, context.line.offset + context.column)),
        )
    {
        return None;
    }
    let contained = contexts[0]
        .rules
        .auto_closing
        .iter()
        .filter(|candidate| {
            candidate.open != pair.open
                && pair.open.contains(&candidate.open)
                && pair.close.ends_with(&candidate.close)
                && contexts.iter().all(|context| {
                    context.after.starts_with(&candidate.close)
                        && context.rules.auto_closing.iter().any(|pair| {
                            pair.open == candidate.open && pair.close == candidate.close
                        })
                })
        })
        .max_by_key(|candidate| candidate.open.len());
    let closing = &pair.close[..pair.close.len() - contained.map_or(0, |pair| pair.close.len())];
    (!closing.is_empty()).then_some((pair, closing))
}

fn replace_at_carets(
    session: &EditorSession,
    selection: &Selection,
    prefixes: &[DocumentEdit],
    text: &str,
    caret_advance: usize,
    is_current: impl Fn() -> bool,
) -> Result<(Selection, usize), EditError> {
    let mut edits = Vec::with_capacity(selection.segments().len() + prefixes.len());
    let mut carets = Vec::with_capacity(selection.segments().len());
    let inserted_length = text.encode_utf16().count();
    let (mut old_end, mut new_end) = (0, 0);
    for (index, segment) in selection.segments().iter().enumerate() {
        let start = segment.start_offset();
        let prefix = prefixes.get(index);
        let inline = prefix.filter(|edit| edit.offset + edit.length == start);
        if let Some(prefix) = prefix.filter(|_| inline.is_none()) {
            new_end += prefix.offset - old_end + prefix.text.len();
            old_end = prefix.offset + prefix.length;
            edits.push(prefix.clone());
        }
        let replacement_start = inline.map_or(start, |edit| edit.offset);
        let insertion =
            new_end + (replacement_start - old_end) + inline.map_or(0, |edit| edit.text.len());
        edits.push(if let Some(prefix) = inline {
            DocumentEdit::new(
                prefix.offset,
                prefix.length + segment.length(),
                format!("{}{text}", prefix.text),
            )
        } else {
            DocumentEdit::new(start, segment.length(), text)
        });
        carets.push(SelectionSegment::empty(insertion + caret_advance));
        old_end = segment.end_offset();
        new_end = insertion + inserted_length;
    }
    let selection = Selection::from_segments(carets, selection.primary_index());
    session.apply_document_edits_and_select(&edits, selection.clone(), is_current)?;
    Ok((selection, edits.len()))
}

fn is_physical(selection: &Selection) -> bool {
    !selection.is_rectangular()
        && selection
            .segments()
            .iter()
            .all(|segment| segment.start_virtual_space() == 0 && segment.end_virtual_space() == 0)
}

fn is_quote(text: &str) -> bool {
    matches!(text, "'" | "\"" | "`")
}

fn typed_closers(
    installation: &TextMateInstallation,
    document: &TextDocument,
    selection: &Selection,
    text: &str,
) -> Option<Vec<SelectionSegment>> {
    let generated = installation.generated_pairs.borrow();
    if generated.is_empty() {
        return None;
    }
    selection
        .segments()
        .iter()
        .map(|segment| {
            let caret = segment.caret;
            if is_quote(text)
                && caret
                    .checked_sub(1)
                    .is_some_and(|offset| document.get_char_at(offset) == Ok(b'\\' as u16))
            {
                return None;
            }
            generated
                .iter()
                .filter(|pair| pair.close_text == text)
                .find_map(|pair| {
                    let (_, close) = pair.ranges(document)?;
                    (close.offset == caret).then_some(SelectionSegment::empty(close.end_offset()))
                })
        })
        .collect()
}

pub(crate) fn try_backspace(editor: &TextEditor) -> Result<bool, EditError> {
    let Some(installation) = editor.textmate_installation() else {
        return Ok(false);
    };
    let session = editor.session();
    let selection = session.selection();
    if !selection.is_empty() || !is_physical(&selection) {
        return Ok(false);
    }
    let document = session.document();
    let Some(ranges): Option<Vec<_>> = ({
        let pairs = installation.generated_pairs.borrow();
        selection
            .segments()
            .iter()
            .map(|segment| {
                pairs.iter().find_map(|pair| {
                    let (open, close) = pair.ranges(document)?;
                    (open.end_offset() == segment.caret
                        && close.offset == segment.caret
                        && pair.open_text.chars().count() == 1
                        && pair.close_text.chars().count() == 1)
                        .then_some(SimpleSegment::new(open.offset, open.length + close.length))
                })
            })
            .collect()
    }) else {
        return Ok(false);
    };
    let mut removed = 0;
    let mut carets = Vec::with_capacity(ranges.len());
    let mut edits = Vec::with_capacity(ranges.len());
    for range in ranges {
        let caret = range
            .offset
            .checked_sub(removed)
            .ok_or(EditError::InvalidSelection)?;
        carets.push(SelectionSegment::empty(caret));
        removed += range.length;
        edits.push(DocumentEdit::new(range.offset, range.length, ""));
    }
    apply_pair_edits(
        editor,
        &edits,
        Selection::from_segments(carets, selection.primary_index()),
    )?;
    Ok(true)
}

fn surround(
    editor: &TextEditor,
    contexts: &[InputContext<'_>],
    selection: &Selection,
    text: &str,
) -> Result<bool, EditError> {
    let pairs: Option<Vec<_>> = contexts
        .iter()
        .map(|context| {
            context
                .rules
                .surrounding
                .iter()
                .rev()
                .find(|pair| pair.open == text)
        })
        .collect();
    let Some(pairs) = pairs else { return Ok(false) };
    let pair = pairs[0];
    if pairs.iter().any(|candidate| candidate.close != pair.close) {
        return Ok(false);
    }
    let session = editor.session();
    let snapshot = session.document().create_snapshot();
    let mut edits = Vec::with_capacity(selection.segments().len() * 2);
    let mut selections = Vec::with_capacity(selection.segments().len());
    let open_length = pair.open.encode_utf16().count();
    let close_length = pair.close.encode_utf16().count();
    let mut inserted = 0;
    for segment in selection.segments() {
        let selected = snapshot.get_text(segment.start_offset(), segment.length())?;
        if selected.trim_matches([' ', '\t', '\r', '\n']).is_empty()
            || is_quote(text) && is_quote(&selected)
        {
            return Ok(false);
        }
        edits.push(DocumentEdit::new(segment.start_offset(), 0, &pair.open));
        edits.push(DocumentEdit::new(segment.end_offset(), 0, &pair.close));
        let start = segment.start_offset() + inserted + open_length;
        let end = start + segment.length();
        selections.push(if segment.is_reversed() {
            SelectionSegment::new(end, start)
        } else {
            SelectionSegment::new(start, end)
        });
        inserted += open_length + close_length;
    }
    apply_pair_edits(
        editor,
        &edits,
        Selection::from_segments(selections, selection.primary_index()),
    )?;
    Ok(true)
}

fn apply_pair_edits(
    editor: &TextEditor,
    edits: &[DocumentEdit],
    selection: Selection,
) -> Result<(), EditError> {
    let target = EditTarget::new(editor);
    editor
        .session()
        .apply_document_edits_and_select(edits, selection, || target.is_current(editor))
}

fn match_openers<'a>(
    contexts: &[InputContext<'a>],
    text: &str,
) -> ferroui_textmate::Result<Option<Vec<&'a AutoPair>>> {
    let mut matched: Option<Vec<&AutoPair>> = None;
    for candidate in &contexts[0].rules.auto_closing {
        if !candidate.open.ends_with(text)
            || matched
                .as_ref()
                .is_some_and(|old| old[0].open.len() >= candidate.open.len())
        {
            continue;
        }
        let mut all = Vec::with_capacity(contexts.len());
        for context in contexts {
            let Some(pair) = context.match_opener(candidate, text)? else {
                break;
            };
            all.push(pair);
        }
        if all.len() == contexts.len() {
            matched = Some(all);
        }
    }
    Ok(matched)
}

struct InputContext<'a> {
    before: String,
    after: String,
    line: DocumentLine,
    column: usize,
    language: u8,
    rules: &'a LanguagePairs,
}

impl<'a> InputContext<'a> {
    fn new(
        snapshot: &TextSnapshot,
        offset: usize,
        installation: &TextMateInstallation,
        document: &TextDocument,
        input: &'a InputLanguages,
    ) -> Result<Option<Self>, EditError> {
        let line = snapshot.get_line_by_offset(offset)?;
        let column = (offset - line.offset).min(line.length);
        let Some(scope) =
            installation.input_scope(document, input, line.line_number, column, line.length)
        else {
            return Ok(None);
        };
        let Some(rules) = input.get(scope.id) else {
            return Ok(None);
        };
        Ok(Some(Self {
            before: snapshot.get_text(line.offset + scope.start, column - scope.start)?,
            after: snapshot.get_text(line.offset + column, scope.end - column)?,
            line,
            column,
            language: scope.id,
            rules: &rules.pairs,
        }))
    }

    fn match_opener(
        &self,
        common: &AutoPair,
        typed: &str,
    ) -> ferroui_textmate::Result<Option<&'a AutoPair>> {
        for pair in &self.rules.auto_closing {
            if pair.open == common.open
                && pair.close == common.close
                && self.rules.matches_opener(pair, typed, &self.before)?
            {
                return Ok(Some(pair));
            }
        }
        Ok(None)
    }

    fn allows_pair(
        &self,
        installation: &TextMateInstallation,
        document: &TextDocument,
        pair: &AutoPair,
        text: &str,
    ) -> bool {
        if !self.rules.allows_following(&self.after, is_quote(text)) {
            return false;
        }
        if matches!(text, "'" | "\"")
            && pair.open == text
            && self.before.chars().next_back().is_some_and(|previous| {
                previous.is_alphanumeric() || matches!(previous, '_' | '\\')
            })
        {
            return false;
        }
        if pair.not_in.is_empty() {
            return true;
        }
        let neutral = ('0'..='9')
            .chain('a'..='z')
            .chain('A'..='Z')
            .find(|&character| !pair.open.contains(character) && !pair.close.contains(character));
        neutral.is_none_or(|character| {
            let Ok(mut prospective) = document.get_text(self.line.offset, self.column) else {
                return false;
            };
            let Ok(after) = document.get_text(
                self.line.offset + self.column,
                self.line.length - self.column,
            ) else {
                return false;
            };
            prospective.push(character);
            prospective.push_str(&after);
            installation
                .prospective_token_type(document, self.line.line_number, &prospective, self.column)
                .is_some_and(|token| pair.allows_token_type(token))
        })
    }
}
