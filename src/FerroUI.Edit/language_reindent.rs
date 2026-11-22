use crate::input_languages::{InputLanguages, InputToken};
use crate::language_input::{format_indentation, indentation_column, indentation_len, input_text};
use crate::text_editor::EditTarget;
use crate::TextEditor;
use ferroui_edit_core::document::{DocumentEdit, DocumentLine, TextSnapshot};
use ferroui_edit_core::editing::{EditError, EditorOptions, IndentationError};
use ferroui_textmate::{IndentationMatch, StandardTokenType};
use std::rc::Rc;

pub(crate) fn reindent(editor: &TextEditor) -> Result<(), EditError> {
    let target = EditTarget::new(editor);
    let session = editor.session();
    let strategy = session.indentation_strategy();
    let configured = editor.textmate_installation().and_then(|installation| {
        let input = installation.input_languages();
        input
            .has_indentation_rules()
            .then_some((installation, input))
    });
    let Some(((installation, input), strategy)) =
        configured.zip(strategy.filter(|strategy| strategy.allows_language_indentation()))
    else {
        let result = session.reindent_selection_checked(|| target.is_current(editor));
        editor.selection_changed();
        return result;
    };

    let document = session.document();
    let snapshot = document.create_snapshot();
    let selection = session.selection();
    let options = session.options();
    let provider = session.read_only_section_provider();
    let ranges = session.reindent_line_ranges()?;
    let last = ranges.last().map_or(1, |(_, end)| *end);
    if !installation.advance_to(document, last, last) {
        return Err(
            IndentationError::LanguageRules(installation.last_error().unwrap_or_else(|| {
                "Language tokens are unavailable for the reindent range".to_owned()
            }))
            .into(),
        );
    }

    let mut edits = Vec::new();
    for (begin, end) in ranges {
        let mut planner = ReindentPlanner::new(&input, &options);
        // A selected range inherits the existing indentation of the line above it.
        for number in begin.saturating_sub(1).max(1)..=end {
            let line = snapshot.get_line_by_number(number)?;
            let tokens = installation
                .input_tokens(document, number, &input)
                .ok_or(EditError::StaleEditPlan)?;
            if let Some(edit) = planner.plan_line(&snapshot, line, &tokens, number >= begin)? {
                edits.push(edit);
            }
        }
    }
    if edits.is_empty() {
        return Ok(());
    }
    editor.apply_checked_document_edits(document, &edits, || {
        let session = editor.session();
        target.is_current(editor)
            && session.selection() == selection
            && session.options() == options
            && Rc::ptr_eq(&provider, &session.read_only_section_provider())
            && session
                .indentation_strategy()
                .is_some_and(|current| Rc::ptr_eq(&current, &strategy))
            && editor
                .textmate_installation()
                .is_some_and(|current| Rc::ptr_eq(&current, &installation))
            && Rc::ptr_eq(&installation.input_languages(), &input)
    })
}

struct ReindentPlanner<'a> {
    input: &'a InputLanguages,
    options: &'a EditorOptions,
    size: usize,
    scopes: Vec<IndentationScope>,
}

struct IndentationScope {
    id: u8,
    floor: Option<usize>,
    level: Option<IndentationLevel>,
}

struct IndentationLevel {
    base: usize,
    next: usize,
}

impl<'a> ReindentPlanner<'a> {
    fn new(input: &'a InputLanguages, options: &'a EditorOptions) -> Self {
        Self {
            input,
            options,
            size: options.indentation_size.clamp(1, 256),
            scopes: Vec::new(),
        }
    }

    fn plan_line(
        &mut self,
        snapshot: &TextSnapshot,
        line: DocumentLine,
        tokens: &[InputToken],
        editable: bool,
    ) -> Result<Option<DocumentEdit>, EditError> {
        if tokens
            .first()
            .is_some_and(|token| token.kind == StandardTokenType::String)
        {
            // Leading whitespace inside a multi-line literal is part of its value.
            return Ok(None);
        }
        let raw = snapshot.get_text(line.offset, line.length)?;
        let old = &raw[..indentation_len(&raw)];
        let column = indentation_column(old, self.size);
        let root = self.input.grammar.initial_language;
        let first_content = tokens
            .partition_point(|token| token.start <= old.len())
            .saturating_sub(1);
        let leading = tokens.get(first_content).map_or_else(
            || self.scopes.last().map_or(root, |scope| scope.id),
            |token| token.id,
        );
        let trailing = tokens.last().map_or(leading, |token| token.id);
        let sections = tokens.chunk_by(|first, second| first.id == second.id);
        let mut text = old.to_owned();
        let mut last = None;
        for section in sections {
            let first = section.first().unwrap();
            let end = section.last().unwrap().end;
            let range = first.start.max(old.len())..end;
            if range.is_empty() {
                continue;
            }
            if first.id == leading {
                text.push_str(&input_text(self.input, snapshot, line, range, section)?);
            } else if first.id == trailing {
                last = Some((range, section));
            }
        }
        let target = self.indent_scope(leading, &text, column, !editable, line.line_number)?;
        if let Some((range, section)) = last {
            // The last scope can continue on the following line or close a parent.
            let suffix = input_text(self.input, snapshot, line, range, section)?;
            self.indent_scope(
                trailing,
                &suffix,
                target.unwrap_or(column),
                false,
                line.line_number,
            )?;
        }
        let Some(target) = target.filter(|_| editable) else {
            return Ok(None);
        };
        let indent = format_indentation(target, self.options, self.size);
        Ok((indent != old).then(|| DocumentEdit::new(line.offset, old.len(), indent)))
    }

    fn scope(&mut self, id: u8) -> &mut IndentationScope {
        if let Some(index) = self.scopes.iter().rposition(|scope| scope.id == id) {
            self.scopes.truncate(index + 1);
        } else {
            if id == self.input.grammar.initial_language {
                self.scopes.clear();
            }
            let floor = self
                .scopes
                .last()
                .and_then(|scope| scope.level.as_ref().map(|level| level.next));
            self.scopes.push(IndentationScope {
                id,
                floor,
                level: None,
            });
        }
        self.scopes.last_mut().unwrap()
    }

    fn indent_scope(
        &mut self,
        id: u8,
        text: &str,
        actual: usize,
        anchor: bool,
        line: usize,
    ) -> Result<Option<usize>, EditError> {
        let size = self.size;
        let options = self.options;
        let rules = self
            .input
            .get(id)
            .and_then(|rules| rules.indentation.as_ref());
        let scope = self.scope(id);
        let Some(rules) = rules else { return Ok(None) };
        let classify = |text: &str| {
            rules
                .classify(text)
                .map_err(|error| IndentationError::LanguageRules(format!("Line {line}: {error}")))
        };
        let original = classify(text)?;
        if scope.level.is_none() && (anchor || scope.floor.is_none()) {
            if !original.ignore {
                let mut level = IndentationLevel {
                    base: actual,
                    next: actual,
                };
                level.advance(original, size);
                scope.level = Some(level);
            }
            return Ok(None);
        }

        let floor = scope.floor.unwrap_or(0);
        let level = scope.level.get_or_insert(IndentationLevel {
            base: floor,
            next: floor,
        });
        let old_prefix = indentation_len(text);
        let expected_prefix = format_indentation(level.next, options, size);
        let rule = if original.ignore || text[..old_prefix] == expected_prefix {
            original
        } else {
            classify(&format!("{expected_prefix}{}", &text[old_prefix..]))?
        };
        if rule.decrease {
            let unshift = |column: usize| (column.saturating_sub(1) / size * size).max(floor);
            level.base = unshift(level.base);
            level.next = unshift(level.next);
        }
        let target = level.next;
        level.advance(rule, size);
        Ok(Some(target))
    }
}

impl IndentationLevel {
    fn advance(&mut self, rule: IndentationMatch, size: usize) {
        if rule.ignore {
            return;
        }
        let shift = |column| (column / size + 1) * size;
        if rule.increase {
            self.base = shift(self.base);
            self.next = self.base;
        } else if rule.next_line {
            self.next = shift(self.next);
        } else {
            self.next = self.base;
        }
    }
}
