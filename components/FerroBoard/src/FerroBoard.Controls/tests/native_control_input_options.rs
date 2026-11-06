#[path = "support/headless.rs"]
mod headless;

use ferroui_base::{
    Ref,
    input::{
        InputElement,
        text_input::{
            TextInputContentType as Content, TextInputMethodClientRequestedEventArgs,
            TextInputOptions,
        },
    },
};
use ferroui_controls::TextBox;

fn native_options(field: &Ref<TextBox>) -> TextInputOptions {
    let request = TextInputMethodClientRequestedEventArgs::new();
    request.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    field.raise_event(&request);
    request.client().unwrap().input_options(
        &field.clone().upcast(),
        TextInputOptions::from_styled_element(field),
    )
}

#[test]
fn sensitive_plain_text_remains_multiline_without_password_semantics() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_accepts_return(true);
        TextInputOptions::set_multiline(&field, true);
        TextInputOptions::set_is_sensitive(&field, true);
        let options = native_options(&field);
        assert_eq!(options.content_type, Content::Normal);
        assert!(options.multiline && options.is_sensitive);
        assert_eq!(field.password_char(), '\0');
        let attributes =
            ferroboard_controls::input::TextBoxInputTarget::compute_attributes(&field).unwrap();
        assert!(attributes.is_private() && attributes.is_multiline());
        assert!(!attributes.is_password());
    });
}

#[test]
fn masked_control_derives_secure_options_without_mutating_content_type() {
    headless::run(|| {
        for content in [
            Content::Normal,
            Content::Name,
            Content::Digits,
            Content::Number,
            Content::Pin,
        ] {
            let field = TextBox::new();
            TextInputOptions::set_content_type(&field, content);
            TextInputOptions::set_multiline(&field, true);
            TextInputOptions::set_show_suggestions(&field, Some(true));
            field.set_password_char('●');
            let expected = if matches!(content, Content::Digits | Content::Number | Content::Pin) {
                Content::Pin
            } else {
                Content::Password
            };
            for reveal in [false, true] {
                field.set_reveal_password(reveal);
                let options = native_options(&field);
                assert_eq!(options.content_type, expected);
                assert!(options.is_sensitive);
                assert!(!options.multiline);
                assert_eq!(options.show_suggestions, Some(false));
                assert_eq!(TextInputOptions::get_content_type(&field), content);
                assert!(TextInputOptions::get_multiline(&field));
                assert_eq!(TextInputOptions::get_show_suggestions(&field), Some(true));
            }
            field.set_password_char('\0');
            let options = native_options(&field);
            assert_eq!(options.content_type, content);
            assert!(options.multiline);
            assert!(!options.is_sensitive);
        }
    });
}

#[test]
fn mask_changes_request_native_refresh_and_respect_subscription_disposal() {
    headless::run(|| {
        use std::{cell::Cell, rc::Rc};
        let field = TextBox::new();
        let request = TextInputMethodClientRequestedEventArgs::new();
        request.set_routed_event(Some(
            InputElement::text_input_method_client_requested_event(),
        ));
        field.raise_event(&request);
        let client = request.client().unwrap();
        let count = Rc::new(Cell::new(0));
        let calls = count.clone();
        let subscription = client.reset_requested(Rc::new(move || calls.set(calls.get() + 1)));
        field.set_password_char('●');
        assert_eq!(count.get(), 1);
        field.set_reveal_password(true);
        assert_eq!(count.get(), 1);
        field.set_password_char('\0');
        assert_eq!(count.get(), 2);
        subscription.dispose();
        field.set_password_char('●');
        assert_eq!(count.get(), 2);
    });
}
