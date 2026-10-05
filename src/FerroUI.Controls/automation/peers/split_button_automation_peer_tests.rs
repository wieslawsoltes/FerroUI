//! Port of the reference `SplitButtonAutomationPeerTests` and
//! `ToggleSplitButtonAutomationPeerTests`.

use super::{
    AutomationControlType, AutomationPeer, ControlAutomationPeer, SplitButtonAutomationPeer,
    ToggleSplitButtonAutomationPeer,
};
use crate::automation::provider::{IExpandCollapseProvider, IInvokeProvider, IToggleProvider, ToggleState};
use crate::automation::{AutomationPropertyChangedEventArgs, ExpandCollapsePatternIdentifiers, ExpandCollapseState};
use crate::test_support::test_scope;
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Control, Flyout, SplitButton, ToggleSplitButton, Window};
use ferroui_base::{AnyValue, BoxedValue, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn create_peer(control: &Control) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}

fn state_of(value: Option<&BoxedValue>) -> Option<ExpandCollapseState> {
    let value: &dyn AnyValue = &**value?;
    value.downcast_ref::<ExpandCollapseState>().copied()
}

mod split_button_automation_peer_tests {
    use super::*;

    #[test]
    fn creates_split_button_automation_peer() {
        let _scope = test_scope();
        let target = SplitButton::new();
        let peer = create_peer(&target);

        assert!(std::ptr::eq(peer.get_type(), SplitButtonAutomationPeer::TYPE));
    }

    #[test]
    fn implements_i_expand_collapse_provider() {
        let _scope = test_scope();
        let target = SplitButton::new();
        let peer = create_peer(&target);

        assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
    }

    #[test]
    fn implements_i_invoke_provider() {
        let _scope = test_scope();
        let target = SplitButton::new();
        let peer = create_peer(&target);

        assert!(peer.get_provider::<dyn IInvokeProvider>().is_some());
    }

    #[test]
    fn control_type_is_split_button() {
        let _scope = test_scope();
        let target = SplitButton::new();
        let peer = create_peer(&target).cast::<SplitButtonAutomationPeer>().unwrap();

        assert_eq!(AutomationControlType::SplitButton, peer.get_automation_control_type());
    }

    #[test]
    fn class_name_is_split_button() {
        let _scope = test_scope();
        let target = SplitButton::new();
        let peer = create_peer(&target).cast::<SplitButtonAutomationPeer>().unwrap();

        assert_eq!("SplitButton", peer.get_class_name());
    }

    #[test]
    fn shows_menu_is_true() {
        let _scope = test_scope();
        let target = SplitButton::new();
        let peer = create_peer(&target).get_provider::<dyn IExpandCollapseProvider>().unwrap();

        assert!(peer.shows_menu());
    }

    #[test]
    fn invoke_triggers_click() {
        let _scope = test_scope();
        let clicked = Rc::new(Cell::new(0));
        let target = SplitButton::new();
        let peer = create_peer(&target).get_provider::<dyn IInvokeProvider>().unwrap();

        let counter = clicked.clone();
        target.click(move |_, _| counter.set(counter.get() + 1));
        peer.invoke().unwrap();

        assert_eq!(1, clicked.get());
    }

    #[test]
    fn expand_collapse_state_tracks_flyout() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let target = SplitButton::new();
        target.set_flyout(&Flyout::new());
        let window = Window::new();
        window.set_content(Some(Control::boxed(&target)));
        window.show();

        let peer = create_peer(&target).cast::<SplitButtonAutomationPeer>().unwrap();

        assert_eq!(ExpandCollapseState::Collapsed, peer.expand_collapse_state());

        peer.expand().unwrap();
        assert_eq!(ExpandCollapseState::Expanded, peer.expand_collapse_state());
        assert_eq!(Some(true), target.flyout().map(|flyout| flyout.is_open()));

        peer.collapse().unwrap();
        assert_eq!(ExpandCollapseState::Collapsed, peer.expand_collapse_state());
        assert!(!target.flyout().unwrap().is_open());
    }

    #[test]
    fn expand_collapse_raises_property_changed() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let target = SplitButton::new();
        target.set_flyout(&Flyout::new());
        let window = Window::new();
        window.set_content(Some(Control::boxed(&target)));
        window.show();

        let peer = create_peer(&target).cast::<SplitButtonAutomationPeer>().unwrap();
        let changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(None));
        let sink = changed.clone();
        peer.property_changed(move |e| {
            if e.property() == ExpandCollapsePatternIdentifiers::expand_collapse_state_property() {
                *sink.borrow_mut() = Some(e.clone());
            }
        });

        peer.expand().unwrap();

        let changed = changed.borrow();
        assert!(changed.is_some());
        let changed = changed.as_ref().unwrap();
        assert_eq!(Some(ExpandCollapseState::Collapsed), state_of(changed.old_value()));
        assert_eq!(Some(ExpandCollapseState::Expanded), state_of(changed.new_value()));
    }
}

mod toggle_split_button_automation_peer_tests {
    use super::*;

    #[test]
    fn creates_toggle_split_button_automation_peer() {
        let _scope = test_scope();
        let target = ToggleSplitButton::new();
        let peer = create_peer(&target);

        assert!(std::ptr::eq(peer.get_type(), ToggleSplitButtonAutomationPeer::TYPE));
    }

    #[test]
    fn implements_i_toggle_provider() {
        let _scope = test_scope();
        let target = ToggleSplitButton::new();
        let peer = create_peer(&target);

        assert!(peer.get_provider::<dyn IToggleProvider>().is_some());
    }

    #[test]
    fn control_type_is_split_button() {
        let _scope = test_scope();
        let target = ToggleSplitButton::new();
        let peer = create_peer(&target).cast::<SplitButtonAutomationPeer>().unwrap();

        assert_eq!(AutomationControlType::SplitButton, peer.get_automation_control_type());
    }

    #[test]
    fn class_name_is_toggle_split_button() {
        let _scope = test_scope();
        let target = ToggleSplitButton::new();
        let peer = create_peer(&target).cast::<ToggleSplitButtonAutomationPeer>().unwrap();

        assert_eq!("ToggleSplitButton", peer.get_class_name());
    }

    #[test]
    fn toggle_changes_is_checked_and_fires_click() {
        let _scope = test_scope();
        let clicked = Rc::new(Cell::new(0));
        let target = ToggleSplitButton::new();
        let peer = create_peer(&target).get_provider::<dyn IToggleProvider>().unwrap();

        assert_eq!(ToggleState::Off, peer.toggle_state());

        let counter = clicked.clone();
        target.click(move |_, _| counter.set(counter.get() + 1));
        peer.toggle().unwrap();

        assert!(target.is_checked());
        assert_eq!(ToggleState::On, peer.toggle_state());
        assert_eq!(1, clicked.get());
    }
}
