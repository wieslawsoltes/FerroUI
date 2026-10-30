//! Inherited view presentation is not an edit of the stored rich document.

use crate::documents::{Inline, RichTextDocument, Run, TextElement};
use ferroui_base::media::{
    Brushes, FlowDirection, FontFeature, FontFeatureCollection, TextDecoration,
    TextDecorationCollection,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Rect, Size};
use std::cell::Cell;
use std::rc::Rc;
#[allow(unused_imports)]
use crate::{RichTextBlock, RichTextBlockOverflow};
#[allow(unused_imports)]
use crate::rich_text::*;

fn layout(view: &RichTextBlock) {
    view.measure(Size::new(400.0, 300.0));
    view.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));
    assert!(view.is_measure_valid());
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
