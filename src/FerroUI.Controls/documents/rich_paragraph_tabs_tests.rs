// Source-derived contracts and independent paragraph/layout/RTF regressions.
use super::{
    DocumentSnapshot, Paragraph, ParagraphFormat, RichTextDocument, TabAlignment, TabLeader,
    TextTabStop, TextTabStops,
};
use crate::rich_text::{
    RichEditClipboardFormat, RichEditTextDocument, RichTextTransfer, RtfError, RtfReader,
    RtfWriter, TextGetOptions, TextSetOptions,
};
use crate::test_support::test_scope;
use crate::{RichEditBox, RichTextBlock, RichTextBlockOverflow};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FlowDirection, TextAlignment};
use ferroui_base::{Rect, Size};

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
fn rich_paragraph_tabs_survive_snapshot_split_format_queries_and_undo() {
    let _scope = test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("a\tb"));
    let mut format = ParagraphFormat::default();
    format
        .add_tab(72.0, TabAlignment::Right, TabLeader::Dots)
        .unwrap();
    document
        .document_range()
        .unwrap()
        .set_paragraph_format(&format)
        .unwrap();
    let before = document.document().snapshot();
    document.get_range(1, 1).unwrap().set_text("\n").unwrap();
    assert!(document
        .document()
        .snapshot()
        .paragraphs
        .iter()
        .all(|p| p.format.tab_count() == 1));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_document_default_tabs_apply_to_existing_and_new_paragraphs_and_undo_together() {
    let _scope = test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("a\tb\nc\td"));
    assert_eq!(document.default_tab_stop(), 36.0);
    let before = document.document().snapshot();
    document.set_default_tab_stop(54.0).unwrap();
    assert_eq!(document.default_tab_stop(), 54.0);
    assert!(document
        .document()
        .snapshot()
        .paragraphs
        .iter()
        .all(|p| p.format.default_tab_stop == Some(72.0)));
    let formatted = document.document().snapshot();
    document.get_range(1, 1).unwrap().set_text("\n").unwrap();
    assert!(document
        .document()
        .snapshot()
        .paragraphs
        .iter()
        .all(|p| p.format.default_tab_stop == Some(72.0)));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), formatted);
    assert!(document.undo().unwrap());
    assert_eq!(document.default_tab_stop(), 36.0);
    assert_eq!(document.document().snapshot(), before);
    for value in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(document.set_default_tab_stop(value).is_err());
        assert_eq!(document.document().snapshot(), before);
    }
}

#[test]
fn rich_paragraph_tabs_rtf_document_defaults_replace_and_undo_with_the_story() {
    let _scope = test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("original"));
    document.set_default_tab_stop(54.0).unwrap();
    let original = document.document().snapshot();
    document
        .set_text_with_options(TextSetOptions::FORMAT_RTF, r"{\rtf1\deftab1440 a\tab b}")
        .unwrap();
    assert_eq!(document.default_tab_stop(), 72.0);
    assert_eq!(document.document().text(), "a\tb");
    assert!(document.undo().unwrap());
    assert_eq!(document.default_tab_stop(), 54.0);
    assert_eq!(document.document().snapshot(), original);
    document
        .set_text_with_options(TextSetOptions::FORMAT_RTF, r"{\rtf1 a\tab b}")
        .unwrap();
    assert_eq!(document.default_tab_stop(), 36.0);
    assert!(document.undo().unwrap());
    assert_eq!(document.default_tab_stop(), 54.0);
    assert_eq!(document.document().snapshot(), original);
}

#[test]
fn rich_paragraph_tabs_rtf_round_trip_all_alignments_leaders_and_document_default() {
    let _scope = test_scope();
    let text = r"{\rtf1\ansi\deftab1080\pard\tx720\tqc\tldot\tx1440\tqr\tlhyph\tx2160\tqdec\tlul\tx2880\tlth\tx3600\tleq\tx4320\tb5040 a\tab b}";
    let original = RtfReader::read_str(text).unwrap();
    let tabs = original.paragraphs[0].format.tab_stops.clone().unwrap();
    assert_eq!(tabs.len(), 7);
    assert_eq!(original.paragraphs[0].format.default_tab_stop, Some(72.0));
    let encoded = RtfWriter::write(&original).unwrap();
    let restored = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(restored.text(), "a\tb");
    assert_eq!(
        restored.paragraphs[0].format.tab_stops.as_ref(),
        Some(&tabs)
    );
    assert_eq!(restored.paragraphs[0].format.default_tab_stop, Some(72.0));
}

#[test]
fn rich_paragraph_tabs_rtf_pard_resets_explicit_tabs_and_retains_default_spacing() {
    let decoded =
        RtfReader::read_str(r"{\rtf1\deftab1080\pard\tqr\tldot\tx1440 a\tab 1\par\pard b\tab 2}")
            .unwrap();
    assert_eq!(decoded.paragraphs[0].format.tab_count(), 1);
    assert_eq!(decoded.paragraphs[1].format.tab_count(), 0);
    assert!(decoded
        .paragraphs
        .iter()
        .all(|p| p.format.default_tab_stop == Some(72.0)));
    for text in [r"{\rtf1\tx-1 a}", r"{\rtf1\deftab0 a}", r"{\rtf1\tx a}"] {
        assert!(RtfReader::read_str(text).is_err(), "{text}");
    }
    assert!(matches!(
        RtfReader::read_str(r"{\rtf1\tlmdot\tx1440 a}"),
        Err(RtfError::UnsupportedFeature(_))
    ));
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

fn measure_tabs(view: &RichTextBlock, height: f64) {
    view.measure(Size::new(300.0, height));
    view.arrange(Rect::new(0.0, 0.0, 300.0, height));
}

#[test]
fn rich_paragraph_tabs_automatic_interval_tracks_paragraph_font_and_explicit_override() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.set_font_size(12.0);
    let paragraph = Paragraph::with_text("a\tb");
    view.blocks().add(paragraph.clone());
    measure_tabs(&view, 100.0);
    assert!((view.get_rect_from_character_index(2).x - 48.0).abs() < 0.01);
    view.set_font_size(20.0);
    measure_tabs(&view, 100.0);
    assert!((view.get_rect_from_character_index(2).x - 80.0).abs() < 0.01);
    paragraph.set_font_size(16.0);
    measure_tabs(&view, 100.0);
    assert!((view.get_rect_from_character_index(2).x - 64.0).abs() < 0.01);
    paragraph.set_default_tab_stop(72.0);
    paragraph.set_font_size(24.0);
    measure_tabs(&view, 100.0);
    assert!((view.get_rect_from_character_index(2).x - 72.0).abs() < 0.01);
    paragraph.clear_value(Paragraph::default_tab_stop_property());
    measure_tabs(&view, 100.0);
    assert!((view.get_rect_from_character_index(2).x - 96.0).abs() < 0.01);
    assert_eq!(
        view.document().snapshot().paragraphs[0]
            .format
            .default_tab_stop,
        None,
        "automatic intervals are resolved by the host without mutating snapshot formatting"
    );
    let exported = RtfReader::read_str(&view.export_rtf().unwrap()).unwrap();
    assert_eq!(exported.paragraphs[0].format.default_tab_stop, Some(96.0));
}

#[test]
fn rich_paragraph_tabs_automatic_interval_follows_source_across_overflow_and_rtl() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    view.set_font_size(20.0);
    let first = Paragraph::with_text("a\tb");
    first.set_font_size(12.0);
    view.blocks().add(first);
    view.blocks().add(Paragraph::with_text("c\td"));
    let overflow = RichTextBlockOverflow::new();
    overflow.set_font_size(8.0);
    view.set_overflow_content_target(Some(overflow.clone()));
    measure_tabs(&view, 20.0);
    measure_tabs(&overflow, 200.0);
    assert!(view.has_overflow_content());
    assert_eq!(overflow.content_end(), view.document().text_length());
    assert!((view.get_rect_from_character_index(2).x - 48.0).abs() < 0.01);
    assert!((overflow.get_rect_from_character_index(6).x - 80.0).abs() < 0.01);
    assert!(matches!(
        view.export_rtf(),
        Err(RtfError::UnsupportedFeature(
            "different default tab spacing across paragraphs"
        ))
    ));

    let rtl = RichTextBlock::new();
    rtl.set_font_size(20.0);
    let paragraph = Paragraph::with_text("אב\t12");
    paragraph.set_flow_direction(FlowDirection::RightToLeft);
    paragraph.set_text_alignment(TextAlignment::Start);
    rtl.blocks().add(paragraph);
    measure_tabs(&rtl, 100.0);
    assert!((rtl.get_rect_from_character_index(3).x - 200.0).abs() < 0.01);
}

#[test]
fn rich_paragraph_tabs_editor_default_remains_document_based_when_font_changes() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let control = RichEditBox::new();
    let paragraph = Paragraph::with_text("a\tb");
    paragraph.set_font_size(20.0);
    control.blocks().add(paragraph);
    let presenter = control.text_presenter();
    measure_tabs(&presenter, 100.0);
    assert_eq!(control.text_document().default_tab_stop(), 36.0);
    assert!((presenter.get_rect_from_character_index(2).x - 48.0).abs() < 0.01);
    control.text_document().set_default_tab_stop(54.0).unwrap();
    measure_tabs(&presenter, 100.0);
    assert!((presenter.get_rect_from_character_index(2).x - 72.0).abs() < 0.01);
    assert!(control.text_document().undo().unwrap());
    measure_tabs(&presenter, 100.0);
    assert!((presenter.get_rect_from_character_index(2).x - 48.0).abs() < 0.01);
    let exported = RtfReader::read_str(&control.export_rtf().unwrap()).unwrap();
    assert_eq!(exported.paragraphs[0].format.default_tab_stop, Some(48.0));
}

#[test]
fn rich_paragraph_tabs_raw_snapshot_export_requires_unambiguous_context() {
    let snapshot = DocumentSnapshot::from_text("a\tb");
    assert!(matches!(
        RtfWriter::write(&snapshot),
        Err(RtfError::UnsupportedFeature(
            "automatic default tab spacing requires a layout context"
        ))
    ));
    let encoded = RtfWriter::write_with_default_tab_stop(&snapshot, 80.0).unwrap();
    assert_eq!(
        RtfReader::read_str(&encoded).unwrap().paragraphs[0]
            .format
            .default_tab_stop,
        Some(80.0)
    );
    assert_eq!(snapshot.paragraphs[0].format.default_tab_stop, None);
    let decoded = RtfReader::read_str(r"{\rtf1 a\tab b}").unwrap();
    assert_eq!(
        decoded.paragraphs[0].format.default_tab_stop,
        Some(48.0),
        "RTF without deftab has a known 720-twip interval, unlike a raw framework snapshot"
    );
    assert!(RtfWriter::write(&decoded).is_ok());
}

#[test]
fn rich_paragraph_tabs_rtf_range_insertion_uses_destination_document_default() {
    let _scope = test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::from_text("a"));
    document.set_default_tab_stop(54.0).unwrap();
    let original = document.document().snapshot();
    document
        .get_range(1, 1)
        .unwrap()
        .set_text_with_options(
            TextSetOptions::FORMAT_RTF,
            r"{\rtf1\deftab1440 x\tab 1\par\tx1800 y\tab 2}",
        )
        .unwrap();
    assert_eq!(document.default_tab_stop(), 54.0);
    let snapshot = document.document().snapshot();
    assert!(snapshot
        .paragraphs
        .iter()
        .all(|p| p.format.default_tab_stop == Some(72.0)));
    assert_eq!(snapshot.paragraphs[1].format.get_tab(0).unwrap().0, 90.0);
    let encoded = document.get_text(TextGetOptions::FORMAT_RTF).unwrap();
    assert!(encoded.contains(r"\deftab1080"));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), original);
    assert_eq!(document.default_tab_stop(), 54.0);
}

#[test]
fn rich_paragraph_tabs_clipboard_resolves_external_defaults_and_preserves_exact_snapshots() {
    use ferroui_base::input::{AsyncDataTransferExtensions, DataTransfer, DataTransferItem};
    use std::future::Future;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};
    fn ready<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("The in-memory transfer must complete synchronously"),
        }
    }
    let _scope = test_scope();
    let original = DocumentSnapshot::from_text("a\tb\nc\td");
    let transfer = RichTextTransfer::create_with_default_tab_stop(
        &original,
        RichEditClipboardFormat::AllFormats,
        72.0,
    );
    let exact = ready(RichTextTransfer::read_for_insertion(
        transfer.as_ref(),
        false,
        48.0,
    ))
    .unwrap()
    .unwrap();
    assert_eq!(exact, original, "the in-process snapshot remains lossless");
    let rtf_format = RichTextTransfer::rtf_formats().remove(0);
    let rtf = ready(transfer.try_get_value_async(&rtf_format))
        .unwrap()
        .unwrap();
    assert!(RtfReader::read(&rtf)
        .unwrap()
        .paragraphs
        .iter()
        .all(|p| p.format.default_tab_stop == Some(72.0)));
    let external = DataTransfer::new();
    let item = DataTransferItem::new();
    item.set(&rtf_format, Some(Rc::<[u8]>::from(rtf.as_ref())));
    external.add(item);
    let inserted = ready(RichTextTransfer::read_for_insertion(
        external.as_ref(),
        false,
        48.0,
    ))
    .unwrap()
    .unwrap();
    assert!(inserted
        .paragraphs
        .iter()
        .all(|p| p.format.default_tab_stop == Some(48.0)));
    assert_eq!(inserted.text(), original.text());
}
