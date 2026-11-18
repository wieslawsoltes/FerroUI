//! The text pattern of a plain `TextBox` (#430). FerroUI tests, not upstream ones:
//! upstream's text box peer has the value pattern only.

use super::{
    ITextProvider, ITextProvider2, ITextRangeProvider, IValueProvider, ProviderError,
    SupportedTextSelection, TextAttribute, TextAttributeValue, TextPatternRangeEndpoint as Endpoint, TextProviderError,
    TextUnit,
};
use crate::automation::peers::ControlAutomationPeer;
use crate::test_support::TestRoot;
use crate::text_box_tests::{templated_text_box, test_scope};
use crate::TextBox;
use ferroui_base::media::TextWrapping;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Point, Rect, Ref, Size, Thickness};
use std::cell::Cell;
use std::rc::Rc;

fn provider(text_box: &TextBox) -> Rc<dyn ITextProvider> {
    ControlAutomationPeer::create_peer_for_element(text_box)
        .get_provider::<dyn ITextProvider>()
        .expect("a text box exposes the Text pattern")
}

fn found(provider: &dyn ITextProvider, text: &str) -> Rc<dyn ITextRangeProvider> {
    provider
        .document_range()
        .unwrap()
        .find_text(text, false, false)
        .unwrap()
        .unwrap()
}

fn not_available(error: ProviderError) -> bool {
    error.downcast_ref::<TextProviderError>() == Some(&TextProviderError::ElementNotAvailable)
}

/// A text box in a root, measured and arranged in `size`.
fn laid_out(text: &str, size: Size) -> (Ref<TextBox>, Ref<TestRoot>) {
    let text_box = templated_text_box(Some(text));
    text_box.set_margin(Thickness::new(20.0, 10.0, 0.0, 0.0));
    let root = TestRoot::with_child(text_box.clone());
    root.measure(size);
    root.arrange(Rect::from_size(size));
    (text_box, root)
}

#[test]
fn a_text_box_offers_the_text_and_value_patterns_over_its_text() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    text_box.set_text(Some("a😀b plain"));
    let peer = ControlAutomationPeer::create_peer_for_element(&text_box);
    assert!(peer.get_provider::<dyn IValueProvider>().is_some());
    assert!(peer.get_provider::<dyn ITextProvider2>().is_some());
    let text = peer.get_provider::<dyn ITextProvider>().unwrap();
    assert!(text.peer() == peer);
    assert_eq!(text.supported_text_selection().unwrap(), SupportedTextSelection::Single);
    let document = text.document_range().unwrap();
    assert_eq!(document.get_text(None).unwrap(), "a😀b plain");
    assert_eq!(document.get_text(Some(2)).unwrap(), "a");
    assert_eq!(document.get_text(Some(3)).unwrap(), "a😀");
    assert!(document.get_enclosing_element().unwrap() == peer);
    assert!(document.get_children().unwrap().is_empty());
    let emoji = found(&*text, "😀");
    assert_eq!(emoji.compare_endpoints(Endpoint::End, &*emoji, Endpoint::Start).unwrap(), 2);
    assert!(text.range_from_child(&peer).is_err());
    text_box.set_text(None);
    assert_eq!(document.get_text(None).unwrap(), "");
}

#[test]
fn the_selection_is_one_range_and_the_caret_an_empty_one() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    text_box.set_text(Some("alpha beta"));
    let peer = ControlAutomationPeer::create_peer_for_element(&text_box);
    let text = peer.get_provider::<dyn ITextProvider>().unwrap();
    let document = text.document_range().unwrap();

    text_box.set_selection_start(6);
    text_box.set_selection_end(10);
    let selection = text.get_selection().unwrap();
    assert_eq!(selection.len(), 1);
    assert_eq!(selection[0].get_text(None).unwrap(), "beta");

    // A selection made from its end is the same range.
    text_box.set_selection_start(10);
    text_box.set_selection_end(6);
    assert_eq!(text.get_selection().unwrap()[0].get_text(None).unwrap(), "beta");

    text_box.set_caret_index(3);
    let caret = text.get_selection().unwrap().remove(0);
    assert_eq!(caret.get_text(None).unwrap(), "");
    assert_eq!(caret.compare_endpoints(Endpoint::Start, &*document, Endpoint::Start).unwrap(), 3);
    assert_eq!(caret.compare_endpoints(Endpoint::End, &*caret, Endpoint::Start).unwrap(), 0);
    assert!(caret.get_bounding_rectangles().unwrap().is_empty());

    let text2 = peer.get_provider::<dyn ITextProvider2>().unwrap();
    let caret = text2.get_caret_range().unwrap();
    assert!(!caret.is_active);
    let caret = caret.range.unwrap();
    assert_eq!(caret.compare_endpoints(Endpoint::Start, &*document, Endpoint::Start).unwrap(), 3);
    assert_eq!(caret.get_text(None).unwrap(), "");
}

#[test]
fn selecting_a_range_selects_the_text_of_the_box() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    text_box.set_text(Some("alpha beta gamma"));
    let text = provider(&text_box);

    found(&*text, "beta").select().unwrap();
    assert_eq!((text_box.selection_start(), text_box.selection_end()), (6, 10));
    assert_eq!(text_box.caret_index(), 10);

    found(&*text, "gamma").add_to_selection().unwrap_err();
    found(&*text, " gam").add_to_selection().unwrap();
    assert_eq!((text_box.selection_start(), text_box.selection_end()), (6, 14));
    found(&*text, "beta").remove_from_selection().unwrap();
    assert_eq!((text_box.selection_start(), text_box.selection_end()), (10, 14));

    text_box.set_is_enabled(false);
    assert!(found(&*text, "alpha").select().is_err());
}

#[test]
fn ranges_move_by_character_word_paragraph_and_document() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    text_box.set_text(Some("one two\nthree"));
    let text = provider(&text_box);
    let document = text.document_range().unwrap();

    let caret = text.get_selection().unwrap().remove(0);
    assert_eq!(caret.move_by(TextUnit::Character, 3).unwrap(), 3);
    assert_eq!(caret.compare_endpoints(Endpoint::Start, &*document, Endpoint::Start).unwrap(), 3);
    assert_eq!(caret.move_by(TextUnit::Character, -10).unwrap(), -3);
    assert_eq!(caret.move_endpoint_by_unit(Endpoint::End, TextUnit::Word, 1).unwrap(), 1);
    assert_eq!(caret.get_text(None).unwrap(), "one");

    let range = found(&*text, "w");
    range.expand_to_enclosing_unit(TextUnit::Character).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "w");
    range.expand_to_enclosing_unit(TextUnit::Word).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "two");
    // Text that is not laid out has no lines: a line is a paragraph.
    range.expand_to_enclosing_unit(TextUnit::Line).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "one two\n");
    range.expand_to_enclosing_unit(TextUnit::Paragraph).unwrap();
    assert_eq!(range.get_text(None).unwrap(), "one two\n");
    assert_eq!(range.move_by(TextUnit::Paragraph, 1).unwrap(), 1);
    assert_eq!(range.get_text(None).unwrap(), "three");
    range.expand_to_enclosing_unit(TextUnit::Document).unwrap();
    assert!(range.compare(&*document).unwrap());

    let word = found(&*text, "one");
    assert_eq!(word.move_by(TextUnit::Word, 2).unwrap(), 2);
    assert_eq!(word.get_text(None).unwrap(), "two");
    word.move_endpoint_by_range(Endpoint::End, &*document, Endpoint::End).unwrap();
    assert_eq!(word.get_text(None).unwrap(), "two\nthree");
    assert!(word.find_text("THREE", false, true).unwrap().is_some());
    assert!(word.find_text("THREE", false, false).unwrap().is_none());
    assert!(word.find_text("one", false, false).unwrap().is_none());
}

#[test]
fn line_units_follow_the_layout_of_the_presenter() {
    let _scope = test_scope();
    let text_box = templated_text_box(Some("alpha beta gamma delta"));
    text_box.set_text_wrapping(TextWrapping::Wrap);
    let root = TestRoot::with_child(text_box.clone());
    root.measure(Size::new(70.0, 500.0));
    root.arrange(Rect::new(0.0, 0.0, 70.0, 500.0));
    let text = provider(&text_box);
    let range = found(&*text, "gamma");
    range.expand_to_enclosing_unit(TextUnit::Line).unwrap();
    let presenter = text_box.presenter().expect("the template has a presenter");
    let layout = presenter.text_layout();
    let line = layout
        .text_lines()
        .iter()
        .map(|line| line.first_text_source_index() as usize..(line.first_text_source_index() + line.length()) as usize)
        .find(|line| line.start <= 11 && 11 < line.end)
        .unwrap();
    assert_eq!(range.get_text(None).unwrap(), "alpha beta gamma delta"[line].to_owned());
}

#[test]
fn geometry_is_in_root_coordinates_from_the_layout_of_the_presenter() {
    let _scope = test_scope();
    let (text_box, _root) = laid_out("alpha beta", Size::new(200.0, 80.0));
    let text = provider(&text_box);
    let all = text.document_range().unwrap();
    let rectangles = all.get_bounding_rectangles().unwrap();
    assert!(!rectangles.is_empty());
    assert!(rectangles[0].x >= 20.0);
    assert!(rectangles[0].y >= 10.0);

    let first = rectangles[0];
    let hit = text.range_from_point(Point::new(first.x + 0.01, first.y + first.height / 2.0)).unwrap();
    hit.expand_to_enclosing_unit(TextUnit::Character).unwrap();
    assert_eq!(hit.get_text(None).unwrap(), "a");
    let beta = found(&*text, "beta").get_bounding_rectangles().unwrap();
    assert_eq!(beta.len(), 1);
    assert!(beta[0].x > first.x);
    assert!(!text.get_visible_ranges().unwrap().is_empty());
    assert_eq!(text.get_visible_ranges().unwrap()[0].get_text(None).unwrap(), "alpha beta");

    text_box.set_is_visible(false);
    assert!(all.get_bounding_rectangles().unwrap().is_empty());
    assert_eq!(text.get_visible_ranges().unwrap()[0].get_text(None).unwrap(), "");
}

#[test]
fn the_text_has_the_format_of_the_control() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    text_box.set_text(Some("plain"));
    text_box.set_font_size(16.0);
    let text = provider(&text_box);
    let document = text.document_range().unwrap();
    assert_eq!(document.get_attribute_value(TextAttribute::FontSize).unwrap(), TextAttributeValue::Number(12.0));
    assert_eq!(document.get_attribute_value(TextAttribute::IsReadOnly).unwrap(), TextAttributeValue::Bool(false));
    text_box.set_is_read_only(true);
    assert_eq!(document.get_attribute_value(TextAttribute::IsReadOnly).unwrap(), TextAttributeValue::Bool(true));
    assert!(document
        .find_attribute(TextAttribute::IsReadOnly, &TextAttributeValue::Bool(true), false)
        .unwrap()
        .is_some_and(|range| range.compare(&*document).unwrap()));
    assert!(document
        .find_attribute(TextAttribute::IsReadOnly, &TextAttributeValue::Bool(false), false)
        .unwrap()
        .is_none());
    assert_eq!(
        document.get_attribute_value(TextAttribute::Unknown(-1)).unwrap(),
        TextAttributeValue::NotSupported
    );
}

#[test]
fn a_password_box_discloses_no_text_through_the_text_pattern() {
    let _scope = test_scope();
    let field = TextBox::new();
    field.set_text(Some("synthetic fixture"));
    let peer = ControlAutomationPeer::create_peer_for_element(&field);
    let retained = peer.get_provider::<dyn ITextProvider>().unwrap();
    let range = retained.document_range().unwrap();
    let changes = Rc::new(Cell::new(0));
    let sink = changes.clone();
    let changed = retained.text_changed(Rc::new(move || sink.set(sink.get() + 1))).unwrap();
    let selections = Rc::new(Cell::new(0));
    let sink = selections.clone();
    let selected = retained.text_selection_changed(Rc::new(move || sink.set(sink.get() + 1))).unwrap();

    field.set_password_char('●');
    for reveal in [false, true] {
        field.set_reveal_password(reveal);
        assert!(peer.get_provider::<dyn ITextProvider>().is_none());
        assert!(peer.get_provider::<dyn ITextProvider2>().is_none());
        assert!(not_available(retained.document_range().err().unwrap()));
        assert!(not_available(retained.get_selection().err().unwrap()));
        assert!(not_available(range.get_text(None).unwrap_err()));
        assert!(not_available(range.clone_range().err().unwrap()));
        assert!(range.find_text("secret", false, false).is_err());
    }
    field.set_text(Some("secret"));
    field.set_caret_index(2);
    assert_eq!((changes.get(), selections.get()), (0, 0));

    field.set_password_char('\0');
    assert_eq!(range.get_text(None).unwrap(), "secret");
    changed.dispose();
    selected.dispose();
}

#[test]
fn text_and_selection_changes_are_observed_until_disposed() {
    let _scope = test_scope();
    let text_box = TextBox::new();
    text_box.set_text(Some("alpha beta"));
    let text = provider(&text_box);
    let changes = Rc::new(Cell::new(0));
    let sink = changes.clone();
    let changed = text.text_changed(Rc::new(move || sink.set(sink.get() + 1))).unwrap();
    let selections = Rc::new(Cell::new(0));
    let sink = selections.clone();
    let selected = text.text_selection_changed(Rc::new(move || sink.set(sink.get() + 1))).unwrap();

    // A caret move is a selection change.
    text_box.set_caret_index(4);
    assert_eq!(selections.get(), 1);
    text_box.set_selection_end(7);
    assert_eq!(selections.get(), 2);
    assert_eq!(changes.get(), 0);

    text_box.set_text(Some("other"));
    assert_eq!(changes.get(), 1);

    changed.dispose();
    selected.dispose();
    let before = selections.get();
    text_box.set_text(Some("again"));
    text_box.set_caret_index(1);
    assert_eq!(changes.get(), 1);
    assert_eq!(selections.get(), before);
}
