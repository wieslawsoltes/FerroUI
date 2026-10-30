//! Requested-family line metrics and measured embedded controls through real
//! rich layout. The line box follows the open source formulas; platform font
//! selection, shaping and rasterization retain their separate evidence status.

use crate::documents::{EmbeddedControlRun, InlineUIContainer, Paragraph};
use crate::test_support::test_scope;
use crate::{Border, RichTextBlock, RichTextBlockOverflow, TextBlock};
use ferroui_base::media::text_formatting::testing::{TestFont, TextTestScope};
use ferroui_base::media::text_formatting::{DrawableTextRun, GenericTextRunProperties};
use ferroui_base::media::{FontFamily, TextLineBounds, TextWrapping, Typeface};
use ferroui_base::{Point, Rect, Ref, Size, Thickness};
use std::rc::Rc;

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-8,
        "expected {expected}, got {actual}"
    );
}

fn rich() -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    view.set_font_size(10.0);
    view.set_text_line_bounds(TextLineBounds::Tight);
    view.set_text_wrapping(TextWrapping::NoWrap);
    view
}

fn measure(view: &RichTextBlock, height: f64) {
    view.measure(Size::new(300.0, height));
    view.arrange(Rect::new(0.0, 0.0, 300.0, height));
}

fn add_child(view: &RichTextBlock, child: Ref<crate::Control>) {
    let paragraph = Paragraph::new();
    paragraph.add_child(InlineUIContainer::with_child(child));
    view.blocks().add(paragraph);
}

#[test]
fn rich_family_metrics_nested_textblock_uses_measured_baseline_padding_and_full_object_box() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for mode in [
        TextLineBounds::Full,
        TextLineBounds::TrimToCapHeight,
        TextLineBounds::TrimToBaseline,
        TextLineBounds::Tight,
    ] {
        let view = rich();
        view.set_text_line_bounds(mode);
        let child = TextBlock::new();
        child.set_font_size(20.0);
        child.set_text_line_bounds(TextLineBounds::Full);
        child.set_padding(Thickness::new(0.0, 3.0, 0.0, 5.0));
        child.set_text(Some("H"));
        add_child(&view, child.clone().upcast());
        measure(&view, 100.0);
        close(child.desired_size().height, 30.0);
        close(child.text_layout().baseline(), 17.0);
        // Its attached override stays zero; the rich object captures the
        // actual line baseline plus the child text's top padding.
        close(child.baseline_offset(), 0.0);
        let layout = view.page_layout();
        close(layout.lines[0].line.baseline(), 20.0);
        close(layout.lines[0].line.height(), 30.0);
        close(child.bounds().y, 0.0);
        close(child.bounds().height, 30.0);
        close(view.get_rects_for_range(0..1)[0].height, 30.0);
    }
}

#[test]
fn rich_family_metrics_nested_rich_and_overflow_use_their_first_measured_line_baselines() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let child = RichTextBlock::new();
    child.set_font_size(20.0);
    child.set_text_line_bounds(TextLineBounds::Full);
    child.set_height(22.0);
    child.blocks().add(Paragraph::with_text("a\nb\nc"));
    let overflow = RichTextBlockOverflow::new();
    overflow.set_height(44.0);
    child.set_overflow_content_target(Some(overflow.clone()));
    let paragraph = Paragraph::new();
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    paragraph.add_child(InlineUIContainer::with_child(overflow.clone()));
    view.blocks().add(paragraph);
    measure(&view, 100.0);

    close(child.baseline_offset(), 16.0);
    close(overflow.baseline_offset(), 16.0);
    close(child.desired_size().height, 22.0);
    close(overflow.desired_size().height, 44.0);
    let layout = view.page_layout();
    close(layout.lines[0].line.baseline(), 16.0);
    close(layout.lines[0].line.height(), 44.0);
    close(child.bounds().y, 0.0);
    close(overflow.bounds().y, 0.0);
    assert_eq!(child.content_end(), overflow.content_start());
    assert_eq!(overflow.content_end(), child.document().text_length());
}

#[test]
fn rich_family_metrics_attached_baseline_override_retains_nonzero_and_zero_semantics() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for (override_value, expected_baseline) in [(7.0, 7.0), (0.0, 16.0)] {
        let view = rich();
        let child = RichTextBlock::new();
        child.set_font_size(20.0);
        child.set_text_line_bounds(TextLineBounds::Full);
        child.blocks().add(Paragraph::with_text("H"));
        TextBlock::set_baseline_offset_on(&child, override_value);
        add_child(&view, child.clone().upcast());
        measure(&view, 100.0);
        close(child.baseline_offset(), 16.0);
        close(
            view.page_layout().lines[0].line.baseline(),
            expected_baseline,
        );
        close(view.page_layout().lines[0].line.height(), 22.0);
    }
}

#[test]
fn rich_family_metrics_captured_object_size_matches_run_geometry_after_independent_child_measure() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = rich();
    let child = Border::new();
    child.set_width(10.0);
    child.set_height(30.0);
    add_child(&view, child.clone().upcast());
    measure(&view, 100.0);
    let old = view.page_layout();
    close(old.lines[0].line.width(), 10.0);
    let snapshot = view.document().snapshot();

    // Its height and baseline stay equal: invalidation must also retain the
    // size that the inline last measured, not just read the new desired size.
    child.set_width(42.0);
    child.measure(Size::new(300.0, 100.0));
    assert_eq!(child.desired_size(), Size::new(42.0, 30.0));
    let old_objects = old.embedded_controls(Point::default());
    assert_eq!(old_objects[0].1.size(), Size::new(10.0, 30.0));
    let old_runs = old.lines[0].line.text_runs();
    assert_eq!(
        old_runs[0].as_drawable().unwrap().size(),
        Size::new(10.0, 30.0)
    );
    drop(old_runs);

    measure(&view, 100.0);
    let current = view.page_layout();
    assert!(Rc::ptr_eq(&old.source, &current.source));
    close(current.lines[0].line.width(), 42.0);
    assert_eq!(
        current.embedded_controls(Point::default())[0].1.size(),
        Size::new(42.0, 30.0)
    );
    assert_eq!(view.document().snapshot(), snapshot);
}

#[test]
fn rich_family_metrics_absent_and_stale_nested_caches_never_force_formatting() {
    let _scope = test_scope();
    let text_scope = TextTestScope::new();
    let child = RichTextBlock::new();
    child.set_font_size(20.0);
    child.blocks().add(Paragraph::with_text("H"));
    let properties = Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::default_typeface(),
        20.0,
    ));
    let run = EmbeddedControlRun::new(child.clone().upcast(), properties);
    assert!(child.cached_page_layout().is_none());
    let unmeasured = run.with_measured_metrics();
    close(unmeasured.baseline(), child.desired_size().height);
    assert_eq!(text_scope.shaper().shape_call_count(), 0);
    assert!(child.cached_page_layout().is_none());

    measure(&child, 100.0);
    let measured = run.with_measured_metrics();
    close(measured.baseline(), 16.0);
    close(measured.size().height, 22.0);
    let calls = text_scope.shaper().shape_call_count();
    child.set_font_size(40.0);
    assert!(!child.is_measure_valid());
    assert!(child.cached_page_layout().is_none());
    let stale = run.with_measured_metrics();
    close(stale.baseline(), child.desired_size().height);
    assert_eq!(text_scope.shaper().shape_call_count(), calls);
    assert!(child.cached_page_layout().is_none());
    measure(&child, 100.0);
    close(run.with_measured_metrics().baseline(), 32.0);
    // A prior captured run remains an immutable layout input.
    close(measured.baseline(), 16.0);
    close(measured.size().height, 22.0);
}

#[test]
fn rich_family_metrics_existing_embedded_constructor_retains_legacy_desired_bottom_baseline() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let child = RichTextBlock::new();
    child.set_font_size(20.0);
    child.blocks().add(Paragraph::with_text("H"));
    measure(&child, 100.0);
    let properties = Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::default_typeface(),
        20.0,
    ));
    let run = EmbeddedControlRun::new(child.clone().upcast(), properties);
    close(run.baseline(), 22.0);
    close(run.with_measured_metrics().baseline(), 16.0);
}
