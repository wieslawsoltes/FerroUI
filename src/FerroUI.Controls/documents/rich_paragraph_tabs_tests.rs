// Source-derived contracts and independent paragraph/layout/RTF regressions.
use super::{
    DocumentSnapshot, Paragraph, ParagraphFormat, RichTextDocument, TabAlignment, TabLeader,
    TextTabStop, TextTabStops,
};
use crate::test_support::test_scope;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FlowDirection, TextAlignment};
use ferroui_base::{Rect, Size};
#[allow(unused_imports)]
use crate::{RichTextBlock, RichTextBlockOverflow};

#[test]
fn rich_paragraph_tabs_use_native_values_points_and_sorted_replacement() {
    assert_eq!(
        [
            TabAlignment::Left as i32,
            TabAlignment::Center as i32,
            TabAlignment::Right as i32,
            TabAlignment::Decimal as i32,
            TabAlignment::Bar as i32
        ],
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        [
            TabLeader::Spaces as i32,
            TabLeader::Dots as i32,
            TabLeader::Dashes as i32,
            TabLeader::Lines as i32,
            TabLeader::ThickLines as i32,
            TabLeader::Equals as i32
        ],
        [0, 1, 2, 3, 4, 5]
    );
    let mut format = ParagraphFormat::default();
    format
        .add_tab(72.0, TabAlignment::Right, TabLeader::Dots)
        .unwrap();
    format
        .add_tab(36.0, TabAlignment::Center, TabLeader::Dashes)
        .unwrap();
    format
        .add_tab(72.0, TabAlignment::Decimal, TabLeader::Equals)
        .unwrap();
    assert_eq!(format.tab_count(), 2);
    assert_eq!(
        format.get_tab(0).unwrap(),
        (36.0, TabAlignment::Center, TabLeader::Dashes)
    );
    assert_eq!(
        format.get_tab(1).unwrap(),
        (72.0, TabAlignment::Decimal, TabLeader::Equals)
    );
    assert_eq!(
        format.tab_stops.as_ref().unwrap().as_slice()[0].position,
        48.0
    );
    format.delete_tab(36.0).unwrap();
    format.delete_tab(36.0).unwrap();
    assert_eq!(format.tab_count(), 1);
    format.clear_all_tabs();
    assert_eq!(format.tab_count(), 0);
    assert!(format.get_tab(0).is_err());
}

#[test]
fn rich_paragraph_tabs_reject_invalid_edits_atomically_and_enforce_native_limit() {
    let mut format = ParagraphFormat::default();
    for i in 1..=63 {
        format
            .add_tab(i as f64, TabAlignment::Left, TabLeader::Spaces)
            .unwrap();
    }
    let original = format.clone();
    for value in [-1.0, f64::INFINITY, f64::NAN, f64::MAX] {
        assert!(format
            .add_tab(value, TabAlignment::Left, TabLeader::Spaces)
            .is_err());
        assert_eq!(format, original);
    }
    assert!(format
        .add_tab(64.0, TabAlignment::Left, TabLeader::Spaces)
        .is_err());
    assert_eq!(format, original);
    format
        .add_tab(63.0, TabAlignment::Right, TabLeader::Dots)
        .unwrap();
    assert_eq!(format.tab_count(), 63);
    let clone = format.clone();
    format.clear_all_tabs();
    assert_eq!(
        clone.tab_count(),
        63,
        "snapshot clones must remain immutable"
    );
}

#[test]
fn rich_paragraph_tabs_layout_positions_invalidate_with_the_property() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.set_font_size(12.0);
    let paragraph = Paragraph::with_text("a\tb");
    paragraph.set_tab_stops(
        TextTabStops::new([TextTabStop::new(
            80.0,
            TabAlignment::Left,
            TabLeader::Spaces,
        )])
        .unwrap(),
    );
    view.blocks().add(paragraph.clone());
    view.measure(Size::new(300.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
    let x = view.get_rect_from_character_index(2).x;
    assert!((x - 80.0).abs() < 0.01, "{x}");
    let revision = view.document().revision();
    paragraph.set_tab_stops(
        TextTabStops::new([TextTabStop::new(
            120.0,
            TabAlignment::Left,
            TabLeader::Spaces,
        )])
        .unwrap(),
    );
    assert!(view.document().revision() > revision);
    view.measure(Size::new(300.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
    assert!((view.get_rect_from_character_index(2).x - 120.0).abs() < 0.01);
}

#[test]
fn rich_paragraph_tabs_rtl_indent_keeps_the_page_origin_and_first_line_leading_edge() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.set_font_size(12.0);
    let paragraph = Paragraph::with_text("אב\t123");
    paragraph.set_flow_direction(FlowDirection::RightToLeft);
    paragraph.set_text_alignment(TextAlignment::Start);
    paragraph.set_text_indent(20.0);
    paragraph.set_left_indent(10.0);
    paragraph.set_right_indent(10.0);
    paragraph.set_tab_stops(
        TextTabStops::new([TextTabStop::new(
            60.0,
            TabAlignment::Right,
            TabLeader::Spaces,
        )])
        .unwrap(),
    );
    view.blocks().add(paragraph);
    view.measure(Size::new(300.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
    assert!((view.get_rect_from_character_index(0).x - 270.0).abs() < 0.01);
    assert!((view.get_rect_from_character_index(3).x - 222.0).abs() < 0.01);
}
