use super::TextPresenter;
use crate::test_support::test_scope;
use ferroui_base::media::text_formatting::{LogicalDirection, TextParagraphStyle};
use ferroui_base::media::{FlowDirection, TextAlignment, TextWrapping};
use ferroui_base::utilities::ValueSpan;
use ferroui_base::{Point, Size};
use std::rc::Rc;

#[test]
fn paragraph_caret_navigation_uses_real_spacing_and_first_line_offsets() {
    let _scope = test_scope();
    let field = TextPresenter::new();
    field.set_text(Some("AB\nCD"));
    field.set_font_size(12.0);
    field.set_line_height(20.0);
    field.set_text_wrapping(TextWrapping::Wrap);
    field.set_text_paragraph_styles(Some(Rc::from(vec![
        ValueSpan::new(0, 3, TextParagraphStyle { indent: 10.0, space_before: 4.0, space_after: 9.0, ..Default::default() }),
        ValueSpan::new(3, 2, TextParagraphStyle { line_height: Some(30.0), space_before: 7.0,
            text_alignment: Some(TextAlignment::Left), ..Default::default() }),
    ])));
    field.measure(Size::new(100.0, f64::INFINITY));
    field.move_caret_to_text_position(0, false);
    assert_eq!(field.cursor_rectangle().position(), Point::new(10.0, 4.0));
    field.move_caret_vertical(LogicalDirection::Forward);
    assert_eq!(field.cursor_rectangle().y, 40.0);
    assert_eq!(field.cursor_rectangle().height, 30.0);
    assert!(field.caret_index() >= 3);
    field.move_caret_vertical(LogicalDirection::Backward);
    assert_eq!(field.cursor_rectangle().y, 4.0);
    assert_eq!(field.cursor_rectangle().height, 20.0);
    let layout = field.text_layout();
    let styles: ferroui_base::media::text_formatting::TextParagraphStyles = Rc::from(vec![ValueSpan::new(0, 3, TextParagraphStyle {
        flow_direction: Some(FlowDirection::RightToLeft), ..Default::default()
    })]);
    field.set_text_paragraph_styles(Some(styles));
    assert!(!Rc::ptr_eq(&layout, &field.text_layout()), "Changing paragraph direction must invalidate cached shaping");
    assert_eq!(field.text_layout().line_flow_direction(0), FlowDirection::RightToLeft);
}

#[test]
fn paragraph_ime_insertion_styles_each_temporary_line_and_restores_the_original_layout() {
    let _scope = test_scope();
    let field = TextPresenter::new();
    field.set_text(Some("A🦀\nB"));
    let styles: ferroui_base::media::text_formatting::TextParagraphStyles = Rc::from(vec![ValueSpan::new(0, 4, TextParagraphStyle {
        indent: 8.0, space_before: 5.0, line_height: Some(23.0), ..Default::default()
    })]);
    field.set_text_paragraph_styles(Some(styles.clone()));
    field.set_caret_index(3);
    field.measure(Size::new(200.0, f64::INFINITY));
    let original_height = field.text_layout().height();
    field.set_preedit_text(Some("\n漢"));
    field.set_preedit_text_cursor_position(Some(2));
    let provisional = field.text_layout();
    assert_eq!(provisional.text_lines().len(), 3);
    assert_eq!(provisional.line_origin(0).x, 8.0);
    assert_eq!(provisional.line_origin(1).x, 8.0);
    assert_eq!(provisional.text_lines()[1].height(), 23.0);
    assert!(Rc::ptr_eq(field.text_paragraph_styles().as_ref().unwrap(), &styles));
    field.set_preedit_text(None);
    assert_eq!(field.text_layout().height(), original_height);
    assert_eq!(field.text_layout().text_lines().len(), 2);
}

#[test]
fn paragraph_preedit_replacement_uses_confirmed_ranges_and_blur_restores_the_collection() {
    use crate::{Button, StackPanel};
    use crate::test_support::TestRoot;
    use crate::test_support_buttons::focus_scope;
    use ferroui_base::input::InputElement;
    use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
    use ferroui_base::media::text_formatting::TextParagraphStyles;
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = crate::text_box_tests::templated_text_box(Some("A🦀\nB\nC"));
    target.set_clear_selection_on_lost_focus(false);
    let styles: TextParagraphStyles = Rc::from(vec![
        ValueSpan::new(0, 4, TextParagraphStyle { indent: 6.0, line_height: Some(20.0), ..Default::default() }),
        ValueSpan::new(4, 2, TextParagraphStyle { flow_direction: Some(FlowDirection::RightToLeft), line_height: Some(30.0), ..Default::default() }),
    ]);
    target.set_text_paragraph_styles(Some(styles.clone()));
    target.apply_template();
    let presenter = target.find_descendant_of_type::<TextPresenter>(false).unwrap();
    // The test template predates paragraph styles. The production Simple and
    // Fluent templates bind this property; mirror their exact value here.
    presenter.set_text_paragraph_styles(Some(styles.clone()));
    let other = Button::new();
    let panel = StackPanel::new();
    panel.children().add(target.clone());
    panel.children().add(other.clone());
    let root = TestRoot::with_child(panel);
    root.execute_initial_layout_pass();
    assert!(target.focus());
    target.set_selection_start(1);
    target.set_selection_end(5);
    let args = TextInputMethodClientRequestedEventArgs::new();
    args.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));
    target.raise_event(&args);
    let client = args.client().unwrap();
    client.set_preedit_text(Some("X"));
    assert_eq!(target.text().as_deref(), Some("A\nC"));
    client.set_preedit_text_with_cursor(Some("X\nY"), Some(3));
    let visible = presenter.text_layout();
    assert_eq!(visible.text_lines().len(), 3);
    assert_eq!(visible.line_origin(0).x, 6.0);
    assert_eq!(visible.line_origin(1).x, 6.0);
    assert_eq!(visible.line_flow_direction(1), FlowDirection::LeftToRight);
    assert_eq!(visible.text_lines()[1].height(), 20.0);
    assert!(other.focus());
    assert_eq!(target.text().as_deref(), Some("A🦀\nB\nC"));
    assert_eq!((target.selection_start(), target.selection_end()), (1,5));
    assert!(Rc::ptr_eq(target.text_paragraph_styles().as_ref().unwrap(), &styles));
    assert_eq!(presenter.text_layout().line_flow_direction(1), FlowDirection::RightToLeft);
}

#[test]
fn repeated_paragraph_ime_insertion_uses_caret_instead_of_a_text_diff() {
    let _scope = test_scope();
    let field = TextPresenter::new();
    field.set_text(Some("A\nA\n"));
    field.set_line_height(20.0);
    let styles: ferroui_base::media::text_formatting::TextParagraphStyles = Rc::from(vec![
        ValueSpan::new(0, 2, TextParagraphStyle { indent: 10.0, ..Default::default() }),
        ValueSpan::new(2, 2, TextParagraphStyle { indent: 20.0, ..Default::default() }),
        ValueSpan::new(4, 0, TextParagraphStyle { indent: 30.0, ..Default::default() }),
    ]);
    field.set_text_paragraph_styles(Some(styles.clone()));
    field.set_caret_index(0);
    field.measure(Size::new(200.0, f64::INFINITY));
    field.set_preedit_text(Some("A\n"));
    field.set_preedit_text_cursor_position(Some(2));
    let layout = field.text_layout();
    assert_eq!(layout.text_lines().len(), 4);
    assert_eq!((0..4).map(|index| layout.line_origin(index).x).collect::<Vec<_>>(), [10.0, 10.0, 20.0, 30.0]);
    assert_eq!(field.cursor_rectangle().y, layout.line_origin(1).y);
    assert_eq!(field.text().as_deref(), Some("A\nA\n"));
    assert!(Rc::ptr_eq(field.text_paragraph_styles().as_ref().unwrap(), &styles));
    field.set_preedit_text(None);
    let restored = field.text_layout();
    assert_eq!((0..3).map(|index| restored.line_origin(index).x).collect::<Vec<_>>(), [10.0, 20.0, 30.0]);
}

#[test]
fn repeated_paragraph_ime_replacement_uses_owned_selection_and_restores_on_blur() {
    use crate::{Button, StackPanel};
    use crate::test_support::TestRoot;
    use crate::test_support_buttons::focus_scope;
    use ferroui_base::input::InputElement;
    use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
    use ferroui_base::media::text_formatting::TextParagraphStyles;
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = crate::text_box_tests::templated_text_box(Some("A\nA\nA\n"));
    target.set_clear_selection_on_lost_focus(false);
    let styles: TextParagraphStyles = Rc::from(vec![
        ValueSpan::new(0, 2, TextParagraphStyle { indent: 10.0, ..Default::default() }),
        ValueSpan::new(2, 2, TextParagraphStyle { indent: 20.0, ..Default::default() }),
        ValueSpan::new(4, 2, TextParagraphStyle { indent: 30.0, ..Default::default() }),
        ValueSpan::new(6, 0, TextParagraphStyle { indent: 40.0, ..Default::default() }),
    ]);
    target.set_text_paragraph_styles(Some(styles.clone()));
    target.apply_template();
    let presenter = target.find_descendant_of_type::<TextPresenter>(false).unwrap();
    // This minimal test template predates the production theme binding.
    presenter.set_text_paragraph_styles(Some(styles.clone()));
    let other = Button::new();
    let panel = StackPanel::new();
    panel.children().add(target.clone()); panel.children().add(other.clone());
    let root = TestRoot::with_child(panel); root.execute_initial_layout_pass();
    assert!(target.focus());
    target.set_selection_start(0); target.set_selection_end(4);
    let args = TextInputMethodClientRequestedEventArgs::new();
    args.set_routed_event(Some(InputElement::text_input_method_client_requested_event()));
    target.raise_event(&args);
    let client = args.client().unwrap();
    client.set_preedit_text(Some("X"));
    assert_eq!(target.text().as_deref(), Some("A\n"));
    client.set_preedit_text_with_cursor(Some("A\nA\nA\n"), Some(6));
    let visible = presenter.text_layout();
    assert_eq!(visible.text_lines().len(), 5);
    assert_eq!((0..5).map(|index| visible.line_origin(index).x).collect::<Vec<_>>(), [10.0, 10.0, 10.0, 30.0, 40.0]);
    assert!(Rc::ptr_eq(target.text_paragraph_styles().as_ref().unwrap(), &styles));
    assert!(other.focus());
    assert_eq!(target.text().as_deref(), Some("A\nA\nA\n"));
    assert_eq!((target.selection_start(), target.selection_end()), (0, 4));
    let restored = presenter.text_layout();
    assert_eq!((0..4).map(|index| restored.line_origin(index).x).collect::<Vec<_>>(), [10.0, 20.0, 30.0, 40.0]);
}
