use super::TextLineBreak;
use crate::media::text_formatting::testing::{
    line_text, CustomDrawableRun, InvisibleRun, ListTextSource, TextTestScope,
};
use crate::media::text_formatting::{
    GenericTextParagraphProperties, GenericTextRunProperties, ShapedTextRun, TextCharacters,
    TextFormatter, TextLineMetricsMode, TextParagraphProperties, TextRun, TextRunProperties,
};
use crate::media::{FlowDirection, TextAlignment, TextWrapping, Typeface};
use crate::utilities::ReadOnlyMemory;
use crate::Size;
use std::rc::Rc;

fn properties() -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::default_typeface(),
        20.0,
    ))
}

fn object(width: f64) -> Rc<dyn TextRun> {
    Rc::new(CustomDrawableRun::with_size(
        Size::new(width, 30.0),
        23.0,
        properties(),
    ))
}

#[test]
fn continuation_objects_rebind_without_consuming_or_reshaping_text() {
    let scope = TextTestScope::new();
    let props = properties();
    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::new(
            ReadOnlyMemory::from_str("x"),
            props.clone(),
        )),
        object(40.0),
        Rc::new(TextCharacters::new(
            ReadOnlyMemory::from_str("אבcd"),
            props.clone(),
        )),
    ]);
    let paragraph = GenericTextParagraphProperties::with_options(
        props,
        TextAlignment::Left,
        TextWrapping::Wrap,
        f64::NAN,
        0.0,
    );
    paragraph.set_line_metrics_mode(TextLineMetricsMode::FontFamily);
    let paragraph: Rc<dyn TextParagraphProperties> = Rc::new(paragraph);
    let formatter = <dyn TextFormatter>::current();
    let first = formatter
        .format_line(&source, 0, 10.0, &paragraph, None)
        .unwrap();
    assert_eq!(first.length(), 1);
    let original = first.text_line_break().unwrap();
    let calls = scope.shaper().shape_call_count();
    let mut replacements = 0;
    let fork = original.fork_with_object_runs(|run| {
        assert!(run.is::<CustomDrawableRun>());
        replacements += 1;
        Some(object(15.0))
    });
    assert_eq!(replacements, 1);
    assert_eq!(fork.flow_direction(), original.flow_direction());
    assert_eq!(fork.line_metrics_mode(), TextLineMetricsMode::FontFamily);
    let mapped = formatter
        .format_line(&source, 1, 200.0, &paragraph, Some(&fork))
        .unwrap();
    let unchanged = formatter
        .format_line(&source, 1, 200.0, &paragraph, Some(&original.fork()))
        .unwrap();
    assert_eq!(mapped.length(), unchanged.length());
    assert_eq!(line_text(&*mapped), line_text(&*unchanged));
    assert_eq!(mapped.width() + 25.0, unchanged.width());
    assert_eq!(mapped.baseline(), unchanged.baseline());
    assert_eq!(scope.shaper().shape_call_count(), calls);
    assert!(original.remaining_runs.borrow().is_some());
    mapped.dispose();
    unchanged.dispose();
    first.dispose();
}

#[test]
fn continuation_objects_reject_length_and_drawable_contract_changes() {
    let original = TextLineBreak::new(None, FlowDirection::RightToLeft, true);
    original.is_wrapping.set(true);
    *original.remaining_runs.borrow_mut() = Some(vec![object(40.0)]);
    for replacement in [
        Rc::new(CustomDrawableRun::new(properties()).with_length(2)) as Rc<dyn TextRun>,
        Rc::new(InvisibleRun::new(1)),
    ] {
        let fork = original.fork_with_object_runs(|_| Some(replacement.clone()));
        let retained = fork.remaining_runs.borrow();
        let run = &retained.as_ref().unwrap()[0];
        assert_eq!(run.length(), 1);
        assert_eq!(run.as_drawable().unwrap().size().width, 40.0);
        assert_eq!(fork.flow_direction(), FlowDirection::RightToLeft);
        assert!(fork.is_split());
    }
}

#[test]
fn continuation_objects_allow_original_release_during_mapper_callback() {
    let scope = TextTestScope::new();
    let props = properties();
    let source = ListTextSource::new(vec![
        Rc::new(TextCharacters::new(
            ReadOnlyMemory::from_str("x"),
            props.clone(),
        )),
        object(40.0),
        Rc::new(TextCharacters::new(
            ReadOnlyMemory::from_str("tail"),
            props.clone(),
        )),
    ]);
    let paragraph: Rc<dyn TextParagraphProperties> =
        Rc::new(GenericTextParagraphProperties::with_options(
            props,
            TextAlignment::Left,
            TextWrapping::Wrap,
            f64::NAN,
            0.0,
        ));
    let formatter = <dyn TextFormatter>::current();
    let first = formatter
        .format_line(&source, 0, 10.0, &paragraph, None)
        .unwrap();
    let original = first.text_line_break().unwrap();
    let calls = scope.shaper().shape_call_count();
    let fork = original.fork_with_object_runs(|_| {
        original.release_remaining_runs();
        Some(object(15.0))
    });
    assert!(original.remaining_runs.borrow().is_none());
    let continuation = formatter
        .format_line(&source, 1, 200.0, &paragraph, Some(&fork))
        .unwrap();
    assert_eq!(line_text(&*continuation), "tail");
    assert_eq!(scope.shaper().shape_call_count(), calls);
    assert!(continuation
        .text_runs()
        .iter()
        .any(|run| run.is::<ShapedTextRun>()));
    continuation.dispose();
    first.dispose();
}

#[test]
fn continuation_objects_do_not_call_mapper_for_an_unwrapped_break() {
    let original = TextLineBreak::new(None, FlowDirection::LeftToRight, false);
    let fork = original.fork_with_object_runs(|_| panic!("no object to replace"));
    assert!(fork.remaining_runs.borrow().is_none());
    assert!(!fork.is_split());
}
