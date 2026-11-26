//! Paragraph lists (#1257): marker texts, numbering, layout and undoable list edits.
use super::list_marker::list_marker_text;
use super::{
    ListNumbering, MarkerAlignment, MarkerStyle, MarkerType, Paragraph, ParagraphFormat, RichTextDocument,
};
use crate::rich_text::{EditKind, EditorSelection, RichTextEditor};
use crate::test_support::test_scope;
use crate::RichTextBlock;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::FlowDirection;
use ferroui_base::{Point, Rect, Size};

#[test]
fn list_enums_keep_the_text_object_model_values() {
    assert_eq!(
        [MarkerType::None as i32, MarkerType::Bullet as i32, MarkerType::UppercaseRoman as i32],
        [0, 1, 6]
    );
    assert_eq!(MarkerType::DevanagariNumeric as i32, 23);
    assert_eq!(
        [
            MarkerStyle::Parenthesis as i32,
            MarkerStyle::Parentheses as i32,
            MarkerStyle::Period as i32,
            MarkerStyle::Plain as i32,
            MarkerStyle::NoNumber as i32,
            MarkerStyle::Minus as i32,
        ],
        [0, 0x10000, 0x20000, 0x30000, 0x40000, 0x80000]
    );
    assert_eq!(MarkerType::default(), MarkerType::None);
    assert_eq!(MarkerStyle::default(), MarkerStyle::Parenthesis);
    assert_eq!(MarkerAlignment::default(), MarkerAlignment::Left);
}

#[test]
fn list_marker_texts_follow_type_style_and_level() {
    use MarkerStyle as S;
    use MarkerType as T;
    let text = |kind, style, number| list_marker_text(kind, style, 1, number);
    assert_eq!(list_marker_text(T::Bullet, S::Period, 1, 3), "\u{2022}");
    assert_eq!(list_marker_text(T::Bullet, S::Period, 2, 1), "\u{25E6}");
    assert_eq!(list_marker_text(T::Bullet, S::Period, 3, 1), "\u{25AA}");
    assert_eq!(list_marker_text(T::Bullet, S::Period, 4, 1), "\u{2022}");
    assert_eq!(text(T::None, S::Period, 1), "");
    assert_eq!(text(T::Arabic, S::Parenthesis, 1), "1)");
    assert_eq!(text(T::Arabic, S::Parentheses, 12), "(12)");
    assert_eq!(text(T::Arabic, S::Period, 3), "3.");
    assert_eq!(text(T::Arabic, S::Plain, 4), "4");
    assert_eq!(text(T::Arabic, S::Minus, 5), "5-");
    assert_eq!(text(T::Arabic, S::NoNumber, 5), "");
    assert_eq!(text(T::LowercaseEnglishLetter, S::Period, 1), "a.");
    assert_eq!(text(T::LowercaseEnglishLetter, S::Period, 27), "aa.");
    assert_eq!(text(T::UppercaseEnglishLetter, S::Plain, 28), "AB");
    assert_eq!(text(T::UppercaseRoman, S::Period, 4), "IV.");
    assert_eq!(text(T::UppercaseRoman, S::Plain, 1994), "MCMXCIV");
    assert_eq!(text(T::LowercaseRoman, S::Parenthesis, 9), "ix)");
    // Outside the range of a numbering system the number is written in Arabic digits.
    assert_eq!(text(T::UppercaseRoman, S::Plain, 4000), "4000");
    assert_eq!(text(T::LowercaseEnglishLetter, S::Plain, 0), "0");
    assert_eq!(text(T::UnicodeSequence, S::Period, 0x41), "A");
    assert_eq!(text(T::CircledNumber, S::Period, 1), "\u{2460}");
    assert_eq!(text(T::CircledNumber, S::Period, 21), "\u{3251}");
    assert_eq!(text(T::CircledNumber, S::Period, 51), "51");
    assert_eq!(text(T::BlackCircleWingding, S::Plain, 1), "\u{2776}");
    assert_eq!(text(T::WhiteCircleWingding, S::Plain, 10), "\u{2789}");
    assert_eq!(text(T::ArabicWide, S::Plain, 12), "\u{FF11}\u{FF12}");
    assert_eq!(text(T::ThaiNumeric, S::Plain, 3), "\u{0E53}");
    assert_eq!(text(T::DevanagariNumeric, S::Plain, 7), "\u{096D}");
    assert_eq!(text(T::SimplifiedChinese, S::Plain, 10), "十");
    assert_eq!(text(T::SimplifiedChinese, S::Plain, 21), "二十一");
    assert_eq!(text(T::SimplifiedChinese, S::Plain, 101), "一百零一");
    assert_eq!(text(T::TraditionalChinese, S::Plain, 3), "參");
    assert_eq!(text(T::Hebrew, S::Plain, 15), "טו");
    assert_eq!(text(T::Hebrew, S::Plain, 123), "קכג");
    assert_eq!(text(T::ArabicAbjad, S::Plain, 3), "ج");
    assert_eq!(text(T::ArabicDictionary, S::Plain, 3), "ت");
    assert_eq!(text(T::ThaiAlphabetic, S::Plain, 1), "ก");
    assert_eq!(text(T::DevanagariVowel, S::Plain, 2), "आ");
    assert_eq!(text(T::DevanagariConsonant, S::Plain, 1), "क");
}

#[test]
fn list_numbering_continues_restarts_and_nests() {
    use MarkerStyle as S;
    use MarkerType as T;
    let mut numbering = ListNumbering::default();
    let mut next = |kind, start, level| numbering.next(kind, S::Period, start, level).map(|(_, text)| text);
    assert_eq!(next(T::Arabic, 1, 1).as_deref(), Some("1."));
    assert_eq!(next(T::Arabic, 1, 1).as_deref(), Some("2."));
    // A nested sequence does not interrupt its parent.
    assert_eq!(next(T::LowercaseEnglishLetter, 1, 2).as_deref(), Some("a."));
    assert_eq!(next(T::LowercaseEnglishLetter, 1, 2).as_deref(), Some("b."));
    assert_eq!(next(T::Arabic, 1, 1).as_deref(), Some("3."));
    // A nested sequence after its parent's next item starts again.
    assert_eq!(next(T::LowercaseEnglishLetter, 1, 2).as_deref(), Some("a."));
    // Level 0 of a list paragraph is level 1.
    assert_eq!(next(T::Arabic, 1, 0).as_deref(), Some("4."));
    // Another start, type or style starts another sequence.
    assert_eq!(next(T::Arabic, 7, 1).as_deref(), Some("7."));
    assert_eq!(next(T::UppercaseRoman, 7, 1).as_deref(), Some("VII."));
    // A paragraph outside a list ends every sequence.
    assert_eq!(next(T::None, 1, 0), None);
    assert_eq!(next(T::UppercaseRoman, 7, 1).as_deref(), Some("VII."));
}

#[test]
fn list_paragraph_format_round_trips_validates_and_overlays() {
    let _scope = test_scope();
    let paragraph = Paragraph::with_text("item");
    let format = ParagraphFormat {
        list_type: Some(MarkerType::UppercaseRoman),
        list_style: Some(MarkerStyle::Period),
        list_start: Some(3),
        list_tab: Some(30.0),
        list_level_index: Some(2),
        list_alignment: Some(MarkerAlignment::Right),
        ..Default::default()
    };
    format.try_apply_to(&paragraph).unwrap();
    assert_eq!(paragraph.list_type(), MarkerType::UppercaseRoman);
    assert_eq!(paragraph.list_style(), MarkerStyle::Period);
    assert_eq!(paragraph.list_start(), 3);
    assert_eq!(paragraph.list_tab(), 30.0);
    assert_eq!(paragraph.list_level_index(), 2);
    assert_eq!(paragraph.list_alignment(), MarkerAlignment::Right);
    let captured = ParagraphFormat::from_paragraph(&paragraph);
    assert_eq!(captured.list_type, format.list_type);
    assert_eq!(captured.list_level_index, format.list_level_index);
    assert_eq!(ParagraphFormat::effective(&paragraph).list_alignment, Some(MarkerAlignment::Right));
    let overlaid = captured.overlay(&ParagraphFormat {
        list_type: Some(MarkerType::Bullet),
        ..Default::default()
    });
    assert_eq!(overlaid.list_type, Some(MarkerType::Bullet));
    assert_eq!(overlaid.list_start, Some(3));
    for invalid in [
        ParagraphFormat { list_tab: Some(-1.0), ..Default::default() },
        ParagraphFormat { list_tab: Some(f64::NAN), ..Default::default() },
        ParagraphFormat { list_level_index: Some(10), ..Default::default() },
        ParagraphFormat { list_level_index: Some(-1), ..Default::default() },
    ] {
        assert!(invalid.try_apply_to(&paragraph).is_err());
    }
    assert_eq!(paragraph.list_level_index(), 2);
}

fn list_view(paragraphs: &[(&str, MarkerType, i32)]) -> ferroui_base::Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.set_font_size(12.0);
    for (text, kind, level) in paragraphs {
        let paragraph = Paragraph::with_text(text);
        paragraph.set_list_type(*kind);
        paragraph.set_list_style(MarkerStyle::Period);
        paragraph.set_list_level_index(*level);
        view.blocks().add(paragraph);
    }
    view.measure(Size::new(300.0, 200.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
    view
}

#[test]
fn list_paragraphs_lay_out_after_their_markers_and_number_in_sequence() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = list_view(&[
        ("one", MarkerType::UppercaseRoman, 1),
        ("two", MarkerType::UppercaseRoman, 1),
        ("nested", MarkerType::Bullet, 2),
        ("plain", MarkerType::None, 0),
    ]);
    // The default marker space is two ems of the paragraph's font: 24 at 12 pixels, unless a
    // marker is wider; a nested level adds one more space.
    let one = view.get_rect_from_character_index(0).x;
    assert!(one >= 24.0 - 0.01, "{one}");
    let nested = view.get_rect_from_character_index(8).x;
    assert!((nested - 48.0).abs() < 0.01, "{nested}");
    let plain = view.get_rect_from_character_index(15).x;
    assert!(plain.abs() < 0.01, "{plain}");
    let markers = view.page_layout().list_markers(Point::default());
    let texts: Vec<&str> = markers.iter().map(|(text, _)| text.as_str()).collect();
    assert_eq!(texts, ["I.", "II.", "\u{25E6}"]);
    assert!(markers[0].1.x.abs() < 0.01);
    assert!((markers[2].1.x - 24.0).abs() < 0.01);
    // Each marker is on the line of its paragraph's first line.
    assert!(markers[0].1.y < markers[1].1.y && markers[1].1.y < markers[2].1.y);
}

#[test]
fn list_markers_follow_property_changes_and_right_to_left_paragraphs() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = list_view(&[("a", MarkerType::Arabic, 1), ("b", MarkerType::Arabic, 1)]);
    let paragraphs = view.rich_document().paragraphs();
    paragraphs[0].set_list_start(5);
    paragraphs[1].set_list_start(5);
    view.measure(Size::new(300.0, 200.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
    let texts: Vec<String> = view
        .page_layout()
        .list_markers(Point::default())
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    assert_eq!(texts, ["5.", "6."]);
    paragraphs[0].set_list_type(MarkerType::None);
    paragraphs[1].set_flow_direction(FlowDirection::RightToLeft);
    paragraphs[1].set_list_tab(40.0);
    view.measure(Size::new(300.0, 200.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
    let markers = view.page_layout().list_markers(Point::default());
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].0, "5.");
    // A right-to-left list paragraph has its marker at the right edge.
    assert!(markers[0].1.x > 300.0 - 40.0 - 0.01, "{:?}", markers[0].1);
    assert!(view.get_rect_from_character_index(0).x.abs() < 0.01);
}

fn editor(text: &str) -> RichTextEditor {
    let editor = RichTextEditor::new(RichTextDocument::from_text(text));
    editor.set_selection(EditorSelection::caret(0)).unwrap();
    editor
}

fn list_types(editor: &RichTextEditor) -> Vec<MarkerType> {
    editor.document().paragraphs().iter().map(|paragraph| paragraph.list_type()).collect()
}

fn list_levels(editor: &RichTextEditor) -> Vec<i32> {
    editor.document().paragraphs().iter().map(|paragraph| paragraph.list_level_index()).collect()
}

#[test]
fn list_types_apply_change_and_clear_on_a_selection_with_undo() {
    let _scope = test_scope();
    let editor = editor("one\ntwo\nthree");
    // The selection covers "ne\ntw": the first two paragraphs.
    editor.set_selection(EditorSelection::new(1, 6)).unwrap();
    let list = |kind| ParagraphFormat { list_type: Some(kind), ..Default::default() };
    editor.apply_paragraph_format(&list(MarkerType::Bullet)).unwrap();
    assert_eq!(list_types(&editor), [MarkerType::Bullet, MarkerType::Bullet, MarkerType::None]);
    editor.apply_paragraph_format(&list(MarkerType::UppercaseRoman)).unwrap();
    assert_eq!(
        list_types(&editor),
        [MarkerType::UppercaseRoman, MarkerType::UppercaseRoman, MarkerType::None]
    );
    editor.apply_paragraph_format(&list(MarkerType::None)).unwrap();
    assert_eq!(list_types(&editor), [MarkerType::None; 3]);
    assert!(editor.undo().unwrap());
    assert_eq!(
        list_types(&editor),
        [MarkerType::UppercaseRoman, MarkerType::UppercaseRoman, MarkerType::None]
    );
    assert!(editor.undo().unwrap());
    assert_eq!(list_types(&editor), [MarkerType::Bullet, MarkerType::Bullet, MarkerType::None]);
    assert!(editor.undo().unwrap());
    assert_eq!(list_types(&editor), [MarkerType::None; 3]);
    assert!(editor.redo().unwrap());
    assert_eq!(list_types(&editor), [MarkerType::Bullet, MarkerType::Bullet, MarkerType::None]);
    assert_eq!(editor.document().text(), "one\ntwo\nthree");
}

#[test]
fn list_levels_indent_and_outdent_as_one_undo_unit() {
    let _scope = test_scope();
    let editor = editor("one\ntwo\nthree");
    editor.set_selection(EditorSelection::new(0, 13)).unwrap();
    editor
        .apply_paragraph_format(&ParagraphFormat { list_type: Some(MarkerType::Arabic), ..Default::default() })
        .unwrap();
    // A selection that ends at the start of a paragraph does not change its level.
    editor.set_selection(EditorSelection::new(4, 8)).unwrap();
    assert!(editor.tab_changes_list_level());
    assert!(editor.change_list_level(1).unwrap());
    assert_eq!(list_levels(&editor), [0, 2, 0]);
    assert!(editor.change_list_level(1).unwrap());
    assert_eq!(list_levels(&editor), [0, 3, 0]);
    assert!(editor.change_list_level(-5).unwrap());
    assert_eq!(list_levels(&editor), [0, 1, 0]);
    // Level 1 is the shallowest level of a list.
    assert!(!editor.change_list_level(-1).unwrap());
    editor.set_selection(EditorSelection::new(0, 13)).unwrap();
    assert!(editor.change_list_level(9).unwrap());
    assert_eq!(list_levels(&editor), [9, 9, 9]);
    assert!(editor.undo().unwrap());
    assert_eq!(list_levels(&editor), [0, 1, 0]);
    assert!(editor.undo().unwrap());
    assert_eq!(list_levels(&editor), [0, 3, 0]);
}

#[test]
fn tab_changes_the_level_only_at_the_start_of_a_list_paragraph() {
    let _scope = test_scope();
    let editor = editor("one\ntwo");
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    assert!(!editor.tab_changes_list_level());
    editor
        .apply_paragraph_format(&ParagraphFormat { list_type: Some(MarkerType::Bullet), ..Default::default() })
        .unwrap();
    assert!(editor.tab_changes_list_level());
    editor.set_selection(EditorSelection::caret(5)).unwrap();
    assert!(!editor.tab_changes_list_level());
    // A selection of the item changes its level; within its text the tab replaces it.
    editor.set_selection(EditorSelection::new(4, 6)).unwrap();
    assert!(editor.tab_changes_list_level());
    editor.set_selection(EditorSelection::new(5, 6)).unwrap();
    assert!(!editor.tab_changes_list_level());
}

#[test]
fn enter_continues_a_list_and_ends_it_in_an_empty_item() {
    let _scope = test_scope();
    let editor = editor("one");
    editor
        .apply_paragraph_format(&ParagraphFormat {
            list_type: Some(MarkerType::Bullet),
            list_level_index: Some(2),
            ..Default::default()
        })
        .unwrap();
    editor.set_selection(EditorSelection::caret(3)).unwrap();
    // Enter in a list item that holds text continues the list.
    assert!(!editor.end_list_on_enter().unwrap());
    editor.insert_text("\n", EditKind::Typing).unwrap();
    assert_eq!(list_types(&editor), [MarkerType::Bullet, MarkerType::Bullet]);
    assert_eq!(list_levels(&editor), [2, 2]);
    // Enter in an empty nested item moves it one level up, then out of the list.
    assert!(editor.end_list_on_enter().unwrap());
    assert_eq!(list_levels(&editor), [2, 1]);
    assert!(editor.end_list_on_enter().unwrap());
    assert_eq!(list_types(&editor), [MarkerType::Bullet, MarkerType::None]);
    assert!(!editor.end_list_on_enter().unwrap());
    assert!(editor.undo().unwrap());
    assert_eq!(list_types(&editor), [MarkerType::Bullet, MarkerType::Bullet]);
}

#[test]
fn backspace_at_the_start_of_a_list_item_removes_its_marker() {
    let _scope = test_scope();
    let editor = editor("one\ntwo");
    editor.set_selection(EditorSelection::new(0, 7)).unwrap();
    editor
        .apply_paragraph_format(&ParagraphFormat { list_type: Some(MarkerType::Arabic), ..Default::default() })
        .unwrap();
    editor.set_selection(EditorSelection::caret(5)).unwrap();
    assert!(!editor.remove_list_on_backspace().unwrap());
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    assert!(editor.remove_list_on_backspace().unwrap());
    assert_eq!(list_types(&editor), [MarkerType::Arabic, MarkerType::None]);
    assert_eq!(editor.document().text(), "one\ntwo");
    // Without the marker Backspace deletes again.
    assert!(!editor.remove_list_on_backspace().unwrap());
    assert!(editor.undo().unwrap());
    assert_eq!(list_types(&editor), [MarkerType::Arabic, MarkerType::Arabic]);
}
