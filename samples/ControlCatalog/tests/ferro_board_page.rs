//! Integration of the detachable keyboard in the real compiled catalog page.
use super::support::start_catalog_application;
use crate::pages::FerroBoardPage;
use ferroboard_controls::{ScreenKeyboardHost, input::ProtectedPasswordTextBox};
use ferroboard_core::editor::InputKind;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    FerroLocator,
    input::{IKeyboardDevice, KeyboardDevice},
};
use ferroui_controls::{Control, TextBox, Window};
use std::rc::Rc;

#[test]
fn ferro_board_page_routes_native_fields_and_keeps_demo_state_private() {
    let _app = start_catalog_application();
    // Styled-window services omit input devices. Use the real SDK keyboard for focus routing.
    FerroLocator::current_mutable()
        .bind::<dyn IKeyboardDevice>()
        .to_constant(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>);
    let page = FerroBoardPage::new();
    let host = page.get_control::<ScreenKeyboardHost>("KeyboardHost");
    let message = page.get_control::<TextBox>("KeyboardMessage");
    let email = page.get_control::<TextBox>("KeyboardEmail");
    let password = page.get_control::<ProtectedPasswordTextBox>("KeyboardPassword");
    let window = Window::new();
    window.set_width(1100.0);
    window.set_height(1000.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);
    let engine = host.keyboard().engine();
    assert!(engine.settings().incognito_mode());
    assert!(!engine.settings().learn_words());
    assert!(!engine.settings().clipboard_history_enabled());
    assert!(!engine.settings().emoji_history_enabled());
    assert!(message.focus());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(host.is_keyboard_open());
    engine.type_text(&"Hello".into()).unwrap();
    assert_eq!(message.text().as_deref(), Some("Hello"));
    assert!(message.is_focused());
    assert!(email.focus());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(engine.attributes().kind(), InputKind::Email);
    engine.type_text(&"a@example.com".into()).unwrap();
    assert_eq!(email.text().as_deref(), Some("a@example.com"));
    assert_eq!(message.text().as_deref(), Some("Hello"));
    assert!(password.focus());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(engine.attributes().is_password());
    window.close();
}
