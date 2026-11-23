//! The password, its mask and its reveal modes.

use crate::{register_types, PasswordBox, PasswordRevealMode};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use std::cell::Cell;
use std::rc::Rc;

fn start() -> ferroui_controls::testing::UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start(TestServices::styled_window())
}

#[test]
fn the_text_is_masked_by_default() {
    let _app = start();
    assert_eq!('\u{25CF}', PasswordBox::new().password_char());
}

#[test]
fn the_password_and_the_text_are_the_same_value() {
    let _app = start();
    let target = PasswordBox::new();

    target.set_password("secret");
    assert_eq!(Some("secret".to_string()), target.text());

    target.set_text(Some("other"));
    assert_eq!("other", target.password());
}

#[test]
fn a_change_raises_password_changed_once() {
    let _app = start();
    let target = PasswordBox::new();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let _changed = target.password_changed(move || seen.set(seen.get() + 1));

    target.set_password("one");
    assert_eq!(1, count.get());
    target.set_text(Some("two"));
    assert_eq!(2, count.get());
}

#[test]
fn a_handler_can_clear_an_invalid_password() {
    let _app = start();
    let target = PasswordBox::new();
    let weak = target.downgrade();
    let _changed = target.password_changed(move || {
        if let Some(target) = weak.upgrade() {
            if target.password() == "Password" {
                target.set_password("");
            }
        }
    });

    target.set_password("Password");
    assert_eq!("", target.password());
    assert_eq!(Some(String::new()), target.text().or(Some(String::new())));
}

#[test]
fn the_reveal_modes_set_the_button_and_the_mask() {
    let _app = start();
    let target = PasswordBox::new();
    assert!(target.classes().contains("revealPasswordButton"));
    assert!(!target.reveal_password());

    target.set_password_reveal_mode(PasswordRevealMode::Visible);
    assert!(!target.classes().contains("revealPasswordButton"));
    assert!(target.reveal_password());

    target.set_password_reveal_mode(PasswordRevealMode::Hidden);
    assert!(!target.classes().contains("revealPasswordButton"));
    assert!(!target.reveal_password());
}
