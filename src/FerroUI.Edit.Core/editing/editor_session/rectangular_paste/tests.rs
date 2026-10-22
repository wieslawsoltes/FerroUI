use super::*;
use crate::document::TextLocation;

fn editor(text: &str) -> EditorSession {
    let editor = EditorSession::new(TextDocument::from_text(text));
    editor.set_options(EditorOptions {
        enable_virtual_space: true,
        convert_tabs_to_spaces: true,
        ..EditorOptions::default()
    });
    editor
}

fn caret(
    editor: &EditorSession,
    line: usize,
    column: usize,
    virtual_space: usize,
) -> SelectionSegment {
    let line = editor.document().get_line_by_number(line).unwrap();
    SelectionSegment::with_virtual_space(
        line.offset + column,
        virtual_space,
        line.offset + column,
        virtual_space,
    )
}

#[test]
fn rectangle_paste_preserves_suffixes_and_delimiters_in_one_undo_unit() {
    let original = "abcd\r\nxyzw\rpqrs\nlast";
    let editor = editor(original);
    editor.set_caret_offset(1).unwrap();
    let changes = Rc::new(Cell::new(0));
    let listener = changes.clone();
    let subscription = editor
        .document()
        .text_changed(move || listener.set(listener.get() + 1));
    assert!(editor.paste_rectangle("1\r\n22\r😀", false).unwrap());
    assert_eq!(editor.document().text(), "a1bcd\r\nx22yzw\rp😀qrs\nlast");
    assert_eq!(changes.get(), 1);
    assert_eq!(
        editor
            .document()
            .get_location(editor.caret_offset())
            .unwrap(),
        TextLocation::new(3, 3)
    );
    assert!(!editor.selection().is_rectangular());
    assert_eq!(editor.selection().segments().len(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), original);
    assert!(!editor.document().can_undo());
    assert!(editor.redo().unwrap());
    assert_eq!(editor.document().text(), "a1bcd\r\nx22yzw\rp😀qrs\nlast");
    assert!(!editor.document().can_redo());
    subscription.dispose();
}

#[test]
fn all_input_newlines_and_trailing_empty_row_preserve_document_newlines() {
    for separator in ["\n", "\r", "\r\n"] {
        let editor = editor("a\r\nb\rc\nlast");
        editor.set_caret_offset(1).unwrap();
        assert!(editor
            .paste_rectangle(&format!("Q{separator}R{separator}"), true)
            .unwrap());
        assert_eq!(editor.document().text(), "aQ\r\nbR\rc\nlast");
        assert_eq!(editor.selection().segments().len(), 3);
        assert!(editor.selection().segments()[2].is_empty());
        assert_eq!(editor.selected_text().unwrap(), "Q\nR\n");
    }
    let editor = editor("a\r\nb\rc\nd");
    editor.set_caret_offset(1).unwrap();
    assert!(editor.paste_rectangle("Q\r\nR\nS\rT", false).unwrap());
    assert_eq!(editor.document().text(), "aQ\r\nbR\rcS\ndT");
}

#[test]
fn insufficient_rows_return_false_and_leave_ordinary_paste_to_the_caller() {
    let editor = editor("x\ny");
    editor.set_caret_offset(2).unwrap();
    let selection = editor.selection();
    assert!(!editor.paste_rectangle("A\r\nB", true).unwrap());
    assert_eq!(editor.document().text(), "x\ny");
    assert_eq!(editor.selection(), selection);
    assert_eq!(editor.document().version().revision(), 0);
    assert!(!editor.document().can_undo());
    editor.replace_selection_with_text("A\r\nB").unwrap();
    assert_eq!(editor.document().text(), "x\nA\r\nBy");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "x\ny");

    editor.set_caret_offset(3).unwrap();
    let selection = editor.selection();
    assert!(!editor.paste_rectangle("A\n", false).unwrap());
    assert_eq!(editor.document().text(), "x\ny");
    assert_eq!(editor.selection(), selection);
    assert!(editor.document().can_redo());
}

#[test]
fn nonempty_or_multiple_selections_return_false_without_changing_state() {
    for selection in [
        Selection::simple(0, 1),
        Selection::from_segments([SelectionSegment::empty(0), SelectionSegment::empty(2)], 1),
        Selection::from_rectangle_segments([SelectionSegment::with_virtual_space(1, 1, 1, 3)], 0),
    ] {
        let editor = editor("a\nb");
        editor.set_selection(selection.clone()).unwrap();
        assert!(!editor.paste_rectangle("Q\nR", false).unwrap());
        assert_eq!(editor.document().text(), "a\nb");
        assert_eq!(editor.selection(), selection);
        assert!(!editor.document().can_undo());
    }
}

#[test]
fn disabled_virtual_space_checks_final_row_but_can_pad_short_intermediate_rows() {
    let editor = EditorSession::new(TextDocument::from_text("abcdef\nx\nabcdef"));
    editor.set_caret_offset(4).unwrap();
    let selection = editor.selection();
    assert!(!editor.paste_rectangle("Q\nR", false).unwrap());
    assert_eq!(editor.document().text(), "abcdef\nx\nabcdef");
    assert_eq!(editor.selection(), selection);
    assert!(!editor.document().can_undo());

    assert!(editor.paste_rectangle("Q\nR\nS", false).unwrap());
    assert_eq!(editor.document().text(), "abcdQef\nx   R\nabcdSef");
    editor.undo().unwrap();
    editor.set_options(EditorOptions {
        enable_virtual_space: true,
        ..EditorOptions::default()
    });
    editor.set_caret_offset(4).unwrap();
    assert!(editor.paste_rectangle("Q\nR", false).unwrap());
    assert_eq!(editor.document().text(), "abcdQef\nx   R\nabcdef");
}

#[test]
fn tab_padding_and_utf16_selection_exclude_materialized_virtual_space() {
    let editor = EditorSession::new(TextDocument::from_text("\t\n\nx"));
    editor.set_options(EditorOptions {
        enable_virtual_space: true,
        ..EditorOptions::default()
    });
    editor.set_caret_position(1, 2).unwrap();
    assert!(editor.paste_rectangle("A\nB\n😀", true).unwrap());
    assert_eq!(editor.document().text(), "\t  A\n\t  B\nx     😀");
    assert!(editor.selection().is_rectangular());
    assert_eq!(editor.selection().primary_index(), 2);
    assert_eq!(editor.selected_text().unwrap(), "A\nB\n😀");
    assert_eq!(
        editor
            .selection()
            .segments()
            .iter()
            .map(|segment| segment.length())
            .collect::<Vec<_>>(),
        [1, 1, 2]
    );
    editor.undo().unwrap();
    assert_eq!(editor.document().text(), "\t\n\nx");
    assert!(!editor.document().can_undo());
}

#[test]
fn logical_projection_keeps_surrogates_combining_clusters_and_tabs_intact() {
    let editor = editor("😀e\u{301}Z\n\tk\nx");
    editor.set_caret_offset(4).unwrap();
    assert!(editor.paste_rectangle("Q\nR\nS", true).unwrap());
    assert_eq!(editor.document().text(), "😀e\u{301}QZ\n\tRk\nx S");
    assert_eq!(editor.selected_text().unwrap(), "Q\nR\nS");
    assert_eq!(
        editor.selection().segments()[0],
        SelectionSegment::new(4, 5)
    );
}

#[test]
fn prepared_native_positions_can_use_different_columns_and_skip_folded_rows() {
    let editor = editor("abcd\nhidden\nwx\nz");
    let positions =
        Selection::from_rectangle_segments([caret(&editor, 1, 2, 0), caret(&editor, 3, 2, 1)], 0);
    editor.set_caret_offset(0).unwrap();
    editor.paste_rectangle_at(&positions, "Q\nR", true).unwrap();
    assert_eq!(editor.document().text(), "abQcd\nhidden\nwx R\nz");
    assert_eq!(editor.selected_text().unwrap(), "Q\nR");
    assert_eq!(editor.selection().primary_index(), 0);
    assert!(editor.selection().is_rectangular());
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "abcd\nhidden\nwx\nz");
    assert!(!editor.document().can_undo());
}

#[test]
fn invalid_prepared_rectangles_do_not_touch_live_selection_or_history() {
    for (positions, text) in [
        (Selection::empty(0), "Q"),
        (
            Selection::from_rectangle_segments([SelectionSegment::new(0, 1)], 0),
            "Q",
        ),
        (
            Selection::from_rectangle_segments(
                [SelectionSegment::empty(0), SelectionSegment::empty(1)],
                0,
            ),
            "Q\nR",
        ),
        (
            Selection::from_rectangle_segments(
                [SelectionSegment::empty(0), SelectionSegment::empty(4)],
                0,
            ),
            "Q",
        ),
        (
            Selection::from_rectangle_segments([SelectionSegment::empty(0)], 0),
            "Q\nR",
        ),
        (
            Selection::from_rectangle_segments([SelectionSegment::empty(3)], 0),
            "Q",
        ),
        (
            Selection::from_rectangle_segments([SelectionSegment::empty(99)], 0),
            "Q",
        ),
        (
            Selection::from_rectangle_segments(
                [SelectionSegment::with_virtual_space(1, 2, 1, 2)],
                0,
            ),
            "Q",
        ),
        (
            Selection::from_rectangle_segments(
                [
                    SelectionSegment::with_virtual_space(2, 1, 2, 1),
                    SelectionSegment::with_virtual_space(2, 3, 2, 3),
                ],
                0,
            ),
            "Q\nR",
        ),
    ] {
        let editor = editor("ab\r\ncd");
        editor.set_caret_offset(5).unwrap();
        let selection = editor.selection();
        assert!(editor.paste_rectangle_at(&positions, text, true).is_err());
        assert_eq!(editor.document().text(), "ab\r\ncd");
        assert_eq!(editor.selection(), selection);
        assert_eq!(editor.document().version().revision(), 0);
        assert!(!editor.document().can_undo());
    }
}

struct DenyInsert(usize);
impl IReadOnlySectionProvider for DenyInsert {
    fn can_insert(&self, offset: usize) -> bool {
        offset != self.0
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        vec![segment]
    }
}

#[test]
fn any_protected_row_rejects_every_insertion_and_all_virtual_padding() {
    for protected in [1, 3, 5] {
        let editor = editor("a\nb\nc");
        editor.set_caret_position(1, 2).unwrap();
        editor.set_read_only_section_provider(Rc::new(DenyInsert(protected)));
        let selection = editor.selection();
        assert!(matches!(
            editor.paste_rectangle("Q\nR\nS", false),
            Err(EditError::ReadOnly)
        ));
        assert_eq!(editor.document().text(), "a\nb\nc");
        assert_eq!(editor.selection(), selection);
        assert!(!editor.document().can_undo());
    }
}

#[test]
fn read_only_document_and_session_are_checked_even_for_empty_input() {
    for document_read_only in [false, true] {
        let editor = editor("a\nb");
        if document_read_only {
            editor.document().set_is_read_only(true);
        } else {
            editor.set_is_read_only(true);
        }
        for text in ["Q\nR", ""] {
            assert!(matches!(
                editor.paste_rectangle(text, false),
                Err(EditError::ReadOnly)
            ));
            assert_eq!(editor.document().text(), "a\nb");
            assert_eq!(editor.selection(), Selection::default());
            assert!(!editor.document().can_undo());
        }
    }
}

#[test]
fn update_started_protection_changes_abort_before_first_row() {
    let editor = editor("a\nb");
    editor.set_caret_position(1, 2).unwrap();
    let before = editor.selection();
    let listener = editor.clone();
    let subscription = editor.document().update_started(move || {
        listener.set_read_only_section_provider(Rc::new(DenyInsert(3)));
    });
    assert!(matches!(
        editor.paste_rectangle("Q\nR", true),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.document().text(), "a\nb");
    assert_eq!(editor.selection(), before);
    assert!(!editor.document().can_undo());
    subscription.dispose();
}

#[test]
fn update_started_edits_invalidate_coordinates_without_overwriting_selection() {
    let editor = editor("a\nb");
    editor.set_caret_position(1, 2).unwrap();
    let document = editor.document().clone();
    let subscription = editor
        .document()
        .update_started(move || document.insert(0, "!").unwrap());
    assert!(matches!(
        editor.paste_rectangle("Q\nR", true),
        Err(EditError::StaleEditPlan)
    ));
    assert_eq!(editor.document().text(), "!a\nb");
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 2));
    assert!(!editor.selection().is_rectangular());
    subscription.dispose();
    editor.undo().unwrap();
    assert_eq!(editor.document().text(), "a\nb");
    assert!(!editor.document().can_undo());
}

struct EditDuringPolicy {
    document: TextDocument,
    edited: Cell<bool>,
}
impl IReadOnlySectionProvider for EditDuringPolicy {
    fn can_insert(&self, _offset: usize) -> bool {
        if !self.edited.replace(true) {
            self.document.insert(0, "!").unwrap();
        }
        true
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        vec![segment]
    }
}

#[test]
fn even_first_preflight_policy_callbacks_cannot_make_a_stale_plan_commit() {
    let editor = editor("a\nb");
    editor.set_caret_offset(1).unwrap();
    editor.set_read_only_section_provider(Rc::new(EditDuringPolicy {
        document: editor.document().clone(),
        edited: Cell::new(false),
    }));
    assert!(matches!(
        editor.paste_rectangle("Q\nR", false),
        Err(EditError::StaleEditPlan)
    ));
    assert_eq!(editor.document().text(), "!a\nb");
    assert_eq!(editor.caret_offset(), 2);
    editor.undo().unwrap();
    assert_eq!(editor.document().text(), "a\nb");
    assert!(!editor.document().can_undo());
}

#[test]
fn empty_rows_never_pad_and_single_empty_input_preserves_virtual_caret() {
    let editor = editor("a\nb");
    editor.set_caret_position(1, 3).unwrap();
    let selection = editor.selection();
    assert!(editor.paste_rectangle("", false).unwrap());
    assert_eq!(editor.selection(), selection);
    assert_eq!(editor.document().version().revision(), 0);
    assert!(!editor.document().can_undo());
    assert!(editor.paste_rectangle("Q\n", true).unwrap());
    assert_eq!(editor.document().text(), "a   Q\nb");
    assert_eq!(editor.selected_text().unwrap(), "Q\n");
    assert_eq!(editor.selection().segments()[1].caret_tuple(), (7, 3));
    editor.undo().unwrap();
    assert_eq!(editor.document().text(), "a\nb");
    assert!(!editor.document().can_undo());
}

#[test]
fn excessive_virtual_padding_fails_before_starting_an_update() {
    let editor = editor("a\nb");
    editor.set_caret_position(1, usize::MAX).unwrap();
    let before = editor.selection();
    let starts = Rc::new(Cell::new(0));
    let listener = starts.clone();
    let subscription = editor
        .document()
        .update_started(move || listener.set(listener.get() + 1));
    assert!(matches!(
        editor.paste_rectangle("Q\nR", false),
        Err(EditError::VirtualSpaceTooLarge)
    ));
    assert_eq!(starts.get(), 0);
    assert_eq!(editor.document().text(), "a\nb");
    assert_eq!(editor.selection(), before);
    assert!(!editor.document().can_undo());
    subscription.dispose();
}

#[test]
fn selected_ranges_track_reentrant_consolidated_notification_edits() {
    let editor = editor("a\nb");
    editor.set_caret_offset(1).unwrap();
    let mirrored = Rc::new(Cell::new(false));
    let done = mirrored.clone();
    let listener = editor.clone();
    let subscription = editor.document().text_changed(move || {
        if !done.replace(true) {
            assert_eq!(listener.selected_text().unwrap(), "Q\nR");
            listener.document().insert(0, ">").unwrap();
        }
    });
    assert!(editor.paste_rectangle("Q\nR", true).unwrap());
    assert!(mirrored.get());
    assert_eq!(editor.document().text(), ">aQ\nbR");
    assert_eq!(editor.selected_text().unwrap(), "Q\nR");
    assert_eq!(
        editor.selection().segments()[0],
        SelectionSegment::new(2, 3)
    );
    subscription.dispose();
}
