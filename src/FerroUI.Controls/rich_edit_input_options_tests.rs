use super::*;
use ferroui_base::input::text_input::TextInputContentType;
use ferroui_base::input::{InputScope, InputScopeName, InputScopeNameValue as Scope};

#[test]
fn rich_input_scope_and_prediction_changes_reach_the_focused_backend_without_resetting_composition()
{
    let f = Fixture::new(true);
    let connection = f.connection();
    let session = connection.begin_composition().unwrap();
    session.update("preview", None).unwrap();
    let before = f.owner.document().snapshot();
    let resets = f.native.resets.get();
    let installations = f.native.installations.get();

    f.owner.set_input_scope(TextInputContentType::Email);
    f.owner.set_is_text_prediction_enabled(false);
    assert_eq!(
        f.native.options.borrow().effective_content_type(),
        TextInputContentType::Email
    );
    assert_eq!(f.native.options.borrow().show_suggestions, Some(false));
    assert_eq!(f.native.resets.get(), resets);
    assert_eq!(f.native.installations.get(), installations);
    assert!(Rc::ptr_eq(&connection, &f.connection()));
    assert!(session.is_current());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());

    assert!(session.commit("accepted", 0).unwrap());
    assert_eq!(f.owner.text(), "aacceptedd");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn rich_input_options_follow_the_semantic_owner_while_its_flyout_has_keyboard_focus() {
    for overlay in [false, true] {
        let f = Fixture::new(overlay);
        let connection = f.connection();
        let session = connection.begin_composition().unwrap();
        session.update("preview", None).unwrap();
        let child = Border::new();
        child.set_focusable(true);
        child.set_width(60.0);
        child.set_height(20.0);
        TextInputOptions::set_content_type(&child, TextInputContentType::Digits);
        let flyout = f.flyout(child.clone().upcast());
        let resets = f.native.resets.get();
        let installations = f.native.installations.get();
        flyout.show_at(&f.owner);
        assert!(child.is_focused());

        f.owner.set_input_scope(TextInputContentType::Url);
        f.owner.set_is_text_prediction_enabled(false);
        assert_eq!(
            f.native.options.borrow().effective_content_type(),
            TextInputContentType::Url
        );
        assert_eq!(f.native.options.borrow().show_suggestions, Some(false));
        TextInputOptions::set_content_type(&child, TextInputContentType::Number);
        assert_eq!(
            f.native.options.borrow().effective_content_type(),
            TextInputContentType::Url
        );
        assert_eq!(f.native.resets.get(), resets);
        assert_eq!(f.native.installations.get(), installations);
        assert!(Rc::ptr_eq(&connection, &f.connection()));
        assert!(session.is_current());
        assert_eq!(f.owner.text(), "apreviewd");
        assert!(session.cancel().unwrap());

        f.outside.focus();
        assert!(!connection.is_current());
        let last_options = f.native.options.borrow().clone();
        f.owner.set_input_scope(TextInputContentType::Email);
        f.owner.set_is_text_prediction_enabled(true);
        assert_eq!(*f.native.options.borrow(), last_options);
        flyout.hide();
    }
}

#[test]
fn rich_input_scope_live_names_reach_the_backend_as_immutable_snapshots() {
    let f = Fixture::new(true);
    let connection = f.connection();
    let session = connection.begin_composition().unwrap();
    session.update("preview", None).unwrap();
    let scope = InputScope::with_name_value(Scope::Hiragana);
    let first = scope.names().get(0);
    scope.names().add(InputScopeName::with_name_value(Scope::Text));
    f.owner.set_input_scope(scope.clone());
    let previous = f.native.options.borrow().clone();
    let resets = f.native.resets.get();
    let installations = f.native.installations.get();

    first.set_name_value(Scope::KatakanaFullWidth);
    assert_eq!(f.native.options.borrow().effective_input_scope(), Some(Scope::KatakanaFullWidth));
    assert_eq!(previous.effective_input_scope(), Some(Scope::Hiragana));
    scope.names().move_item(1, 0);
    assert_eq!(f.native.options.borrow().input_scope_names.as_deref(), Some(&[Scope::Text, Scope::KatakanaFullWidth][..]));
    scope.names().remove_at(0);
    assert_eq!(f.native.options.borrow().effective_input_scope(), Some(Scope::KatakanaFullWidth));
    assert_eq!(f.native.resets.get(), resets);
    assert_eq!(f.native.installations.get(), installations);
    assert!(Rc::ptr_eq(&connection, &f.connection()));
    assert!(session.is_current());
    assert!(session.cancel().unwrap());
}

#[test]
fn rich_input_scope_replacement_and_removal_stop_old_scope_updates() {
    let f = Fixture::new(true);
    let old = InputScope::with_name_value(Scope::EmailSmtpAddress);
    let current = InputScope::with_name_value(Scope::Url);
    f.owner.set_input_scope(old.clone());
    f.owner.set_input_scope(current.clone());
    old.names().get(0).set_name_value(Scope::Number);
    assert_eq!(f.native.options.borrow().effective_input_scope(), Some(Scope::Url));
    current.names().get(0).set_name_value(Scope::Search);
    assert_eq!(f.native.options.borrow().effective_input_scope(), Some(Scope::Search));
    f.owner.set_input_scope(None);
    let previous = f.native.options.borrow().clone();
    current.names().get(0).set_name_value(Scope::Number);
    assert_eq!(*f.native.options.borrow(), previous);
}

#[test]
fn rich_input_scope_clear_restores_legacy_hints_without_overwriting_them() {
    let f = Fixture::new(true);
    TextInputOptions::set_content_type(&f.window, TextInputContentType::Email);
    assert_eq!(f.native.options.borrow().effective_content_type(), TextInputContentType::Email);
    let scope = InputScope::with_name_value(Scope::TelephoneNumber);
    f.owner.set_input_scope(scope.clone());
    assert_eq!(f.native.options.borrow().effective_content_type(), TextInputContentType::Digits);
    assert_eq!(TextInputOptions::get_content_type(&f.owner), TextInputContentType::Email);
    TextInputOptions::set_content_type(&f.window, TextInputContentType::Url);
    assert_eq!(f.native.options.borrow().effective_content_type(), TextInputContentType::Digits);
    f.owner.set_input_scope(None);
    assert_eq!(f.native.options.borrow().effective_content_type(), TextInputContentType::Url);
    f.owner.set_is_text_prediction_enabled(false);
    assert_eq!(TextInputOptions::get_content_type(&f.owner), TextInputContentType::Url);
    f.owner.set_input_scope(scope.clone());
    scope.names().clear();
    assert_eq!(f.native.options.borrow().effective_content_type(), TextInputContentType::Url);
}

#[test]
fn rich_input_scope_shared_objects_update_each_owner_without_owning_it() {
    let f = Fixture::new(true);
    let scope = InputScope::with_name_value(Scope::Number);
    let other = RichEditBox::new();
    other.set_input_scope(scope.clone());
    f.owner.set_input_scope(scope.clone());
    scope.names().get(0).set_name_value(Scope::Url);
    assert_eq!(TextInputOptions::from_styled_element(&other).effective_input_scope(), Some(Scope::Url));
    assert_eq!(f.native.options.borrow().effective_input_scope(), Some(Scope::Url));
    let weak = other.downgrade();
    drop(other);
    assert!(weak.upgrade().is_none());
    scope.names().get(0).set_name_value(Scope::EmailSmtpAddress);
    assert_eq!(f.native.options.borrow().effective_input_scope(), Some(Scope::EmailSmtpAddress));
}

#[test]
fn rich_input_scope_live_names_remain_with_the_owner_during_flyout_focus() {
    for overlay in [false, true] {
        let f = Fixture::new(overlay);
        let scope = InputScope::with_name_value(Scope::Hiragana);
        f.owner.set_input_scope(scope.clone());
        let child = Border::new();
        child.set_focusable(true);
        child.set_width(60.0);
        child.set_height(20.0);
        TextInputOptions::set_input_scope_names(&child, Some(Rc::from([Scope::Url])));
        let flyout = f.flyout(child.clone().upcast());
        flyout.show_at(&f.owner);
        assert!(child.is_focused());
        scope.names().get(0).set_name_value(Scope::ChatWithoutEmoji);
        assert_eq!(f.native.options.borrow().effective_input_scope(), Some(Scope::ChatWithoutEmoji));
        f.outside.focus();
        let last = f.native.options.borrow().clone();
        scope.names().clear();
        assert_eq!(*f.native.options.borrow(), last);
        flyout.hide();
    }
}
