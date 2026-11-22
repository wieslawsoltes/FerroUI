use super::*;
use crate::tests::{laid_out_editor, test_scope};
use ferroui_base::{Rect, Size};
use ferroui_controls::automation::provider::{
    IValueProvider, TextAttribute, TextAttributeValue, TextPatternRangeEndpoint as Endpoint,
    TextScrollPlacement, TextUnit,
};
use ferroui_controls::{Control, Window};
use ferroui_edit_core::editing::{Selection, SelectionSegment, TextSegmentReadOnlySectionProvider};
use ferroui_edit_core::highlighting::HighlightingDefinition;
use std::cell::Cell;

fn provider(editor: &TextEditor) -> Rc<dyn ITextProvider> {
    ControlAutomationPeer::create_peer_for_element(editor)
        .get_provider::<dyn ITextProvider>()
        .unwrap()
}

fn find(range: &dyn ITextRangeProvider, text: &str) -> TextRange {
    range.find_text(text, false, false).unwrap().unwrap()
}

fn rooted(text: &str, size: Size, wrap: bool) -> (Ref<TextEditor>, Ref<Window>) {
    let editor = laid_out_editor(text);
    editor.set_word_wrap(wrap);
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    window.measure(size);
    window.arrange(Rect::from_size(size));
    let area = editor.text_area();
    area.measure(size);
    area.arrange(Rect::from_size(size));
    (editor, window)
}

#[test]
fn registered_patterns_expose_live_utf16_ranges_and_expire_on_rebinding() {
    let _scope = test_scope();
    let editor = laid_out_editor("a😀b CAFÉ café");
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    assert!(peer.get_provider::<dyn IValueProvider>().is_some());
    assert!(peer.get_provider::<dyn ITextProvider2>().is_some());
    let text = provider(&editor);
    let range = text.document_range().unwrap();
    let emoji = find(&*range, "😀");
    assert_eq!(range.get_text(Some(2)).unwrap(), "a");
    assert_eq!(range.get_text(Some(3)).unwrap(), "a😀");
    assert_eq!(
        range
            .find_text("CaFé", true, true)
            .unwrap()
            .unwrap()
            .get_text(None)
            .unwrap(),
        "café"
    );
    let doc = editor.document();
    doc.insert(1, "é").unwrap();
    assert_eq!(emoji.get_text(None).unwrap(), "😀");
    assert_eq!(
        emoji
            .compare_endpoints(Endpoint::Start, &*range, Endpoint::Start)
            .unwrap(),
        2
    );
    doc.remove(2, 2).unwrap();
    assert!(emoji.get_text(None).unwrap().is_empty());
    let another = TextEditor::new();
    another.set_document(doc.clone());
    let other_range = provider(&another).document_range().unwrap();
    assert_eq!(
        range
            .compare(&*other_range)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::IncompatibleRange)
    );
    editor.set_document(TextDocument::new());
    editor.set_document(doc);
    assert_eq!(
        range
            .get_text(None)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
}

#[test]
fn indexed_text_uses_the_same_scalar_projection_as_the_exposed_value() {
    let _scope = test_scope();
    let editor = TextEditor::new();
    let document = TextDocument::from_utf16(&[0x61, 0xd83d, 0xde00, 0, 0xd800, 0x65, 0x301]);
    editor.set_document(document.clone());
    let text = provider(&editor);
    assert_eq!(text.text_character_count().unwrap(), Some(6));
    assert_eq!(text.utf16_offset_from_character(1).unwrap(), Some(1));
    assert_eq!(text.utf16_offset_from_character(6).unwrap(), Some(7));
    for index in 0..6 {
        let start = text.utf16_offset_from_character(index).unwrap().unwrap();
        let end = text.utf16_offset_from_character(index + 1).unwrap().unwrap();
        let range = text.range_from_offsets(start..end).unwrap().unwrap();
        assert_eq!(range.get_text(None).unwrap(), editor.text().chars().nth(index).unwrap().to_string());
    }
    document.remove(0, 7).unwrap();
    assert_eq!(text.text_character_count().unwrap(), Some(0));
    editor.set_document(TextDocument::from_text("😀\r\n"));
    assert_eq!(text.text_character_count().unwrap(), Some(3));
}

#[test]
fn indexed_logical_lines_follow_utf16_delimiters_wraps_and_edits() {
    let _scope = test_scope();
    let first_line = format!("first {}😀\r\n", "words ".repeat(24));
    let first_end = first_line.encode_utf16().count();
    let story = format!("{first_line}x\ry\n");
    let (editor, _window) = rooted(&story, Size::new(108.0, 58.0), true);
    let text = provider(&editor);
    let visual_row = find(text.document_range().unwrap().as_ref(), "first");
    visual_row.expand_to_enclosing_unit(TextUnit::Line).unwrap();
    assert!(visual_row.text_offsets().unwrap().unwrap().end < first_end);

    for (index, range) in [0..first_end, first_end..first_end + 2, first_end + 2..first_end + 4]
        .into_iter().enumerate()
    {
        assert_eq!(text.logical_line_range(index).unwrap(), Some(range.clone()));
        for offset in range {
            assert_eq!(text.logical_line_index(offset).unwrap(), Some(index));
        }
    }
    let end = first_end + 4;
    assert_eq!(text.logical_line_range(3).unwrap(), Some(end..end));
    assert_eq!(text.logical_line_index(end).unwrap(), Some(3));
    assert!(text.logical_line_range(4).is_err());
    assert!(text.logical_line_range(usize::MAX).is_err());
    assert!(text.logical_line_index(end + 1).is_err());

    editor.document().remove(first_end - 2, 2).unwrap();
    assert_eq!(text.logical_line_range(0).unwrap(), Some(0..first_end));
    assert_eq!(text.logical_line_index(first_end - 1).unwrap(), Some(0));
    assert_eq!(text.logical_line_index(first_end).unwrap(), Some(1));
    editor.set_document(TextDocument::new());
    assert_eq!(text.logical_line_range(0).unwrap(), Some(0..0));
    assert_eq!(text.logical_line_index(0).unwrap(), Some(0));
}

#[test]
fn a_retained_document_range_follows_the_story_across_empty_and_nonempty_edits() {
    let _scope = test_scope();
    let editor = laid_out_editor("");
    let document = editor.document();
    let text = provider(&editor);
    let full = text.document_range().unwrap();
    let clone = full.clone_range().unwrap();
    let caret = text.get_selection().unwrap().remove(0);
    let page = caret.clone_range().unwrap();
    page.expand_to_enclosing_unit(TextUnit::Page).unwrap();

    document.insert(0, "😀\r\nline").unwrap();
    let found = find(&*full, "😀");
    let narrowed = full.clone_range().unwrap();
    narrowed.expand_to_enclosing_unit(TextUnit::Character).unwrap();
    let expanded = found.clone_range().unwrap();
    expanded.expand_to_enclosing_unit(TextUnit::Document).unwrap();
    let moved = full.clone_range().unwrap();
    assert_eq!(moved.move_endpoint_by_unit(Endpoint::End, TextUnit::Character, -1).unwrap(), -1);
    let explicit = text.range_from_offsets(0..document.text_length()).unwrap().unwrap();
    full.move_endpoint_by_range(Endpoint::End, &*clone, Endpoint::End).unwrap();

    document.insert(0, "left ").unwrap();
    document.insert(document.text_length(), " right").unwrap();
    assert_eq!(caret.get_text(None).unwrap(), "");
    assert_eq!(narrowed.get_text(None).unwrap(), "😀");
    assert_eq!(found.text_offsets().unwrap(), Some(5..7));
    assert_eq!(moved.get_text(None).unwrap(), "😀\r\nlin");
    assert_eq!(explicit.get_text(None).unwrap(), "😀\r\nline");
    for range in [&full, &clone, &expanded, &page] {
        assert_eq!(range.get_text(None).unwrap(), "left 😀\r\nline right");
    }

    document.remove(0, document.text_length()).unwrap();
    assert_eq!(full.text_offsets().unwrap(), Some(0..0));
    document.insert(0, "refilled").unwrap();
    for range in [&full, &clone, &expanded, &page] {
        assert_eq!(range.get_text(None).unwrap(), "refilled");
        assert!(range.compare(&*text.document_range().unwrap()).unwrap());
    }
    for range in [&caret, &found, &narrowed, &moved, &explicit] {
        assert_eq!(range.get_text(None).unwrap(), "");
    }
    editor.set_document(TextDocument::from_text("a new story"));
    assert_eq!(
        clone.get_text(None).unwrap_err().downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
}

#[test]
fn whole_value_replacement_uses_the_same_protected_ranges_as_targeted_edits() {
    let _scope = test_scope();
    let editor = laid_out_editor("a😀b");
    let document = editor.document();
    let undo_count = document.undo_count();
    let version = document.version();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(1, 2).unwrap();
    editor.session().set_read_only_section_provider(protected);
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let value = peer.get_provider::<dyn IValueProvider>().unwrap();
    let text = peer.get_provider::<dyn ITextProvider>().unwrap();

    assert!(!value.is_read_only());
    let refused = value.set_value(Some("replacement")).unwrap_err();
    assert!(matches!(refused.downcast_ref::<EditError>(), Some(EditError::ReadOnly)));
    assert_eq!(editor.text(), "a😀b");
    assert_eq!(document.undo_count(), undo_count);
    assert_eq!(document.version(), version);
    assert!(text.replace_text(3..4, "β").unwrap());
    assert_eq!(editor.text(), "a😀β");
    document.set_is_read_only(true);
    assert!(value.is_read_only());
}

#[test]
fn automation_value_replacement_does_not_edit_a_document_rebound_during_preflight() {
    let _scope = test_scope();
    let editor = laid_out_editor("original");
    let original = editor.document();
    let undo_count = original.undo_count();
    let version = original.version();
    let weak = editor.downgrade();
    let _subscription = original.update_started(move || {
        weak.upgrade().unwrap().set_document(TextDocument::from_text("current"));
    });
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let value = peer.get_provider::<dyn IValueProvider>().unwrap();
    let refused = value.set_value(Some("stale")).unwrap_err();
    assert!(matches!(refused.downcast_ref::<EditError>(), Some(EditError::StaleEditPlan)));
    assert_eq!(original.text(), "original");
    assert_eq!(editor.text(), "current");
    assert_eq!(original.undo_count(), undo_count);
    assert_eq!(original.version(), version);
}

#[test]
fn absolute_utf16_ranges_can_replace_one_selection_and_follow_edits() {
    let _scope = test_scope();
    let editor = laid_out_editor("a😀bc def");
    let text = provider(&editor);
    let emoji = text.range_from_offsets(1..3).unwrap().unwrap();
    assert_eq!(emoji.get_text(None).unwrap(), "😀");
    assert!(text.range_from_offsets(1..10).is_err());

    let secondary = text.range_from_offsets(6..9).unwrap().unwrap();
    let replacement = text.range_from_offsets(4..5).unwrap().unwrap();
    emoji.select().unwrap();
    secondary.add_to_selection().unwrap();
    assert!(secondary.replace_in_selection(&*replacement).unwrap());
    assert!(!secondary.replace_in_selection(&*replacement).unwrap());
    let offsets: Vec<_> = text
        .get_selection()
        .unwrap()
        .iter()
        .map(|range| range.text_offsets().unwrap().unwrap())
        .collect();
    assert_eq!(offsets, [1..3, 4..5]);
    assert_eq!(editor.caret_offset(), 5);

    editor.document().insert(0, "x").unwrap();
    assert_eq!(emoji.text_offsets().unwrap(), Some(2..4));
    assert_eq!(emoji.get_text(None).unwrap(), "😀");
}

#[test]
fn replacing_an_accessible_selection_keeps_its_direction_and_primary_caret() {
    use ferroui_edit_core::editing::{Selection, SelectionSegment};

    let _scope = test_scope();
    let editor = laid_out_editor("abcde fgh");
    let left = SelectionSegment::new(0, 3);
    editor.set_selection(Selection::from_segments(
        [left, SelectionSegment::new(9, 6)],
        1,
    )).unwrap();
    let text = provider(&editor);
    let range = text.range_from_offsets(6..9).unwrap().unwrap();
    let replacement = text.range_from_offsets(3..5).unwrap().unwrap();
    assert!(range.replace_in_selection(&*replacement).unwrap());
    let selection = editor.selection();
    assert_eq!(selection.segments(), &[left, SelectionSegment::new(5, 3)]);
    assert_eq!(selection.primary_index(), 1);
    let text2 = text.peer().get_provider::<dyn ITextProvider2>().unwrap();
    let caret = text2.get_caret_range().unwrap().range.unwrap();
    assert_eq!(caret.text_offsets().unwrap(), Some(3..3));
}

#[test]
fn content_deltas_follow_utf16_edits_undo_and_document_replacement() {
    let _scope = test_scope();
    let editor = laid_out_editor("a😀b");
    let provider = provider(&editor);
    let edits = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = edits.clone();
    let subscription = provider
        .text_edited(Rc::new(move |edit| {
            observed.borrow_mut().push(edit.clone());
        }))
        .unwrap()
        .unwrap();
    let original = editor.document();
    original.replace(1, 2, "é").unwrap();
    original.insert(2, "X").unwrap();
    assert!(editor.undo().unwrap());

    editor.set_document(TextDocument::from_text("fresh"));
    original.insert(0, "old").unwrap();
    editor.set_text("fresh").unwrap();

    assert_eq!(
        edits.borrow()
            .iter()
            .map(|edit| (edit.offset, edit.removed.as_str(), edit.inserted.as_str()))
            .collect::<Vec<_>>(),
        [(1, "😀", "é"), (2, "", "X"), (2, "X", ""), (0, "aéb", "fresh")],
    );
    subscription.dispose();
    editor.document().insert(0, "quiet").unwrap();
    assert_eq!(edits.borrow().len(), 4);
}

#[test]
fn projected_content_deltas_cover_split_surrogates_and_scalar_offsets() {
    let _scope = test_scope();
    let editor = laid_out_editor("😀b😀c");
    let provider = provider(&editor);
    assert_eq!(provider.utf16_offset_from_character(2).unwrap(), Some(3));
    assert_eq!(provider.character_offset_from_utf16(4).unwrap(), Some(2));

    let edits = Rc::new(std::cell::RefCell::new(Vec::new()));
    let updates = edits.clone();
    let subscription = provider.text_edited(Rc::new(move |edit| updates.borrow_mut().push(edit.clone())))
        .unwrap().unwrap();
    let document = editor.document();
    document.insert(4, "x").unwrap();
    assert_eq!(editor.text(), "😀b�x�c");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "😀b😀c");
    assert_eq!(edits.borrow().iter().map(|edit| {
        (edit.offset, edit.character_offset, edit.removed.as_str(), edit.inserted.as_str())
    }).collect::<Vec<_>>(), [(3, 2, "😀", "�x�"), (3, 2, "�x�", "😀")]);
    subscription.dispose();
}

#[test]
fn content_delta_subscribers_see_reentrant_document_replacement_in_order() {
    let _scope = test_scope();
    let editor = laid_out_editor("a");
    let provider = provider(&editor);
    let weak = editor.downgrade();
    let rebind = provider
        .text_edited(Rc::new(move |edit| {
            if edit.inserted == "b" {
                weak.upgrade().unwrap().set_document(TextDocument::from_text("new"));
            }
        }))
        .unwrap()
        .unwrap();
    let observed = Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = observed.clone();
    let record = provider
        .text_edited(Rc::new(move |edit| sink.borrow_mut().push(edit.inserted.clone())))
        .unwrap()
        .unwrap();

    editor.document().insert(1, "b").unwrap();
    assert_eq!(editor.text(), "new");
    assert_eq!(observed.borrow().as_slice(), ["b", "new"]);
    rebind.dispose();
    record.dispose();
}

#[test]
fn selection_pattern_preserves_independent_ranges_and_reports_caret_changes() {
    let _scope = test_scope();
    let editor = laid_out_editor("abcdef xyz");
    let text = provider(&editor);
    let text2 = text.peer().get_provider::<dyn ITextProvider2>().unwrap();
    let doc = text.document_range().unwrap();
    assert_eq!(
        text.supported_text_selection().unwrap(),
        SupportedTextSelection::Multiple
    );
    let changes = Rc::new(Cell::new(0));
    let calls = changes.clone();
    let subscription = text
        .text_selection_changed(Rc::new(move || calls.set(calls.get() + 1)))
        .unwrap();

    find(&*doc, "abcdef").select().unwrap();
    find(&*doc, "cd").remove_from_selection().unwrap();
    find(&*doc, "xyz").add_to_selection().unwrap();
    let selected = text
        .get_selection()
        .unwrap()
        .iter()
        .map(|range| range.get_text(None).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(selected, ["ab", "ef", "xyz"]);
    let caret = text2.get_caret_range().unwrap();
    assert!(!caret.is_active);
    assert_eq!(
        caret
            .range
            .unwrap()
            .compare_endpoints(Endpoint::Start, &*doc, Endpoint::End)
            .unwrap(),
        0
    );
    assert_eq!(changes.get(), 3);
    subscription.dispose();
    editor.set_caret_offset(0).unwrap();
    assert_eq!(changes.get(), 3);
}

#[test]
fn accessibility_selection_no_ops_preserve_column_mode_and_the_primary_caret() {
    let _scope = test_scope();
    let editor = laid_out_editor("abcdef\nuvwxyz");
    let text = provider(&editor);
    let column = Selection::from_rectangle_segments(
        [SelectionSegment::new(3, 1), SelectionSegment::new(10, 8)],
        1,
    );
    editor.set_selection(column.clone()).unwrap();
    assert!(editor.selection().is_rectangular());
    let caret = editor.caret_offset();
    let changes = Rc::new(Cell::new(0));
    let calls = changes.clone();
    let _subscription = text
        .text_selection_changed(Rc::new(move || calls.set(calls.get() + 1)))
        .unwrap();
    let outside = text.range_from_offsets(4..6).unwrap().unwrap();
    let primary = text.range_from_offsets(8..10).unwrap().unwrap();

    outside.remove_from_selection().unwrap();
    assert_eq!(editor.selection(), column);
    assert!(primary.replace_in_selection(&*primary).unwrap());
    assert_eq!(editor.selection(), column);
    primary.add_to_selection().unwrap();
    assert_eq!(editor.selection(), column);
    assert_eq!(editor.caret_offset(), caret);
    assert_eq!(changes.get(), 0);
}

#[test]
fn accessibility_selection_edits_keep_unchanged_virtual_endpoints() {
    let _scope = test_scope();
    let editor = laid_out_editor("abc\r\nxyz");
    let text = provider(&editor);
    let doc = text.document_range().unwrap();
    let full = find(&*doc, "\r\nxyz");
    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options);

    for reversed in [false, true] {
        let directed = |start, start_extra, end, end_extra| {
            if reversed {
                SelectionSegment::with_virtual_space(end, end_extra, start, start_extra)
            } else {
                SelectionSegment::with_virtual_space(start, start_extra, end, end_extra)
            }
        };
        let original = directed(3, 2, 8, 4);
        editor
            .set_selection(Selection::from_segments([original], 0))
            .unwrap();
        find(&*doc, "y").remove_from_selection().unwrap();
        assert_eq!(
            editor.selection().segments(),
            &[directed(3, 2, 6, 0), directed(7, 0, 8, 4)]
        );
        assert_eq!(editor.selection().primary_index(), usize::from(!reversed));
        assert_eq!(
            editor.selection().primary().caret_tuple(),
            original.caret_tuple()
        );
        let selected = text.get_selection().unwrap();
        assert_eq!(selected[0].get_text(None).unwrap(), "\r\nx");
        assert_eq!(selected[1].get_text(None).unwrap(), "z");

        for (query, expected) in [
            ("\r\nx", directed(3, 2, 6, 0)),
            ("yz", directed(6, 0, 8, 4)),
        ] {
            editor
                .set_selection(Selection::from_segments([original], 0))
                .unwrap();
            let target = find(&*doc, query);
            assert!(full.replace_in_selection(&*target).unwrap());
            assert_eq!(editor.selection().primary(), expected);
        }

        editor
            .set_selection(Selection::from_segments([original], 0))
            .unwrap();
        let caret = doc.clone_range().unwrap();
        caret
            .move_endpoint_by_range(Endpoint::Start, &*doc, Endpoint::End)
            .unwrap();
        assert!(full.replace_in_selection(&*caret).unwrap());
        assert_eq!(editor.selection(), Selection::empty(8));
    }
    assert_eq!(editor.text(), "abc\r\nxyz");
}

#[test]
fn character_movement_keeps_clusters_and_line_endings_intact() {
    let _scope = test_scope();
    let editor = laid_out_editor("x a\u{301}😀\r\nlast");
    let text = provider(&editor);
    let doc = text.document_range().unwrap();
    let range = find(&*doc, "😀");
    assert_eq!(range.move_by(TextUnit::Character, -1).unwrap(), -1);
    assert_eq!(range.get_text(None).unwrap(), "a\u{301}");
    assert_eq!(
        range
            .move_endpoint_by_unit(Endpoint::End, TextUnit::Character, 2)
            .unwrap(),
        2
    );
    assert_eq!(range.get_text(None).unwrap(), "a\u{301}😀\r\n");
    assert_eq!(
        range
            .move_endpoint_by_unit(Endpoint::Start, TextUnit::Character, i32::MIN)
            .unwrap(),
        -2
    );
    assert!(range.get_text(None).unwrap().starts_with("x "));
    range
        .move_endpoint_by_range(Endpoint::Start, &*doc, Endpoint::End)
        .unwrap();
    assert!(range.get_text(None).unwrap().is_empty());
    range.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "last");
}

#[test]
fn word_ranges_extend_to_the_next_lexical_start() {
    let _scope = test_scope();
    let editor = laid_out_editor("  Alpha,\t beta!\r\n\r\n  café 𐐷.");
    let text = provider(&editor);
    let document = text.document_range().unwrap();
    let word = find(&*document, "ph");
    word.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert_eq!(word.get_text(None).unwrap(), "Alpha,\t ");
    assert_eq!(word.move_by(TextUnit::Word, 1).unwrap(), 1);
    assert_eq!(word.get_text(None).unwrap(), "beta!\r\n\r\n  ");
    assert_eq!(word.move_by(TextUnit::Word, 1).unwrap(), 1);
    assert_eq!(word.get_text(None).unwrap(), "café ");
    assert_eq!(word.move_by(TextUnit::Word, i32::MAX).unwrap(), 1);
    assert_eq!(word.get_text(None).unwrap(), "𐐷.");

    let blank_line = find(&*document, "\r\n\r\n");
    blank_line
        .move_endpoint_by_unit(Endpoint::Start, TextUnit::Character, 1)
        .unwrap();
    blank_line.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert_eq!(blank_line.get_text(None).unwrap(), "beta!\r\n\r\n  ");
    assert_eq!(blank_line.move_endpoint_by_unit(Endpoint::End, TextUnit::Word, -2).unwrap(), -2);
    assert_eq!(blank_line.text_offsets().unwrap().unwrap(), 2..2);

    let leading = find(&*document, "  ");
    leading.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert_eq!(leading.get_text(None).unwrap(), "  ");
    assert_eq!(leading.move_by(TextUnit::Word, 1).unwrap(), 1);
    assert_eq!(leading.get_text(None).unwrap(), "Alpha,\t ");
}

#[test]
fn word_ranges_keep_blank_documents_navigable() {
    let _scope = test_scope();
    let editor = laid_out_editor(" \r\n\t\r\n");
    editor.set_caret_offset(0).unwrap();
    let text = provider(&editor);
    let document = text.document_range().unwrap();
    let caret = text.get_selection().unwrap().remove(0);
    assert_eq!(caret.move_by(TextUnit::Word, i32::MAX).unwrap(), 1);
    assert_eq!(caret.text_offsets().unwrap().unwrap(), 6..6);
    assert_eq!(caret.move_by(TextUnit::Word, i32::MIN).unwrap(), -1);
    caret.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert!(caret.compare(&*document).unwrap());
}

#[test]
fn format_ranges_report_real_styles_and_observe_format_and_document_changes() {
    let _scope = test_scope();
    let editor = laid_out_editor("x TALL x\nTALL");
    let text = provider(&editor);
    let doc = text.document_range().unwrap();
    let changes = Rc::new(Cell::new(0));
    let calls = changes.clone();
    let subscription = text
        .text_changed(Rc::new(move || calls.set(calls.get() + 1)))
        .unwrap();
    let definition = HighlightingDefinition::load(
        r#"<SyntaxDefinition><RuleSet><Rule fontSize="56">TALL</Rule></RuleSet></SyntaxDefinition>"#,
    ).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    let tall = TextAttributeValue::Number(42.0);
    assert_eq!(
        doc.get_attribute_value(TextAttribute::FontSize).unwrap(),
        TextAttributeValue::Mixed
    );
    let first = doc
        .find_attribute(TextAttribute::FontSize, &tall, false)
        .unwrap()
        .unwrap();
    let last = doc
        .find_attribute(TextAttribute::FontSize, &tall, true)
        .unwrap()
        .unwrap();
    assert_eq!(first.get_text(None).unwrap(), "TALL");
    assert_eq!(last.get_text(None).unwrap(), "TALL");
    assert_eq!(
        last.compare_endpoints(Endpoint::End, &*doc, Endpoint::End)
            .unwrap(),
        0
    );
    let format = find(&*doc, "ALL");
    format.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(format.get_text(None).unwrap(), "TALL");
    editor.document().insert(0, "#").unwrap();
    editor.set_document(TextDocument::new());
    assert_eq!(changes.get(), 3);
    subscription.dispose();
    editor.set_font_size(20.0);
    assert_eq!(changes.get(), 3);
}

#[test]
fn style_runs_clip_utf16_offsets_and_preserve_rich_formatting_across_lines() {
    use crate::{RichTextColorizer, VisualLineTransformerRef};
    use ferroui_base::media::Colors;
    use ferroui_edit_core::highlighting::RichTextModel;
    use std::cell::RefCell;

    let _scope = test_scope();
    let editor = laid_out_editor("a😀b\r\nc\0z");
    let mut model = RichTextModel::new();
    model.set_foreground(1, 2, Some("Red")).unwrap();
    model.set_font_size(1, 2, Some(24.0)).unwrap();
    model.set_background(6, 3, Some("Blue")).unwrap();
    model.set_underline(6, 3, Some(true)).unwrap();
    let transformer: VisualLineTransformerRef =
        Rc::new(RichTextColorizer::new(Rc::new(RefCell::new(model)))).into();
    editor.text_area().line_transformers().add(transformer);
    let text = provider(&editor);
    let runs = text.text_style_runs(2..8).unwrap().unwrap();
    let spans: Vec<_> = runs.iter().map(|run| run.range.clone()).collect();
    assert_eq!(spans, [2..3, 3..6, 6..8]);
    assert!(runs[0].attributes.contains(&(TextAttribute::ForegroundColor, TextAttributeValue::Color(Colors::RED))));
    assert!(runs[0].attributes.contains(&(TextAttribute::FontSize, TextAttributeValue::Number(18.0))));
    assert!(runs[2].attributes.contains(&(TextAttribute::BackgroundColor, TextAttributeValue::Color(Colors::BLUE))));
    assert!(runs[2].attributes.contains(&(TextAttribute::UnderlineStyle, TextAttributeValue::Integer(1))));
    assert!(text.text_style_runs(9..9).unwrap().unwrap().is_empty());
    assert!(text.text_style_runs(9..10).is_err());
}

#[test]
fn rich_text_overlays_notify_live_format_observers_after_redraw() {
    use crate::{RichTextColorizer, VisualLineTransformerRef};
    use ferroui_base::media::Colors;
    use ferroui_base::threading::Dispatcher;
    use ferroui_edit_core::highlighting::RichTextModel;
    use std::cell::RefCell;

    let _scope = test_scope();
    let editor = laid_out_editor(&format!("a😀b\r\n{}", "row\r\n".repeat(80)));
    let area = editor.text_area();
    let provider = provider(&editor);
    let document = provider.document_range().unwrap();
    let emoji = find(&*document, "😀");
    let base = emoji
        .get_attribute_value(TextAttribute::ForegroundColor)
        .unwrap();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let recorded = changes.clone();
    let live_range = emoji.clone();
    let subscription = provider
        .text_attributes_changed(Rc::new(move || {
            recorded.borrow_mut().push(
                live_range
                    .get_attribute_value(TextAttribute::ForegroundColor)
                    .unwrap(),
            );
        }))
        .unwrap()
        .unwrap();

    let model = Rc::new(RefCell::new(RichTextModel::new()));
    model
        .borrow_mut()
        .set_foreground(1, 2, Some("Red"))
        .unwrap();
    let extension: VisualLineTransformerRef = Rc::new(RichTextColorizer::new(model.clone())).into();
    area.line_transformers().add(extension.clone());
    Dispatcher::ui_thread().run_jobs(None);
    let red = TextAttributeValue::Color(Colors::RED);
    assert_eq!(changes.borrow().as_slice(), [red.clone()]);
    let found = document
        .find_attribute(TextAttribute::ForegroundColor, &red, false)
        .unwrap()
        .unwrap();
    assert_eq!(found.get_text(None).unwrap(), "😀");
    assert_eq!(provider.text_attribute_run(2).unwrap().unwrap().range, 1..3);

    model
        .borrow_mut()
        .set_foreground(1, 2, Some("Blue"))
        .unwrap();
    area.redraw_range(1..3).unwrap();
    area.redraw_range(1..3).unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    let blue = TextAttributeValue::Color(Colors::BLUE);
    assert_eq!(changes.borrow().as_slice(), [red, blue.clone()]);
    assert_eq!(
        emoji
            .get_attribute_value(TextAttribute::ForegroundColor)
            .unwrap(),
        blue
    );

    let length = editor.document().text_length();
    model
        .borrow_mut()
        .set_foreground(length - 5, 3, Some("Green"))
        .unwrap();
    area.redraw_range(length - 5..length - 2).unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 3);
    assert_eq!(
        provider
            .text_attribute_run(length - 5)
            .unwrap()
            .unwrap()
            .range,
        length - 5..length - 2
    );

    area.line_transformers().remove(&extension);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().last(), Some(&base));
    assert_eq!(changes.borrow().len(), 4);
    area.redraw();
    editor.set_document(TextDocument::new());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().len(), 4);
    subscription.dispose();
}

#[test]
fn format_observers_wait_for_the_editor_to_finish_rendering() {
    use crate::tests::render_editor;
    use crate::ColorizingTransformer;
    use ferroui_base::threading::Dispatcher;

    let _scope = test_scope();
    let editor = laid_out_editor("test");
    let text = provider(&editor);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let subscription = text
        .text_attributes_changed(Rc::new(move || observed.set(observed.get() + 1)))
        .unwrap()
        .unwrap();
    let in_transformer = calls.clone();
    let transformer = ColorizingTransformer::new(move |_, _| {
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(in_transformer.get(), 0);
        Ok(())
    });
    editor.text_area().line_transformers().add(Rc::new(transformer).into());
    render_editor(&editor);
    assert_eq!(calls.get(), 0);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(calls.get(), 1);
    subscription.dispose();
}

#[test]
fn selection_paint_keeps_the_accessible_foreground_and_format_boundaries() {
    use crate::tests::render_editor;
    use crate::{RichTextColorizer, VisualLineTransformerRef};
    use ferroui_base::media::{Color, Colors, SolidColorBrush};
    use ferroui_base::threading::Dispatcher;
    use ferroui_edit_core::highlighting::RichTextModel;
    use std::cell::RefCell;

    let _scope = test_scope();
    let editor = laid_out_editor("x ab😀cd x\n");
    let area = editor.text_area();
    let model = Rc::new(RefCell::new(RichTextModel::new()));
    model.borrow_mut().set_foreground(2, 6, Some("Red")).unwrap();
    let transformer: VisualLineTransformerRef = Rc::new(RichTextColorizer::new(model.clone())).into();
    area.line_transformers().add(transformer);
    let selection_color = Color::from_uint32(0xff338811);
    let selection_brush = SolidColorBrush::from_uint32(0xff338811);
    area.set_selection_foreground(Some(selection_brush.into()));
    Dispatcher::ui_thread().run_jobs(None);

    let provider = provider(&editor);
    let document = provider.document_range().unwrap();
    let emoji = find(&*document, "😀");
    let foreground = TextAttribute::ForegroundColor;
    let red = TextAttributeValue::Color(Colors::RED);
    assert_eq!(emoji.get_attribute_value(foreground).unwrap(), red);
    assert_eq!(provider.text_attribute_run(4).unwrap().unwrap().range, 2..8);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    let subscription = provider
        .text_attributes_changed(Rc::new(move || observed.set(observed.get() + 1)))
        .unwrap()
        .unwrap();

    emoji.select().unwrap();
    assert_eq!(provider.get_selection().unwrap()[0].get_text(None).unwrap(), "😀");
    assert!(render_editor(&editor).iter().any(|entry| entry.starts_with("DrawGlyphRun #ff338811")));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.get(), 0);
    assert_eq!(emoji.get_attribute_value(foreground).unwrap(), red);
    assert_eq!(provider.text_attribute_run(4).unwrap().unwrap().range, 2..8);
    let color_run = document.find_attribute(foreground, &red, false).unwrap().unwrap();
    assert_eq!(color_run.get_text(None).unwrap(), "ab😀cd");
    assert!(document.find_attribute(foreground, &TextAttributeValue::Color(selection_color), false).unwrap().is_none());
    let unit = emoji.clone_range().unwrap();
    unit.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert!(unit.compare(&*color_run).unwrap());

    model.borrow_mut().set_foreground(2, 6, Some("Blue")).unwrap();
    area.redraw_range(2..8).unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.get(), 1);
    assert_eq!(emoji.get_attribute_value(foreground).unwrap(), TextAttributeValue::Color(Colors::BLUE));
    assert_eq!(provider.get_selection().unwrap()[0].get_text(None).unwrap(), "😀");
    subscription.dispose();
}

#[test]
fn script_alignment_splits_text_attribute_runs() {
    use crate::{ColorizingTransformer, VisualLineTransformerRef};
    use ferroui_base::media::BaselineAlignment;

    let _scope = test_scope();
    let editor = laid_out_editor("abc");
    let transformer: VisualLineTransformerRef = Rc::new(ColorizingTransformer::new(|_, line| {
        line.change_document_part(1, 2, |element| {
            element.data_mut().text_run_properties_mut().set_baseline_alignment(BaselineAlignment::Superscript);
        })?;
        line.change_document_part(2, 3, |element| {
            element.data_mut().text_run_properties_mut().set_baseline_alignment(BaselineAlignment::Subscript);
        })
    })).into();
    editor.text_area().line_transformers().add(transformer);

    let provider = provider(&editor);
    let normal = provider.text_attribute_run(0).unwrap().unwrap();
    let super_run = provider.text_attribute_run(1).unwrap().unwrap();
    let sub_run = provider.text_attribute_run(2).unwrap().unwrap();
    assert_eq!(normal.range, 0..1);
    assert_eq!(super_run.range, 1..2);
    assert_eq!(sub_run.range, 2..3);
    assert!(super_run.attributes.contains(&(TextAttribute::IsSuperscript, TextAttributeValue::Bool(true))));
    assert!(sub_run.attributes.contains(&(TextAttribute::IsSubscript, TextAttributeValue::Bool(true))));
    let document = provider.document_range().unwrap();
    let letter = find(&*document, "b");
    letter.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(letter.get_text(None).unwrap(), "b");
}

#[test]
fn format_navigation_spans_lines_and_stops_at_protected_text() {
    let _scope = test_scope();
    let editor = laid_out_editor("alpha\r\n😀beta\r\ngamma");
    let text = provider(&editor);
    let document = text.document_range().unwrap();
    let run = find(&*document, "😀");
    run.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert!(run.compare(&*document).unwrap());

    let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    policy.add(7, 2).unwrap();
    editor.session().set_read_only_section_provider(policy);
    run.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(run.get_text(None).unwrap(), "alpha\r\n");
    assert_eq!(run.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(run.get_text(None).unwrap(), "😀");
    assert_eq!(run.get_attribute_value(TextAttribute::IsReadOnly).unwrap(), TextAttributeValue::Bool(true));
    assert_eq!(run.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(run.get_text(None).unwrap(), "beta\r\ngamma");
    let caret = text.range_from_offsets(7..7).unwrap().unwrap();
    assert_eq!(caret.move_endpoint_by_unit(Endpoint::Start, TextUnit::Format, -1).unwrap(), -1);
    assert_eq!(caret.get_text(None).unwrap(), "alpha\r\n");
    assert_eq!(run.move_by(TextUnit::Format, -2).unwrap(), -2);
    assert_eq!(run.get_text(None).unwrap(), "alpha\r\n");
    let length = editor.document().text_length();
    let end = text.range_from_offsets(length..length).unwrap().unwrap();
    end.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(end.get_text(None).unwrap(), "beta\r\ngamma");
}

#[test]
fn format_navigation_stops_at_hyperlink_edges() {
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;

    let _scope = test_scope();
    let editor = laid_out_editor("alpha https://example.org next\r\nline");
    editor.text_area().set_selection_foreground(Some(Rc::new(
        ImmutableSolidColorBrush::from_uint32(0xff338811),
    )));
    let text = provider(&editor);
    let document = text.document_range().unwrap();
    let run = find(&*document, "example");
    run.select().unwrap();
    run.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(run.get_text(None).unwrap(), "https://example.org");
    assert_eq!(run.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(run.get_text(None).unwrap(), " next\r\nline");
    assert_eq!(run.move_by(TextUnit::Format, -2).unwrap(), -2);
    assert_eq!(run.get_text(None).unwrap(), "alpha ");
}

#[test]
fn hyperlink_ranges_expose_their_styling_and_keep_distinct_adjacent_links() {
    use crate::{ColorizingTransformer, LinkElementGenerator, LinkTextStyle};
    use ferroui_base::media::{Colors, TextDecorations, immutable::ImmutableSolidColorBrush};

    let _scope = test_scope();
    let editor = laid_out_editor("ab😀cd rest");
    let mut generator = LinkElementGenerator::with_pattern("ab😀|cd", |_| {
        url::Url::parse("https://example.org").ok()
    }).unwrap();
    generator.style = LinkTextStyle {
        foreground: Some(Rc::new(ImmutableSolidColorBrush::new(Colors::RED))),
        background: Some(Rc::new(ImmutableSolidColorBrush::new(Colors::YELLOW))),
        underline: true,
    };
    let area = editor.text_area();
    area.element_generators().add(Rc::new(generator).into());
    area.line_transformers().add(Rc::new(ColorizingTransformer::new(|_, builder| {
        builder.change_document_part(2, 4, |element| {
            element.data_mut().text_run_properties_mut()
                .set_text_decorations(Some(TextDecorations::underline()));
        })
    })).into());

    let provider = provider(&editor);
    let document = provider.document_range().unwrap();
    let emoji = find(&*document, "😀");
    let foreground = TextAttributeValue::Color(Colors::RED);
    assert_eq!(emoji.get_attribute_value(TextAttribute::ForegroundColor).unwrap(), foreground);
    assert_eq!(emoji.get_attribute_value(TextAttribute::BackgroundColor).unwrap(), TextAttributeValue::Color(Colors::YELLOW));
    assert_eq!(emoji.get_attribute_value(TextAttribute::UnderlineStyle).unwrap(), TextAttributeValue::Integer(1));
    let found = document.find_attribute(TextAttribute::ForegroundColor, &foreground, false).unwrap().unwrap();
    assert_eq!(found.get_text(None).unwrap(), "ab😀cd");
    assert_eq!(provider.text_attribute_run(3).unwrap().unwrap().range, 0..4);
    assert_eq!(provider.text_attribute_run(4).unwrap().unwrap().range, 4..6);
    emoji.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(emoji.get_text(None).unwrap(), "ab😀");
    assert_eq!(emoji.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(emoji.get_text(None).unwrap(), "cd");
    let names: Vec<_> = document.get_children().unwrap().iter().map(|link| link.get_name()).collect();
    assert_eq!(names, ["ab😀", "cd"]);
}

#[test]
fn editor_hyperlink_peer_follows_edits_and_invokes_the_existing_uri_route() {
    use crate::{OpenUriRoutedEventArgs, VisualLineLinkText};
    use ferroui_controls::automation::peers::AutomationControlType;
    use ferroui_controls::automation::provider::{IInvokeProvider, ITextChildProvider};
    use std::cell::RefCell;

    let _scope = test_scope();
    let uri = "https://example.org";
    let normalized_uri = url::Url::parse(uri).unwrap();
    let (editor, _window) = rooted(&format!("visit {uri} now"), Size::new(320.0, 140.0), true);
    let area = editor.text_area();
    let owner = ControlAutomationPeer::create_peer_for_element(&editor);
    let provider = provider(&editor);
    let document = provider.document_range().unwrap();
    let children = document.get_children().unwrap();
    assert_eq!(children.len(), 1);
    let link = &children[0];
    assert_eq!(link.get_automation_control_type(), AutomationControlType::Hyperlink);
    assert_eq!(link.get_name(), uri);
    assert!(owner.get_children().contains(link));
    assert_eq!(link.get_parent(), Some(owner.clone()));
    assert!(!link.get_bounding_rectangle().is_empty());
    assert_eq!(document.get_enclosing_element().unwrap(), owner);

    let partial = find(&*document, "example");
    partial.select().unwrap();
    assert_eq!(partial.get_enclosing_element().unwrap(), *link);
    assert!(partial.get_children().unwrap().is_empty());
    let child = link.get_provider::<dyn ITextChildProvider>().unwrap();
    let range = child.text_range().unwrap();
    assert_eq!(child.text_container().unwrap(), owner);
    assert_eq!(range.get_text(None).unwrap(), uri);
    assert_eq!(range.get_enclosing_element().unwrap(), *link);
    assert!(range.get_children().unwrap().is_empty());
    assert!(range.compare(&*provider.range_from_child(link).unwrap()).unwrap());
    area.ensure_visual_lines().unwrap();
    let changes = Rc::new(Cell::new(0));
    let notifications = changes.clone();
    let _subscription = owner.children_changed(move || notifications.set(notifications.get() + 1));

    let opened = Rc::new(RefCell::new(Vec::new()));
    let observed = opened.clone();
    area.add_handler(&VisualLineLinkText::open_uri_event(), move |_, event: &OpenUriRoutedEventArgs| {
        observed.borrow_mut().push(event.uri.as_str().to_owned());
        event.set_handled(true);
    });
    let invoke = link.get_provider::<dyn IInvokeProvider>().unwrap();
    editor.document().insert(0, "Please ").unwrap();
    area.redraw();
    assert!(owner.get_children().contains(link));
    assert_eq!(child.text_range().unwrap().get_text(None).unwrap(), uri);
    invoke.invoke().unwrap();
    assert_eq!(opened.borrow().as_slice(), [normalized_uri.as_str()]);
    editor.set_is_enabled(false);
    assert!(invoke.invoke().is_err());
    editor.set_is_enabled(true);
    let mut options = editor.options();
    options.enable_hyperlinks = false;
    editor.set_options(options);
    assert!(child.text_range().is_err());
    assert!(!link.is_element_available());
    assert!(invoke.invoke().is_err());
    assert!(!owner.get_children().contains(link));
    area.ensure_visual_lines().unwrap();
    assert_eq!(changes.get(), 1);
    assert_eq!(opened.borrow().as_slice(), [normalized_uri.as_str()]);
}

#[test]
fn object_hit_testing_returns_the_complete_wrapped_link_with_clipped_geometry() {
    use crate::ColorizingTransformer;
    use ferroui_base::media::TextDecorations;

    let _scope = test_scope();
    let uri = "https://example.org/folder/another/filename";
    let (editor, _window) = rooted(
        &format!("visit {uri}\n{}last", "row\n".repeat(40)),
        Size::new(157.0, 150.0),
        true,
    );
    // A style change splits the visual element within the same logical link.
    editor.text_area().line_transformers().add(Rc::new(ColorizingTransformer::new(|_, builder| {
        if builder.first_document_line().offset() != 0 {
            return Ok(());
        }
        builder.change_document_part(14, 16, |element| {
            element.data_mut().text_run_properties_mut()
                .set_text_decorations(Some(TextDecorations::underline()));
        })
    })).into());
    let text = provider(&editor);
    let document = text.document_range().unwrap();
    let link = find(&*document, uri);
    let rectangles = link.get_bounding_rectangles().unwrap();
    assert!(rectangles.len() >= 2);
    let points: Vec<_> = rectangles.iter().map(|rect| {
        Point::new(rect.right() - 0.5, rect.y + rect.height / 2.0)
    }).collect();
    for &point in &points {
        let object = text.embedded_range_from_point(point).unwrap()
            .unwrap_or_else(|| panic!("No hyperlink range at {point:?}"));
        assert!(object.compare(&*link).unwrap());
        assert_eq!(object.get_enclosing_element().unwrap().get_name(), uri);
        assert!(text.range_from_point(point).unwrap().get_text(None).unwrap().is_empty());
    }
    let plain = find(&*document, "visit").get_bounding_rectangles().unwrap()[0];
    assert!(text.embedded_range_from_point(Point::new(plain.x + 1.0, plain.y + 2.0)).unwrap().is_none());
    find(&*document, "last").scroll_into_view(false).unwrap();
    for point in points {
        assert!(text.embedded_range_from_point(point).unwrap().is_none());
    }
}

#[test]
fn editor_hyperlink_peer_projects_offscreen_links_and_expires_on_document_swap() {
    use ferroui_controls::automation::provider::ITextChildProvider;

    let _scope = test_scope();
    let uri = "https://example.org";
    let source = format!("{}{uri}", "line\n".repeat(60));
    let (editor, _window) = rooted(&source, Size::new(320.0, 140.0), false);
    let owner = ControlAutomationPeer::create_peer_for_element(&editor);
    let provider = provider(&editor);
    let range = provider.document_range().unwrap();
    let children = range.get_children().unwrap();
    assert_eq!(children.len(), 1);
    let link = &children[0];
    assert!(!owner.get_children().contains(link));
    assert!(link.is_offscreen());
    link.bring_into_view();
    assert!(owner.get_children().contains(link));
    assert!(!link.is_offscreen());

    let child = link.get_provider::<dyn ITextChildProvider>().unwrap();
    let new_document = TextDocument::new();
    new_document.insert(0, uri).unwrap();
    editor.set_document(new_document);
    assert!(child.text_range().is_err());
    let whole = provider.document_range().unwrap();
    let new_children = whole.get_children().unwrap();
    assert_eq!(whole.get_enclosing_element().unwrap(), owner);
    assert_eq!(new_children.len(), 1);
    assert_ne!(new_children[0], *link);
    let new_child = provider.range_from_child(&new_children[0]).unwrap();
    assert_eq!(new_child.get_enclosing_element().unwrap(), new_children[0]);
    assert!(new_child.get_children().unwrap().is_empty());
}

#[test]
fn built_in_link_discovery_skips_hidden_rows_and_keeps_the_visible_suffix_of_a_fold() {
    let _scope = test_scope();
    let source = format!(
        "https://collapsed.example.org\r\n{}begin {{\r\nhttps://folded.example.org\r\n}} www.example.org and name@example.org\r\nhttps://example.net/path",
        "let member = path::items.len();\r\n".repeat(60),
    );
    let (editor, _window) = rooted(&source, Size::new(320.0, 140.0), false);
    let document = editor.document();
    let area = editor.text_area();
    let first = document.get_live_line_by_number(1).unwrap();
    let collapsed = area.collapse_lines(&first, &first).unwrap();
    let folds = editor.install_folding();
    let start = source.find('{').unwrap();
    let end = source.find('}').unwrap() + 1;
    let fold = folds.create_folding(start, end).unwrap();
    fold.set_is_folded(true);

    let provider = provider(&editor);
    let range = provider.document_range().unwrap();
    let names = || range.get_children().unwrap().into_iter().map(|link| link.get_name()).collect::<Vec<_>>();
    assert_eq!(names(), ["www.example.org", "name@example.org", "https://example.net/path"]);
    let start = source.find("www.example.org").unwrap();
    let clipped = provider.range_from_offsets(start + 1..source.len()).unwrap().unwrap();
    let clipped_names = clipped.get_children().unwrap().into_iter().map(|link| link.get_name()).collect::<Vec<_>>();
    assert_eq!(clipped_names, ["name@example.org", "https://example.net/path"]);

    collapsed.uncollapse();
    fold.set_is_folded(false);
    assert_eq!(names(), [
        "https://collapsed.example.org", "https://folded.example.org", "www.example.org",
        "name@example.org", "https://example.net/path",
    ]);
}

#[test]
fn link_discovery_runs_custom_construction_hooks_for_offscreen_lines() {
    let _scope = test_scope();
    let editor = laid_out_editor(&"ordinary text\n".repeat(40));
    let mut options = editor.options();
    options.enable_hyperlinks = false;
    options.enable_email_hyperlinks = false;
    editor.set_options(options);
    let visited = Rc::new(Cell::new(0));
    let last = visited.clone();
    let _subscription = editor.text_area().visual_line_construction_starting(move |event| {
        last.set(last.get().max(event.first_line_in_view.line_number()));
    });
    assert!(provider(&editor).document_range().unwrap().get_children().unwrap().is_empty());
    assert_eq!(visited.get(), 40);
}

#[test]
fn batched_formatting_reports_inherited_style_and_protected_run_boundaries() {
    use ferroui_base::media::{FontStyle, FontWeight};
    use ferroui_edit_core::editing::TextSegmentReadOnlySectionProvider;
    use TextAttribute as A;
    use TextAttributeValue as V;

    let _scope = test_scope();
    let editor = laid_out_editor("😀 TALL tail");
    let provider = provider(&editor);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    let subscription = provider.text_attributes_changed(Rc::new(move || {
        observed.set(observed.get() + 1);
    })).unwrap().unwrap();

    let definition = HighlightingDefinition::load(r##"
        <SyntaxDefinition><RuleSet><Rule fontSize="56" fontStyle="Oblique"
          fontWeight="Bold" foreground="#112233" underline="true"
          strikethrough="true">TALL</Rule></RuleSet></SyntaxDefinition>
    "##).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    policy.add(4, 1).unwrap();
    editor.session().set_read_only_section_provider(policy);

    let inherited = provider.default_text_attributes().unwrap();
    let unformatted = provider.text_attribute_run(0).unwrap().unwrap();
    assert_eq!(unformatted.range, 0..3);
    assert_eq!(unformatted.attributes, inherited);
    assert!(inherited.contains(&(A::FontStyle, V::String("normal".into()))));
    assert!(inherited.contains(&(A::UnderlineStyle, V::Integer(0))));

    let left = provider.text_attribute_run(3).unwrap().unwrap();
    let protected = provider.text_attribute_run(4).unwrap().unwrap();
    let right = provider.text_attribute_run(5).unwrap().unwrap();
    assert_eq!(left.range, 3..4);
    assert_eq!(protected.range, 4..5);
    assert_eq!(right.range, 5..7);
    assert_eq!(left.attributes, right.attributes);
    for (attribute, value) in [
        (A::FontSize, V::Number(42.0)),
        (A::FontStyle, V::String("oblique".into())),
        (A::FontWeight, V::Integer(700)),
        (A::ForegroundColor, V::Color(ferroui_base::media::Color::from_rgb(17, 34, 51))),
        (A::UnderlineStyle, V::Integer(1)),
        (A::StrikethroughStyle, V::Integer(1)),
        (A::IsReadOnly, V::Bool(true)),
    ] {
        assert!(protected.attributes.contains(&(attribute, value)));
    }
    assert!(left.attributes.contains(&(A::IsReadOnly, V::Bool(false))));
    assert!(provider.text_attribute_run(13).unwrap().is_none());

    editor.document().insert(12, "!").unwrap();
    assert_eq!(changes.get(), 1);
    editor.set_font_size(24.0);
    assert_eq!(changes.get(), 2);
    assert!(provider.default_text_attributes().unwrap().contains(&(A::FontSize, V::Number(18.0))));
    editor.set_font_style(FontStyle::Italic);
    editor.set_font_weight(FontWeight::SemiBold);
    assert_eq!(changes.get(), 4);
    let plain = provider.text_attribute_run(0).unwrap().unwrap();
    assert!(plain.attributes.contains(&(A::FontStyle, V::String("italic".into()))));
    assert!(plain.attributes.contains(&(A::FontWeight, V::Integer(600))));
    subscription.dispose();
    editor.set_font_size(28.0);
    assert_eq!(changes.get(), 4);
}

#[test]
fn geometry_uses_wrapped_rows_root_coordinates_and_clips_offscreen_text() {
    let _scope = test_scope();
    let size = Size::new(108.0, 58.0);
    let (editor, _window) = rooted(
        &format!(
            "first {}\n{}\nlast",
            "words ".repeat(24),
            "row\n".repeat(40)
        ),
        size,
        true,
    );
    let text = provider(&editor);
    let doc = text.document_range().unwrap();
    let first = find(&*doc, "first");
    let row = first.clone_range().unwrap();
    row.expand_to_enclosing_unit(TextUnit::Line).unwrap();
    assert!(row.get_text(None).unwrap().starts_with("first "));
    assert!(row.get_text(None).unwrap().len() < 24 * 6);
    let rect = first.get_bounding_rectangles().unwrap()[0];
    let hit = text
        .range_from_point(Point::new(rect.x + 1.0, rect.y + rect.height / 2.0))
        .unwrap();
    assert_eq!(
        hit.compare_endpoints(Endpoint::Start, &*first, Endpoint::Start)
            .unwrap(),
        0
    );
    assert!(
        text.get_visible_ranges()
            .unwrap()
            .iter()
            .all(|range| !range.get_text(None).unwrap().contains("last"))
    );
    let last = find(&*doc, "last");
    assert!(last.get_bounding_rectangles().unwrap().is_empty());
    last.scroll_into_view(false).unwrap();
    assert!(editor.selection().is_empty());
    assert!(first.get_bounding_rectangles().unwrap().is_empty());
    assert!(!last.get_bounding_rectangles().unwrap().is_empty());
    assert!(
        text.get_visible_ranges()
            .unwrap()
            .iter()
            .any(|range| range.get_text(None).unwrap().contains("last"))
    );
}

#[test]
fn scroll_placement_uses_native_bounds_without_moving_the_selection() {
    use ferroui_base::input::IScrollable;

    let _scope = test_scope();
    let (editor, window) = rooted(
        &format!("{}{}😀 אבג marker{}\n{}last", "row\n".repeat(40),
            "x".repeat(75), "y".repeat(80), "row\n".repeat(40)),
        Size::new(180.0, 112.0),
        false,
    );
    let area = editor.text_area();
    let text = provider(&editor);
    let document = text.document_range().unwrap();
    let target = find(&*document, "😀 אבג marker");
    let last = find(&*document, "last");
    let selection = editor.selection();
    let viewport = area.content_viewport();
    let transform = area.transform_to_visual(&window).unwrap();
    let top = viewport.top_left().transform(transform);
    let bottom = viewport.bottom_right().transform(transform);
    let bounds = || target.get_bounding_rectangles().unwrap().into_iter().reduce(|a, b| a.union(b)).unwrap();
    let near = |actual: f64, wanted: f64| {
        assert!((actual - wanted).abs() <= 1.0, "wanted {wanted}, got {actual}");
    };

    assert!(target.get_bounding_rectangles().unwrap().is_empty());
    assert!(target.scroll_to(TextScrollPlacement::TopLeft).unwrap());
    near(bounds().left(), top.x);
    near(bounds().top(), top.y);
    assert!(target.scroll_to(TextScrollPlacement::BottomRight).unwrap());
    near(bounds().right(), bottom.x);
    near(bounds().bottom(), bottom.y);

    let point = Point::new(top.x + 23.0, top.y + 26.0);
    assert!(target.scroll_to(TextScrollPlacement::Point(point)).unwrap());
    near(bounds().left(), point.x);
    near(bounds().top(), point.y);
    let offset = area.offset();
    assert!(target.scroll_to(TextScrollPlacement::Nearest).unwrap());
    assert_eq!(area.offset(), offset);
    let outside = Point::new(top.x - 3.0, top.y - 3.0);
    assert!(!target.scroll_to(TextScrollPlacement::Point(outside)).unwrap());
    assert_eq!(area.offset(), offset);

    assert!(document.scroll_to(TextScrollPlacement::Bottom).unwrap());
    assert!(!last.get_bounding_rectangles().unwrap().is_empty());
    assert_eq!(editor.selection(), selection);
}

#[test]
fn folded_document_text_is_searchable_but_omitted_from_the_visible_projection() {
    let _scope = test_scope();
    let (editor, _window) = rooted("pre HIDDEN\nsuffix end", Size::new(340.0, 100.0), false);
    let folding = editor.install_folding();
    let section = folding.create_folding(4, 10).unwrap();
    section.set_is_folded(true);
    let text = provider(&editor);
    let doc = text.document_range().unwrap();
    let hidden = find(&*doc, "HIDDEN");
    assert_eq!(
        hidden.get_attribute_value(TextAttribute::IsHidden).unwrap(),
        TextAttributeValue::Bool(true)
    );
    assert!(hidden.get_bounding_rectangles().unwrap().is_empty());
    hidden.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(hidden.get_text(None).unwrap(), "HIDDEN");
    assert_eq!(hidden.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(hidden.get_text(None).unwrap(), "\nsuffix end");
    assert_eq!(
        doc.get_attribute_value(TextAttribute::IsHidden).unwrap(),
        TextAttributeValue::Mixed
    );
    let visible = text
        .get_visible_ranges()
        .unwrap()
        .iter()
        .map(|range| range.get_text(None).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(visible, ["pre ", "\nsuffix end"]);
}

#[test]
fn explicitly_hidden_rows_remain_navigable_and_notify_visibility_changes() {
    use ferroui_base::input::IScrollable;
    use ferroui_base::threading::Dispatcher;
    use std::cell::RefCell;

    let _scope = test_scope();
    let source = "top hidden\r\nbefore\r\nhidden one\r\nhidden two\r\nafter";
    let (editor, _window) = rooted(source, Size::new(340.0, 180.0), false);
    let document = editor.document();
    let area = editor.text_area();
    let text = provider(&editor);
    let range = text.document_range().unwrap();
    let top = find(&*range, "top hidden");
    let hidden = find(&*range, "hidden two");
    let after = find(&*range, "after");
    let changes = Rc::new(RefCell::new(Vec::new()));
    let reported = changes.clone();
    let observed = hidden.clone();
    let subscription = text.text_attributes_changed(Rc::new(move || {
        reported.borrow_mut().push(observed.get_attribute_value(TextAttribute::IsHidden).unwrap());
    })).unwrap().unwrap();
    let line = |number| document.get_live_line_by_number(number).unwrap();

    let first = area.collapse_lines(&line(1), &line(1)).unwrap();
    let middle = area.collapse_lines(&line(3), &line(4)).unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    let yes = TextAttributeValue::Bool(true);
    assert_eq!(changes.borrow().as_slice(), [yes.clone()]);
    assert_eq!(range.get_text(None).unwrap(), source);
    assert_eq!(top.get_attribute_value(TextAttribute::IsHidden).unwrap(), yes);
    assert_eq!(hidden.get_attribute_value(TextAttribute::IsHidden).unwrap(), yes);
    assert_eq!(range.get_attribute_value(TextAttribute::IsHidden).unwrap(), TextAttributeValue::Mixed);
    let found = range.find_attribute(TextAttribute::IsHidden, &yes, true).unwrap().unwrap();
    assert_eq!(found.get_text(None).unwrap(), "hidden one\r\nhidden two\r\n");
    let font = after.get_attribute_value(TextAttribute::FontName).unwrap();
    assert_eq!(range.get_attribute_value(TextAttribute::FontName).unwrap(), font);
    assert_eq!(range.find_attribute(TextAttribute::FontName, &font, true).unwrap().unwrap().get_text(None).unwrap(), source);

    let run = text.text_attribute_run(source.find("hidden two").unwrap()).unwrap().unwrap();
    assert!(run.attributes.contains(&(TextAttribute::IsHidden, yes.clone())));
    let unit = hidden.clone_range().unwrap();
    unit.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert!(unit.compare(&*found).unwrap());
    let row = hidden.clone_range().unwrap();
    row.expand_to_enclosing_unit(TextUnit::Line).unwrap();
    assert_eq!(row.get_text(None).unwrap(), "hidden two\r\n");
    assert_eq!(row.move_by(TextUnit::Line, 1).unwrap(), 1);
    assert_eq!(row.get_text(None).unwrap(), "after");
    assert_eq!(row.move_by(TextUnit::Line, -1).unwrap(), -1);
    assert_eq!(row.get_text(None).unwrap(), "hidden two\r\n");

    assert!(top.get_bounding_rectangles().unwrap().is_empty());
    assert!(hidden.get_bounding_rectangles().unwrap().is_empty());
    let offset = area.offset();
    assert!(!top.scroll_to(TextScrollPlacement::Nearest).unwrap());
    hidden.scroll_into_view(false).unwrap();
    assert_eq!(area.offset(), offset);
    assert!(range.scroll_to(TextScrollPlacement::Nearest).unwrap());
    let visible = text.get_visible_ranges().unwrap();
    let visible: Vec<_> = visible.iter().map(|range| range.get_text(None).unwrap()).collect();
    assert_eq!(visible, ["before\r\n", "after"]);

    let all = area.collapse_lines(&line(1), &line(5)).unwrap();
    assert!(range.get_bounding_rectangles().unwrap().is_empty());
    assert!(!range.scroll_to(TextScrollPlacement::Top).unwrap());
    assert_eq!(range.get_attribute_value(TextAttribute::IsHidden).unwrap(), yes);
    assert_eq!(text.get_visible_ranges().unwrap()[0].get_text(None).unwrap(), "");
    all.uncollapse();
    first.uncollapse();
    middle.uncollapse();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(changes.borrow().as_slice(), [yes, TextAttributeValue::Bool(false)]);
    assert!(!hidden.get_bounding_rectangles().unwrap().is_empty());
    subscription.dispose();
}

#[test]
fn explicit_collapse_preserves_the_suffix_projected_by_a_custom_multiline_element() {
    use crate::{FormattedTextElement, ITextRunConstructionContext, ITextViewConnect, VisualLineElement, VisualLineElementGenerator, VisualLineError};

    struct Label(std::ops::Range<usize>);
    impl ITextViewConnect for Label {}
    impl VisualLineElementGenerator for Label {
        fn get_first_interested_offset(
            &self, start: usize, _context: &dyn ITextRunConstructionContext,
        ) -> Result<Option<usize>, VisualLineError> {
            Ok((start <= self.0.start).then_some(self.0.start))
        }

        fn construct_element(
            &self, _offset: usize, context: &dyn ITextRunConstructionContext,
        ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
            Ok(Some(Box::new(FormattedTextElement::new(
                "label", self.0.len(), &**context.global_text_run_properties(),
            )?)))
        }
    }

    let _scope = test_scope();
    let source = "head {\r\ninside\r\n} suffix\r\nlast";
    let (editor, _window) = rooted(source, Size::new(340.0, 140.0), false);
    let document = editor.document();
    let area = editor.text_area();
    let start = source.find('{').unwrap();
    let end = source.find('}').unwrap() + 1;
    area.element_generators().add(Rc::new(Label(start..end)).into());
    let first = document.get_live_line_by_number(2).unwrap();
    let last = document.get_live_line_by_number(3).unwrap();
    let _section = area.collapse_lines(&first, &last).unwrap();
    let text = provider(&editor);
    let range = text.document_range().unwrap();
    let suffix = find(&*range, "suffix");
    assert_eq!(suffix.get_attribute_value(TextAttribute::IsHidden).unwrap(), TextAttributeValue::Bool(false));
    assert!(!suffix.get_bounding_rectangles().unwrap().is_empty());
    let visible = text.get_visible_ranges().unwrap();
    assert!(visible.iter().any(|range| range.get_text(None).unwrap().contains("suffix")));
    let run = text.text_attribute_run(source.find("suffix").unwrap()).unwrap().unwrap();
    assert!(run.attributes.contains(&(TextAttribute::IsHidden, TextAttributeValue::Bool(false))));
}

#[test]
fn a_protected_span_exposes_a_distinct_read_only_attribute() {
    use ferroui_edit_core::editing::TextSegmentReadOnlySectionProvider;
    let _scope = test_scope();
    let editor = laid_out_editor("pre protected post");
    let read_only = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    read_only.add(4, 9).unwrap();
    editor.session().set_read_only_section_provider(read_only);
    let doc = provider(&editor).document_range().unwrap();
    let attribute = TextAttribute::IsReadOnly;
    assert_eq!(
        doc.get_attribute_value(attribute).unwrap(),
        TextAttributeValue::Mixed
    );
    let found = doc
        .find_attribute(attribute, &TextAttributeValue::Bool(true), false)
        .unwrap()
        .unwrap();
    assert_eq!(found.get_text(None).unwrap(), "protected");
    assert_eq!(
        find(&*doc, "post").get_attribute_value(attribute).unwrap(),
        TextAttributeValue::Bool(false)
    );
}

#[test]
fn the_visible_projection_respects_horizontal_scrolling_and_bidi_runs() {
    use ferroui_base::Vector;
    use ferroui_base::input::IScrollable;
    let _scope = test_scope();
    let (editor, _window) = rooted(
        "left אבגדה middle 1234567890 very-far-right",
        Size::new(112.0, 60.0),
        false,
    );
    let area = editor.text_area();
    let text = provider(&editor);
    let doc = text.document_range().unwrap();
    let rtl = find(&*doc, "אבגדה");
    let right = find(&*doc, "very-far-right");
    assert!(!rtl.get_bounding_rectangles().unwrap().is_empty());
    assert!(right.get_bounding_rectangles().unwrap().is_empty());
    let visible = text
        .get_visible_ranges()
        .unwrap()
        .iter()
        .map(|range| range.get_text(None).unwrap())
        .collect::<String>();
    assert!(visible.starts_with("left אבגדה"), "{visible:?}");
    assert!(!visible.contains("very-far-right"));
    assert!(area.can_horizontally_scroll());
    area.set_offset(Vector::new(area.extent().width, 0.0));
    assert!(rtl.get_bounding_rectangles().unwrap().is_empty());
    let visible = text
        .get_visible_ranges()
        .unwrap()
        .iter()
        .map(|range| range.get_text(None).unwrap())
        .collect::<String>();
    assert!(visible.ends_with("very-far-right"), "{visible:?}");
}
