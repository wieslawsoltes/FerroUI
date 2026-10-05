//! Port of the reference `ComboBoxAutomationPeerTests`.

use super::ControlAutomationPeer;
use crate::automation::provider::IValueProvider;
use crate::automation::ValuePatternIdentifiers;
use crate::test_support::test_scope;
use crate::ComboBox;
use ferroui_base::{AnyValue, Ref};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn value(combo_box: &Ref<ComboBox>) -> Rc<dyn IValueProvider> {
    ControlAutomationPeer::create_peer_for_element(combo_box)
        .get_provider::<dyn IValueProvider>()
        .expect("the peer of a combo box provides the value contract")
}

fn editable_combo_box() -> Ref<ComboBox> {
    let combo_box = ComboBox::new();
    combo_box.set_is_editable(true);
    combo_box
}

#[test]
fn non_editable_combo_box_is_read_only() {
    let _scope = test_scope();
    // The peer does not keep its control alive.
    let combo_box = ComboBox::new();
    let provider = value(&combo_box);

    assert!(provider.is_read_only());
}

#[test]
fn editable_combo_box_is_not_read_only() {
    let _scope = test_scope();
    let combo_box = editable_combo_box();
    let provider = value(&combo_box);

    assert!(!provider.is_read_only());
}

#[test]
fn value_returns_text_when_editable() {
    let _scope = test_scope();
    let combo_box = editable_combo_box();
    combo_box.set_text(Some("hello".to_string()));
    let provider = value(&combo_box);

    assert_eq!(Some("hello".to_string()), provider.value());
}

#[test]
fn set_value_updates_text_when_editable() {
    let _scope = test_scope();
    let combo_box = editable_combo_box();
    let provider = value(&combo_box);

    provider.set_value(Some("typed")).unwrap();

    assert_eq!(Some("typed".to_string()), combo_box.text());
    assert_eq!(Some("typed".to_string()), provider.value());
}

#[test]
fn set_value_throws_when_not_editable() {
    let _scope = test_scope();
    // The peer does not keep its control alive.
    let combo_box = ComboBox::new();
    let provider = value(&combo_box);

    assert!(catch_unwind(AssertUnwindSafe(|| provider.set_value(Some("x")))).is_err());
}

#[test]
fn text_change_raises_value_property_changed() {
    let _scope = test_scope();
    let combo_box = editable_combo_box();
    let peer = ControlAutomationPeer::create_peer_for_element(&combo_box);

    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    peer.property_changed(move |e| {
        if std::ptr::eq(e.property(), ValuePatternIdentifiers::value_property()) {
            let new_value = e.new_value().and_then(|value| {
                let value: &dyn AnyValue = &**value;
                value.downcast_ref::<String>().cloned()
            });
            assert_eq!(Some("abc".to_string()), new_value);
            count.set(count.get() + 1);
        }
    });

    combo_box.set_text(Some("abc".to_string()));

    assert_eq!(1, raised.get());
}
