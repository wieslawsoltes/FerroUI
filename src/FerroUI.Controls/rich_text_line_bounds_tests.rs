//! Rich views consume the framework's TextLineBounds metrics. The native
//! stacking transform is tested separately from the shared formatter's
//! retained half-gap and fallback-font policy.

use crate::documents::{InlineUIContainer, LineSpacingRule, LineStackingStrategy, Paragraph, Run};
use crate::test_support::test_scope;
use crate::{Border, RichTextBlock, RichTextBlockOverflow, TextBlock};
use ferroui_base::media::text_formatting::testing::{
    DrawCall, RecordingDrawingSink, TestFont, TextTestScope,
};
use ferroui_base::media::{Brushes, TextLineBounds, TextTrimming, TextWrapping};
use ferroui_base::metadata::{from_markup_value, into_markup_value, MarkupType};
use ferroui_base::{Point, Rect, Ref, Size, StaticType};
use std::rc::Rc;

const MODES: [(TextLineBounds, f64, f64); 4] = [
    (TextLineBounds::Full, 22.0, 17.0),
    (TextLineBounds::TrimToCapHeight, 16.2, 11.2),
    (TextLineBounds::TrimToBaseline, 17.0, 17.0),
    (TextLineBounds::Tight, 11.2, 11.2),
];

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

fn rich(text: &str, bounds: TextLineBounds) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.set_font_size(20.0);
    view.set_foreground(Some(Brushes::black()));
    view.set_text_wrapping(TextWrapping::NoWrap);
    view.set_text_line_bounds(bounds);
    view.blocks().add(Paragraph::with_text(text));
    view
}

fn measure(view: &RichTextBlock, height: f64) {
    view.measure(Size::new(300.0, height));
    view.arrange(Rect::new(0.0, 0.0, 300.0, height));
}

#[test]
fn rich_text_line_bounds_all_modes_reach_each_line_and_empty_document() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (mode, height, baseline) in MODES {
        let view = rich("Hxg\nあg", mode);
        measure(&view, 200.0);
        let layout = view.page_layout();
        assert_eq!(layout.source.text_line_bounds, mode);
        assert_eq!(layout.lines.len(), 2);
        close(layout.size.height, 2.0 * height);
        for (index, line) in layout.lines.iter().enumerate() {
            close(line.line.height(), height);
            close(line.line.baseline(), baseline);
            close(line.allocation_y, index as f64 * height);
            close(line.advance, height);
        }

        view.blocks().clear();
        measure(&view, 200.0);
        let empty = view.page_layout();
        assert_eq!(empty.lines.len(), 1);
        close(empty.lines[0].line.height(), height);
        close(empty.lines[0].line.baseline(), baseline);
        close(view.get_rect_from_character_index(0).height, height);
        assert_eq!(view.content_start(), 0);
        assert_eq!(view.content_end(), 0);
    }
}

#[test]
fn rich_text_line_bounds_inheritance_local_override_and_clear_rebuild_the_source() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let host = Border::new();
    let view = rich("Hxg", TextLineBounds::Full);
    view.clear_value(RichTextBlock::text_line_bounds_property());
    host.set_child(Some(view.clone().upcast()));
    measure(&view, 200.0);
    let full = view.page_layout();
    let document = view.document().snapshot();

    TextBlock::set_text_line_bounds_on(&host, TextLineBounds::Tight);
    assert_eq!(view.text_line_bounds(), TextLineBounds::Tight);
    assert!(!view.is_measure_valid());
    assert!(!view.clip_to_bounds());
    measure(&view, 200.0);
    let tight = view.page_layout();
    assert!(!Rc::ptr_eq(&full.source, &tight.source));
    close(tight.size.height, 11.2);

    view.set_text_line_bounds(TextLineBounds::Full);
    measure(&view, 200.0);
    close(view.page_layout().size.height, 22.0);
    assert!(view.clip_to_bounds());

    view.clear_value(RichTextBlock::text_line_bounds_property());
    measure(&view, 200.0);
    close(view.page_layout().size.height, 11.2);
    assert!(!view.clip_to_bounds());
    assert_eq!(view.document().snapshot(), document);
    assert!(std::ptr::eq(
        RichTextBlock::text_line_bounds_property().as_property(),
        TextBlock::text_line_bounds_property().as_property(),
    ));
}

#[test]
fn rich_text_line_bounds_markup_enum_and_property_deliver_real_layout_changes() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    crate::register_types();
    ferroui_base::register_types();
    let metadata = RichTextBlock::TYPE.markup().unwrap();
    let property = metadata.find_property("TextLineBounds").unwrap();
    let value_type = MarkupType::find("FerroUI.Media", "TextLineBounds").unwrap();
    let view = rich("Hxg", TextLineBounds::Full);
    for (name, expected, height) in [
        ("Full", TextLineBounds::Full, 22.0),
        ("TrimToCapHeight", TextLineBounds::TrimToCapHeight, 16.2),
        ("TrimToBaseline", TextLineBounds::TrimToBaseline, 17.0),
        ("Tight", TextLineBounds::Tight, 11.2),
    ] {
        let member = value_type.find_enum_member(name, false).unwrap();
        assert_eq!(member.value, expected as i64);
        let parsed = Some((member.get)());
        (property.set.unwrap())(&[into_markup_value(view.clone()), parsed]).unwrap();
        let value = (property.get.unwrap())(&[into_markup_value(view.clone())]).unwrap();
        assert_eq!(from_markup_value::<TextLineBounds>(&value), Some(expected));
        measure(&view, 200.0);
        close(view.page_layout().size.height, height);
    }
    assert_eq!(TextLineBounds::Full as i32, 0);
    assert_eq!(TextLineBounds::TrimToCapHeight as i32, 1);
    assert_eq!(TextLineBounds::TrimToBaseline as i32, 2);
    assert_eq!(TextLineBounds::Tight as i32, 3);
}

#[test]
fn rich_text_line_bounds_stacking_uses_family_mode_ratio_on_framework_lines() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    // At em 20, the shared formatter produces cap-trimmed height 16.2.
    // The pinned family stacking ratio is 560 / (560 + 200 + 100) = 28/43.
    // Baseline and Tight bounds have ratio 1. These allocation results are
    // intentionally distinct from a native formatter/rasterization claim.
    let cases = [
        (
            TextLineBounds::TrimToCapHeight,
            6.0,
            -285.6 / 43.0,
            543.6 / 43.0,
            411.0 / 43.0,
        ),
        (
            TextLineBounds::TrimToCapHeight,
            26.0,
            274.4 / 43.0,
            843.6 / 43.0,
            971.0 / 43.0,
        ),
        (TextLineBounds::TrimToBaseline, 6.0, -11.0, 17.0, 6.0),
        (TextLineBounds::TrimToBaseline, 26.0, 9.0, 17.0, 26.0),
        (TextLineBounds::Tight, 6.0, -5.2, 11.2, 6.0),
        (TextLineBounds::Tight, 26.0, 14.8, 11.2, 26.0),
    ];
    for (mode, requested, offset, first, last) in cases {
        let view = rich("a\nb\nc", mode);
        view.set_line_height(requested);
        view.set_line_stacking_strategy(LineStackingStrategy::BlockLineHeight);
        measure(&view, 200.0);
        for (index, line) in view.page_layout().lines.iter().enumerate() {
            close(line.advance, requested);
            close(line.origin.y, index as f64 * requested + offset);
        }

        view.set_line_stacking_strategy(LineStackingStrategy::BaselineToBaseline);
        measure(&view, 200.0);
        let layout = view.page_layout();
        assert_eq!(layout.lines.len(), 3);
        close(layout.lines[0].advance, first);
        close(layout.lines[0].origin.y, 0.0);
        close(layout.lines[1].advance, requested);
        close(layout.lines[1].origin.y, first + offset);
        close(layout.lines[2].advance, last);
        close(layout.lines[2].origin.y, first + requested + offset);
        close(layout.size.height, first + requested + last);
    }
}

#[test]
fn rich_text_line_bounds_draw_origin_and_range_caret_boxes_share_the_formatted_line() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (mode, height, baseline) in MODES {
        let view = rich("H\ng", mode);
        measure(&view, 200.0);
        let layout = view.page_layout();
        let mut sink = RecordingDrawingSink::new();
        layout.draw(&mut sink, Point::new(3.0, 7.0));
        for (index, letter) in ['H', 'g'].into_iter().enumerate() {
            let entry = &layout.lines[index];
            let calls = sink.calls();
            let origin = calls
                .iter()
                .find_map(|call| match call {
                    DrawCall::GlyphRun { text, origin } if text.contains(letter) => Some(*origin),
                    _ => None,
                })
                .unwrap();
            // Each shaped run retains its ordinary baseline 17. Only its
            // placement within the line changes when the line is bounded.
            close(origin.y, 7.0 + index as f64 * height + baseline - 17.0);
            close(origin.x, 3.0);
            let caret = view.get_rect_from_character_index(entry.start());
            let range = view.get_rects_for_range(entry.start()..entry.start() + 1);
            assert_eq!(range.len(), 1);
            close(caret.y, index as f64 * height);
            close(caret.height, height);
            close(range[0].y, caret.y);
            close(range[0].height, caret.height);
            close(range[0].width, 10.0);
            assert_eq!(
                view.hit_test_text_position(Point::new(0.0, caret.y + height / 2.0)),
                entry.start()
            );
        }
        assert_eq!(sink.transform_depth(), 0);
    }
}

#[test]
fn rich_text_line_bounds_overflow_uses_head_policy_and_reflows_without_document_edits() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let head = rich("a\nb\nc", TextLineBounds::Full);
    let overflow = RichTextBlockOverflow::new();
    // The native overflow has no independent line-bounds property. The rich
    // base class exposes one, but connected pages use the source owner's mode.
    overflow.set_text_line_bounds(TextLineBounds::TrimToBaseline);
    head.set_overflow_content_target(Some(overflow.clone()));
    let document = head.document().snapshot();
    for (mode, height, baseline) in MODES {
        head.set_text_line_bounds(mode);
        measure(&head, height * 1.1);
        measure(&overflow, height * 2.1);
        let first = head.page_layout();
        let second = overflow.page_layout();
        assert!(Rc::ptr_eq(&first.source, &second.source));
        assert_eq!(first.lines.len(), 1);
        assert_eq!(second.lines.len(), 2);
        assert_eq!(second.source.text_line_bounds, mode);
        assert_eq!(head.content_end(), overflow.content_start());
        assert_eq!(overflow.content_end(), head.document().text_length());
        for entry in &second.lines {
            close(entry.line.height(), height);
            close(entry.line.baseline(), baseline);
            close(entry.advance, height);
        }
        assert_eq!(head.clip_to_bounds(), mode == TextLineBounds::Full);
        assert_eq!(overflow.clip_to_bounds(), mode == TextLineBounds::Full);
    }
    assert_eq!(head.document().snapshot(), document);
    head.set_overflow_content_target(None);
    assert!(!overflow.clip_to_bounds());
    overflow.set_text_line_bounds(TextLineBounds::Full);
    assert!(overflow.clip_to_bounds());
}

#[test]
fn rich_text_line_bounds_clipping_keeps_local_choice_and_unlinked_pages() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let head = rich("a\nb\nc", TextLineBounds::Tight);
    let second = RichTextBlockOverflow::new();
    let third = RichTextBlockOverflow::new();
    second.set_overflow_content_target(Some(third.clone()));
    head.set_overflow_content_target(Some(second.clone()));
    assert!(!head.clip_to_bounds());
    assert!(!second.clip_to_bounds());
    assert!(!third.clip_to_bounds());
    head.set_text_line_bounds(TextLineBounds::Full);
    assert!(second.clip_to_bounds());
    assert!(third.clip_to_bounds());

    second.set_clip_to_bounds(false);
    head.set_text_line_bounds(TextLineBounds::Tight);
    head.set_overflow_content_target(None);
    assert!(!second.clip_to_bounds());
    assert!(third.clip_to_bounds());
    second.set_clip_to_bounds(true);
    assert!(second.clip_to_bounds());
    second.set_text_line_bounds(TextLineBounds::Tight);
    assert!(!second.clip_to_bounds());
    assert!(!third.clip_to_bounds());
    second.set_text_line_bounds(TextLineBounds::Full);
    assert!(second.clip_to_bounds());
    assert!(third.clip_to_bounds());
}

#[test]
fn rich_text_line_bounds_mixed_font_metrics_match_shared_textblock_policy() {
    let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
    let _text = TextTestScope::with_fonts(vec![
        TestFont::new("Latin")
            .with_ranges(&[(0x20, 0x7e)])
            .with_line_gap(100),
        TestFont::new("CJK")
            .with_ranges(&[(0x3000, 0x30ff)])
            .with_line_gap(500),
    ]);
    assert_eq!(
        ferroui_base::media::FontManager::current()
            .default_font_family()
            .name(),
        "Latin"
    );
    for (mode, expected_height, expected_baseline) in [
        (TextLineBounds::Full, 30.0, 21.0),
        (TextLineBounds::TrimToCapHeight, 20.2, 11.2),
        (TextLineBounds::TrimToBaseline, 21.0, 21.0),
        (TextLineBounds::Tight, 11.2, 11.2),
    ] {
        let rich = rich("Hあg", mode);
        let plain = TextBlock::new();
        plain.set_font_size(20.0);
        plain.set_text(Some("Hあg"));
        plain.set_text_line_bounds(mode);
        measure(&rich, 200.0);
        plain.measure(Size::new(300.0, 200.0));
        plain.arrange(Rect::new(0.0, 0.0, 300.0, 200.0));
        let rich_layout = rich.page_layout();
        let plain_layout = plain.text_layout();
        let plain_lines = plain_layout.text_lines();
        assert_eq!(rich_layout.lines.len(), 1);
        assert_eq!(plain_lines.len(), 1);
        close(rich_layout.lines[0].line.height(), expected_height);
        close(rich_layout.lines[0].line.baseline(), expected_baseline);
        close(plain_lines[0].height(), expected_height);
        close(plain_lines[0].baseline(), expected_baseline);
    }
}

#[test]
fn rich_text_line_bounds_explicit_spacing_remains_authoritative() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (mode, height, _) in MODES {
        let view = rich("H\ng", mode);
        view.set_line_height(6.0);
        view.set_line_stacking_strategy(LineStackingStrategy::BlockLineHeight);
        let paragraph = view.document().paragraphs().into_iter().next().unwrap();
        paragraph.set_line_spacing_rule(LineSpacingRule::Multiple);
        paragraph.set_line_spacing(2.0);
        measure(&view, 200.0);
        let layout = view.page_layout();
        for entry in &layout.lines {
            close(entry.line.height(), height);
            close(entry.advance, 2.0 * height);
        }
        let ranges = view.get_rects_for_range(0..1);
        close(ranges[0].height, height);
        close(view.get_rect_from_character_index(0).height, 2.0 * height);
        drop(layout);

        paragraph.set_line_spacing_rule(LineSpacingRule::Exactly);
        paragraph.set_line_spacing(23.0);
        measure(&view, 200.0);
        for entry in &view.page_layout().lines {
            close(entry.line.height(), 23.0);
            close(entry.advance, 23.0);
        }
    }
}

#[test]
fn rich_text_line_bounds_trimming_keeps_visible_ranges_and_embedded_ink() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("abcdefgh\nsecond", TextLineBounds::Tight);
    view.set_max_lines(1);
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    view.measure(Size::new(40.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 40.0, 100.0));
    let layout = view.page_layout();
    assert_eq!(layout.lines.len(), 1);
    assert!(view.is_text_trimmed());
    close(layout.lines[0].line.height(), 11.2);
    let visible = view.get_rects_for_range(0..1);
    assert_eq!(visible.len(), 1);
    close(visible[0].height, 11.2);
    assert!(view.get_rects_for_range(7..8).is_empty());
    drop(layout);

    let child = Border::new();
    child.set_width(30.0);
    child.set_height(40.0);
    let paragraph = Paragraph::with_text("H");
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    paragraph.add_child(Run::with_text(Some("g")));
    view.blocks().clear();
    view.blocks().add(paragraph);
    view.set_max_lines(0);
    measure(&view, 200.0);
    assert_eq!(child.bounds().size(), Size::new(30.0, 40.0));
    assert!(!view.clip_to_bounds());
    assert_eq!(view.text(), "H\u{fffc}g");
}
