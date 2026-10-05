use super::{Bold, Italic, Run, Span};
use crate::test_support::test_scope;
use ferroui_base::media::text_formatting::TextRun;
use ferroui_base::media::{Brushes, FontStretch, FontStyle, FontWeight, IBrush};
use std::rc::Rc;

#[test]
fn should_inherit_font_weight_in_nested_inlines() {
    let _scope = test_scope();
    let bold = Bold::new();
    let span = Span::new();
    let run = Run::with_text(Some("Test"));
    span.inlines().add(run);
    bold.inlines().add(span);

    let mut text_runs: Vec<Rc<dyn TextRun>> = Vec::new();
    bold.build_text_run(&mut text_runs);

    let run_properties = text_runs[0].properties().unwrap();
    assert_eq!(run_properties.typeface().weight(), FontWeight::Bold);
}

#[test]
fn should_inherit_font_style_in_nested_inlines() {
    let _scope = test_scope();
    let italic = Italic::new();
    let span = Span::new();
    let run = Run::with_text(Some("Test"));
    span.inlines().add(run);
    italic.inlines().add(span);

    let mut text_runs: Vec<Rc<dyn TextRun>> = Vec::new();
    italic.build_text_run(&mut text_runs);

    let run_properties = text_runs[0].properties().unwrap();
    assert_eq!(run_properties.typeface().style(), FontStyle::Italic);
}

#[test]
fn should_inherit_font_stretch_in_nested_inlines() {
    let _scope = test_scope();
    let span = Span::new();
    let inner_span = Span::new();
    let run = Run::with_text(Some("Test"));
    span.set_font_stretch(FontStretch::Condensed);
    inner_span.inlines().add(run);
    span.inlines().add(inner_span);

    let mut text_runs: Vec<Rc<dyn TextRun>> = Vec::new();
    span.build_text_run(&mut text_runs);

    let run_properties = text_runs[0].properties().unwrap();
    assert_eq!(run_properties.typeface().stretch(), FontStretch::Condensed);
}

#[test]
fn should_inherit_background_in_nested_inlines() {
    let _scope = test_scope();
    let background_brush: Rc<dyn IBrush> = Brushes::red();
    let span = Span::new();
    let inner_span = Span::new();
    let run = Run::with_text(Some("Test"));

    span.set_background(Some(background_brush.clone()));
    inner_span.inlines().add(run);
    span.inlines().add(inner_span);

    let mut text_runs: Vec<Rc<dyn TextRun>> = Vec::new();
    span.build_text_run(&mut text_runs);

    let run_properties = text_runs[0].properties().unwrap();
    assert!(run_properties.background_brush() == Some(&background_brush));
}
