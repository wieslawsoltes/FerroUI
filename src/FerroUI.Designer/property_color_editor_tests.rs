use super::*;
use ferroui_base::input::KeyEventArgs;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn popup_button(editor: &PropertyColorEditor, label: &str) -> Ref<Button> {
    Control::from_boxed(&editor.flyout().content().unwrap())
        .unwrap()
        .get_logical_descendants()
        .filter_map(|control| control.cast::<Button>())
        .find(|button| {
            button
                .content()
                .as_ref()
                .and_then(|value| value.downcast_ref::<String>())
                .is_some_and(|value| value == label)
        })
        .unwrap()
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

fn key(text: &TextBox, key: Key) {
    let mut event = KeyEventArgs::new();
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.key = key;
    text.raise_event(&event);
}

#[test]
fn native_color_view_stages_spectrum_changes_and_commits_one_value() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = PropertyColorEditor::new().unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let commits = seen.clone();
    editor.on_commit(move |value| commits.borrow_mut().push(value));
    editor.configure(Some("Red"), false).unwrap();
    editor.color_view().set_color(Color::from_rgb(1, 2, 3));
    editor.color_view().set_color(Color::from_rgb(4, 5, 6));
    assert!(
        seen.borrow().is_empty(),
        "native spectrum changes are only a draft"
    );
    click(&popup_button(&editor, "Cancel"));
    assert!(
        seen.borrow().is_empty(),
        "cancel never authors the popup draft"
    );
    assert_eq!(editor.text_editor().text().as_deref(), Some("Red"));
    editor
        .color_view()
        .set_color(Color::from_argb(128, 12, 34, 56));
    click(&popup_button(&editor, "Apply color"));
    assert_eq!(*seen.borrow(), ["#800C2238"]);
}

#[test]
fn color_editor_preserves_named_colors_expressions_and_explicit_mixed_values() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = PropertyColorEditor::new().unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let commits = seen.clone();
    editor.on_commit(move |value| commits.borrow_mut().push(value));
    editor.configure(Some("Red"), false).unwrap();
    editor.color_view().set_color(Color::parse("Red").unwrap());
    click(&popup_button(&editor, "Apply color"));
    assert_eq!(
        *seen.borrow(),
        ["Red"],
        "the same color preserves named source spelling; explicit confirmation can materialize an inherited value"
    );
    editor
        .text_editor()
        .set_text(Some("{DynamicResource Accent}"));
    assert!(
        !editor.swatch_button().is_enabled(),
        "expressions are never flattened by the picker"
    );
    key(&editor.text_editor(), Key::Enter);
    assert_eq!(*seen.borrow(), ["Red", "{DynamicResource Accent}"]);
    key(&editor.text_editor(), Key::Escape);
    assert_eq!(editor.text_editor().text().as_deref(), Some("Red"));
    assert!(editor.swatch_button().is_enabled());
    assert!(editor.configure(Some("{Binding Brush}"), false).is_err());
    assert_eq!(editor.text_editor().text().as_deref(), Some("Red"));
    editor.configure(None, true).unwrap();
    editor
        .color_view()
        .set_color(Color::from_rgb(255, 255, 255));
    click(&popup_button(&editor, "Apply color"));
    assert_eq!(
        *seen.borrow(),
        ["Red", "{DynamicResource Accent}", "#FFFFFFFF"]
    );
}
