use std::rc::Rc;
use super::{GenericTextParagraphProperties, TextLayout, TextLayoutOptions, TextParagraphStyle, TextParagraphStyles};
use super::testing::{run_properties, SingleBufferTextSource, TextTestScope};
use crate::media::{Brushes, CharacterHit, FlowDirection, TextAlignment, TextLineBounds, TextWrapping, Typeface};
use crate::utilities::ValueSpan;
use crate::Point;

fn layout(text: &str, styles: Vec<ValueSpan<TextParagraphStyle>>, width: f64) -> TextLayout {
    TextLayout::new_with_paragraph_styles(text, Typeface::default_typeface(), TextLayoutOptions {
        font_size: 12.0, foreground: Some(Brushes::black()), max_width: width,
        text_wrapping: TextWrapping::Wrap, line_height: 20.0, ..Default::default()
    }, Some(Rc::from(styles)))
}

#[test]
fn paragraph_offsets_direction_and_spacing_share_hit_and_selection_geometry() {
    let _scope = TextTestScope::new();
    let text = layout("AB\nאב", vec![
        ValueSpan::new(0, 3, TextParagraphStyle { indent: 4.0, space_before: 3.0, space_after: 5.0, ..Default::default() }),
        ValueSpan::new(3, 2, TextParagraphStyle { flow_direction: Some(FlowDirection::RightToLeft),
            text_alignment: Some(TextAlignment::Right), line_height: Some(30.0), indent: 8.0,
            space_before: 7.0, space_after: 9.0, ..Default::default() }),
    ], 100.0);
    assert_eq!(text.text_lines().len(), 2);
    assert_eq!(text.line_origin(0), Point::new(4.0, 3.0));
    assert_eq!(text.line_origin(1), Point::new(0.0, 35.0));
    assert_eq!(text.height(), 74.0);
    assert_eq!(text.line_flow_direction(0), FlowDirection::LeftToRight);
    assert_eq!(text.line_flow_direction(1), FlowDirection::RightToLeft);
    assert_eq!(text.baseline(), text.text_lines()[0].baseline() + 3.0);
    for (index, range) in [(0, (0,2)), (1, (3,2))] {
        let selected = text.hit_test_text_range(range.0, range.1);
        assert!(!selected.is_empty());
        for rect in selected {
            assert_eq!(rect.y, text.line_origin(index).y);
            assert!(text.hit_test_point(Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)).is_inside());
        }
        let line = &text.text_lines()[index];
        let x = text.line_origin(index).x + line.get_distance_from_character_hit(CharacterHit::new(range.0));
        assert_eq!(text.hit_test_text_position(range.0).x, x);
        assert_eq!(text.hit_test_text_position(range.0).y, text.line_origin(index).y);
    }
    assert!(!text.hit_test_point(Point::new(5.0, 29.0)).is_inside(), "Paragraph spacing is not glyph content");
}

#[test]
fn paragraph_indent_only_reduces_first_wrapped_line_and_does_not_leak_to_next_paragraph() {
    let _scope = TextTestScope::new();
    let value = "aaaa aaaa aaaa\nBB";
    let text = layout(value, vec![ValueSpan::new(0, 15, TextParagraphStyle {
        indent: 24.0, line_height: Some(22.0), ..Default::default()
    })], 60.0);
    assert!(text.text_lines().len() >= 3);
    assert_eq!(text.line_origin(0).x, 24.0);
    for index in 1..text.text_lines().len() { assert_eq!(text.line_origin(index).x, 0.0); }
    let last = text.text_lines().last().unwrap();
    assert_eq!(last.first_text_source_index(), 15);
    assert_eq!(last.height(), 20.0);
    assert_eq!(text.text_lines()[0].height(), 22.0);
    assert_eq!(text.text_lines().iter().map(|line| line.length()).sum::<i32>(), value.encode_utf16().count() as i32);
}

#[test]
fn trailing_empty_paragraph_has_its_own_caret_height_indent_and_spacing() {
    let _scope = TextTestScope::new();
    let text = layout("A🦀\r\n", vec![ValueSpan::new(5, 0, TextParagraphStyle {
        line_height: Some(34.0), indent: 6.0, space_before: 5.0, space_after: 7.0, ..Default::default()
    })], 200.0);
    assert_eq!(text.text_lines().len(), 2);
    assert_eq!(text.line_origin(1), Point::new(6.0, 25.0));
    assert_eq!(text.height(), 66.0);
    assert_eq!(text.hit_test_text_position(5).height, 34.0);
    assert_eq!(text.hit_test_text_position(5).x, 6.0);
}

#[test]
fn absent_paragraph_overrides_preserve_legacy_layout_lines_and_all_hit_positions() {
    let _scope = TextTestScope::new();
    let value = "A🦀 BC\r\nאב cd\n";
    let options = TextLayoutOptions { max_width: 48.0, text_wrapping: TextWrapping::Wrap, ..Default::default() };
    let legacy = TextLayout::new(value, Typeface::default_typeface(), options.clone());
    let paragraphs = TextLayout::new_with_paragraph_styles(value, Typeface::default_typeface(), options,
        Some(TextParagraphStyles::from([])));
    assert_eq!(legacy.height(), paragraphs.height());
    assert_eq!(legacy.width_including_trailing_whitespace(), paragraphs.width_including_trailing_whitespace());
    assert_eq!(legacy.text_lines().len(), paragraphs.text_lines().len());
    for at in 0..=value.encode_utf16().count() as i32 {
        assert_eq!(legacy.hit_test_text_position(at), paragraphs.hit_test_text_position(at));
    }
}

#[test]
fn paragraph_styles_inherit_trimmed_line_bounds_across_wraps_breaks_and_empty_paragraphs() {
    let _scope = TextTestScope::new();
    let value = "Hxg Hxg\nאבg\n";
    let length = value.encode_utf16().count() as i32;
    let styles = TextParagraphStyles::from([
        ValueSpan::new(0, 8, TextParagraphStyle::default()),
        ValueSpan::new(8, 4, TextParagraphStyle::default()),
        ValueSpan::new(length, 0, TextParagraphStyle::default()),
    ]);
    let build = |bounds, styles| {
        let runs = run_properties(12.0);
        let properties = GenericTextParagraphProperties::with_all(
            FlowDirection::LeftToRight,
            TextAlignment::Left,
            true,
            false,
            runs.clone(),
            TextWrapping::Wrap,
            f64::NAN,
            0.0,
            0.0,
        );
        properties.set_text_line_bounds(bounds);
        properties.set_line_spacing(1.25);
        TextLayout::from_text_source_with_paragraph_styles(
            Rc::new(SingleBufferTextSource::new(value, runs)),
            Rc::new(properties),
            None,
            30.0,
            f64::INFINITY,
            0,
            None,
            styles,
        )
    };

    // The deterministic 12-DIP test font has ascent 9.6, descent 2.4,
    // line gap 1.2 and cap height 6.72. Its six-DIP advance wraps the first
    // paragraph once. A default style must preserve all four inherited boxes,
    // including the otherwise-empty final paragraph and their caret geometry.
    for (bounds, baseline, height) in [
        (TextLineBounds::Full, 10.2, 13.2),
        (TextLineBounds::TrimToCapHeight, 6.72, 9.72),
        (TextLineBounds::TrimToBaseline, 10.2, 10.2),
        (TextLineBounds::Tight, 6.72, 6.72),
    ] {
        let inherited = build(bounds, None);
        let styled = build(bounds, Some(styles.clone()));
        assert_eq!(inherited.text_lines().len(), 4, "{bounds:?}");
        assert_eq!(styled.text_lines().len(), inherited.text_lines().len());
        assert_eq!(styled.height(), inherited.height(), "{bounds:?}");
        assert_eq!(styled.baseline(), inherited.baseline(), "{bounds:?}");
        assert_eq!(
            styled.width_including_trailing_whitespace(),
            inherited.width_including_trailing_whitespace(),
            "{bounds:?}"
        );
        for (index, (actual, expected)) in styled
            .text_lines()
            .iter()
            .zip(inherited.text_lines())
            .enumerate()
        {
            assert_eq!(
                actual.first_text_source_index(),
                expected.first_text_source_index()
            );
            assert_eq!(actual.length(), expected.length());
            assert_eq!(
                actual.height(),
                expected.height(),
                "{bounds:?} line {index}"
            );
            assert_eq!(
                actual.baseline(),
                expected.baseline(),
                "{bounds:?} line {index}"
            );
            assert_eq!(styled.line_origin(index), inherited.line_origin(index));
            assert!(
                (actual.baseline() - baseline).abs() < 1e-9,
                "{bounds:?} line {index}"
            );
            assert!(
                (actual.height() - height - 1.25).abs() < 1e-9,
                "{bounds:?} line {index}"
            );
        }
        for at in 0..=length {
            assert_eq!(
                styled.hit_test_text_position(at),
                inherited.hit_test_text_position(at),
                "{bounds:?} caret {at}"
            );
        }
        assert_eq!(
            styled.hit_test_text_range(0, length),
            inherited.hit_test_text_range(0, length),
            "{bounds:?} selection"
        );
    }
}
