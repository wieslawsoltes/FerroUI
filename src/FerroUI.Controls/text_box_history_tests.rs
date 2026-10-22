use super::*;
use crate::presenters::TextPresenter;
use crate::text_box_tests::{templated_text_box, test_scope};
use ferroui_base::media::text_formatting::{
    GenericTextRunProperties, TextParagraphStyle, TextRunProperties,
};
use ferroui_base::media::Typeface;
use ferroui_base::utilities::ValueSpan;
use ferroui_base::{Ref, Size};
use std::cell::{Cell, RefCell};

#[derive(Debug)]
struct DocumentSnapshot {
    text: String,
    label: &'static str,
}

fn field(text: &str) -> Ref<TextBox> {
    let field = templated_text_box(Some(text));
    field.set_accepts_return(true);
    field.measure(Size::new(400.0, 240.0));
    field
}

fn snapshot(text: &str, label: &'static str) -> Rc<DocumentSnapshot> {
    Rc::new(DocumentSnapshot {
        text: text.into(),
        label,
    })
}

fn current(field: &TextBox) -> Rc<DocumentSnapshot> {
    field
        .text_undo_state()
        .expect("application snapshot")
        .downcast()
        .expect("document snapshot type")
}

fn character_styles(length: i32, size: f64) -> TextStyleOverrides {
    let properties: Rc<dyn TextRunProperties> = Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::default(),
        size,
    ));
    Rc::from([ValueSpan::new(0, length, properties)])
}

fn paragraph_styles(length: i32, indent: f64) -> TextParagraphStyles {
    Rc::from([ValueSpan::new(
        0,
        length,
        TextParagraphStyle {
            indent,
            ..Default::default()
        },
    )])
}

fn assign(
    field: &TextBox,
    state: Rc<DocumentSnapshot>,
    styles: TextStyleOverrides,
    paragraphs: TextParagraphStyles,
) {
    field.set_text_style_overrides(Some(styles));
    field.set_text_paragraph_styles(Some(paragraphs));
    field.set_text_undo_state(Some(state));
}

#[test]
fn native_rich_history_restores_both_styles_and_application_state_before_text_changing() {
    let _scope = test_scope();
    let field = field("🦀\nB");
    let original = snapshot("🦀\nB", "original");
    let edited = snapshot("B", "edited");
    let original_styles = character_styles(4, 30.0);
    let edited_styles = character_styles(1, 18.0);
    let original_paragraphs = paragraph_styles(3, 16.0);
    let edited_paragraphs = paragraph_styles(1, 4.0);
    assign(
        &field,
        original.clone(),
        original_styles.clone(),
        original_paragraphs.clone(),
    );
    let observed = Rc::new(RefCell::new(Vec::new()));
    let recorded = observed.clone();
    let weak = field.downgrade();
    let (after, after_styles, after_paragraphs) = (
        edited.clone(),
        edited_styles.clone(),
        edited_paragraphs.clone(),
    );
    field.text_changing(move |_, _| {
        let field = weak.upgrade().unwrap();
        if field.is_restoring_text_history() {
            let state = current(&field);
            assert_eq!(state.text, field.text().unwrap());
            recorded.borrow_mut().push((
                state.label,
                field.text_style_overrides().unwrap(),
                field.text_paragraph_styles().unwrap(),
            ));
        } else {
            assign(
                &field,
                after.clone(),
                after_styles.clone(),
                after_paragraphs.clone(),
            );
        }
    });
    field.set_selection_start(0);
    field.set_selection_end(3);
    field.set_selected_text(None);
    assert_eq!(field.text().as_deref(), Some("B"));
    assert!(Rc::ptr_eq(&current(&field), &edited));
    field.undo();
    assert_eq!(field.text().as_deref(), Some("🦀\nB"));
    assert!(Rc::ptr_eq(&current(&field), &original));
    assert!(Rc::ptr_eq(
        field.text_style_overrides().as_ref().unwrap(),
        &original_styles
    ));
    assert!(Rc::ptr_eq(
        field.text_paragraph_styles().as_ref().unwrap(),
        &original_paragraphs
    ));
    assert!(!field.is_restoring_text_history());
    field.redo();
    assert_eq!(field.text().as_deref(), Some("B"));
    assert!(Rc::ptr_eq(&current(&field), &edited));
    let observed = observed.borrow();
    assert_eq!(
        observed.iter().map(|entry| entry.0).collect::<Vec<_>>(),
        ["original", "edited"]
    );
    assert!(Rc::ptr_eq(&observed[0].1, &original_styles));
    assert!(Rc::ptr_eq(&observed[0].2, &original_paragraphs));
    assert!(Rc::ptr_eq(&observed[1].1, &edited_styles));
    assert!(Rc::ptr_eq(&observed[1].2, &edited_paragraphs));
    let presenter = field
        .find_descendant_of_type::<TextPresenter>(false)
        .unwrap();
    assert_eq!(presenter.text(), field.text());
}

fn record_native_edits(field: &Ref<TextBox>) {
    let weak = field.downgrade();
    field.text_changing(move |_, _| {
        let field = weak.upgrade().unwrap();
        if !field.is_restoring_text_history() {
            field.set_text_undo_state(Some(snapshot(&field.text().unwrap(), "edit")));
        }
    });
}

#[test]
fn application_snapshot_updates_neither_add_history_nor_overwrite_future_redo() {
    let _scope = test_scope();
    let field = field("A");
    field.set_text_undo_state(Some(snapshot("A", "initial")));
    for _ in 0..16 {
        field.set_text_undo_state(Some(snapshot("A", "style-only")));
    }
    assert!(!field.can_undo());
    assert!(!field.can_redo());
    record_native_edits(&field);
    field.set_caret_index(1);
    field.set_selected_text(Some("B"));
    field.set_selected_text(Some("C"));
    let final_state = current(&field);
    field.undo();
    assert_eq!(field.text().as_deref(), Some("AB"));
    assert!(field.can_redo());
    field.set_text_undo_state(Some(snapshot("AB", "updated while undone")));
    assert!(field.can_redo());
    field.redo();
    assert_eq!(field.text().as_deref(), Some("ABC"));
    assert!(Rc::ptr_eq(&current(&field), &final_state));
}

#[test]
fn rich_history_style_callback_replacement_preserves_newer_host_text_styles_and_selection() {
    let _scope = test_scope();
    let field = field("A");
    assign(
        &field,
        snapshot("A", "initial"),
        character_styles(1, 30.0),
        paragraph_styles(1, 16.0),
    );
    record_native_edits(&field);
    field.set_caret_index(1);
    field.set_selected_text(Some("B"));
    assign(
        &field,
        snapshot("AB", "edit"),
        character_styles(2, 18.0),
        paragraph_styles(2, 4.0),
    );
    let replacement = snapshot("host", "host");
    let replacement_styles = character_styles(4, 24.0);
    let replacement_paragraphs = paragraph_styles(4, 9.0);
    let replaced = Rc::new(Cell::new(false));
    let seen = replaced.clone();
    let weak = field.downgrade();
    let (state, styles, paragraphs) = (
        replacement.clone(),
        replacement_styles.clone(),
        replacement_paragraphs.clone(),
    );
    let _subscription = field.property_changed(move |event| {
        let field = weak.upgrade().unwrap();
        if field.is_restoring_text_history()
            && event.property() == TextBox::text_style_overrides_property().as_property()
            && !seen.replace(true)
        {
            field.set_text(Some("host"));
            assign(&field, state.clone(), styles.clone(), paragraphs.clone());
            field.set_caret_index(3);
            field.set_selection_start(1);
            field.set_selection_end(3);
        }
    });
    field.undo();
    assert!(replaced.get());
    assert_eq!(field.text().as_deref(), Some("host"));
    assert!(Rc::ptr_eq(&current(&field), &replacement));
    assert!(Rc::ptr_eq(
        field.text_style_overrides().as_ref().unwrap(),
        &replacement_styles
    ));
    assert!(Rc::ptr_eq(
        field.text_paragraph_styles().as_ref().unwrap(),
        &replacement_paragraphs
    ));
    assert_eq!(
        (
            field.caret_index(),
            field.selection_start(),
            field.selection_end()
        ),
        (3, 1, 3)
    );
}

#[test]
fn rich_history_text_callback_round_trip_is_not_overwritten_by_stale_caret() {
    let _scope = test_scope();
    let field = field("AAAA");
    field.set_text_undo_state(Some(snapshot("AAAA", "initial")));
    field.set_caret_index(3);
    record_native_edits(&field);
    field.set_selected_text(Some("B"));
    assert_eq!(field.text().as_deref(), Some("AAABA"));
    let replacement = snapshot("AAAA", "new handler state");
    let fired = Rc::new(Cell::new(false));
    let seen = fired.clone();
    let weak = field.downgrade();
    let state = replacement.clone();
    field.text_changing(move |_, _| {
        let field = weak.upgrade().unwrap();
        if field.is_restoring_text_history() && !seen.replace(true) {
            field.set_text(Some("temporary"));
            field.set_text(Some("AAAA"));
            field.set_text_undo_state(Some(state.clone()));
            field.set_caret_index(1);
            field.set_selection_start(0);
            field.set_selection_end(2);
        }
    });
    field.undo();
    assert!(fired.get());
    assert_eq!(field.text().as_deref(), Some("AAAA"));
    assert!(Rc::ptr_eq(&current(&field), &replacement));
    assert_eq!(
        (
            field.caret_index(),
            field.selection_start(),
            field.selection_end()
        ),
        (1, 0, 2)
    );
}

#[test]
fn rich_payloads_are_released_by_the_native_history_limit_and_clear() {
    let _scope = test_scope();
    let field = field("A");
    field.set_undo_limit(2);
    let first = snapshot("A", "first");
    let first_weak = Rc::downgrade(&first);
    field.set_text_undo_state(Some(first));
    record_native_edits(&field);
    field.set_caret_index(1);
    field.set_selected_text(Some("B"));
    field.set_selected_text(Some("C"));
    let final_weak = Rc::downgrade(&current(&field));
    field.undo();
    assert_eq!(field.text().as_deref(), Some("AB"));
    assert!(
        !field.can_undo(),
        "Only the two native checkpoints are retained"
    );
    assert!(first_weak.upgrade().is_none());
    field.set_is_undo_enabled(false);
    field.set_text_undo_state(None);
    assert!(final_weak.upgrade().is_none());
    assert!(field.text_undo_state().is_none());
}
