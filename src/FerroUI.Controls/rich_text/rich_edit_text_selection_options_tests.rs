// Literal native expectations are distinct from FerroUI transaction policies.
use super::*;
use crate::documents::{Hyperlink, InlineUIContainer, LineBreak, Run};
use crate::test_support::test_scope;
use crate::{Border, RichEditBox};
use ferroui_base::media::{FontStyle, FontWeight};
use ferroui_base::threading::Dispatcher;
use std::cell::Cell;

#[path = "rich_edit_text_selection_options_native_cases.rs"]
mod native_cases;
use native_cases::{NATIVE_CASES, NATIVE_FIXTURES};

type NativeState = (usize, usize, u32, i32);

struct NativeCase {
    fixture: usize,
    before: NativeState,
    requested: u32,
    after_set: NativeState,
    typed: &'static str,
    after: NativeState,
    story: &'static str,
}

const fn native(
    fixture: usize,
    before: NativeState,
    requested: u32,
    after_set: NativeState,
    typed: &'static str,
    after: NativeState,
    story: &'static str,
) -> NativeCase {
    NativeCase {
        fixture,
        before,
        requested,
        after_set,
        typed,
        after,
        story,
    }
}

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn observed(selection: &RichEditTextSelection) -> NativeState {
    let directed = selection.selection();
    (
        directed.anchor,
        directed.active,
        selection.options().bits(),
        selection.selection_type() as i32,
    )
}

#[test]
fn rich_selection_options_and_type_retain_native_public_values() {
    assert_eq!(SelectionOptions::NONE.bits(), 0);
    assert_eq!(SelectionOptions::START_ACTIVE.bits(), 1);
    assert_eq!(SelectionOptions::AT_END_OF_LINE.bits(), 2);
    assert_eq!(SelectionOptions::OVERTYPE.bits(), 4);
    assert_eq!(SelectionOptions::ACTIVE.bits(), 8);
    assert_eq!(SelectionOptions::REPLACE.bits(), 16);
    assert_eq!(SelectionOptions::all().bits(), 31);
    assert_eq!(SelectionType::None as i32, 0);
    assert_eq!(SelectionType::InsertionPoint as i32, 1);
    assert_eq!(SelectionType::Normal as i32, 2);
    assert_eq!(SelectionType::InlineShape as i32, 7);
    assert_eq!(SelectionType::Shape as i32, 8);
}

#[test]
fn rich_selection_options_matches_316_system_msftedit_observations() {
    let _scope = Dispatcher::unit_test_scope();
    assert_eq!(NATIVE_FIXTURES.len(), 19);
    assert_eq!(NATIVE_CASES.len(), 316);
    for (index, case) in NATIVE_CASES.iter().enumerate() {
        let (name, text) = NATIVE_FIXTURES[case.fixture];
        let document = services(&text.replace('\r', "\n"));
        let selection = document.selection();
        selection.set_range(case.before.0, case.before.1).unwrap();
        assert_eq!(observed(&selection), case.before, "{index}: {name}: before");
        selection
            .set_options(SelectionOptions::from_bits(case.requested).unwrap())
            .unwrap();
        assert_eq!(
            observed(&selection),
            case.after_set,
            "{index}: {name}: setter"
        );
        assert_eq!(document.text(), text.replace('\r', "\n"));
        selection.type_text(case.typed).unwrap();
        assert_eq!(observed(&selection), case.after, "{index}: {name}: typing");
        // Native GetText includes one synthetic terminal EOP. Real CR paragraphs
        // occupy the same UTF-16 positions as FerroUI's structural LF paragraphs.
        let expected = case
            .story
            .strip_suffix('\r')
            .expect("native terminal EOP")
            .replace('\r', "\n");
        assert_eq!(document.text(), expected, "{index}: {name}: exact story");
    }
}

#[test]
fn rich_selection_options_are_shared_and_drive_control_typing() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abcdef").unwrap();
    owner.select(2, 2);
    let document = owner.text_document();
    document.clear_undo_redo_history();
    let first = document.selection();
    let second = document.selection();
    first
        .set_options(
            SelectionOptions::OVERTYPE
                | SelectionOptions::ACTIVE
                | SelectionOptions::AT_END_OF_LINE,
        )
        .unwrap();
    assert_eq!(observed(&second), (2, 2, 31, 1));
    assert!(owner.insert_text("XY").unwrap());
    assert_eq!(owner.text(), "abXYef");
    assert_eq!(observed(&first), (4, 4, 29, 1));
    assert_eq!(observed(&second), observed(&first));
    assert!(document.undo().unwrap());
    assert_eq!(owner.text(), "abcdef");
    assert!(second
        .options()
        .contains(SelectionOptions::OVERTYPE | SelectionOptions::ACTIVE));
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_options_unknown_bits_leave_preview_and_history_untouched() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    let selection = document.selection();
    selection.set_range(1, 4).unwrap();
    selection.type_text("Q").unwrap();
    document.editor().set_preedit("preview", Some(3)).unwrap();
    let before = document.document().snapshot();
    let state = observed(&selection);
    let history = (document.can_undo(), document.can_redo());
    assert!(matches!(
        selection.set_options(SelectionOptions::from_bits_retain(1 << 31)),
        Err(EditError::Document(DocumentError::InvalidFormat(_)))
    ));
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(observed(&selection), state);
    assert_eq!((document.can_undo(), document.can_redo()), history);
    assert!(document.editor().is_composing());
}

#[test]
fn rich_selection_options_cancelled_direction_keeps_existing_modes() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abcdef").unwrap();
    owner.select(1, 4);
    let selection = owner.text_document().selection();
    selection.set_options(SelectionOptions::OVERTYPE).unwrap();
    owner.selection_changing(|_, args| args.set_cancel(true));
    selection
        .set_options(SelectionOptions::START_ACTIVE | SelectionOptions::ACTIVE)
        .unwrap();
    assert_eq!(observed(&selection), (1, 4, 20, 2));
}

#[test]
fn rich_selection_options_redirected_direction_keeps_existing_modes() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abcdef").unwrap();
    owner.select(1, 4);
    let selection = owner.text_document().selection();
    selection.set_options(SelectionOptions::OVERTYPE).unwrap();
    let weak = owner.downgrade();
    owner.selection_changing(move |_, args| {
        weak.upgrade().unwrap().select(0, 0);
        args.set_cancel(true);
    });
    selection
        .set_options(SelectionOptions::START_ACTIVE | SelectionOptions::ACTIVE)
        .unwrap();
    assert_eq!(observed(&selection), (0, 0, 21, 1));
}

#[test]
fn rich_selection_options_nested_same_value_intent_owns_modes() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abcdef").unwrap();
    owner.select(1, 4);
    let selection = owner.text_document().selection();
    selection.set_options(SelectionOptions::OVERTYPE).unwrap();
    let nested = selection.clone();
    let called = Rc::new(Cell::new(false));
    let observed_call = called.clone();
    owner.selection_changing(move |_, _| {
        if !called.replace(true) {
            assert_eq!(nested.selection(), EditorSelection::new(4, 1));
            nested
                .set_options(SelectionOptions::START_ACTIVE | SelectionOptions::OVERTYPE)
                .unwrap();
        }
    });
    selection
        .set_options(SelectionOptions::START_ACTIVE | SelectionOptions::ACTIVE)
        .unwrap();
    assert!(observed_call.get());
    assert_eq!(observed(&selection), (4, 1, 21, 2));
}

#[test]
fn rich_selection_options_nested_navigation_owns_affinity_and_modes() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abcdef").unwrap();
    owner.select(1, 4);
    let document = owner.text_document();
    let selection = document.selection();
    selection.set_options(SelectionOptions::OVERTYPE).unwrap();
    document.editor().remember_vertical_navigation_x(42.0);
    let nested = selection.clone();
    let called = Rc::new(Cell::new(false));
    let observed_call = called.clone();
    owner.selection_changing(move |_, _| {
        if !called.replace(true) {
            let requested = nested.selection();
            nested
                .set_range(requested.anchor, requested.active)
                .unwrap();
        }
    });
    selection
        .set_options(SelectionOptions::START_ACTIVE | SelectionOptions::ACTIVE)
        .unwrap();
    assert!(observed_call.get());
    assert_eq!(observed(&selection), (4, 1, 21, 2));
    assert_eq!(document.editor().vertical_navigation_x(), None);
}

#[test]
fn rich_selection_options_callback_model_edit_rejects_stale_modes() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_text("abcdef").unwrap();
    owner.select(1, 4);
    let document = owner.text_document();
    let selection = document.selection();
    selection.set_options(SelectionOptions::OVERTYPE).unwrap();
    let model = document.document();
    let revision = model.revision();
    let batch = model.begin_edit();
    let callback_model = model.clone();
    let called = Rc::new(Cell::new(false));
    let observed_call = called.clone();
    owner.selection_changing(move |_, _| {
        if !called.replace(true) {
            callback_model
                .replace_text(0..callback_model.text_length(), "external")
                .unwrap();
        }
    });
    assert!(matches!(
        selection.set_options(SelectionOptions::START_ACTIVE | SelectionOptions::ACTIVE),
        Err(EditError::ReentrantEdit)
    ));
    assert!(observed_call.get());
    assert_eq!(document.text(), "external");
    assert_eq!(model.revision(), revision);
    assert!(selection.options().contains(SelectionOptions::OVERTYPE));
    assert!(!selection.options().contains(SelectionOptions::ACTIVE));
    assert!(!document.can_undo());
    drop(batch);
    assert_eq!(document.text(), "external");
}

#[test]
fn rich_selection_options_mode_only_changes_preserve_redo_and_text_undo_keeps_modes() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab");
    let selection = document.selection();
    selection.set_range(1, 1).unwrap();
    selection.type_text("X").unwrap();
    assert!(document.undo().unwrap());
    assert!(document.can_redo());
    selection
        .set_options(SelectionOptions::OVERTYPE | SelectionOptions::AT_END_OF_LINE)
        .unwrap();
    assert!(!document.can_undo());
    assert!(document.can_redo());
    assert!(document.redo().unwrap());
    assert_eq!(document.text(), "aXb");
    assert_eq!(observed(&selection), (2, 2, 23, 1));
    selection.set_options(SelectionOptions::ACTIVE).unwrap();
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "ab");
    assert_eq!(observed(&selection), (1, 1, 25, 1));
    assert!(document.redo().unwrap());
    assert_eq!(observed(&selection), (2, 2, 25, 1));
}

#[test]
fn rich_selection_options_failed_group_restores_modes_and_direction() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    let selection = document.selection();
    selection.set_range(4, 1).unwrap();
    selection
        .set_options(
            SelectionOptions::START_ACTIVE
                | SelectionOptions::OVERTYPE
                | SelectionOptions::AT_END_OF_LINE,
        )
        .unwrap();
    let before = document.document().snapshot();
    let state = observed(&selection);
    let result: Result<(), EditError> = document.editor().edit_group(|editor| {
        editor.set_selection_options(SelectionOptions::ACTIVE)?;
        editor.type_text("Q")?;
        editor.set_selection_options(SelectionOptions::NONE)?;
        Err(EditError::ReadOnly)
    });
    assert!(matches!(result, Err(EditError::ReadOnly)));
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(observed(&selection), state);
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_options_successful_group_keeps_current_modes_when_undone() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    document.selection().set_range(1, 2).unwrap();
    document
        .editor()
        .edit_group(|editor| {
            editor.set_selection_options(SelectionOptions::OVERTYPE)?;
            editor.type_text("XY")?;
            editor
                .set_selection_options(SelectionOptions::ACTIVE | SelectionOptions::AT_END_OF_LINE)
        })
        .unwrap();
    assert_eq!(document.text(), "aXYd");
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "abcd");
    assert_eq!(observed(&document.selection()), (1, 2, 26, 2));
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_options_empty_type_text_keeps_preview_and_read_only_state() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    let selection = document.selection();
    selection.set_range(1, 4).unwrap();
    selection
        .set_options(SelectionOptions::OVERTYPE | SelectionOptions::AT_END_OF_LINE)
        .unwrap();
    assert!(!selection.type_text("").unwrap());
    assert_eq!(selection.selection(), EditorSelection::new(1, 4));
    document.editor().set_preedit("preview", Some(3)).unwrap();
    document.editor().set_is_read_only(true);
    let snapshot = document.document().snapshot();
    let state = observed(&selection);
    assert!(!selection.type_text("").unwrap());
    assert_eq!(document.document().snapshot(), snapshot);
    assert_eq!(observed(&selection), state);
    assert!(document.editor().is_composing());
    assert!(document.editor().is_read_only());
}

#[test]
fn rich_selection_options_read_only_allows_modes_and_rejects_nonempty_typing() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcdef");
    document.editor().set_is_read_only(true);
    let selection = document.selection();
    selection
        .set_options(SelectionOptions::OVERTYPE | SelectionOptions::ACTIVE)
        .unwrap();
    let before = observed(&selection);
    assert!(matches!(selection.type_text("X"), Err(EditError::ReadOnly)));
    assert_eq!(document.text(), "abcdef");
    assert_eq!(observed(&selection), before);
    assert!(!document.can_undo());
}

#[test]
fn rich_selection_options_overtype_length_limit_consumes_only_accepted_targets() {
    let _scope = Dispatcher::unit_test_scope();
    for (source, (start, end), limit, typed, expected, caret, changed) in [
        ("abcd", (1, 1), 4, "X😀", "aXcd", 2, true),
        ("abcd", (1, 1), 4, "😀", "abcd", 1, false),
        ("a😀bc", (1, 1), 5, "XY", "aXYc", 3, true),
        ("abcd", (1, 3), 4, "X😀", "aX😀", 4, true),
        ("a\nb", (0, 0), 3, "XY", "X\nb", 1, true),
    ] {
        let document = services(source);
        let selection = document.selection();
        document.editor().set_max_length(limit);
        selection.set_range(start, end).unwrap();
        selection
            .set_options(SelectionOptions::OVERTYPE | SelectionOptions::AT_END_OF_LINE)
            .unwrap();
        let before = selection.selection();
        assert_eq!(
            selection.type_text(typed).unwrap(),
            changed,
            "{source:?} {typed:?}"
        );
        assert_eq!(document.text(), expected);
        assert_eq!(selection.selection(), EditorSelection::caret(caret));
        assert_eq!(
            selection
                .options()
                .contains(SelectionOptions::AT_END_OF_LINE),
            !changed
        );
        assert_eq!(document.undo().unwrap(), changed);
        assert_eq!(document.text(), source);
        assert_eq!(selection.selection(), before);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_selection_options_typing_publication_preserves_newer_projected_modes() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    let editor = document.editor();
    editor
        .set_selection_options(SelectionOptions::OVERTYPE | SelectionOptions::AT_END_OF_LINE)
        .unwrap();
    let weak = Rc::downgrade(&editor);
    let called = Rc::new(Cell::new(false));
    let observed_call = called.clone();
    editor.set_selection_projection(Some(Rc::new(move |selection| {
        if !called.replace(true) {
            weak.upgrade()
                .unwrap()
                .set_selection_options(SelectionOptions::ACTIVE | SelectionOptions::AT_END_OF_LINE)
                .unwrap();
        }
        selection
    })));
    assert!(document.selection().type_text("X").unwrap());
    assert!(observed_call.get());
    assert_eq!(document.text(), "Xbc");
    assert_eq!(observed(&document.selection()), (1, 1, 27, 1));
}

#[test]
fn rich_selection_options_typing_changed_handler_cannot_reenter_modes() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    let editor = document.editor();
    editor
        .set_selection_options(SelectionOptions::OVERTYPE | SelectionOptions::AT_END_OF_LINE)
        .unwrap();
    let weak = Rc::downgrade(&editor);
    let rejected = Rc::new(Cell::new(false));
    let observed_rejection = rejected.clone();
    let subscription = editor.changed(move || {
        rejected.set(matches!(
            weak.upgrade()
                .unwrap()
                .set_selection_options(SelectionOptions::ACTIVE),
            Err(EditError::ReentrantEdit)
        ));
    });
    assert!(document.selection().type_text("X").unwrap());
    assert!(observed_rejection.get());
    assert_eq!(document.text(), "Xbc");
    assert_eq!(observed(&document.selection()), (1, 1, 21, 1));
    subscription.dispose();
}

#[test]
fn rich_selection_options_overtype_preserves_rich_content_and_one_undo_unit() {
    let _scope = test_scope();
    let model = RichTextDocument::new();
    let paragraph = Paragraph::new();
    paragraph.set_font_size(18.0);
    let run = Run::with_text(Some("ab"));
    run.set_font_weight(FontWeight::Bold);
    paragraph.inlines().add(run);
    let link = Hyperlink::new();
    link.set_navigate_uri(Some("https://example.test/target".into()));
    link.set_target_name(Some("frame".into()));
    link.set_font_style(FontStyle::Italic);
    link.add_child_text("cd");
    paragraph.inlines().add(link);
    let object = InlineUIContainer::new();
    object.set_child(Border::new());
    paragraph.inlines().add(object);
    paragraph.inlines().add(Run::with_text(Some("ef")));
    model.blocks().add(paragraph);
    let document = RichEditTextDocument::new(model.clone());
    let selection = document.selection();
    selection
        .set_character_format(&CharacterFormat {
            font_size: Some(22.0),
            ..Default::default()
        })
        .unwrap();
    document.clear_undo_redo_history();
    selection.set_options(SelectionOptions::OVERTYPE).unwrap();
    let before = model.snapshot();
    let tail = model.slice(2..model.text_length()).unwrap();
    assert!(selection.type_text("XY").unwrap());
    assert_eq!(model.text(), "XYcd\u{fffc}ef");
    assert_eq!(model.character_format_at(0).unwrap().font_size, Some(22.0));
    assert_eq!(
        model.slice(2..model.text_length()).unwrap(),
        tail,
        "hyperlink, format and embedded control identity survive"
    );
    let after = model.snapshot();
    assert!(document.undo().unwrap());
    assert_eq!(model.snapshot(), before);
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(model.snapshot(), after);
}

fn soft_break_count(inlines: &[InlineSnapshot]) -> usize {
    inlines
        .iter()
        .map(|inline| match &inline.kind {
            InlineSnapshotKind::LineBreak => 1,
            InlineSnapshotKind::Span(children)
            | InlineSnapshotKind::Bold(children)
            | InlineSnapshotKind::Italic(children)
            | InlineSnapshotKind::Underline(children)
            | InlineSnapshotKind::Hyperlink {
                inlines: children, ..
            } => soft_break_count(children),
            _ => 0,
        })
        .sum()
}

#[test]
fn rich_selection_options_preserves_structured_soft_breaks_as_model_policy() {
    let _scope = test_scope();
    // Protect both Windows CRLF and Unix LF LineBreak text. Native VT/U+2028
    // soft-break behavior has not been characterized by the system probe.
    for start in [0, 1] {
        let model = RichTextDocument::new();
        let paragraph = Paragraph::new();
        paragraph.inlines().add(Run::with_text(Some("a")));
        paragraph.inlines().add(LineBreak::new());
        paragraph.inlines().add(Run::with_text(Some("b")));
        model.blocks().add(paragraph);
        let original = model.text();
        let line_break = &original[1..original.len() - 1];
        assert!(matches!(line_break, "\n" | "\r\n"));
        let before = model.snapshot();
        let document = RichEditTextDocument::new(model.clone());
        let selection = document.selection();
        selection.set_range(start, start).unwrap();
        selection.set_options(SelectionOptions::OVERTYPE).unwrap();
        selection.type_text("XY").unwrap();
        assert_eq!(
            model.text(),
            format!("{}XY{line_break}b", if start == 0 { "" } else { "a" })
        );
        let after = model.snapshot();
        assert_eq!(after.paragraphs.len(), 1);
        assert_eq!(soft_break_count(&after.paragraphs[0].inlines), 1);
        assert!(document.undo().unwrap());
        assert_eq!(model.snapshot(), before);
        assert_eq!(selection.selection(), EditorSelection::caret(start));
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_selection_options_type_text_distinguishes_typed_and_bulk_crlf() {
    let _scope = Dispatcher::unit_test_scope();
    for (typed, expected, caret) in [(true, "a\n\nb", 3), (false, "a\nb", 2)] {
        let document = services("ab");
        document.selection().set_range(1, 1).unwrap();
        if typed {
            document.selection().type_text("\r\n").unwrap();
        } else {
            document
                .editor()
                .insert_text("\r\n", EditKind::Paste)
                .unwrap();
        }
        assert_eq!(document.text(), expected);
        assert_eq!(
            document.selection().selection(),
            EditorSelection::caret(caret)
        );
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "ab");
    }
}
