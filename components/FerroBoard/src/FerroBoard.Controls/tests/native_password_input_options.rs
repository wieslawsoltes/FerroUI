#[path = "support/headless.rs"]
mod headless;

use ferroboard_controls::input::{PasswordBoxInputTarget, ProtectedPasswordTextBox};
use ferroboard_core::{editor::ITextInputTarget, text::Utf16Text};
use ferroui_base::input::text_input::{TextInputContentType, TextInputOptions, TextSelection};
use ferroui_base::{FerroObject, FerroObjectExtensions, reactive::ObservableExt};
use ferroui_controls::TextBox;
use std::{cell::Cell, rc::Rc};

#[test]
fn reentrant_mask_removal_releases_the_native_privacy_value() {
    headless::run(|| {
        let field = TextBox::new();
        let once = Rc::new(Cell::new(false));
        let flag = once.clone();
        let receiver = field.downgrade();
        let object: &FerroObject = &field;
        let subscription = FerroObjectExtensions::get_observable(
            object,
            TextInputOptions::is_sensitive_property(),
        )
        .subscribe_fn(move |private| {
            if private && !flag.replace(true) {
                receiver.upgrade().unwrap().set_password_char('\0');
            }
        });
        field.set_password_char('●');
        assert!(once.get());
        assert_eq!(field.password_char(), '\0');
        assert!(!TextInputOptions::get_is_sensitive(&field));
        subscription.dispose();
    });
}

#[test]
fn masked_native_fields_remain_sensitive_through_reveal_and_restore_original_options() {
    headless::run(|| {
        let field = TextBox::new();
        TextInputOptions::set_is_sensitive(&field, false);
        TextInputOptions::set_content_type(&field, TextInputContentType::Pin);
        field.set_password_char('●');
        assert!(TextInputOptions::get_is_sensitive(&field));
        TextInputOptions::set_is_sensitive(&field, false);
        assert!(TextInputOptions::get_is_sensitive(&field));
        field.set_reveal_password(true);
        assert!(TextInputOptions::get_is_sensitive(&field));
        assert_eq!(
            TextInputOptions::get_content_type(&field),
            TextInputContentType::Pin
        );
        field.set_password_char('\0');
        assert!(!TextInputOptions::get_is_sensitive(&field));
    });
}

#[test]
fn protected_password_defaults_and_unmasked_mode_keep_native_input_private() {
    headless::run(|| {
        let field = ProtectedPasswordTextBox::new();
        assert_eq!(
            TextInputOptions::get_content_type(&field),
            TextInputContentType::Password
        );
        assert!(TextInputOptions::get_is_sensitive(&field));
        field.set_password_char('\0');
        TextInputOptions::set_is_sensitive(&field, false);
        assert!(TextInputOptions::get_is_sensitive(&field));
        TextInputOptions::set_content_type(&field, TextInputContentType::Pin);
        assert_eq!(
            TextInputOptions::get_content_type(&field),
            TextInputContentType::Pin
        );
    });
}

#[test]
fn password_replacement_updates_the_real_native_utf16_caret_without_own_notifications() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_password_char('●');
        let target = PasswordBoxInputTarget::new(field.clone(), None).unwrap();
        let changes = Rc::new(Cell::new(0));
        let count = changes.clone();
        target
            .changed()
            .add(Rc::new(move |_| count.set(count.get() + 1)));
        target.try_replace(0, 0, &"a😀".into()).unwrap();
        assert_eq!(
            (
                field.caret_index(),
                field.selection_start(),
                field.selection_end()
            ),
            (3, 3, 3)
        );
        target.try_replace(1, 2, &"β".into()).unwrap();
        assert_eq!(field.text().as_deref(), Some("aβ"));
        assert_eq!(
            (
                field.caret_index(),
                field.selection_start(),
                field.selection_end()
            ),
            (2, 2, 2)
        );
        assert_eq!(changes.get(), 0);
    });
}

#[test]
fn reentrant_external_password_edit_keeps_its_native_caret() {
    headless::run(|| {
        let field = TextBox::new();
        field.set_password_char('●');
        let target = PasswordBoxInputTarget::new(field.clone(), None).unwrap();
        let receiver = field.downgrade();
        field.text_changing(move |_, _| {
            let field = receiver.upgrade().unwrap();
            if field.text().as_deref() == Some("q") {
                field.set_text(Some("external"));
                field.set_selection_and_caret(TextSelection::new(3, 3), 3);
            }
        });
        target.try_replace(0, 0, &"q".into()).unwrap();
        assert_eq!(target.text(), Utf16Text::from("external"));
        assert_eq!(field.caret_index(), 3);
    });
}
