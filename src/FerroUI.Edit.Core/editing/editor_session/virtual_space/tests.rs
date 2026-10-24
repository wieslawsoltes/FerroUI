use super::*;
use crate::editing::RectangleSelection;

fn editor(text: &str) -> EditorSession {
    let editor = EditorSession::new(TextDocument::from_text(text));
    editor.set_options(EditorOptions {
        enable_virtual_space: true,
        convert_tabs_to_spaces: true,
        ..EditorOptions::default()
    });
    editor
}

fn rectangle(editor: &EditorSession, first: usize, last: usize, left: usize, right: usize) {
    let selection = RectangleSelection {
        anchor_line: first,
        caret_line: last,
        anchor_column: left,
        caret_column: right,
    }
    .to_document_selection(editor.document(), editor.options().indentation_size)
    .unwrap();
    editor.set_selection(selection).unwrap();
}

#[test]
fn virtual_range_direction_and_coalescing_use_both_coordinates() {
    let range = SelectionSegment::with_virtual_space(7, 5, 7, 2);
    assert_eq!(range.start_tuple(), (7, 2));
    assert_eq!(range.end_tuple(), (7, 5));
    assert_eq!(range.length(), 0);
    assert!(!range.is_empty());
    assert!(range.is_reversed());
    let selection = Selection::from_rectangle_segments(
        [
            SelectionSegment::with_virtual_space(7, 1, 7, 1),
            SelectionSegment::with_virtual_space(7, 4, 7, 4),
        ],
        1,
    );
    assert_eq!(selection.segments().len(), 2);
    assert_eq!(selection.primary().caret_tuple(), (7, 4));
    assert!(selection.is_rectangular());
    let expanded =
        Selection::from_segments([range, SelectionSegment::with_virtual_space(7, 4, 7, 8)], 0);
    assert_eq!(
        expanded.segments(),
        &[SelectionSegment::with_virtual_space(7, 8, 7, 2)]
    );
}

#[test]
fn virtual_positions_require_line_end_and_external_edits_normalize_them() {
    let editor = editor("ab\ncd");
    assert!(matches!(
        editor.set_caret_position(1, 2),
        Err(EditError::InvalidSelection)
    ));
    assert!(matches!(
        editor.set_caret_position(6, 0),
        Err(EditError::InvalidSelection)
    ));
    editor.set_caret_position(2, 3).unwrap();
    editor.document().insert(2, "XYZ").unwrap();
    assert_eq!(editor.selection().primary().caret_tuple(), (5, 3));
    editor.document().remove(5, 1).unwrap();
    assert_eq!(editor.selection().primary().caret_tuple(), (5, 0));
    assert_eq!(editor.document().text(), "abXYZcd");
}

#[test]
fn rectangle_projects_tabs_graphemes_and_crlf_without_losing_virtual_cells() {
    let editor = editor("\tabc\r\n😀e\u{301}\nx");
    rectangle(&editor, 2, 0, 6, 4);
    let selection = editor.selection();
    assert!(selection.is_rectangular());
    assert_eq!(selection.primary_index(), 0);
    assert_eq!(
        selection.segments(),
        &[
            SelectionSegment::new(3, 1),
            SelectionSegment::with_virtual_space(10, 4, 10, 2),
            SelectionSegment::with_virtual_space(12, 5, 12, 3),
        ]
    );
    editor.set_options(EditorOptions::default());
    assert_eq!(editor.selection(), selection);
}

#[test]
fn left_right_and_selection_collapsing_preserve_virtual_endpoints() {
    let editor = editor("ab\nc");
    editor.set_caret_offset(2).unwrap();
    editor.move_caret(CaretMovement::CharacterRight, false);
    editor.move_caret(CaretMovement::CharacterRight, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 2));
    editor.move_caret(CaretMovement::CharacterLeft, true);
    assert_eq!(
        editor.selection().primary(),
        SelectionSegment::with_virtual_space(2, 2, 2, 1)
    );
    editor.move_caret(CaretMovement::CharacterRight, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 2));
    editor.move_caret(CaretMovement::WordLeft, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 0));
    editor.move_caret(CaretMovement::WordRight, false);
    assert_eq!(editor.caret_offset(), 3);
    editor.move_caret(CaretMovement::CharacterLeft, false);
    assert_eq!(editor.caret_offset(), 3);
    assert!(!editor.document().can_undo());
}

#[test]
fn vertical_navigation_retains_visual_column_across_short_lines_and_tabs() {
    let editor = editor("\tabc\nx\n😀abcdef");
    editor.set_caret_offset(3).unwrap();
    editor.move_caret(CaretMovement::LineDown, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (6, 5));
    editor.move_caret(CaretMovement::LineDown, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (14, 0));
    editor.move_caret(CaretMovement::LineUp, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (6, 5));
    editor.move_caret(CaretMovement::LineUp, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (3, 0));
    editor.move_caret(CaretMovement::LineUp, false);
    assert_eq!(editor.caret_offset(), 3);
}

#[test]
fn vertical_navigation_remembers_clamped_column_when_virtual_space_is_disabled() {
    let editor = EditorSession::new(TextDocument::from_text("abcdef\nx\nabcdef"));
    editor.set_caret_offset(4).unwrap();
    editor.move_caret(CaretMovement::LineDown, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (8, 0));
    editor.move_caret(CaretMovement::LineDown, false);
    assert_eq!(editor.selection().primary().caret_tuple(), (13, 0));
    editor.set_caret_offset(10).unwrap();
    editor.move_caret(CaretMovement::LineUp, false);
    editor.move_caret(CaretMovement::LineUp, false);
    assert_eq!(editor.caret_offset(), 1);
}

#[test]
fn inserting_at_virtual_caret_materializes_padding_once_in_one_undo_unit() {
    let editor = editor("ab");
    editor.set_caret_position(2, 3).unwrap();
    editor.replace_selection_with_text("😀").unwrap();
    assert_eq!(editor.document().text(), "ab   😀");
    assert_eq!(editor.selection().primary().caret_tuple(), (7, 0));
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "ab");
    assert!(!editor.document().can_undo());
    assert!(editor.redo().unwrap());
    assert_eq!(editor.document().text(), "ab   😀");
}

#[test]
fn padding_uses_tabs_only_on_empty_or_tab_only_lines() {
    for (text, expected) in [
        ("", "\t  X"),
        ("\t", "\t\t  X"),
        ("x", "x      X"),
        (" ", "       X"),
    ] {
        let editor = EditorSession::new(TextDocument::from_text(text));
        editor.set_options(EditorOptions {
            enable_virtual_space: true,
            ..EditorOptions::default()
        });
        editor
            .set_caret_position(text.encode_utf16().count(), 6)
            .unwrap();
        editor.replace_selection_with_text("X").unwrap();
        assert_eq!(editor.document().text(), expected);
    }
}

#[test]
fn deleting_virtual_selection_and_backspacing_change_only_selection() {
    let editor = editor("ab");
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::with_virtual_space(2, 5, 2, 2)],
            0,
        ))
        .unwrap();
    editor.delete(false, false).unwrap();
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 2));
    editor.delete(true, false).unwrap();
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 1));
    editor.delete(true, true).unwrap();
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 0));
    assert_eq!(editor.document().text(), "ab");
    assert_eq!(editor.document().version().revision(), 0);
    assert!(!editor.document().can_undo());
}

#[test]
fn forward_delete_in_virtual_space_removes_newline_without_padding() {
    let editor = editor("ab\r\ncd");
    editor.set_caret_position(2, 7).unwrap();
    editor.delete(false, false).unwrap();
    assert_eq!(editor.document().text(), "abcd");
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 0));
    editor.undo().unwrap();
    editor.set_caret_offset(4).unwrap();
    editor.delete(true, false).unwrap();
    assert_eq!(editor.document().text(), "abcd");
}

#[test]
fn newline_never_materializes_unused_virtual_padding() {
    let editor = editor("  ab");
    editor.set_caret_position(4, 8).unwrap();
    editor.insert_new_line().unwrap();
    assert_eq!(editor.document().text(), "  ab\n  ");
    assert_eq!(editor.caret_virtual_space(), 0);
    editor.undo().unwrap();
    editor.set_indentation_strategy(None);
    editor.set_caret_position(4, 8).unwrap();
    editor.insert_new_line().unwrap();
    assert_eq!(editor.document().text(), "  ab\n");
}

#[test]
fn tab_targets_visual_column_including_virtual_space() {
    let editor = editor("\tx");
    editor.set_caret_position(2, 1).unwrap();
    editor.insert_tab().unwrap();
    assert_eq!(editor.document().text(), "\tx   ");
    assert_eq!(visual_column(&editor.document().text(), 4), 8);
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::with_virtual_space(5, 4, 5, 2)],
            0,
        ))
        .unwrap();
    editor.insert_tab().unwrap();
    assert_eq!(visual_column(&editor.document().text(), 4), 12);
}

#[test]
fn rectangle_single_line_typing_pads_each_short_row_and_preserves_mode() {
    let editor = editor("abc\nx\n");
    rectangle(&editor, 0, 2, 4, 6);
    editor.set_options(EditorOptions {
        convert_tabs_to_spaces: true,
        ..EditorOptions::default()
    });
    editor.replace_selection_with_text("X").unwrap();
    assert_eq!(editor.document().text(), "abc X\nx   X\n    X");
    assert!(editor.selection().is_rectangular());
    assert_eq!(
        editor.selection().segments(),
        &[
            SelectionSegment::empty(5),
            SelectionSegment::empty(11),
            SelectionSegment::empty(17),
        ]
    );
    assert_eq!(editor.selection().primary_index(), 2);
    editor.replace_selection_with_text("Y").unwrap();
    assert_eq!(editor.document().text(), "abc XY\nx   XY\n    XY");
    editor.undo().unwrap();
    editor.undo().unwrap();
    assert_eq!(editor.document().text(), "abc\nx\n");
    assert!(!editor.document().can_undo());
}

#[test]
fn rectangular_multiline_paste_is_bounded_and_retains_remaining_newlines() {
    let editor = editor("a\nb\nc");
    rectangle(&editor, 0, 1, 1, 1);
    editor.replace_selection_with_text("Q\r\nR\nS\rT").unwrap();
    assert_eq!(editor.document().text(), "aQ\nbR\nS\rT\nc");
    assert!(!editor.selection().is_rectangular());
    assert_eq!(editor.selection().segments(), &[SelectionSegment::empty(5)]);
    editor.undo().unwrap();
    assert_eq!(editor.document().text(), "a\nb\nc");
    assert!(!editor.document().can_undo());
    rectangle(&editor, 0, 2, 1, 1);
    editor.replace_selection_with_text("Q\nR").unwrap();
    assert_eq!(editor.document().text(), "aQ\nbR\nc");
    assert_eq!(editor.selection().segments().len(), 1);
}

#[test]
fn rectangle_deletion_never_joins_logical_lines() {
    let editor = editor("ab\nc\n");
    rectangle(&editor, 0, 2, 0, 0);
    editor.delete(true, false).unwrap();
    assert_eq!(editor.document().text(), "ab\nc\n");
    rectangle(&editor, 0, 2, 4, 4);
    editor.delete(false, false).unwrap();
    assert_eq!(editor.document().text(), "ab\nc\n");
    assert_eq!(editor.selection().segments()[2].caret_virtual_space, 4);
    assert!(editor.selection().is_rectangular());
    assert!(!editor.document().can_undo());
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
fn protected_rectangular_insertion_is_rejected_before_any_padding_or_edit() {
    let editor = editor("a\nb");
    rectangle(&editor, 0, 1, 3, 3);
    let selection = editor.selection();
    editor.set_read_only_section_provider(Rc::new(DenyInsert(3)));
    assert!(matches!(
        editor.replace_selection_with_text("X"),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.document().text(), "a\nb");
    assert_eq!(editor.selection(), selection);
    assert!(!editor.document().can_undo());
}

#[test]
fn distinct_virtual_carets_at_one_physical_end_fail_atomically() {
    let editor = editor("a");
    let selection = Selection::from_segments(
        [
            SelectionSegment::with_virtual_space(1, 2, 1, 2),
            SelectionSegment::with_virtual_space(1, 4, 1, 4),
        ],
        1,
    );
    editor.set_selection(selection.clone()).unwrap();
    assert!(matches!(
        editor.replace_selection_with_text("X"),
        Err(EditError::AmbiguousVirtualCarets)
    ));
    assert!(matches!(
        editor.insert_new_line(),
        Err(EditError::AmbiguousVirtualCarets)
    ));
    assert_eq!(editor.document().text(), "a");
    assert_eq!(editor.selection(), selection);
    assert!(!editor.document().can_undo());
}

#[test]
fn impossible_padding_is_an_error_and_leaves_document_unchanged() {
    let editor = editor("a");
    editor.set_caret_position(1, usize::MAX).unwrap();
    assert!(matches!(
        editor.replace_selection_with_text("X"),
        Err(EditError::VirtualSpaceTooLarge)
    ));
    assert_eq!(editor.document().text(), "a");
    assert!(!editor.document().can_undo());
}

#[test]
fn update_start_protection_and_document_changes_are_revalidated() {
    let editor = editor("a");
    editor.set_caret_position(1, 2).unwrap();
    let protected_editor = editor.clone();
    let subscription = editor
        .document()
        .update_started(move || protected_editor.set_is_read_only(true));
    assert!(matches!(
        editor.replace_selection_with_text("X"),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.document().text(), "a");
    subscription.dispose();
    editor.set_is_read_only(false);
    let document = editor.document().clone();
    let subscription = editor
        .document()
        .update_started(move || document.insert(0, "q").unwrap());
    assert!(matches!(
        editor.replace_selection_with_text("X"),
        Err(EditError::StaleEditPlan)
    ));
    assert_eq!(editor.document().text(), "qa");
    assert_eq!(editor.selection().primary().caret_tuple(), (2, 2));
    subscription.dispose();
}

#[test]
fn virtual_only_line_transforms_on_empty_first_line_do_not_underflow() {
    let editor = editor("\nx  ");
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::with_virtual_space(0, 4, 0, 1)],
            0,
        ))
        .unwrap();
    editor
        .transform_text(TextTransformation::RemoveTrailingWhitespace)
        .unwrap();
    assert_eq!(editor.document().text(), "\nx  ");
    editor.indent_selection(false).unwrap();
    assert_eq!(editor.document().text(), "    \nx  ");
}

#[test]
fn adjacent_replacement_carets_stay_on_their_own_side_of_an_insertion() {
    let editor = editor("abcd");
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::new(0, 2), SelectionSegment::new(2, 4)],
            0,
        ))
        .unwrap();
    editor
        .replace_selections(&["X".into(), "YY".into()])
        .unwrap();
    assert_eq!(editor.document().text(), "XYY");
    assert_eq!(
        editor.selection().segments(),
        &[SelectionSegment::empty(1), SelectionSegment::empty(3)]
    );
}
