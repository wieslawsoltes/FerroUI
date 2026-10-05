//! Port of the reference `ComplexControlAutomationPeerTests` (the classes
//! of the controls this library has).

use super::{AutoCompleteBoxAutomationPeer, AutomationControlType, AutomationPeer, ControlAutomationPeer};
use crate::automation::provider::{IExpandCollapseProvider, IInvokeProvider, IValueProvider};
use crate::automation::{
    AutomationPropertyChangedEventArgs, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
    ValuePatternIdentifiers,
};
use crate::test_support::{string_of, test_scope};
use crate::{AutoCompleteBox, Control, ItemsSource};
use ferroui_base::{AnyValue, BoxedValue, Ref};
use std::cell::RefCell;
use std::rc::Rc;

fn create_peer(control: &Control) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}

fn state_of(value: Option<&BoxedValue>) -> Option<ExpandCollapseState> {
    let value: &dyn AnyValue = &**value?;
    value.downcast_ref::<ExpandCollapseState>().copied()
}

mod auto_complete_box_automation_peer_tests {
    use super::*;

    #[test]
    fn creates_auto_complete_box_automation_peer() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target);

        assert!(std::ptr::eq(peer.get_type(), AutoCompleteBoxAutomationPeer::TYPE));
    }

    #[test]
    fn implements_i_expand_collapse_and_i_value_providers() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target);

        assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
        assert!(peer.get_provider::<dyn IValueProvider>().is_some());
        assert!(peer.get_provider::<dyn IInvokeProvider>().is_none());
    }

    #[test]
    fn control_type_is_group_and_class_name_is_auto_complete_box() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target).cast::<AutoCompleteBoxAutomationPeer>().unwrap();

        assert_eq!(AutomationControlType::Group, peer.get_automation_control_type());
        assert_eq!("AutoCompleteBox", peer.get_class_name());
    }

    #[test]
    fn expand_collapse_tracks_is_drop_down_open() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        target.set_items_source(Some(ItemsSource::from_strs(["alpha"])));
        target.set_text(Some("a"));
        let peer = create_peer(&target).get_provider::<dyn IExpandCollapseProvider>().unwrap();
        assert!(peer.shows_menu());

        target.set_is_drop_down_open(false);
        assert!(!target.is_drop_down_open());

        peer.expand().unwrap();
        assert!(target.is_drop_down_open());
        assert_eq!(ExpandCollapseState::Expanded, peer.expand_collapse_state());

        peer.collapse().unwrap();
        assert!(!target.is_drop_down_open());
        assert_eq!(ExpandCollapseState::Collapsed, peer.expand_collapse_state());
    }

    #[test]
    fn value_tracks_and_sets_text() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        target.set_text(Some("one"));
        let peer = create_peer(&target).get_provider::<dyn IValueProvider>().unwrap();

        assert_eq!(Some("one".to_string()), peer.value());
        peer.set_value(Some("two")).unwrap();
        assert_eq!(Some("two".to_string()), target.text());
        assert_eq!(Some("two".to_string()), peer.value());
    }

    #[test]
    fn value_provider_is_mutable() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target).get_provider::<dyn IValueProvider>().unwrap();

        assert!(!peer.is_read_only());
    }

    #[test]
    fn property_change_events_raise_for_drop_down_and_text() {
        let _scope = test_scope();
        let target = AutoCompleteBox::new();
        let peer = create_peer(&target).cast::<AutoCompleteBoxAutomationPeer>().unwrap();

        let expand_collapse_changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> =
            Rc::new(RefCell::new(None));
        let value_changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(None));
        let (expand_collapse_sink, value_sink) = (expand_collapse_changed.clone(), value_changed.clone());
        peer.property_changed(move |e| {
            if e.property() == ExpandCollapsePatternIdentifiers::expand_collapse_state_property() {
                *expand_collapse_sink.borrow_mut() = Some(e.clone());
            } else if e.property() == ValuePatternIdentifiers::value_property() {
                *value_sink.borrow_mut() = Some(e.clone());
            }
        });

        target.set_is_drop_down_open(true);
        let expand_collapse_changed = expand_collapse_changed.borrow();
        assert!(expand_collapse_changed.is_some());
        let expand_collapse_changed = expand_collapse_changed.as_ref().unwrap();
        assert!(
            expand_collapse_changed.property() == ExpandCollapsePatternIdentifiers::expand_collapse_state_property()
        );
        assert_eq!(Some(ExpandCollapseState::Collapsed), state_of(expand_collapse_changed.old_value()));
        assert_eq!(Some(ExpandCollapseState::Expanded), state_of(expand_collapse_changed.new_value()));

        target.set_text(Some("query"));
        let value_changed = value_changed.borrow();
        assert!(value_changed.is_some());
        let value_changed = value_changed.as_ref().unwrap();
        assert!(value_changed.property() == ValuePatternIdentifiers::value_property());
        assert_eq!(Some(String::new()), value_changed.old_value().and_then(string_of));
        assert_eq!(Some("query".to_string()), value_changed.new_value().and_then(string_of));
    }
}
