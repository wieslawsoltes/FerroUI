// Independent contract regressions for the pinned Microsoft.UI.Text paragraph
// APIs and the RTF 1.9.1 paragraph spacing controls. These are not upstream tests.
use super::{LineSpacingRule, Paragraph, ParagraphFormat, RichTextDocument};
use crate::rich_text::{RtfError, RtfReader, RtfWriter};
use crate::test_support::test_scope;
use crate::RichTextBlock;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::{Rect, Size};

#[test]
fn rich_paragraph_line_spacing_uses_native_values_and_points() {
    assert_eq!(LineSpacingRule::Undefined as i32, 0);
    assert_eq!(LineSpacingRule::Single as i32, 1);
    assert_eq!(LineSpacingRule::OneAndHalf as i32, 2);
    assert_eq!(LineSpacingRule::Double as i32, 3);
    assert_eq!(LineSpacingRule::AtLeast as i32, 4);
    assert_eq!(LineSpacingRule::Exactly as i32, 5);
    assert_eq!(LineSpacingRule::Multiple as i32, 6);
    assert_eq!(LineSpacingRule::Percent as i32, 7);
    let mut format = ParagraphFormat::default();
    format
        .set_line_spacing(LineSpacingRule::Exactly, 18.0)
        .unwrap();
    assert_eq!(format.line_spacing, Some(24.0));
    assert_eq!(format.get_line_spacing(), (LineSpacingRule::Exactly, 18.0));
    format
        .set_line_spacing(LineSpacingRule::Multiple, 1.25)
        .unwrap();
    assert_eq!(format.get_line_spacing(), (LineSpacingRule::Multiple, 1.25));
    let before = format.clone();
    for (rule, value) in [
        (LineSpacingRule::Percent, 100.0),
        (LineSpacingRule::Multiple, 0.0),
        (LineSpacingRule::Exactly, f64::INFINITY),
        (LineSpacingRule::Single, f64::NAN),
        (LineSpacingRule::AtLeast, -1.0),
    ] {
        assert!(format.set_line_spacing(rule, value).is_err());
        assert_eq!(format, before, "a rejected patch must be atomic");
    }
}

#[test]
fn rich_paragraph_effective_format_and_snapshot_keep_spacing() {
    let _scope = test_scope();
    let paragraph = Paragraph::with_text("line");
    let effective = ParagraphFormat::effective(&paragraph);
    paragraph.set_font_size(paragraph.font_size());
    paragraph.set_text_alignment(paragraph.text_alignment());
    paragraph.set_line_spacing(1.0);
    paragraph.set_line_spacing_rule(LineSpacingRule::Undefined);
    assert_eq!(effective, ParagraphFormat::effective(&paragraph));

    let document = RichTextDocument::new();
    document.blocks().add(paragraph.clone());
    let mut format = ParagraphFormat::default();
    format
        .set_line_spacing(LineSpacingRule::Multiple, 1.5)
        .unwrap();
    format.try_apply_to(&paragraph).unwrap();
    let original = document.snapshot();
    document.replace_text(2..2, "\n").unwrap();
    assert!(document
        .snapshot()
        .paragraphs
        .iter()
        .all(|p| p.format.get_line_spacing() == (LineSpacingRule::Multiple, 1.5)));
    document.restore(&original).unwrap();
    assert_eq!(document.snapshot(), original);
}

#[test]
fn rich_paragraph_spacing_rules_drive_native_line_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (rule, spacing, factor) in [
        (LineSpacingRule::Single, 0.0, 1.0),
        (LineSpacingRule::OneAndHalf, 0.0, 1.5),
        (LineSpacingRule::Double, 0.0, 2.0),
        (LineSpacingRule::Multiple, 1.25, 1.25),
    ] {
        let view = RichTextBlock::new();
        let paragraph = Paragraph::with_text("first\nsecond");
        paragraph.set_line_spacing(spacing);
        paragraph.set_line_spacing_rule(rule);
        view.blocks().add(paragraph);
        view.measure(Size::new(300.0, f64::INFINITY));
        view.arrange(Rect::new(0.0, 0.0, 300.0, 300.0));
        let layout = view.page_layout();
        assert_eq!(layout.lines.len(), 2, "{rule:?}");
        let first = &layout.lines[0];
        assert!(
            (first.advance - first.line.height() * factor).abs() < 1e-8,
            "{rule:?}"
        );
        assert!((layout.lines[1].origin.y - first.origin.y - first.advance).abs() < 1e-8);
    }
}

#[test]
fn rich_paragraph_exact_spacing_controls_height_and_invalidates_overflow() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("first\nsecond\nthird");
    paragraph.set_line_spacing(8.0);
    paragraph.set_line_spacing_rule(LineSpacingRule::Exactly);
    view.blocks().add(paragraph.clone());
    view.measure(Size::new(300.0, 20.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 20.0));
    assert_eq!(view.page_layout().lines.len(), 2);
    assert_eq!(view.page_layout().lines[0].advance, 8.0);
    assert!(view.has_overflow_content());
    paragraph.set_line_spacing(6.0);
    view.measure(Size::new(300.0, 20.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 20.0));
    assert_eq!(view.page_layout().lines.len(), 3);
    assert!(!view.has_overflow_content());
}

#[test]
fn rich_rtf_multiple_spacing_accepts_both_control_orders() {
    for input in [
        r"{\rtf1\sl360\slmult1 first\par second}",
        r"{\rtf1\slmult1\sl360 first\par second}",
    ] {
        let document = RtfReader::read_str(input).unwrap();
        assert_eq!(document.text(), "first\nsecond");
        for paragraph in &document.paragraphs {
            assert_eq!(
                paragraph.format.get_line_spacing(),
                (LineSpacingRule::Multiple, 1.5)
            );
        }
        let output = RtfWriter::write(&document).unwrap();
        assert!(output.contains(r"\sl360\slmult1"));
        let imported = RtfReader::read_str(&output).unwrap();
        assert_eq!(imported.text(), document.text());
        assert!(imported
            .paragraphs
            .iter()
            .all(|p| p.format.get_line_spacing() == (LineSpacingRule::Multiple, 1.5)));
    }
}

#[test]
fn rich_rtf_exact_minimum_and_reset_spacing_are_preserved() {
    let input = r"{\rtf1\sl-360\slmult0 exact\par\sl480 minimum\par\pard natural}";
    let document = RtfReader::read_str(input).unwrap();
    assert_eq!(
        document.paragraphs[0].format.get_line_spacing(),
        (LineSpacingRule::Exactly, 18.0)
    );
    assert_eq!(
        document.paragraphs[1].format.get_line_spacing(),
        (LineSpacingRule::AtLeast, 24.0)
    );
    assert_eq!(
        document.paragraphs[2].format.get_line_spacing().0,
        LineSpacingRule::Undefined
    );
    let output = RtfWriter::write(&document).unwrap();
    let imported = RtfReader::read_str(&output).unwrap();
    for (a, b) in imported.paragraphs.iter().zip(&document.paragraphs) {
        assert_eq!(a.format.get_line_spacing(), b.format.get_line_spacing());
    }
}

#[test]
fn rich_rtf_rejects_invalid_spacing_and_unrepresentable_metrics() {
    for input in [
        r"{\rtf1\slmult2 text}",
        r"{\rtf1\sl-240\slmult1 text}",
        r"{\rtf1\slmult1\sl-240 text}",
        r"{\rtf1\sb-20 text}",
    ] {
        assert!(
            matches!(
                RtfReader::read_str(input),
                Err(RtfError::InvalidParameter(_))
            ),
            "{input}"
        );
    }
}

#[test]
fn rich_paragraph_indents_use_points_and_preserve_hanging_values() {
    let _scope = test_scope();
    let mut format = ParagraphFormat::default();
    format.set_indents(-9.0, 18.0, -3.0).unwrap();
    assert_eq!(format.text_indent, Some(-12.0));
    assert_eq!(format.left_indent, Some(24.0));
    assert_eq!(format.right_indent, Some(-4.0));
    assert_eq!(format.get_indents(), (-9.0, 18.0, -3.0));
    let paragraph = Paragraph::with_text("hanging paragraph");
    format.try_apply_to(&paragraph).unwrap();
    assert_eq!(
        ParagraphFormat::from_paragraph(&paragraph).get_indents(),
        format.get_indents()
    );
    let before = format.clone();
    assert!(format.set_indents(2.0, f64::INFINITY, 1.0).is_err());
    assert_eq!(format, before);
}

#[test]
fn rich_paragraph_indentation_affects_line_origin_and_invalidates_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("first\nsecond");
    view.blocks().add(paragraph.clone());
    view.measure(Size::new(300.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
    let original_x = view.page_layout().lines[0].origin.x;
    paragraph.set_left_indent(-10.0);
    view.measure(Size::new(300.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100.0));
    assert_eq!(view.page_layout().lines[0].origin.x, original_x - 10.0);
    assert_eq!(view.page_layout().lines[1].origin.x, original_x - 10.0);
}

#[test]
fn rich_rtf_negative_indents_round_trip_without_negative_margins() {
    let document = RtfReader::read_str(r"{\rtf1\fi-180\li360\ri-60 text}").unwrap();
    let format = &document.paragraphs[0].format;
    assert_eq!(format.get_indents(), (-9.0, 18.0, -3.0));
    assert!(format.margin.is_none());
    let imported = RtfReader::read_str(&RtfWriter::write(&document).unwrap()).unwrap();
    assert_eq!(
        imported.paragraphs[0].format.get_indents(),
        format.get_indents()
    );
    assert_eq!(imported.text(), document.text());
}

#[test]
fn rich_paragraph_spacing_patch_validates_combined_state_before_mutating() {
    let _scope = test_scope();
    let paragraph = Paragraph::with_text("line");
    paragraph.set_line_spacing(0.0);
    let before = ParagraphFormat::from_paragraph(&paragraph);
    let format = ParagraphFormat {
        line_spacing_rule: Some(LineSpacingRule::Exactly),
        ..ParagraphFormat::default()
    };
    assert!(format.try_apply_to(&paragraph).is_err());
    assert_eq!(ParagraphFormat::from_paragraph(&paragraph), before);
}
