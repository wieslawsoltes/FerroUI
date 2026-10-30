use crate::documents::{
    Hyperlink, InlineCollection, InlineUIContainer, Paragraph, RichTextDocument, Run, Span,
    TextGravity,
};
use crate::test_support::test_scope;
use crate::{Border, RichTextBlock, RichTextBlockOverflow};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FontWeight, TextTrimming, TextWrapping};
use ferroui_base::{Point, Rect, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

fn rich(text: &str) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.blocks().add(Paragraph::with_text(text));
    view
}
fn measure(view: &RichTextBlock, width: f64, height: f64) {
    view.measure(Size::new(width, height));
    view.arrange(Rect::new(0.0, 0.0, width, height));
}

fn rich_source_properties(
    view: &RichTextBlock,
    index: i32,
) -> Rc<dyn ferroui_base::media::text_formatting::TextRunProperties> {
    use ferroui_base::media::text_formatting::ITextSource;
    view.page_layout().source.paragraphs[0]
        .source
        .get_text_run(index)
        .unwrap()
        .properties()
        .unwrap()
        .clone()
}

fn rich_brush_color(
    brush: Option<&Rc<dyn ferroui_base::media::IBrush>>,
) -> Option<ferroui_base::media::Color> {
    brush
        .and_then(|brush| brush.as_solid_color_brush())
        .map(|brush| brush.color())
}

#[test]
fn rich_highlights_preserve_inline_format_and_selection_has_foreground_priority() {
    use crate::documents::{TextHighlightRange, TextHighlighter};
    use ferroui_base::media::{Brushes, Colors, FontStyle, TextDecorations};

    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let run = Run::with_text(Some("a😀bc"));
    run.set_font_size(19.0);
    run.set_font_weight(FontWeight::Bold);
    run.set_font_style(FontStyle::Italic);
    run.set_text_decorations(Some(TextDecorations::underline()));
    run.set_foreground(Some(Brushes::green()));
    run.set_background(Some(Brushes::aqua()));
    paragraph.add_child(run.clone());
    view.blocks().add(paragraph);
    let revision = view.document().revision();
    for (start, foreground) in [(1, Brushes::red()), (2, Brushes::blue())] {
        let layer = TextHighlighter::new();
        layer.ranges().add(TextHighlightRange::new(start, 1));
        layer.set_foreground(Some(foreground)).unwrap();
        layer.set_background(Some(Brushes::yellow())).unwrap();
        view.text_highlighters().add(layer);
    }
    measure(&view, 400.0, 100.0);
    let regions = view.page_layout().source.highlights.clone();
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0].range,
        1..3,
        "each layer expands to the complete scalar before priority merging"
    );
    assert_eq!(
        rich_brush_color(regions[0].foreground.as_ref()),
        Some(Colors::BLUE)
    );
    let highlighted = rich_source_properties(&view, 1);
    assert_eq!(highlighted.typeface().weight(), FontWeight::Bold);
    assert_eq!(highlighted.typeface().style(), FontStyle::Italic);
    assert_eq!(highlighted.font_rendering_em_size(), 19.0);
    assert_eq!(highlighted.text_decorations().unwrap().list().count(), 1);
    assert_eq!(
        rich_brush_color(highlighted.foreground_brush()),
        Some(Colors::BLUE)
    );
    assert!(
        highlighted.background_brush().is_none(),
        "the run must not cover the highlight background"
    );

    view.set_selection_foreground_brush(Some(Brushes::white()));
    view.select(1, 3);
    assert_eq!(
        rich_brush_color(rich_source_properties(&view, 1).foreground_brush()),
        Some(Colors::WHITE)
    );
    view.set_selection_foreground_brush(None);
    assert_eq!(
        rich_brush_color(rich_source_properties(&view, 1).foreground_brush()),
        Some(Colors::BLUE)
    );
    view.select(3, 4);
    assert!(
        rich_source_properties(&view, 3)
            .background_brush()
            .is_none()
    );
    view.set_is_text_selection_enabled(false);
    assert_eq!(
        rich_brush_color(rich_source_properties(&view, 3).background_brush()),
        Some(Colors::AQUA)
    );
    view.set_is_text_selection_enabled(true);
    view.set_selection_highlight_color(None);
    assert_eq!(
        rich_brush_color(rich_source_properties(&view, 3).background_brush()),
        Some(Colors::AQUA)
    );
    assert_eq!(view.document().revision(), revision);
    assert_eq!(
        rich_brush_color(run.foreground().as_ref()),
        Some(Colors::GREEN)
    );
}

#[test]
fn rich_highlight_theme_resources_and_overflow_changes_rebuild_only_visual_input() {
    use crate::documents::{TextHighlightRange, TextHighlighter};
    use ferroui_base::media::{Brushes, Colors, IBrush, SolidColorBrush};

    let _scope = test_scope();
    let _text = TextTestScope::new();
    crate::register_types();
    let view = rich("one two three four five six seven eight");
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    view.set_max_lines(1);
    let foreground = SolidColorBrush::with_color(Colors::RED);
    let brush: Rc<dyn IBrush> = foreground.clone().into();
    view.resources()
        .add_value("TextControlHighlighterForeground", brush);
    view.resources()
        .add_value("TextControlHighlighterBackground", Brushes::aqua());
    let highlighter = TextHighlighter::new();
    highlighter
        .ranges()
        .add(TextHighlightRange::new(0, i32::MAX));
    view.text_highlighters().add(highlighter.clone());
    let revision = view.document().revision();
    measure(&view, 75.0, 25.0);
    measure(&overflow, 75.0, 300.0);
    let first = view.page_layout();
    let second = overflow.page_layout();
    assert!(Rc::ptr_eq(&first.source, &second.source));
    assert_eq!(
        rich_brush_color(first.source.highlights[0].foreground.as_ref()),
        Some(Colors::RED)
    );
    assert_eq!(
        rich_brush_color(first.source.highlights[0].background.as_ref()),
        Some(Colors::AQUA)
    );
    assert_eq!(overflow.text_highlighters(), view.text_highlighters());

    foreground.set_color(Colors::BLUE);
    let changed = overflow.page_layout();
    assert!(!Rc::ptr_eq(&second.source, &changed.source));
    assert_eq!(
        rich_brush_color(changed.source.highlights[0].foreground.as_ref()),
        Some(Colors::BLUE)
    );
    highlighter.ranges().set(0, TextHighlightRange::new(4, 3));
    assert_eq!(overflow.page_layout().source.highlights[0].range, 4..7);
    view.resources().set(
        "TextControlHighlighterForeground",
        Some(Rc::new(Brushes::green())),
    );
    assert_eq!(
        rich_brush_color(
            overflow.page_layout().source.highlights[0]
                .foreground
                .as_ref()
        ),
        Some(Colors::GREEN)
    );
    view.text_highlighters().clear();
    assert!(overflow.page_layout().source.highlights.is_empty());
    assert_eq!(view.document().revision(), revision);
}

#[test]
fn rich_document_defaults_are_instance_local() {
    let _scope = test_scope();
    let first = rich("first");
    let second = RichTextBlock::new();
    assert_eq!(first.text(), "first");
    assert_eq!(second.text(), "");
    assert_ne!(first.document(), second.document());
}

#[test]
fn rich_paragraphs_measure_and_report_document_offsets() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one");
    view.blocks().add(Paragraph::with_text("two"));
    measure(&view, 200.0, 100.0);
    assert_eq!(view.text(), "one\ntwo");
    let metrics = view.line_metrics();
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].start, 0);
    assert_eq!(metrics[1].start, 4);
    assert!(metrics[1].bounds.y > metrics[0].bounds.y);
    assert_eq!(view.content_end(), 7);
}

#[test]
fn rich_empty_document_has_a_caret_line() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    measure(&view, 200.0, 100.0);
    assert_eq!(view.line_metrics().len(), 1);
    assert!(view.cursor_rectangle().height > 0.0);
    assert_eq!(view.hit_test_text_position(Point::new(50.0, 5.0)), 0);
}

#[test]
fn rich_selection_preserves_direction_and_scalar_boundaries() {
    let _scope = test_scope();
    let view = rich("a😀b");
    view.select(4, 2);
    assert_eq!(view.selection_start(), 4);
    assert_eq!(view.selection_end(), 1);
    assert_eq!(view.selected_text(), "😀b");
    view.select(0, usize::MAX);
    assert_eq!(view.selected_text(), "a😀b");
}

#[test]
fn rich_inherited_fonts_and_span_replacement_invalidate_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let span = Span::new();
    span.add_child_text("short");
    paragraph.add_child(span.clone());
    view.blocks().add(paragraph.clone());
    view.set_font_size(24.0);
    measure(&view, 500.0, 100.0);
    assert_eq!(paragraph.font_size(), 24.0);
    let width = view.line_metrics()[0].bounds.width;
    let replacement = InlineCollection::new();
    replacement.add_text("much longer content");
    span.set_inlines(replacement);
    measure(&view, 500.0, 100.0);
    assert_eq!(view.text(), "much longer content");
    assert!(view.line_metrics()[0].bounds.width > width);
}

#[test]
fn rich_wrapping_reuses_shaping_and_selection_geometry() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("alpha beta gamma delta");
    measure(&view, 65.0, 300.0);
    assert!(view.line_metrics().len() > 1);
    view.select(0, view.document().text_length());
    assert!(view.selection_rects().len() > 1);
    let rect = view.get_rect_from_character_index(2);
    let hit = view.hit_test_text_position(Point::new(rect.x, rect.y + rect.height / 2.0));
    assert_eq!(hit, 2);
}

#[test]
fn rich_overflow_columns_continue_without_dropping_text_on_reflow() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one two three four five six seven eight nine ten");
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 65.0, 28.0);
    measure(&overflow, 90.0, 300.0);
    assert!(view.has_overflow_content());
    assert_eq!(overflow.content_start(), view.content_end());
    assert_eq!(overflow.content_end(), view.document().text_length());
    let original = overflow.line_metrics();
    measure(&overflow, 80.0, 300.0);
    measure(&overflow, 90.0, 300.0);
    assert_eq!(overflow.line_metrics(), original);
}

#[test]
fn rich_overflow_rejects_cycles_and_second_owners() {
    let _scope = test_scope();
    let first = RichTextBlockOverflow::new();
    let second = RichTextBlockOverflow::new();
    let third = RichTextBlock::new();
    assert!(first.try_set_overflow_content_target(Some(second.clone())));
    assert!(!second.try_set_overflow_content_target(Some(first.clone())));
    assert!(!third.try_set_overflow_content_target(Some(second)));
}

#[test]
fn rich_embedded_controls_keep_text_offsets_and_receive_layout() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("a");
    paragraph.add_child(InlineUIContainer::new());
    let child = Border::new();
    child.set_width(30.0);
    child.set_height(20.0);
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    paragraph.add_child(Run::with_text(Some("b")));
    view.blocks().add(paragraph);
    measure(&view, 200.0, 100.0);
    assert_eq!(view.text(), "a\u{fffc}b");
    assert_eq!(child.bounds().width, 30.0);
    assert!(child.bounds().x > 0.0);
    assert_eq!(view.content_end(), 3);
}

#[test]
fn rich_formatting_edit_rebuilds_runs_without_losing_text() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("format me");
    measure(&view, 200.0, 100.0);
    let mut format = crate::documents::CharacterFormat::default();
    format.font_weight = Some(FontWeight::Bold);
    view.document().apply_format(0..6, &format).unwrap();
    measure(&view, 200.0, 100.0);
    assert_eq!(view.text(), "format me");
    assert_eq!(view.content_end(), 9);
}

#[test]
fn rich_max_lines_produces_a_stable_continuation() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one two three four five");
    view.set_text_wrapping(TextWrapping::Wrap);
    view.set_max_lines(1);
    measure(&view, 40.0, 200.0);
    assert_eq!(view.line_metrics().len(), 1);
    assert!(view.has_overflow_content());
    assert!(view.content_end() < view.document().text_length());
}

#[test]
fn rich_selection_tracks_external_edits_and_is_shared_across_columns() {
    let _scope = test_scope();
    let view = rich("alpha beta gamma");
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    overflow.select(6, 10);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    let _subscription = overflow.selection_changed(move |_| observed.set(observed.get() + 1));
    view.document().replace_text(0..0, "😀 ").unwrap();
    assert_eq!(view.selected_text(), "beta");
    assert_eq!(overflow.selected_text(), "beta");
    assert_eq!(
        (overflow.selection_start(), overflow.selection_end()),
        (9, 13)
    );
    assert_eq!(changes.get(), 1);
}

#[test]
fn rich_document_view_ownership_is_checked_before_detaching_content() {
    let _scope = test_scope();
    let first = rich("first");
    let second = rich("second");
    let shared = first.document();
    assert!(!second.try_set_document(shared.clone()));
    assert_eq!(first.text(), "first");
    assert_eq!(second.text(), "second");
    assert!(first.try_set_document(RichTextDocument::new()));
    assert!(second.try_set_document(shared));
    assert_eq!(second.text(), "first");
    let empty = first.document();
    assert!(!second.try_set_document(empty));
}

#[test]
fn rich_range_geometry_does_not_mutate_selection_and_rejects_invalid_utf16() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("a😀b c d e");
    measure(&view, 40.0, 200.0);
    view.select(3, 4);
    assert!(!view.get_rects_for_range(0..5).is_empty());
    assert!(view.get_rects_for_range(0..2).is_empty());
    assert!(view.get_rects_for_range(0..usize::MAX).is_empty());
    assert_eq!((view.selection_start(), view.selection_end()), (3, 4));
}

#[test]
fn rich_pointer_geometry_keeps_the_insertion_offset_and_finds_overflow_view() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("one two three four five six seven eight");
    let pointer = view
        .document()
        .create_pointer(2, TextGravity::Forward)
        .unwrap();
    assert!(pointer.get_character_rect(TextGravity::Forward).is_none());
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 65.0, 28.0);
    measure(&overflow, 100.0, 300.0);
    let forward = pointer.get_character_rect(TextGravity::Forward).unwrap();
    let backward = pointer.get_character_rect(TextGravity::Backward).unwrap();
    assert_eq!(forward, backward);
    assert_eq!(forward.x, view.get_rect_from_character_index(2).x);
    assert_eq!(forward.width, 0.0);
    let later = view
        .document()
        .create_pointer(overflow.content_start() + 1, TextGravity::Forward)
        .unwrap();
    assert_eq!(later.visual_parent(), Some(overflow.clone().upcast()));
    let rect = later.get_character_rect(TextGravity::Forward).unwrap();
    assert_eq!(
        rect.y,
        overflow.get_rect_from_character_index(later.offset()).y
    );
}

fn key(view: &RichTextBlock, value: Key, release: bool) -> KeyEventArgs {
    let mut event = KeyEventArgs::new();
    event.key = value;
    event.key_modifiers = KeyModifiers::NONE;
    event.set_routed_event(Some(if release {
        InputElement::key_up_event()
    } else {
        InputElement::key_down_event()
    }));
    view.raise_event(&event);
    event
}

#[test]
fn rich_hyperlink_hit_testing_keyboard_focus_and_cancellation_use_the_host() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::with_text("open ");
    let link = Hyperlink::new();
    link.add_child_text("link");
    link.set_navigate_uri(Some("https://example.com".into()));
    paragraph.add_child(link.clone());
    view.blocks().add(paragraph);
    measure(&view, 200.0, 100.0);
    let rect = view.get_rect_from_character_index(6);
    assert_eq!(
        view.hyperlink_at(Point::new(rect.x + 1.0, rect.y + rect.height / 2.0)),
        Some(link.clone())
    );
    assert!(view.hyperlink_at(Point::new(190.0, 95.0)).is_none());
    let invocations = Rc::new(Cell::new(0));
    let observed = invocations.clone();
    let _subscription = link.click(move |event| {
        observed.set(observed.get() + 1);
        event.set_handled(true);
    });
    let revision = view.document().revision();
    assert!(key(&view, Key::Tab, false).handled());
    assert!(link.is_focused());
    assert!(key(&view, Key::Enter, false).handled());
    assert_eq!(invocations.get(), 0);
    assert!(key(&view, Key::Enter, true).handled());
    assert_eq!(invocations.get(), 1);
    assert_eq!(view.document().revision(), revision);
    assert!(!key(&view, Key::Tab, false).handled());
    assert!(!link.is_focused());
    assert!(key(&view, Key::Tab, false).handled());
    assert!(key(&view, Key::Space, false).handled());
    assert_eq!(invocations.get(), 1);
    assert!(key(&view, Key::Space, true).handled());
    assert_eq!(invocations.get(), 2);
}

fn formatted_text(view: &RichTextBlock) -> String {
    let mut text = Vec::new();
    for entry in &view.page_layout().lines {
        for run in entry.line.text_runs().iter() {
            text.extend_from_slice(run.text().span());
        }
    }
    String::from_utf16_lossy(&text)
}

#[test]
fn rich_trimming_uses_real_ellipsis_and_remains_reflowable() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("alpha beta gamma delta epsilon");
    view.set_text_wrapping(TextWrapping::NoWrap);
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    for _ in 0..3 {
        measure(&view, 65.0, 100.0);
        assert!(view.is_text_trimmed());
        assert!(formatted_text(&view).contains('…'));
        measure(&view, 1000.0, 100.0);
        assert!(!view.is_text_trimmed());
        assert!(formatted_text(&view).starts_with(&view.text()));
        assert_eq!(
            view.visible_text_ranges(),
            vec![0..view.document().text_length()]
        );
    }
}

#[test]
fn rich_terminal_truncation_and_linked_overflow_retain_distinct_layout_states() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("first paragraph");
    view.blocks().add(Paragraph::with_text("second paragraph"));
    view.blocks().add(Paragraph::with_text("third paragraph"));
    view.set_max_lines(1);
    view.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
    measure(&view, 300.0, 100.0);
    assert!(view.has_overflow_content());
    assert!(view.is_text_trimmed());
    assert!(formatted_text(&view).contains('…'));
    let overflow = RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    measure(&view, 300.0, 100.0);
    measure(&overflow, 300.0, 100.0);
    assert!(!view.is_text_trimmed());
    assert_eq!(overflow.content_source(), Some(view.clone()));
    assert_eq!(overflow.content_start(), view.content_end());
    assert_eq!(overflow.content_end(), view.document().text_length());
    view.set_overflow_content_target(None);
    measure(&view, 300.0, 100.0);
    assert!(overflow.content_source().is_none());
    assert!(view.is_text_trimmed());
}

#[test]
fn rich_finite_height_trims_and_indentation_respects_paragraph_local_values() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("first line");
    let second = Paragraph::with_text("second line");
    second.set_text_indent(0.0);
    view.blocks().add(second);
    view.set_text_indent(20.0);
    view.set_line_stacking_strategy(crate::documents::LineStackingStrategy::BlockLineHeight);
    view.set_line_height(24.0);
    measure(&view, 300.0, 100.0);
    assert_eq!(view.line_metrics()[0].bounds.x, 20.0);
    assert_eq!(view.line_metrics()[1].bounds.x, 0.0);
    assert_eq!(view.line_metrics()[1].bounds.y, 24.0);
    view.set_text_trimming(<dyn TextTrimming>::word_ellipsis());
    measure(&view, 300.0, 25.0);
    assert_eq!(view.line_metrics().len(), 1);
    assert!(view.is_text_trimmed());
    assert!(view.has_overflow_content());
}

#[test]
fn rich_collapsed_source_ranges_and_geometry_follow_retained_prefixes_and_suffixes() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich("prefix 😀 … abcdefghijklmnopqrstuvwxyz suffix");
    view.set_text_wrapping(TextWrapping::NoWrap);
    for trimming in [
        <dyn TextTrimming>::character_ellipsis(),
        <dyn TextTrimming>::leading_character_ellipsis(),
        <dyn TextTrimming>::prefix_character_ellipsis(),
    ] {
        view.set_text_trimming(trimming);
        measure(&view, 110.0, 100.0);
        assert!(view.is_text_trimmed());
        let ranges = view.visible_text_ranges();
        assert!(!ranges.is_empty());
        let mut previous = 0;
        let mut visible_length = 0;
        for range in ranges {
            assert!(view.document().validate_range(range.clone()).is_ok());
            if previous < range.start {
                assert!(view.get_rects_for_range(previous..range.start).is_empty());
            }
            assert!(!view.get_rects_for_range(range.clone()).is_empty());
            let caret = view.get_rect_from_character_index(range.start);
            assert_eq!(
                view.hit_test_text_position(Point::new(caret.x, caret.y + caret.height / 2.0)),
                range.start
            );
            visible_length += range.len();
            previous = range.end;
        }
        let length = view.document().text_length();
        if previous < length {
            assert!(view.get_rects_for_range(previous..length).is_empty());
        }
        assert!(visible_length < length);
        measure(&view, 1000.0, 100.0);
        assert_eq!(view.visible_text_ranges(), vec![0..length]);
    }
}
