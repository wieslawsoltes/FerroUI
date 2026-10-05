//! Tests of the peers of windows, popups and native control hosts. The
//! reference has no unit tests for these peers; these cover the wiring of
//! the port.

use super::{
    AutomationControlType, AutomationPeer, ControlAutomationPeer, NativeControlHostPeer, PopupAutomationPeer,
    WindowAutomationPeer,
};
use crate::automation::provider::IRootProvider;
use crate::primitives::Popup;
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Button, Control, NativeControlHost, Window};
use ferroui_base::Ref;
use std::cell::Cell;
use std::rc::Rc;

fn create_peer(control: &Control) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}

#[test]
fn window_creates_window_automation_peer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    window.set_title(Some("Title".to_string()));

    let peer = create_peer(&window);

    assert!(std::ptr::eq(peer.get_type(), WindowAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::Window, peer.get_automation_control_type());
    assert_eq!("Title", peer.get_name());
    assert!(peer.get_provider::<dyn IRootProvider>().is_some());
    assert!(peer.get_parent().is_none());
}

#[test]
fn window_children_start_with_the_window_chrome_peer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::new();
    window.set_content(Some(Control::boxed(Button::new())));
    window.show();

    let peer = create_peer(&window);
    let children = peer.get_children();

    assert!(children.len() >= 2);
    let chrome = &children[0];
    assert_eq!(Some("FerroWindowChrome".to_string()), chrome.get_automation_id());
    assert_eq!("WindowChrome", chrome.get_name());
    assert_eq!("WindowChrome", chrome.get_class_name());
    assert_eq!(AutomationControlType::Group, chrome.get_automation_control_type());
    assert!(chrome.get_parent() == Some(peer.clone()));
    assert!(children[1..].iter().all(|child| child.get_parent() == Some(peer.clone())));

    window.close();
}

#[test]
fn window_peer_tracks_focus() {
    let _app = UnitTestApplication::start(TestServices::focusable_window());
    let window = Window::new();
    let button = Button::new();
    window.set_content(Some(Control::boxed(button.clone())));
    window.show();

    let peer = create_peer(&window);
    let root = peer.get_provider::<dyn IRootProvider>().expect("the window peer is a root");
    let raised = Rc::new(Cell::new(0));
    let _subscription = root.focus_changed({
        let raised = raised.clone();
        Rc::new(move || raised.set(raised.get() + 1))
    });

    button.focus();

    assert_eq!(1, raised.get());
    assert!(button.is_focused());
    assert!(root.get_focus() == Some(create_peer(&button)));

    window.close();

    // Closing the window stops the tracking.
    let raised_after_close = raised.get();
    button.focus();
    assert_eq!(raised_after_close, raised.get());
}

#[test]
fn popup_creates_popup_automation_peer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let popup = Popup::new();

    let peer = create_peer(&popup);

    assert!(std::ptr::eq(peer.get_type(), PopupAutomationPeer::TYPE));
    assert!(peer.get_children().is_empty());
    assert!(!peer.is_content_element());
    assert!(!peer.is_control_element());
}

#[test]
fn native_control_host_creates_native_control_host_peer() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = NativeControlHost::new();

    let peer = create_peer(&host);

    assert!(std::ptr::eq(peer.get_type(), NativeControlHostPeer::TYPE));
    assert!(peer.get_children().is_empty());
}
