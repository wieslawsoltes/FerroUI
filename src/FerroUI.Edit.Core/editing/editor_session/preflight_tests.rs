use super::*;

type EditOperation = fn(&EditorSession) -> Result<(), EditError>;

fn operations() -> [EditOperation; 5] {
    [
        |editor| editor.replace_selection_with_text("Q"),
        EditorSession::insert_new_line,
        |editor| editor.indent_selection(false),
        |editor| editor.apply_document_edits(&[DocumentEdit::new(1, 0, "Q")]),
        |editor| editor.transform_text(TextTransformation::Uppercase),
    ]
}

struct MutatingPolicy {
    document: TextDocument,
    mutated: Cell<bool>,
}
impl MutatingPolicy {
    fn mutate_once(&self) {
        if !self.mutated.replace(true) {
            self.document.insert(0, "!").unwrap();
        }
    }
}
impl IReadOnlySectionProvider for MutatingPolicy {
    fn can_insert(&self, _offset: usize) -> bool {
        self.mutate_once();
        true
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        self.mutate_once();
        vec![segment]
    }
}

#[test]
fn provider_edits_reject_original_coordinates_in_every_session_entry_point() {
    for operation in operations() {
        let editor = EditorSession::new(TextDocument::from_text("ab\ncd"));
        editor.set_caret_offset(1).unwrap();
        editor.set_read_only_section_provider(Rc::new(MutatingPolicy {
            document: editor.document().clone(),
            mutated: Cell::new(false),
        }));
        assert!(matches!(operation(&editor), Err(EditError::StaleEditPlan)));
        assert_eq!(editor.document().text(), "!ab\ncd");
        assert_eq!(editor.caret_offset(), 2);
        editor.undo().unwrap();
        assert_eq!(editor.document().text(), "ab\ncd");
        assert!(!editor.document().can_undo());
    }
}

#[test]
fn update_started_edits_reject_original_coordinates_in_every_session_entry_point() {
    for operation in operations() {
        let editor = EditorSession::new(TextDocument::from_text("ab\ncd"));
        editor.set_caret_offset(1).unwrap();
        let document = editor.document().clone();
        let subscription = editor
            .document()
            .update_started(move || document.insert(0, "!").unwrap());
        assert!(matches!(operation(&editor), Err(EditError::StaleEditPlan)));
        assert_eq!(editor.document().text(), "!ab\ncd");
        assert_eq!(editor.caret_offset(), 2);
        subscription.dispose();
        editor.undo().unwrap();
        assert_eq!(editor.document().text(), "ab\ncd");
        assert!(!editor.document().can_undo());
    }
}

#[test]
fn update_started_protection_rejects_every_session_entry_point_without_history() {
    for operation in operations() {
        let editor = EditorSession::new(TextDocument::from_text("ab\ncd"));
        editor.set_caret_offset(1).unwrap();
        let selection = editor.selection();
        let listener = editor.clone();
        let subscription = editor
            .document()
            .update_started(move || listener.set_is_read_only(true));
        assert!(matches!(operation(&editor), Err(EditError::ReadOnly)));
        assert_eq!(editor.document().text(), "ab\ncd");
        assert_eq!(editor.selection(), selection);
        assert!(!editor.document().can_undo());
        subscription.dispose();
    }
}

struct ProtectSecondPreflight {
    editor: EditorSession,
    checks: Cell<usize>,
}
impl IReadOnlySectionProvider for ProtectSecondPreflight {
    fn can_insert(&self, _offset: usize) -> bool {
        let count = self.checks.get() + 1;
        self.checks.set(count);
        if count == 2 {
            self.editor.set_is_read_only(true);
        }
        true
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        vec![segment]
    }
}

#[test]
fn second_policy_callback_cannot_enable_readonly_and_still_commit_the_edit() {
    let editor = EditorSession::new(TextDocument::from_text("ab"));
    editor.set_caret_offset(1).unwrap();
    editor.set_read_only_section_provider(Rc::new(ProtectSecondPreflight {
        editor: editor.clone(),
        checks: Cell::new(0),
    }));
    assert!(matches!(
        editor.replace_selection_with_text("Q"),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.document().text(), "ab");
    assert_eq!(editor.caret_offset(), 1);
    assert!(!editor.document().can_undo());
    editor.set_read_only_section_provider(Rc::new(NoReadOnlySections));
}

#[test]
fn mapped_transform_batch_finishes_after_accepted_policy_changes_and_retains_anchors() {
    let editor = EditorSession::new(TextDocument::from_text("ab\ncd"));
    let anchor = editor.document().create_anchor(1).unwrap();
    let document = editor.document().clone();
    let subscription = editor
        .document()
        .changed(move |_| document.set_is_read_only(true));
    editor
        .transform_text(TextTransformation::Uppercase)
        .unwrap();
    assert_eq!(editor.document().text(), "AB\nCD");
    assert_eq!(anchor.offset().unwrap(), 1);
    assert!(!anchor.is_deleted());
    assert!(editor.document().is_read_only());
    subscription.dispose();
    editor.document().set_is_read_only(false);
    editor.undo().unwrap();
    assert_eq!(editor.document().text(), "ab\ncd");
    assert!(!editor.document().can_undo());
}

#[test]
fn overlapping_plans_are_rejected_before_update_started_callbacks() {
    let editor = EditorSession::new(TextDocument::from_text("abcdef"));
    let document = editor.document().clone();
    let subscription = editor
        .document()
        .update_started(move || document.insert(0, "!").unwrap());
    assert!(matches!(
        editor.apply_document_edits(&[DocumentEdit::new(2, 3, "Q"), DocumentEdit::new(0, 3, "R")]),
        Err(EditError::Document(DocumentError::OverlappingEdits))
    ));
    assert_eq!(editor.document().text(), "abcdef");
    assert!(!editor.document().can_undo());
    subscription.dispose();
}

struct MutatingStrategy(TextDocument);
impl IndentationStrategy for MutatingStrategy {
    fn indent_line(
        &self,
        _document: &TextDocument,
        _line: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError> {
        self.0.insert(0, "!")?;
        Ok(Vec::new())
    }
    fn indent_lines(
        &self,
        _document: &TextDocument,
        _first: usize,
        _last: usize,
    ) -> Result<Vec<DocumentEdit>, IndentationError> {
        self.0.insert(0, "!")?;
        Ok(vec![DocumentEdit::new(0, 0, "Q")])
    }
}

#[test]
fn indentation_strategy_cannot_retarget_newline_or_reindent_plan_by_editing_source() {
    for newline in [false, true] {
        let editor = EditorSession::new(TextDocument::from_text("ab\ncd"));
        editor.set_caret_offset(1).unwrap();
        editor.set_indentation_strategy(Some(Rc::new(MutatingStrategy(editor.document().clone()))));
        let result = if newline {
            editor.insert_new_line()
        } else {
            editor.reindent_selection()
        };
        assert!(matches!(result, Err(EditError::StaleEditPlan)));
        assert_eq!(editor.document().text(), "!ab\ncd");
        assert_eq!(editor.caret_offset(), 2);
        editor.undo().unwrap();
        assert_eq!(editor.document().text(), "ab\ncd");
        assert!(!editor.document().can_undo());
    }
}
