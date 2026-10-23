//! Inherited view presentation is not an edit of the stored rich document.

use crate::documents::{Inline, RichTextDocument, Run, TextElement};
use crate::rich_text::{EditKind, RichTextEditor};
use crate::{RichEditBox, RichTextBlock};
use ferroui_base::media::{
    Brushes, FlowDirection, FontFeature, FontFeatureCollection, TextDecoration,
    TextDecorationCollection,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Rect, Size};
use std::cell::Cell;
use std::rc::Rc;

fn layout(view: &RichTextBlock) {
    view.measure(Size::new(400.0, 300.0));
    view.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));
    assert!(view.is_measure_valid());
}

#[test]
fn rich_inherited_presentation_preserves_undo_and_active_composition() {
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_text("original").unwrap();
    editor.select(8, 8);
    editor.insert_text("!").unwrap();
    editor.set_preedit_text(Some("preview"), Some(2)).unwrap();
    let document = editor.document();
    let before = document.snapshot();
    let revision = document.revision();
    let mutation = document.mutation_version();
    let composition = editor.editor().active_composition_range();
    assert!(composition.is_some());
    let changed = Rc::new(Cell::new(0));
    let count = changed.clone();
    let _subscription = document.changed(move |_| count.set(count.get() + 1));

    editor.set_template(Some(crate::rich_edit_box_tests::rich_edit_box_template()));
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    root.measure(Size::new(400.0, 300.0));
    root.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));
    editor.set_font_size(28.0);
    editor.set_foreground(Some(Brushes::blue()));
    editor.set_language(CultureInfo::get_culture_info("ja-JP"));
    editor.set_letter_spacing(2.0);
    editor.set_flow_direction(FlowDirection::RightToLeft);
    editor.set_text_decorations(Some(TextDecorationCollection::from_items([
        TextDecoration::new(),
    ])));
    root.measure(Size::new(400.0, 300.0));
    root.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));

    assert_eq!(document.revision(), revision);
    assert_eq!(document.mutation_version(), mutation);
    assert_eq!(document.snapshot(), before);
    assert_eq!(changed.get(), 0);
    assert_eq!(editor.editor().active_composition_range(), composition);
    assert!(editor.can_undo());
    editor.cancel_composition().unwrap();
    editor.undo().unwrap();
    assert_eq!(editor.text(), "original");
    editor.redo().unwrap();
    assert_eq!(editor.text(), "original!");
    assert_eq!(editor.document().paragraphs()[0].font_size(), 28.0);
}

#[test]
fn rich_inherited_font_language_and_flow_changes_rebuild_layout_without_model_events() {
    let _scope = crate::test_support::test_scope();
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text("Abc"));
    layout(&view);
    let old_height = view.desired_size().height;
    let document = view.document();
    let before = document.snapshot();
    let revision = document.revision();
    let mutation = document.mutation_version();
    view.set_font_size(36.0);
    assert!(!view.is_measure_valid());
    layout(&view);
    assert!(view.desired_size().height > old_height * 2.0);
    view.set_language(CultureInfo::get_culture_info("tr-TR"));
    assert!(!view.is_measure_valid());
    layout(&view);
    view.set_flow_direction(FlowDirection::RightToLeft);
    assert!(!view.is_measure_valid());
    layout(&view);
    view.clear_value(TextElement::font_size_property());
    assert!(!view.is_measure_valid());
    layout(&view);
    assert_eq!(document.revision(), revision);
    assert_eq!(document.mutation_version(), mutation);
    assert_eq!(document.snapshot(), before);
}

#[test]
fn rich_inherited_mutable_features_and_decorations_only_invalidate_layout() {
    let _scope = crate::test_support::test_scope();
    let view = RichTextBlock::new();
    view.set_document(RichTextDocument::from_text("office"));
    let features = FontFeatureCollection::new();
    view.set_font_features(Some(features.clone()));
    let decoration = TextDecoration::new();
    let decorations = TextDecorationCollection::from_items([decoration.clone()]);
    Inline::set_text_decorations_on(&view, Some(decorations.clone()));
    layout(&view);
    let document = view.document();
    let revision = document.revision();
    let mutation = document.mutation_version();
    let before = document.snapshot();
    features.add(FontFeature::parse("liga=0"));
    assert!(!view.is_measure_valid());
    layout(&view);
    decoration.set_stroke_thickness(3.0);
    assert!(!view.is_measure_valid());
    layout(&view);
    decorations.add(TextDecoration::new());
    assert!(!view.is_measure_valid());
    layout(&view);
    assert_eq!(document.revision(), revision);
    assert_eq!(document.mutation_version(), mutation);
    assert_eq!(document.snapshot(), before);
}

#[test]
fn rich_local_format_assignment_and_clear_remain_semantic_mutations() {
    let _scope = crate::test_support::test_scope();
    let view = RichTextBlock::new();
    view.set_font_size(18.0);
    let document = RichTextDocument::from_text("abc");
    view.set_document(document.clone());
    let editor = RichTextEditor::new(document.clone());
    editor.insert_text("!", EditKind::Typing).unwrap();
    assert!(editor.can_undo());
    let run = document.paragraphs()[0]
        .inlines()
        .get(0)
        .cast::<Run>()
        .unwrap();
    let revision = document.revision();
    let mutation = document.mutation_version();
    run.set_font_size(24.0);
    assert!(document.revision() > revision);
    assert!(document.mutation_version() > mutation);
    assert!(!editor.can_undo());

    let revision = document.revision();
    let mutation = document.mutation_version();
    let batch = document.begin_edit();
    run.clear_value(TextElement::font_size_property());
    assert_eq!(run.font_size(), 18.0);
    assert_eq!(document.revision(), revision);
    assert!(document.mutation_version() > mutation);
    batch.finish();
    assert!(document.revision() > revision);
    assert!(
        document.snapshot().paragraphs[0].inlines[0]
            .format
            .font_size
            .is_none()
    );
}
