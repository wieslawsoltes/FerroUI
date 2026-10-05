//! Port of the reference `FoundationAutomationPeerTests` (the classes of
//! the controls this library has).

use super::{
    AutomationControlType, AutomationPeer, ControlAutomationPeer, NumericUpDownAutomationPeer, ToolTipAutomationPeer,
};
use crate::automation::provider::IRangeValueProvider;
use crate::automation::{AutomationPropertyChangedEventArgs, RangeValuePatternIdentifiers};
use crate::test_support::test_scope;
use crate::{Control, NumericUpDown, ToolTip};
use ferroui_base::utilities::Decimal;
use ferroui_base::{AnyValue, Ref};
use std::cell::RefCell;
use std::rc::Rc;

fn create_peer(control: &Control) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}

mod tool_tip_peer {
    use super::*;

    #[test]
    fn creates_tool_tip_automation_peer() {
        let _scope = test_scope();
        let control = ToolTip::new();
        let peer = create_peer(&control);

        assert!(std::ptr::eq(peer.get_type(), ToolTipAutomationPeer::TYPE));
    }

    #[test]
    fn control_type_is_tool_tip() {
        let _scope = test_scope();
        let control = ToolTip::new();
        let peer = create_peer(&control).cast::<ToolTipAutomationPeer>().unwrap();

        assert_eq!(AutomationControlType::ToolTip, peer.get_automation_control_type());
        assert_eq!("ToolTip", peer.get_class_name());
    }
}

mod numeric_up_down_peer {
    use super::*;

    fn d(text: &str) -> Decimal {
        Decimal::parse(text).unwrap()
    }

    #[test]
    fn creates_numeric_up_down_automation_peer() {
        let _scope = test_scope();
        let control = NumericUpDown::new();
        let peer = create_peer(&control);

        assert!(std::ptr::eq(peer.get_type(), NumericUpDownAutomationPeer::TYPE));
    }

    #[test]
    fn is_spinner_control_type() {
        let _scope = test_scope();
        let control = NumericUpDown::new();
        let peer = create_peer(&control).cast::<NumericUpDownAutomationPeer>().unwrap();

        assert_eq!(AutomationControlType::Spinner, peer.get_automation_control_type());
        assert_eq!("NumericUpDown", peer.get_class_name());
    }

    #[test]
    fn implements_i_range_value_provider() {
        let _scope = test_scope();
        let control = NumericUpDown::new();
        let peer = create_peer(&control);

        assert!(peer.get_provider::<dyn IRangeValueProvider>().is_some());
    }

    #[test]
    fn range_values_reflect_owner() {
        let _scope = test_scope();
        let control = NumericUpDown::new();
        control.set_minimum(d("10"));
        control.set_maximum(d("20"));
        control.set_increment(d("3"));
        control.set_is_read_only(true);
        control.set_numeric_value(Some(d("14")));

        let peer = create_peer(&control).get_provider::<dyn IRangeValueProvider>().unwrap();

        assert_eq!(10.0, peer.minimum());
        assert_eq!(20.0, peer.maximum());
        assert_eq!(14.0, peer.value());
        assert_eq!(3.0, peer.small_change());
        assert_eq!(3.0, peer.large_change());
        assert!(peer.is_read_only());
    }

    #[test]
    fn null_value_reports_default_clamped_to_range() {
        let _scope = test_scope();
        let control = NumericUpDown::new();
        control.set_minimum(d("10"));
        control.set_maximum(d("20"));
        control.set_numeric_value(None);

        let peer = create_peer(&control).get_provider::<dyn IRangeValueProvider>().unwrap();

        assert_eq!(10.0, peer.value());
    }

    #[test]
    fn set_value_updates_owner_value() {
        let _scope = test_scope();
        let control = NumericUpDown::new();
        let peer = create_peer(&control).get_provider::<dyn IRangeValueProvider>().unwrap();

        peer.set_value(42.5).unwrap();

        assert_eq!(Some(d("42.5")), control.value());
    }

    #[test]
    fn property_changed_raises_range_when_value_changes() {
        let _scope = test_scope();
        let control = NumericUpDown::new();
        let peer = create_peer(&control).cast::<NumericUpDownAutomationPeer>().unwrap();
        let changed: Rc<RefCell<Option<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(None));

        let sink = changed.clone();
        peer.property_changed(move |e| {
            if e.property() == RangeValuePatternIdentifiers::value_property() {
                *sink.borrow_mut() = Some(e.clone());
            }
        });

        control.set_numeric_value(Some(d("7.5")));

        let changed = changed.borrow();
        assert!(changed.is_some());
        let changed = changed.as_ref().unwrap();
        assert!(changed.property() == RangeValuePatternIdentifiers::value_property());
        assert!(changed.old_value().is_none());
        let new_value: &dyn AnyValue = &**changed.new_value().unwrap();
        assert_eq!(Some(&d("7.5")), new_value.downcast_ref::<Decimal>());
    }
}
