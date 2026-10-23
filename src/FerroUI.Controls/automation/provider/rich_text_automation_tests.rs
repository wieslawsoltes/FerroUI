use super::{
    IInvokeProvider, ITextChildProvider, ITextEditProvider, ITextProvider, ITextProvider2,
    ITextRangeProvider, IValueProvider, SupportedTextSelection, TextAttribute, TextAttributeValue,
    TextEditChangeType, TextEditTextChangedEventArgs, TextPatternRangeEndpoint as Endpoint,
    TextProviderError, TextUnit,
};
use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::AutomationProperties;
use crate::documents::{
    Bold, CharacterFormat, Hyperlink, InlineUIContainer, Paragraph, RichTextDocument, Run, Span,
    UnderlineStyle,
};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, RichEditBox, RichTextBlock};
use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{
    Brushes, Color, FlowDirection, FontWeight, TextAlignment, TextDecoration,
    TextDecorationCollection, TextDecorationLocation,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroLocator, Point, Rect, Ref, Size, Thickness};
use std::cell::Cell;
use std::rc::Rc;

/// Focus goes through the framework keyboard device, which the layout-only
/// TestRoot does not register. Keep the registration scoped to the test.
struct KeyboardScope(Rc<dyn IDisposable>);

impl KeyboardScope {
    fn new() -> Self {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn IKeyboardDevice>()
            .to_constant(KeyboardDevice::new());
        Self(scope)
    }
}

impl Drop for KeyboardScope {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

fn provider(view: &Control) -> Rc<dyn ITextProvider> {
    ControlAutomationPeer::create_peer_for_element(view)
        .get_provider::<dyn ITextProvider>()
        .expect("Rich text exposes the Text pattern")
}

fn rich(text: &str) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text(text));
    view.set_is_text_selection_enabled(true);
    view
}

fn apply_editor_template(editor: &RichEditBox, padding: Thickness) {
    use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    editor.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
        move |_owner, names| {
            let content = Border::new();
            content.set_name(Some("ContentElement".to_owned()));
            let border = Border::new();
            border.set_padding(padding);
            border.set_child(content.register_in_name_scope(&**names));
            border.upcast()
        },
    )));
    editor.apply_template();
}

fn found(provider: &dyn ITextProvider, text: &str) -> Rc<dyn ITextRangeProvider> {
    provider
        .document_range()
        .unwrap()
        .find_text(text, false, false)
        .unwrap()
        .unwrap()
}

#[test]
fn rich_automation_document_text_preserves_utf16_boundaries_and_read_only_semantics() {
    let _scope = test_scope();
    let view = rich("a😀b\nnext");
    let text = provider(&view);
    let range = text.document_range().unwrap();
    assert_eq!(range.get_text(None).unwrap(), "a😀b\nnext");
    assert_eq!(range.get_text(Some(0)).unwrap(), "");
    assert_eq!(range.get_text(Some(2)).unwrap(), "a");
    assert_eq!(range.get_text(Some(3)).unwrap(), "a😀");
    assert_eq!(
        range
            .get_attribute_value(TextAttribute::IsReadOnly)
            .unwrap(),
        TextAttributeValue::Bool(true)
    );
    assert_eq!(
        text.supported_text_selection().unwrap(),
        SupportedTextSelection::Single
    );
    let emoji = found(&*text, "😀");
    assert_eq!(
        emoji
            .compare_endpoints(Endpoint::End, &*emoji, Endpoint::Start)
            .unwrap(),
        2
    );
}

#[test]
fn rich_automation_editor_exposes_text_and_value_over_the_same_undoable_document() {
    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor.set_text("before").unwrap();
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let value = peer.get_provider::<dyn IValueProvider>().unwrap();
    let text = peer.get_provider::<dyn ITextProvider>().unwrap();
    assert_eq!(
        text.document_range()
            .unwrap()
            .get_attribute_value(TextAttribute::IsReadOnly)
            .unwrap(),
        TextAttributeValue::Bool(false)
    );
    value.set_value(Some("after")).unwrap();
    assert_eq!(
        text.document_range().unwrap().get_text(None).unwrap(),
        "after"
    );
    assert!(editor.undo().unwrap());
    assert_eq!(
        text.document_range().unwrap().get_text(None).unwrap(),
        "before"
    );
    editor.set_is_read_only(true);
    assert_eq!(
        text.document_range()
            .unwrap()
            .get_attribute_value(TextAttribute::IsReadOnly)
            .unwrap(),
        TextAttributeValue::Bool(true)
    );
    assert!(value.set_value(Some("forbidden")).is_err());
    found(&*text, "before").select().unwrap();
    assert_eq!(editor.selected_text(), "before");
}

#[test]
fn rich_automation_selection_matches_the_view_and_rejects_disjoint_selection() {
    let _scope = test_scope();
    let view = rich("alpha beta gamma");
    let text = provider(&view);
    let alpha = found(&*text, "alpha");
    let beta = found(&*text, "beta");
    let gamma = found(&*text, "gamma");
    beta.select().unwrap();
    assert_eq!((view.selection_start(), view.selection_end()), (6, 10));
    assert_eq!(
        text.get_selection().unwrap()[0].get_text(None).unwrap(),
        "beta"
    );
    assert_eq!(
        alpha
            .add_to_selection()
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::MultipleSelectionNotSupported)
    );
    text.document_range().unwrap().select().unwrap();
    assert!(beta.remove_from_selection().is_err());
    alpha.remove_from_selection().unwrap();
    assert_eq!(view.selected_text(), " beta gamma");
    gamma.remove_from_selection().unwrap();
    assert_eq!(view.selected_text(), " beta ");
    view.set_is_text_selection_enabled(false);
    assert_eq!(
        text.supported_text_selection().unwrap(),
        SupportedTextSelection::None
    );
    assert!(text.get_selection().unwrap().is_empty());
    assert_eq!(
        beta.select()
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::SelectionNotSupported)
    );
}

#[test]
fn rich_automation_live_clones_follow_edits_and_ranges_do_not_change_selection() {
    let _scope = test_scope();
    let view = rich("alpha beta");
    view.select(1, 3);
    let text = provider(&view);
    let beta = found(&*text, "beta");
    let clone = beta.clone_range().unwrap();
    clone
        .move_endpoint_by_unit(Endpoint::End, TextUnit::Character, -1)
        .unwrap();
    assert_eq!(clone.get_text(None).unwrap(), "bet");
    assert_eq!(beta.get_text(None).unwrap(), "beta");
    assert!(!clone.compare(&*beta).unwrap());
    assert_eq!((view.selection_start(), view.selection_end()), (1, 3));
    view.document().replace_text(0..0, "x").unwrap();
    assert_eq!(beta.get_text(None).unwrap(), "beta");
    assert_eq!(
        beta.compare_endpoints(
            Endpoint::Start,
            &*text.document_range().unwrap(),
            Endpoint::Start
        )
        .unwrap(),
        7
    );
    assert_eq!(clone.get_text(None).unwrap(), "bet");
}

#[test]
fn rich_automation_movement_uses_uia_normalization_and_grapheme_units() {
    let _scope = test_scope();
    let view = rich("one two a\u{301}😀\nlast");
    let text = provider(&view);
    let range = found(&*text, "ne two");
    range.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "one");
    assert_eq!(range.move_by(TextUnit::Word, 2).unwrap(), 2);
    assert_eq!(range.get_text(None).unwrap(), "two");
    assert_eq!(range.move_by(TextUnit::Word, 0).unwrap(), 0);
    assert_eq!(range.get_text(None).unwrap(), "two");
    let accent = found(&*text, "a\u{301}");
    assert_eq!(accent.move_by(TextUnit::Character, 1).unwrap(), 1);
    assert_eq!(accent.get_text(None).unwrap(), "😀");
    accent
        .move_endpoint_by_range(
            Endpoint::End,
            &*accent.clone_range().unwrap(),
            Endpoint::Start,
        )
        .unwrap();
    assert_eq!(accent.move_by(TextUnit::Character, 1).unwrap(), 1);
    assert_eq!(
        accent
            .compare_endpoints(Endpoint::Start, &*found(&*text, "\n"), Endpoint::Start)
            .unwrap(),
        0
    );
    let last = found(&*text, "last");
    assert_eq!(last.move_by(TextUnit::Word, i32::MAX).unwrap(), 0);
    assert_eq!(last.get_text(None).unwrap(), "last");
}

#[test]
fn rich_automation_endpoint_crossing_collapses_instead_of_reversing() {
    let _scope = test_scope();
    let view = rich("abcdef");
    let text = provider(&view);
    let range = found(&*text, "bc");
    assert_eq!(
        range
            .move_endpoint_by_unit(Endpoint::Start, TextUnit::Character, 4)
            .unwrap(),
        4
    );
    assert_eq!(range.get_text(None).unwrap(), "");
    assert_eq!(
        range
            .compare_endpoints(Endpoint::Start, &*range, Endpoint::End)
            .unwrap(),
        0
    );
    assert_eq!(
        range
            .move_endpoint_by_unit(Endpoint::End, TextUnit::Character, i32::MIN)
            .unwrap(),
        -5
    );
    assert_eq!(
        range
            .compare_endpoints(
                Endpoint::End,
                &*text.document_range().unwrap(),
                Endpoint::Start
            )
            .unwrap(),
        0
    );
}

#[test]
fn rich_automation_attributes_distinguish_uniform_mixed_and_unsupported_values() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_font_size(24.0);
    let paragraph = Paragraph::with_text("plain ");
    paragraph.set_text_alignment(TextAlignment::Center);
    paragraph.set_text_indent(12.0);
    let bold = Bold::new();
    bold.add_child_text("bold");
    paragraph.add_child(bold);
    paragraph.add_child_text(" tail");
    view.blocks().add(paragraph);
    let text = provider(&view);
    let all = text.document_range().unwrap();
    assert_eq!(
        all.get_attribute_value(TextAttribute::FontWeight).unwrap(),
        TextAttributeValue::Mixed
    );
    assert_eq!(
        all.get_attribute_value(TextAttribute::FontSize).unwrap(),
        TextAttributeValue::Number(18.0)
    );
    assert_eq!(
        all.get_attribute_value(TextAttribute::HorizontalTextAlignment)
            .unwrap(),
        TextAttributeValue::Integer(1)
    );
    assert_eq!(
        all.get_attribute_value(TextAttribute::IndentationFirstLine)
            .unwrap(),
        TextAttributeValue::Number(9.0)
    );
    let matched = all
        .find_attribute(
            TextAttribute::FontWeight,
            &TextAttributeValue::Integer(FontWeight::Bold.0),
            false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(matched.get_text(None).unwrap(), "bold");
    assert_eq!(
        all.get_attribute_value(TextAttribute::Unknown(59999))
            .unwrap(),
        TextAttributeValue::NotSupported
    );
    assert!(all
        .find_attribute(
            TextAttribute::Unknown(59999),
            &TextAttributeValue::Bool(true),
            false
        )
        .is_err());
}

#[test]
fn rich_automation_format_units_merge_equal_runs_and_stop_at_hyperlinks() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_foreground(Some(Brushes::black()));
    let paragraph = Paragraph::new();
    for text in ["ab", "cd"] {
        let span = Span::new();
        span.add_child_text(text);
        paragraph.add_child(span);
    }
    let link = Hyperlink::new();
    link.add_child_text("ef");
    link.set_underline_style(UnderlineStyle::None);
    link.set_foreground(Some(Brushes::black()));
    paragraph.add_child(link);
    paragraph.add_child_text("gh");
    view.blocks().add(paragraph);
    let text = provider(&view);
    let range = found(&*text, "b");
    range.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "abcd");
    assert_eq!(range.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(range.get_text(None).unwrap(), "ef");
    assert_eq!(range.move_by(TextUnit::Format, 1).unwrap(), 1);
    assert_eq!(range.get_text(None).unwrap(), "gh");
}

#[test]
fn rich_automation_text_search_retains_unicode_offsets_and_backward_matches() {
    let _scope = test_scope();
    let view = rich("😀İ one ONE");
    let text = provider(&view);
    let all = text.document_range().unwrap();
    let matched = all.find_text("one", true, true).unwrap().unwrap();
    assert_eq!(matched.get_text(None).unwrap(), "ONE");
    let mapped = all.find_text("i\u{307}", false, true).unwrap().unwrap();
    assert_eq!(mapped.get_text(None).unwrap(), "İ");
    assert_eq!(
        mapped
            .compare_endpoints(Endpoint::Start, &*all, Endpoint::Start)
            .unwrap(),
        2
    );
    assert!(all.find_text("", false, false).is_err());
}

#[test]
fn rich_automation_embedded_objects_are_enumerable_and_have_real_ranges() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("a");
    let child = Border::new();
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    paragraph.add_child_text("b");
    view.blocks().add(paragraph);
    let text = provider(&view);
    let all = text.document_range().unwrap();
    let children = all.get_children().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(
        children[0],
        ControlAutomationPeer::create_peer_for_element(&child)
    );
    let range = text.range_from_child(&children[0]).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "\u{fffc}");
    assert_eq!(range.get_enclosing_element().unwrap(), children[0]);
    range.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "\u{fffc}");
    assert_eq!(all.get_enclosing_element().unwrap(), text.peer());
    assert!(text
        .range_from_child(&ControlAutomationPeer::create_peer_for_element(
            &Border::new()
        ))
        .is_err());
}

#[test]
fn rich_automation_replaced_documents_and_foreign_ranges_fail_explicitly() {
    let _scope = test_scope();
    let view = rich("first");
    let other = rich("second");
    let text = provider(&view);
    let range = text.document_range().unwrap();
    let foreign = provider(&other).document_range().unwrap();
    assert_eq!(
        range
            .compare(&*foreign)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::IncompatibleRange)
    );
    view.set_document(RichTextDocument::from_text("replacement"));
    assert_eq!(
        range
            .get_text(None)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
    assert_eq!(
        text.document_range().unwrap().get_text(None).unwrap(),
        "replacement"
    );
}

#[test]
fn rich_automation_retained_ranges_do_not_keep_controls_alive() {
    let _scope = test_scope();
    let view = rich("temporary");
    let text = provider(&view);
    let range = text.document_range().unwrap();
    let weak = view.downgrade();
    drop(view);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        range
            .get_text(None)
            .unwrap_err()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
    assert!(text.document_range().is_err());
}

#[test]
fn rich_automation_notifications_follow_document_replacement_and_dispose_cleanly() {
    let _scope = test_scope();
    let view = rich("initial");
    let original = view.document();
    let text = provider(&view);
    let changes = Rc::new(Cell::new(0));
    let selections = Rc::new(Cell::new(0));
    let sink = changes.clone();
    let changed = text
        .text_changed(Rc::new(move || sink.set(sink.get() + 1)))
        .unwrap();
    let sink = selections.clone();
    let selected = text
        .text_selection_changed(Rc::new(move || sink.set(sink.get() + 1)))
        .unwrap();
    view.select(1, 2);
    assert_eq!(selections.get(), 1);
    original.replace_text(0..0, "x").unwrap();
    assert_eq!(changes.get(), 1);
    view.set_document(RichTextDocument::from_text("new"));
    assert_eq!(changes.get(), 2);
    original.replace_text(0..0, "old").unwrap();
    assert_eq!(changes.get(), 2);
    view.document()
        .apply_format(
            0..1,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(changes.get(), 3);
    changed.dispose();
    selected.dispose();
    let selections_before = selections.get();
    view.document().replace_text(0..0, "again").unwrap();
    assert_eq!(changes.get(), 3);
    assert_eq!(selections.get(), selections_before);
}

#[test]
fn rich_automation_geometry_uses_root_coordinates_and_excludes_hidden_text() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("alpha beta gamma delta");
    view.set_margin(Thickness::new(20.0, 10.0, 0.0, 0.0));
    let root = TestRoot::with_child(view.clone());
    root.measure(Size::new(180.0, 80.0));
    root.arrange(Rect::new(0.0, 0.0, 180.0, 80.0));
    let text = provider(&view);
    let all = text.document_range().unwrap();
    let rectangles = all.get_bounding_rectangles().unwrap();
    assert!(!rectangles.is_empty());
    assert!(rectangles[0].x >= 20.0);
    assert!(rectangles[0].y >= 10.0);
    let first = rectangles[0];
    let point = Point::new(first.x + 0.01, first.y + first.height / 2.0);
    let hit = text.range_from_point(point).unwrap();
    hit.expand_to_enclosing_unit(TextUnit::Character).unwrap();
    assert_eq!(hit.get_text(None).unwrap(), "a");
    assert!(!text.get_visible_ranges().unwrap().is_empty());
    let caret = text.get_selection().unwrap().remove(0);
    assert!(caret.get_bounding_rectangles().unwrap().is_empty());
    view.set_is_visible(false);
    assert!(all.get_bounding_rectangles().unwrap().is_empty());
    assert_eq!(
        text.get_visible_ranges().unwrap()[0]
            .get_text(None)
            .unwrap(),
        ""
    );
}

#[test]
fn rich_automation_line_units_follow_wrapped_layout_and_paragraph_units_follow_breaks() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("alpha beta gamma delta\nnext");
    view.measure(Size::new(70.0, 500.0));
    view.arrange(Rect::new(0.0, 0.0, 70.0, 500.0));
    let text = provider(&view);
    let range = found(&*text, "gamma");
    range.expand_to_enclosing_unit(TextUnit::Line).unwrap();
    let line = view
        .line_metrics()
        .into_iter()
        .find(|line| line.start <= 11 && 11 < line.start + line.length)
        .unwrap();
    assert_eq!(
        range.get_text(None).unwrap(),
        view.document()
            .get_text(line.start..line.start + line.length)
            .unwrap()
    );
    range.expand_to_enclosing_unit(TextUnit::Paragraph).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "alpha beta gamma delta\n");
    range.expand_to_enclosing_unit(TextUnit::Page).unwrap();
    assert_eq!(range.get_text(None).unwrap(), view.document().text());
}

// The following cases are FerroUI integration regressions, not upstream tests.
#[test]
fn rich_automation_text2_caret_uses_active_utf16_endpoint_and_focus_without_selecting() {
    let _scope = test_scope();
    let _keyboard = KeyboardScope::new();
    let editor = RichEditBox::new();
    editor.set_text("a😀bc").unwrap();
    let root = TestRoot::with_child(editor.clone());
    root.measure(Size::new(200.0, 80.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 80.0));
    editor.select(5, 1);
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let provider = peer.get_provider::<dyn ITextProvider2>().unwrap();
    let caret = provider.get_caret_range().unwrap();
    assert!(!caret.is_active);
    let caret = caret.range.unwrap();
    assert_eq!(caret.get_text(None).unwrap(), "");
    let all = provider.document_range().unwrap();
    assert_eq!(
        caret
            .compare_endpoints(Endpoint::Start, &*all, Endpoint::Start)
            .unwrap(),
        1
    );
    assert_eq!((editor.selection_start(), editor.selection_end()), (5, 1));
    assert!(editor.focus());
    assert!(provider.get_caret_range().unwrap().is_active);
    editor.select(1, 5);
    let caret = provider.get_caret_range().unwrap().range.unwrap();
    assert_eq!(
        caret
            .compare_endpoints(Endpoint::Start, &*all, Endpoint::Start)
            .unwrap(),
        5
    );
    editor.set_is_enabled(false);
    assert!(!provider.get_caret_range().unwrap().is_active);
    assert!(provider.get_caret_range().unwrap().range.is_some());
}

#[test]
fn rich_automation_text2_has_no_caret_when_selection_disabled_and_rejects_non_annotations() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text("text"));
    view.set_is_text_selection_enabled(false);
    let peer = ControlAutomationPeer::create_peer_for_element(&view);
    let provider = peer.get_provider::<dyn ITextProvider2>().unwrap();
    let caret = provider.get_caret_range().unwrap();
    assert!(!caret.is_active);
    assert!(caret.range.is_none());
    assert_eq!(
        provider
            .range_from_annotation(&peer)
            .err()
            .unwrap()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::InvalidArgument)
    );
    let weak = view.downgrade();
    drop(view);
    assert!(weak.upgrade().is_none());
    assert!(provider.get_caret_range().is_err());
}

#[test]
fn rich_automation_hyperlinks_have_stable_identity_text_child_ranges_and_invoke() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let prefix = Run::with_text(Some("😀 "));
    paragraph.add_child(prefix.clone());
    let link = Hyperlink::with_text("link😀", Some("https://example.com/"));
    paragraph.add_child(link.clone());
    paragraph.add_child_text("!");
    view.blocks().add(paragraph.clone());
    let text = provider(&view);
    let peer = text.peer();
    let children = peer.get_children();
    assert_eq!(children.len(), 1);
    let child = children[0].clone();
    assert_eq!(child.get_parent(), Some(peer.clone()));
    assert_eq!(
        child.get_automation_control_type(),
        AutomationControlType::Hyperlink
    );
    assert_eq!(child.get_class_name(), "Hyperlink");
    assert_eq!(child.get_name(), "link😀");
    let child_text = child.get_provider::<dyn ITextChildProvider>().unwrap();
    assert_eq!(child_text.text_container().unwrap(), peer);
    let range = child_text.text_range().unwrap();
    assert_eq!(range.get_text(None).unwrap(), "link😀");
    assert!(range
        .compare(&*text.range_from_child(&child).unwrap())
        .unwrap());
    assert_eq!(range.get_enclosing_element().unwrap(), child);
    let all = text.document_range().unwrap();
    assert_eq!(all.get_children().unwrap(), vec![child.clone()]);
    assert_eq!(
        range
            .compare_endpoints(Endpoint::Start, &*all, Endpoint::Start)
            .unwrap(),
        3
    );
    prefix.set_text(Some("zz😀 "));
    assert_eq!(peer.get_children()[0], child);
    assert_eq!(
        child_text
            .text_range()
            .unwrap()
            .compare_endpoints(
                Endpoint::Start,
                &*text.document_range().unwrap(),
                Endpoint::Start
            )
            .unwrap(),
        5
    );
    assert_eq!(range.get_text(None).unwrap(), "link😀");
    let invoked = Rc::new(Cell::new(0));
    let count = invoked.clone();
    link.click(move |args| {
        count.set(count.get() + 1);
        args.set_handled(true);
    });
    let invoke = child.get_provider::<dyn IInvokeProvider>().unwrap();
    invoke.invoke().unwrap();
    assert_eq!(invoked.get(), 1);
    link.set_is_enabled(false);
    assert!(!child.is_enabled());
    assert!(invoke.invoke().is_err());
    assert_eq!(invoked.get(), 1);
    link.set_is_enabled(true);
    view.set_is_enabled(false);
    assert!(invoke.invoke().is_err());
}

#[test]
fn rich_automation_hyperlink_tree_retains_embedded_children_and_innermost_enclosing_peer() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("before ");
    let outer = Hyperlink::with_text("outer ", None);
    let inner = Hyperlink::with_text("inner", None);
    let embedded = Border::new();
    inner.add_child(InlineUIContainer::with_child(embedded.clone()));
    outer.add_child(inner);
    paragraph.add_child(outer);
    view.blocks().add(paragraph);
    let text = provider(&view);
    let root = text.peer();
    let outer_peer = root.get_children()[0].clone();
    assert_eq!(root.get_children().len(), 1);
    let inner_peer = outer_peer.get_children()[0].clone();
    let embedded_peer = inner_peer.get_children()[0].clone();
    assert_eq!(
        embedded_peer,
        ControlAutomationPeer::create_peer_for_element(&embedded)
    );
    assert_eq!(inner_peer.get_parent(), Some(outer_peer));
    assert_eq!(embedded_peer.get_parent(), Some(inner_peer.clone()));
    assert_eq!(
        text.range_from_child(&embedded_peer)
            .unwrap()
            .get_text(None)
            .unwrap(),
        "\u{fffc}"
    );
    assert_eq!(
        found(&*text, "inner").get_enclosing_element().unwrap(),
        inner_peer
    );
    assert!(found(&*text, "inner").get_children().unwrap().is_empty());
    assert_eq!(
        text.range_from_child(&embedded_peer)
            .unwrap()
            .get_enclosing_element()
            .unwrap(),
        embedded_peer
    );
    assert!(text
        .range_from_child(&embedded_peer)
        .unwrap()
        .get_children()
        .unwrap()
        .is_empty());
    assert_eq!(
        text.document_range().unwrap().get_children().unwrap().len(),
        3
    );
}

#[test]
fn rich_automation_hyperlink_metadata_geometry_focus_and_document_detachment_are_live() {
    let _scope = test_scope();
    let _keyboard = KeyboardScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let link = Hyperlink::with_text("Open", Some("https://example.com/"));
    paragraph.add_child(link.clone());
    view.blocks().add(paragraph.clone());
    let root = TestRoot::with_child(view.clone());
    root.measure(Size::new(200.0, 80.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 80.0));
    let peer = provider(&view).peer().get_children()[0].clone();
    let child = peer.get_provider::<dyn ITextChildProvider>().unwrap();
    link.set_value(
        AutomationProperties::name_property(),
        Some("Custom name".to_owned()),
    );
    link.set_value(
        AutomationProperties::help_text_property(),
        Some("Open the site".to_owned()),
    );
    assert_eq!(peer.get_name(), "Custom name");
    assert_eq!(peer.get_help_text(), "Open the site");
    assert!(!peer.get_bounding_rectangle().is_empty());
    assert!(!peer.is_offscreen());
    peer.set_focus();
    assert!(view.is_focused());
    assert!(peer.has_keyboard_focus());
    view.set_is_visible(false);
    assert!(peer.is_offscreen());
    assert!(peer.get_bounding_rectangle().is_empty());
    paragraph.inlines().clear();
    assert_eq!(
        child
            .text_range()
            .err()
            .unwrap()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
    assert!(child.text_container().is_err());
    assert!(peer.get_parent().is_none());
    assert!(peer
        .get_provider::<dyn IInvokeProvider>()
        .unwrap()
        .invoke()
        .is_err());
}

#[test]
fn rich_automation_retained_hyperlink_peer_does_not_retain_document_or_owners() {
    let _scope = test_scope();
    let (child, view, link, document) = {
        let view = RichTextBlock::new();
        let paragraph = Paragraph::new();
        let link = Hyperlink::with_text("temporary", None);
        paragraph.add_child(link.clone());
        view.blocks().add(paragraph);
        let child = provider(&view).peer().get_children()[0]
            .get_provider::<dyn ITextChildProvider>()
            .unwrap();
        (
            child,
            view.downgrade(),
            link.downgrade(),
            view.document().downgrade(),
        )
    };
    assert!(view.upgrade().is_none());
    assert!(link.upgrade().is_none());
    assert!(document.upgrade().is_none());
    assert!(child.text_range().is_err());
    assert!(child.text_container().is_err());
}

#[test]
fn rich_automation_hyperlink_structure_events_follow_identity_changes_only() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("text");
    view.blocks().add(paragraph.clone());
    let peer = provider(&view).peer();
    peer.get_children();
    let changes = Rc::new(Cell::new(0));
    let sink = changes.clone();
    let subscription = peer.children_changed(move || sink.set(sink.get() + 1));
    let link = Hyperlink::with_text("link", None);
    paragraph.add_child(link.clone());
    assert_eq!(changes.get(), 1);
    link.add_child_text(" changed");
    assert_eq!(changes.get(), 1);
    paragraph.inlines().remove(&link.upcast());
    assert_eq!(changes.get(), 2);
    subscription.dispose();
    paragraph.add_child(Hyperlink::with_text("more", None));
    assert_eq!(changes.get(), 2);
}

#[test]
fn rich_automation_extra_attributes_use_effective_decoration_colors_and_logical_margins() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    paragraph.set_flow_direction(FlowDirection::RightToLeft);
    paragraph.set_margin(Thickness::new(8.0, 12.0, 16.0, 20.0));
    paragraph.set_left_indent(-4.0);
    paragraph.set_right_indent(24.0);
    let first = Run::with_text(Some("red"));
    let red = TextDecoration::new();
    red.set_location(TextDecorationLocation::Overline);
    red.set_stroke(Some(Brushes::red()));
    first.set_text_decorations(Some(TextDecorationCollection::from_items([red])));
    let second = Run::with_text(Some("blue"));
    let blue = TextDecoration::new();
    blue.set_location(TextDecorationLocation::Overline);
    blue.set_stroke(Some(Brushes::blue()));
    second.set_text_decorations(Some(TextDecorationCollection::from_items([blue])));
    paragraph.add_child(first);
    paragraph.add_child(second);
    view.blocks().add(paragraph);
    let text = provider(&view);
    let all = text.document_range().unwrap();
    for (attribute, value) in [
        (
            TextAttribute::TextFlowDirections,
            TextAttributeValue::Integer(1),
        ),
        (TextAttribute::MarginTop, TextAttributeValue::Number(9.0)),
        (
            TextAttribute::MarginBottom,
            TextAttributeValue::Number(15.0),
        ),
        (
            TextAttribute::MarginLeading,
            TextAttributeValue::Number(12.0),
        ),
        (
            TextAttribute::MarginTrailing,
            TextAttributeValue::Number(6.0),
        ),
        (
            TextAttribute::IndentationLeading,
            TextAttributeValue::Number(30.0),
        ),
        (
            TextAttribute::IndentationTrailing,
            TextAttributeValue::Number(3.0),
        ),
        (TextAttribute::OverlineStyle, TextAttributeValue::Integer(1)),
        (TextAttribute::OverlineColor, TextAttributeValue::Mixed),
    ] {
        assert_eq!(
            all.get_attribute_value(attribute).unwrap(),
            value,
            "{attribute:?}"
        );
    }
    let red = all
        .find_attribute(
            TextAttribute::OverlineColor,
            &TextAttributeValue::Color(Color::from_rgb(255, 0, 0)),
            false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(red.get_text(None).unwrap(), "red");
    assert_eq!(
        red.get_attribute_value(TextAttribute::UnderlineColor)
            .unwrap(),
        TextAttributeValue::NotSupported
    );
}

#[test]
fn rich_automation_editor_header_description_preserve_explicit_metadata_and_visual_children() {
    use crate::presenters::ContentPresenter;
    use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use ferroui_base::data::TemplateBinding;

    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor.set_header(Some(Rc::new("Editor label".to_owned())));
    editor.set_description(Some(Rc::new("Editor guidance".to_owned())));
    editor.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
        |_owner, names| {
            let panel = crate::StackPanel::new();
            let header = ContentPresenter::new();
            header.set_name(Some("HeaderContentPresenter".to_owned()));
            header.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(RichEditBox::header_property().as_property()),
            );
            panel
                .children()
                .add(header.register_in_name_scope(&**names));
            let content = Border::new();
            content.set_name(Some("ContentElement".to_owned()));
            panel
                .children()
                .add(content.register_in_name_scope(&**names));
            let description = ContentPresenter::new();
            description.set_name(Some("DescriptionPresenter".to_owned()));
            description.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(RichEditBox::description_property().as_property()),
            );
            panel
                .children()
                .add(description.register_in_name_scope(&**names));
            panel.upcast()
        },
    )));
    editor.apply_template();
    let root = TestRoot::with_child(editor.clone());
    root.measure(Size::new(200.0, 80.0));
    root.arrange(Rect::new(0.0, 0.0, 200.0, 80.0));
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    assert_eq!(peer.get_name(), "Editor label");
    assert_eq!(peer.get_help_text(), "Editor guidance");
    let header_peer =
        ControlAutomationPeer::create_peer_for_element(&editor.header_presenter().unwrap());
    let description_peer =
        ControlAutomationPeer::create_peer_for_element(&editor.description_presenter().unwrap());
    let template_children = peer.get_children()[0].get_children();
    assert!(template_children.contains(&header_peer));
    assert!(template_children.contains(&description_peer));
    editor.set_value(
        AutomationProperties::name_property(),
        Some("Explicit name".to_owned()),
    );
    editor.set_value(
        AutomationProperties::help_text_property(),
        Some("Explicit help".to_owned()),
    );
    assert_eq!(peer.get_name(), "Explicit name");
    assert_eq!(peer.get_help_text(), "Explicit help");
}

#[test]
fn rich_automation_overflow_hyperlink_geometry_and_point_ranges_share_the_root() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    view.set_width(75.0);
    view.set_max_lines(1);
    let paragraph = Paragraph::with_text("one two three four five six ");
    paragraph.add_child(Hyperlink::with_text("target", None));
    view.blocks().add(paragraph);
    let overflow = crate::RichTextBlockOverflow::new();
    overflow.set_width(130.0);
    view.set_overflow_content_target(Some(overflow.clone()));
    let panel = crate::StackPanel::new();
    panel.set_orientation(ferroui_base::layout::Orientation::Horizontal);
    panel.children().add(view.clone());
    panel.children().add(overflow.clone());
    let root = TestRoot::with_child(panel);
    root.measure(Size::new(205.0, 160.0));
    root.arrange(Rect::new(0.0, 0.0, 205.0, 160.0));
    assert!(overflow.content_start() > 0);
    let text = provider(&view);
    let target = found(&*text, "target");
    let rectangles = target.get_bounding_rectangles().unwrap();
    assert!(!rectangles.is_empty());
    assert!(rectangles.iter().all(|rect| rect.x >= 75.0));
    let hit = text.range_from_point(rectangles[0].center()).unwrap();
    assert!(
        hit.compare_endpoints(Endpoint::Start, &*target, Endpoint::Start)
            .unwrap()
            >= 0
    );
    assert!(
        hit.compare_endpoints(Endpoint::Start, &*target, Endpoint::End)
            .unwrap()
            <= 0
    );
    let link = text.peer().get_children()[0].clone();
    assert!(!link.is_offscreen());
    assert!(link.get_bounding_rectangle().x >= 75.0);
    assert!(text
        .get_visible_ranges()
        .unwrap()
        .into_iter()
        .map(|range| range.get_text(None).unwrap())
        .collect::<String>()
        .contains("target"));
}

#[test]
fn rich_automation_visible_ranges_exclude_text_collapsed_into_an_ellipsis() {
    let _scope = test_scope();
    let view = rich("alpha beta gamma delta epsilon zeta");
    view.set_max_lines(1);
    view.set_text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis());
    let root = TestRoot::with_child(view.clone());
    root.measure(Size::new(90.0, 40.0));
    root.arrange(Rect::new(0.0, 0.0, 90.0, 40.0));
    assert!(view.is_text_trimmed());
    let visible = provider(&view)
        .get_visible_ranges()
        .unwrap()
        .into_iter()
        .map(|range| range.get_text(None).unwrap())
        .collect::<String>();
    assert!(!visible.is_empty());
    assert!(!visible.contains("zeta"));
    assert!(visible.len() < view.document().text().len());
}

#[test]
fn rich_automation_document_and_hyperlink_ranges_do_not_enumerate_themselves() {
    let _scope = test_scope();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let link = Hyperlink::with_text("only link", None);
    paragraph.add_child(link);
    view.blocks().add(paragraph);
    let text = provider(&view);
    let all = text.document_range().unwrap();
    let link_peer = all.get_children().unwrap()[0].clone();
    // The document and link have identical visible UTF-16 offsets. Their
    // enclosing scopes still differ, as the source's structural endpoints do.
    assert_eq!(all.get_enclosing_element().unwrap(), text.peer());
    let all_clone = all.clone_range().unwrap();
    assert_eq!(all_clone.get_enclosing_element().unwrap(), text.peer());
    assert_eq!(all_clone.get_children().unwrap(), vec![link_peer.clone()]);
    let content = link_peer
        .get_provider::<dyn ITextChildProvider>()
        .unwrap()
        .text_range()
        .unwrap();
    assert_eq!(content.get_enclosing_element().unwrap(), link_peer);
    assert!(content.get_children().unwrap().is_empty());
    assert!(content
        .clone_range()
        .unwrap()
        .get_children()
        .unwrap()
        .is_empty());
    assert!(found(&*text, "link").get_children().unwrap().is_empty());
    content
        .expand_to_enclosing_unit(TextUnit::Document)
        .unwrap();
    assert_eq!(content.get_enclosing_element().unwrap(), text.peer());
    assert_eq!(content.get_children().unwrap(), vec![link_peer]);

    let embedded_view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let embedded = Border::new();
    paragraph.add_child(InlineUIContainer::with_child(embedded.clone()));
    embedded_view.blocks().add(paragraph);
    let text = provider(&embedded_view);
    let all = text.document_range().unwrap();
    let embedded_peer = all.get_children().unwrap()[0].clone();
    assert_eq!(all.get_enclosing_element().unwrap(), text.peer());
    let content = text.range_from_child(&embedded_peer).unwrap();
    assert_eq!(content.get_enclosing_element().unwrap(), embedded_peer);
    assert!(content.get_children().unwrap().is_empty());
}

#[test]
fn rich_automation_scroll_alignment_uses_the_last_selected_line() {
    use crate::presenters::ScrollContentPresenter;
    use crate::primitives::ScrollBarVisibility;
    use crate::templates::FuncControlTemplate;
    use crate::ScrollViewer;

    let _scope = test_scope();
    let view = rich(
        &(0..12)
            .map(|line| format!("line{line}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let scroller = ScrollViewer::new();
    scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroller.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::scroll_viewer_tests::create_template,
    )));
    scroller.set_content(Some(Control::boxed(&view)));
    scroller.apply_template();
    let presenter = scroller
        .presenter()
        .unwrap()
        .cast::<ScrollContentPresenter>()
        .unwrap();
    presenter.attach_to_scroll_viewer();
    presenter.update_child();
    let root = TestRoot::with_child(scroller.clone());
    let size = Size::new(200.0, 30.0);
    root.set_client_size(size);
    root.measure(size);
    root.arrange(Rect::from_size(size));
    let lines = view.line_metrics();
    assert!(lines.len() >= 12);
    assert!(scroller.extent().height > scroller.viewport().height);
    let selected = lines[2].start..lines[5].start;
    let selected_text = view.document().get_text(selected.clone()).unwrap();
    let range = found(&*provider(&view), &selected_text);
    let rectangles = view.get_rects_for_range(selected);
    let expected_bottom = rectangles.last().unwrap().bottom() - scroller.viewport().height;
    let expected_top = rectangles[0].y;
    assert!(expected_bottom > 0.0);
    view.select(1, 3);

    range.scroll_into_view(false).unwrap();
    assert!((scroller.offset().y - expected_bottom).abs() < 1e-8);
    // The child has not been arranged at its new offset yet. Repeating or
    // changing alignment must use extent coordinates, not its stale transform.
    range.scroll_into_view(false).unwrap();
    assert!((scroller.offset().y - expected_bottom).abs() < 1e-8);
    range.scroll_into_view(true).unwrap();
    assert!((scroller.offset().y - expected_top).abs() < 1e-8);
    assert_eq!((view.selection_start(), view.selection_end()), (1, 3));
}

#[test]
fn rich_automation_editor_scroll_alignment_uses_the_presenter_viewport_and_current_offset() {
    use ferroui_base::input::IScrollable;

    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor
        .set_text(
            &(0..12)
                .map(|line| format!("line{line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    editor.select(1, 3);
    editor.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
    let root = TestRoot::with_child(editor.clone());
    let size = Size::new(200.0, 40.0);
    root.set_client_size(size);
    root.measure(size);
    root.arrange(Rect::from_size(size));
    root.layout_manager().execute_layout_pass();
    let presenter = editor.text_presenter();
    assert!(presenter.extent().height > presenter.viewport().height);
    let lines = presenter.line_metrics();
    let selected = lines[2].start..lines[5].start;
    let selected_text = editor.document().get_text(selected.clone()).unwrap();
    let range = found(&*provider(&editor), &selected_text);
    let rectangles = presenter.get_rects_for_range(selected);
    let expected_bottom =
        rectangles.last().unwrap().bottom() + presenter.offset().y - presenter.viewport().height;
    let expected_top = rectangles[0].y + presenter.offset().y;
    range.scroll_into_view(false).unwrap();
    assert!((presenter.offset().y - expected_bottom).abs() < 1e-8);
    // Logical scrolling changes the presenter's text origin immediately. Its
    // subsequent outer bring-into-view must receive the updated local bounds.
    range.scroll_into_view(false).unwrap();
    assert!((presenter.offset().y - expected_bottom).abs() < 1e-8);
    range.scroll_into_view(true).unwrap();
    assert!((presenter.offset().y - expected_top).abs() < 1e-8);
    assert_eq!((editor.selection_start(), editor.selection_end()), (1, 3));
}

#[test]
fn rich_automation_scroll_rejects_text_without_a_laid_out_view() {
    let _scope = test_scope();
    let view = rich("first\nsecond\nthird");
    view.set_max_lines(1);
    let text = provider(&view);
    let unavailable = found(&*text, "third");
    let root = TestRoot::with_child(view.clone());
    for align_to_top in [true, false] {
        assert_eq!(
            unavailable
                .scroll_into_view(align_to_top)
                .err()
                .unwrap()
                .downcast_ref::<TextProviderError>(),
            Some(&TextProviderError::GeometryUnavailable)
        );
    }
    let size = Size::new(200.0, 50.0);
    root.set_client_size(size);
    root.measure(size);
    root.arrange(Rect::from_size(size));
    assert!(view.get_rects_for_range(13..18).is_empty());
    for align_to_top in [true, false] {
        assert_eq!(
            unavailable
                .scroll_into_view(align_to_top)
                .err()
                .unwrap()
                .downcast_ref::<TextProviderError>(),
            Some(&TextProviderError::GeometryUnavailable)
        );
    }
    let available = found(&*text, "first");
    available.scroll_into_view(true).unwrap();
    root.set_child(None::<Ref<Control>>);
    assert_eq!(
        available
            .scroll_into_view(true)
            .err()
            .unwrap()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::GeometryUnavailable)
    );
}

#[test]
fn rich_automation_editor_templates_preserve_owner_identity_ranges_and_presenter_geometry() {
    let _scope = test_scope();
    let _keyboard = KeyboardScope::new();
    let editor = RichEditBox::new();
    editor.set_text("first second").unwrap();
    let presenter = editor.text_presenter();
    let text = provider(&editor);
    let peer = text.peer();
    let word = found(&*text, "second");
    assert!(word.get_bounding_rectangles().unwrap().is_empty());
    apply_editor_template(&editor, Thickness::new(8.0, 12.0, 0.0, 0.0));
    let root = TestRoot::with_child(editor.clone());
    let size = Size::new(240.0, 100.0);
    root.set_client_size(size);
    root.measure(size);
    root.arrange(Rect::from_size(size));
    let first = word.get_bounding_rectangles().unwrap()[0];
    let expected = presenter.get_rects_for_range(6..12)[0]
        .transform_to_aabb(presenter.transform_to_visual(&root).unwrap());
    assert_eq!(first, expected);
    assert!(first.x >= 8.0 && first.y >= 12.0);
    assert!(editor.focus());
    assert!(!presenter.is_focused());
    assert!(
        peer.get_provider::<dyn ITextProvider2>()
            .unwrap()
            .get_caret_range()
            .unwrap()
            .is_active
    );
    word.select().unwrap();
    assert_eq!((editor.selection_start(), editor.selection_end()), (6, 12));
    assert_eq!(editor.editor().selection().range(), 6..12);
    assert_eq!(presenter.document(), editor.document());

    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    let subscription = text
        .text_changed(Rc::new(move || count.set(count.get() + 1)))
        .unwrap();
    apply_editor_template(&editor, Thickness::new(31.0, 29.0, 0.0, 0.0));
    // Descendant invalidations are queued by the framework layout manager.
    // Repeating an unchanged root arrange alone can skip the new template.
    root.layout_manager().execute_layout_pass();
    assert_eq!(presenter, editor.text_presenter());
    assert_eq!(
        peer,
        ControlAutomationPeer::create_peer_for_element(&editor)
    );
    assert_eq!(word.peer().unwrap(), peer);
    assert_eq!(word.get_text(None).unwrap(), "second");
    let second = word.get_bounding_rectangles().unwrap()[0];
    assert!(
        (second.x - first.x - 23.0).abs() < 1e-8,
        "{first:?} -> {second:?}"
    );
    assert!(
        (second.y - first.y - 17.0).abs() < 1e-8,
        "{first:?} -> {second:?}"
    );
    assert_eq!((editor.selection_start(), editor.selection_end()), (6, 12));
    assert_eq!(changes.get(), 0);
    editor.set_is_read_only(true);
    assert_eq!(
        word.get_attribute_value(TextAttribute::IsReadOnly).unwrap(),
        TextAttributeValue::Bool(true)
    );
    subscription.dispose();
}

#[test]
fn rich_automation_retained_editor_presenter_does_not_keep_owner_or_text_provider_alive() {
    let _scope = test_scope();
    let (presenter, text, range, weak_editor) = {
        let editor = RichEditBox::new();
        editor.set_text("retained layout").unwrap();
        let text = provider(&editor);
        let range = text.document_range().unwrap();
        (editor.text_presenter(), text, range, editor.downgrade())
    };
    assert!(weak_editor.upgrade().is_none());
    assert_eq!(presenter.document().text(), "retained layout");
    assert_eq!(
        range
            .get_text(None)
            .err()
            .unwrap()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
    assert!(text.document_range().is_err());
    assert!(text
        .peer()
        .get_provider::<dyn ITextProvider2>()
        .unwrap()
        .get_caret_range()
        .is_err());
    assert_eq!(text.peer().get_name(), "");
}

#[test]
fn rich_automation_editor_raw_presenter_does_not_duplicate_embedded_document_children() {
    let _scope = test_scope();
    let editor = RichEditBox::new();
    let paragraph = Paragraph::new();
    let link = Hyperlink::with_text("link ", None);
    let embedded = Border::new();
    embedded.set_width(16.0);
    embedded.set_height(12.0);
    link.add_child(InlineUIContainer::with_child(embedded.clone()));
    paragraph.add_child(link);
    let document = RichTextDocument::new();
    document.blocks().add(paragraph);
    editor.set_document(document);
    apply_editor_template(&editor, Thickness::uniform(4.0));
    let root = TestRoot::with_child(editor.clone());
    let size = Size::new(200.0, 60.0);
    root.set_client_size(size);
    root.measure(size);
    root.arrange(Rect::from_size(size));
    let text = provider(&editor);
    let link_peer = text
        .document_range()
        .unwrap()
        .get_children()
        .unwrap()
        .into_iter()
        .find(|peer| peer.get_provider::<dyn ITextChildProvider>().is_some())
        .unwrap();
    let embedded_peer = ControlAutomationPeer::create_peer_for_element(&embedded);
    assert_eq!(
        link_peer.get_children().as_ref(),
        &vec![embedded_peer.clone()]
    );
    let presenter_peer = ControlAutomationPeer::create_peer_for_element(&editor.text_presenter());
    assert!(!presenter_peer.is_content_element());
    assert!(!presenter_peer.is_control_element());
    assert!(presenter_peer.get_provider::<dyn ITextProvider>().is_none());
    assert!(presenter_peer.get_children().is_empty());
    let mut pending = text.peer().get_children().as_ref().clone();
    let mut visited = std::collections::HashSet::new();
    while let Some(peer) = pending.pop() {
        assert!(visited.insert(
            std::ptr::from_ref::<crate::automation::peers::AutomationPeer>(&peer) as usize
        ));
        assert!(visited.len() < 32);
        pending.extend(peer.get_children().iter().cloned());
    }
    assert_eq!(embedded_peer.get_parent(), Some(link_peer));
}

// These regressions exercise the portable TextEdit bridge against the actual
// editor. They are additional coverage for the WinUI/SDK provider contract.

#[test]
fn rich_automation_text_edit_composition_range_tracks_utf16_preview_and_rejects_stale_state() {
    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor.set_text("a😀b tail").unwrap();
    editor.select(1, 4);
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let text = peer.get_provider::<dyn ITextProvider>().unwrap();
    let edit = peer.get_provider::<dyn ITextEditProvider>().unwrap();
    assert!(edit.get_active_composition().unwrap().is_none());
    assert!(edit.get_conversion_target().unwrap().is_none());

    editor.set_preedit_text(Some("候😀補"), Some(1)).unwrap();
    let before = editor.editor().selection();
    let revision = editor.document().revision();
    let active = edit.get_active_composition().unwrap().unwrap();
    let all = text.document_range().unwrap();
    assert_eq!(active.get_text(None).unwrap(), "候😀補");
    assert_eq!(active.peer().unwrap(), peer);
    assert_eq!(
        active
            .compare_endpoints(Endpoint::Start, &*all, Endpoint::Start)
            .unwrap(),
        1
    );
    assert_eq!(
        active
            .compare_endpoints(Endpoint::End, &*all, Endpoint::Start)
            .unwrap(),
        5
    );
    let caret = peer
        .get_provider::<dyn ITextProvider2>()
        .unwrap()
        .get_caret_range()
        .unwrap()
        .range
        .unwrap();
    assert_eq!(
        caret
            .compare_endpoints(Endpoint::Start, &*all, Endpoint::Start)
            .unwrap(),
        2
    );
    assert!(edit.get_conversion_target().unwrap().is_none());
    assert_eq!(editor.editor().selection(), before);
    assert_eq!(editor.document().revision(), revision);

    editor.document().replace_text(0..0, "z").unwrap();
    assert!(edit.get_active_composition().unwrap().is_none());
    assert_eq!(active.get_text(None).unwrap(), "候😀補");
    assert_eq!(
        active
            .compare_endpoints(
                Endpoint::Start,
                &*text.document_range().unwrap(),
                Endpoint::Start
            )
            .unwrap(),
        2
    );
    assert!(editor.cancel_composition().unwrap());
    assert_eq!(editor.text(), "za候😀補 tail");

    editor.select(2, 6);
    editor.set_preedit_text(Some(""), Some(0)).unwrap();
    let empty = edit.get_active_composition().unwrap().unwrap();
    assert_eq!(empty.get_text(None).unwrap(), "");
    assert_eq!(
        empty
            .compare_endpoints(Endpoint::Start, &*empty, Endpoint::End)
            .unwrap(),
        0
    );
    editor.commit_composition("完成").unwrap();
    assert!(edit.get_active_composition().unwrap().is_none());
    assert!(edit.get_conversion_target().unwrap().is_none());
}

#[test]
fn rich_automation_text_edit_events_report_accepted_deltas_and_single_finalization() {
    use crate::RichEditCharacterCasing;
    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor.set_text("LR").unwrap();
    editor.set_max_length(6);
    editor.set_accepts_return(false);
    editor.set_character_casing(RichEditCharacterCasing::Upper);
    editor.select(1, 1);
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let edit = peer.get_provider::<dyn ITextEditProvider>().unwrap();
    let events = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = events.clone();
    let active_during_events = Rc::new(std::cell::RefCell::new(Vec::new()));
    let active = active_during_events.clone();
    let weak_provider = Rc::downgrade(&edit);
    let subscription = edit
        .text_edit_text_changed(Rc::new(move |event| {
            let provider = weak_provider.upgrade().unwrap();
            active.borrow_mut().push(
                provider
                    .get_active_composition()
                    .unwrap()
                    .map(|range| range.get_text(None).unwrap()),
            );
            observed.borrow_mut().push(event.clone());
        }))
        .unwrap();

    editor.set_preedit_text(Some("ab😀\r\nZ"), Some(1)).unwrap();
    editor.set_preedit_text(Some("ac😀\r\nZ"), Some(1)).unwrap();
    editor.set_preedit_text(Some("ac😀"), Some(2)).unwrap();
    editor.commit_composition("ax😀\r\nZZ").unwrap();
    assert_eq!(editor.text(), "LAX😀R");
    assert_eq!(
        active_during_events.borrow().as_slice(),
        &[Some("AB😀".to_owned()), Some("AC😀".to_owned()), None]
    );
    assert_eq!(
        events.borrow().as_slice(),
        &[
            TextEditTextChangedEventArgs {
                change_type: TextEditChangeType::Composition,
                changed_data: vec!["AB😀".to_owned()]
            },
            TextEditTextChangedEventArgs {
                change_type: TextEditChangeType::Composition,
                changed_data: vec!["C".to_owned()]
            },
            TextEditTextChangedEventArgs {
                change_type: TextEditChangeType::CompositionFinalized,
                changed_data: vec!["AX😀".to_owned()]
            },
        ]
    );

    events.borrow_mut().clear();
    editor.select(1, 5);
    editor.set_preedit_text(Some("z😀"), Some(1)).unwrap();
    editor.cancel_composition().unwrap();
    assert_eq!(editor.text(), "LAX😀R");
    assert_eq!(
        events.borrow().as_slice(),
        &[
            TextEditTextChangedEventArgs {
                change_type: TextEditChangeType::Composition,
                changed_data: vec!["Z😀".to_owned()]
            },
            TextEditTextChangedEventArgs {
                change_type: TextEditChangeType::CompositionFinalized,
                changed_data: vec![String::new()]
            },
        ]
    );
    events.borrow_mut().clear();
    editor.insert_text("plain").unwrap();
    assert!(events.borrow().is_empty());
    subscription.dispose();
}

#[test]
fn rich_automation_text_edit_observation_follows_document_replacement_and_disposal() {
    let _scope = test_scope();
    let editor = RichEditBox::new();
    editor.set_text("old").unwrap();
    let old_editor = editor.editor();
    let peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let edit = peer.get_provider::<dyn ITextEditProvider>().unwrap();
    let events = Rc::new(std::cell::RefCell::new(Vec::new()));
    let observed = events.clone();
    let subscription = edit
        .text_edit_text_changed(Rc::new(move |event| {
            observed.borrow_mut().push(event.clone())
        }))
        .unwrap();
    editor.set_preedit_text(Some("old preview"), None).unwrap();
    let old_range = edit.get_active_composition().unwrap().unwrap();
    events.borrow_mut().clear();

    editor.set_document(RichTextDocument::from_text("new"));
    assert!(events.borrow().is_empty());
    assert!(edit.get_active_composition().unwrap().is_none());
    assert!(old_range.get_text(None).is_err());
    old_editor.set_preedit("detached preview", None).unwrap();
    assert!(events.borrow().is_empty());
    editor.set_preedit_text(Some("current"), None).unwrap();
    assert_eq!(
        events.borrow().as_slice(),
        &[TextEditTextChangedEventArgs {
            change_type: TextEditChangeType::Composition,
            changed_data: vec!["current".to_owned()]
        },]
    );
    subscription.dispose();
    editor.cancel_composition().unwrap();
    assert_eq!(events.borrow().len(), 1);
}

#[test]
fn rich_automation_text_edit_pattern_is_editor_only_and_does_not_retain_owner() {
    let _scope = test_scope();
    let view = rich("read-only content");
    assert!(ControlAutomationPeer::create_peer_for_element(&view)
        .get_provider::<dyn ITextEditProvider>()
        .is_none());
    let events = Rc::new(Cell::new(0));
    let (edit, range, model, subscription, weak_owner) = {
        let editor = RichEditBox::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&editor);
        editor.set_is_read_only(true);
        let edit = peer.get_provider::<dyn ITextEditProvider>().unwrap();
        assert!(edit.get_active_composition().unwrap().is_none());
        editor.set_is_read_only(false);
        editor.set_preedit_text(Some("active"), None).unwrap();
        let range = edit.get_active_composition().unwrap().unwrap();
        let observed = events.clone();
        let subscription = edit
            .text_edit_text_changed(Rc::new(move |_| observed.set(observed.get() + 1)))
            .unwrap();
        assert!(
            ControlAutomationPeer::create_peer_for_element(&editor.text_presenter())
                .get_provider::<dyn ITextEditProvider>()
                .is_none()
        );
        (
            edit,
            range,
            editor.editor(),
            subscription,
            editor.downgrade(),
        )
    };
    assert!(weak_owner.upgrade().is_none());
    assert_eq!(
        edit.get_active_composition()
            .err()
            .unwrap()
            .downcast_ref::<TextProviderError>(),
        Some(&TextProviderError::ElementNotAvailable)
    );
    assert!(edit.get_conversion_target().is_err());
    assert!(range.get_text(None).is_err());
    model.set_preedit("retained model", None).unwrap();
    assert_eq!(events.get(), 0);
    subscription.dispose();
}
