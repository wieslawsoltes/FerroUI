use crate::content_dialog::{button_state, BUTTON_STATES};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, ContentDialog, ContentDialogButton, ContentDialogError, ContentDialogResult, Window};
use ferroui_base::input::{Key, KeyEventArgs, KeyModifiers, InputElement};
use ferroui_base::Ref;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn window() -> Ref<Window> {
    let window = Window::new();
    window.show();
    window.update_layout();
    window
}

fn dialog() -> Ref<ContentDialog> {
    let dialog = ContentDialog::new();
    dialog.set_primary_button_text(Some("Save"));
    dialog.set_secondary_button_text(Some("Don't Save"));
    dialog.set_close_button_text(Some("Cancel"));
    dialog
}

fn press(dialog: &ContentDialog, key: Key) {
    let mut args = KeyEventArgs::new();
    args.set_routed_event(Some(InputElement::key_down_event()));
    args.key = key;
    args.key_modifiers = KeyModifiers::NONE;
    dialog.raise_event(&args);
}

#[test]
fn button_states_cover_every_combination() {
    let mut seen = Vec::new();
    for primary in [false, true] {
        for secondary in [false, true] {
            for close in [false, true] {
                seen.push(button_state(primary, secondary, close));
            }
        }
    }
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), BUTTON_STATES.len());
    assert!(seen.iter().all(|state| BUTTON_STATES.contains(state)));
}

#[test]
fn a_dialog_needs_a_window_and_a_window_shows_one_at_a_time() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let orphan = Button::new();
    assert_eq!(dialog().show_async(&orphan).err(), Some(ContentDialogError::NoHost));

    let window = window();
    let first = dialog();
    let operation = first.show_async(&window).unwrap();
    assert!(first.is_open());
    assert_eq!(first.show_async(&window).err(), Some(ContentDialogError::AlreadyOpen));
    assert_eq!(dialog().show_async(&window).err(), Some(ContentDialogError::AnotherDialogOpen));

    first.hide();
    assert_eq!(operation.result(), Some(ContentDialogResult::None));
    assert!(!first.is_open());
    assert!(dialog().show_async(&window).is_ok());
    window.close();
}

#[test]
fn escape_closes_through_the_close_button_and_enter_invokes_the_default() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = window();
    let target = dialog();
    let clicks = Rc::new(Cell::new(0));
    let count = clicks.clone();
    target.close_button_click(move |_| count.set(count.get() + 1)).detach();
    let operation = target.show_async(&window).unwrap();
    press(&target, Key::Escape);
    assert_eq!(clicks.get(), 1);
    assert_eq!(operation.result(), Some(ContentDialogResult::None));

    target.set_default_button(ContentDialogButton::Primary);
    let operation = target.show_async(&window).unwrap();
    press(&target, Key::Enter);
    assert_eq!(operation.result(), Some(ContentDialogResult::Primary));

    target.set_is_primary_button_enabled(false);
    let operation = target.show_async(&window).unwrap();
    press(&target, Key::Enter);
    assert_eq!(operation.result(), None, "a disabled default button does nothing");
    target.hide();
    window.close();
}

#[test]
fn closing_can_be_cancelled_and_deferred() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = window();
    let target = dialog();
    let cancel = Rc::new(Cell::new(true));
    let held = Rc::new(RefCell::new(None));
    let (flag, keep) = (cancel.clone(), held.clone());
    target
        .closing(move |args| {
            if flag.get() {
                args.set_cancel(true);
            } else {
                *keep.borrow_mut() = Some(args.get_deferral());
            }
        })
        .detach();
    let operation = target.show_async(&window).unwrap();

    target.hide();
    assert!(target.is_open(), "the handler cancelled");

    cancel.set(false);
    target.hide();
    assert!(target.is_open(), "the deferral holds it");
    held.borrow_mut().take().unwrap().complete();
    assert_eq!(operation.result(), Some(ContentDialogResult::None));
    assert!(!target.is_open());
    window.close();
}

#[test]
fn a_dropped_deferral_does_not_leave_the_dialog_open() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = window();
    let target = dialog();
    target.primary_button_click(|args| drop(args.get_deferral())).detach();
    target.set_default_button(ContentDialogButton::Primary);
    let operation = target.show_async(&window).unwrap();
    press(&target, Key::Enter);
    assert_eq!(operation.result(), Some(ContentDialogResult::Primary));
    window.close();
}

#[test]
fn closing_the_window_ends_the_dialog() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = window();
    let target = dialog();
    let closed = Rc::new(Cell::new(false));
    let flag = closed.clone();
    target.closed(move |_| flag.set(true)).detach();
    let operation = target.show_async(&window).unwrap();
    window.close();
    assert_eq!(operation.result(), Some(ContentDialogResult::None));
    assert!(closed.get());
    assert!(!target.is_open());
}
