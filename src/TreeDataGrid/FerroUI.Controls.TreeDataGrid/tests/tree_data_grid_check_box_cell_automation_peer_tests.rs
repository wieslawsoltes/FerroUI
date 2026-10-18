//! `TreeDataGridCheckBoxCellAutomationPeerTests.cs`: the peer of a check box cell, its toggle state
//! and the toggling of two-state, three-state and read-only cells.

use super::automation_peer_test_helper::create_peer;
use super::support::{assert_panics, start_app};
use crate::automation::peers::TreeDataGridCheckBoxCellAutomationPeer;
use crate::primitives::TreeDataGridCheckBoxCell;
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::provider::{IToggleProvider, ToggleState};

#[test]
fn should_create_check_box_cell_peer() {
    let _app = start_app();
    let cell = TreeDataGridCheckBoxCell::new();
    let peer = create_peer::<TreeDataGridCheckBoxCellAutomationPeer>(&cell);

    assert_eq!(AutomationControlType::CheckBox, peer.get_automation_control_type());
    assert!(peer.is_control_element());
    assert!(peer.is_content_element());
}

#[test]
fn should_map_value_to_toggle_state() {
    let _app = start_app();
    let cell = TreeDataGridCheckBoxCell::new();
    cell.set_value(Some(false));
    let peer = create_peer::<TreeDataGridCheckBoxCellAutomationPeer>(&cell);
    let provider = peer.get_provider::<dyn IToggleProvider>().expect("a toggle provider");

    assert_eq!(ToggleState::Off, provider.toggle_state());
    cell.set_value(Some(true));
    assert_eq!(ToggleState::On, provider.toggle_state());
    cell.set_value(None);
    assert_eq!(ToggleState::Indeterminate, provider.toggle_state());
}

#[test]
fn toggle_should_cycle_two_state_values() {
    let _app = start_app();
    let cell = TreeDataGridCheckBoxCell::new();
    cell.set_is_three_state(false);
    cell.set_value(Some(false));
    let peer = create_peer::<TreeDataGridCheckBoxCellAutomationPeer>(&cell);
    let provider = peer.get_provider::<dyn IToggleProvider>().expect("a toggle provider");

    provider.toggle().expect("the cell is enabled");
    assert_eq!(Some(true), cell.value());
    provider.toggle().expect("the cell is enabled");
    assert_eq!(Some(false), cell.value());
}

#[test]
fn toggle_should_cycle_three_state_values() {
    let _app = start_app();
    let cell = TreeDataGridCheckBoxCell::new();
    cell.set_is_three_state(true);
    cell.set_value(Some(true));
    let peer = create_peer::<TreeDataGridCheckBoxCellAutomationPeer>(&cell);
    let provider = peer.get_provider::<dyn IToggleProvider>().expect("a toggle provider");

    provider.toggle().expect("the cell is enabled");
    assert_eq!(None, cell.value());
    provider.toggle().expect("the cell is enabled");
    assert_eq!(Some(false), cell.value());
    provider.toggle().expect("the cell is enabled");
    assert_eq!(Some(true), cell.value());
}

#[test]
fn toggle_should_throw_for_read_only_cell() {
    let _app = start_app();
    let cell = TreeDataGridCheckBoxCell::new();
    cell.set_is_read_only(true);
    cell.set_value(Some(false));
    let peer = create_peer::<TreeDataGridCheckBoxCellAutomationPeer>(&cell);
    let provider = peer.get_provider::<dyn IToggleProvider>().expect("a toggle provider");

    // `NotSupportedException`: the peer panics on a read-only cell.
    assert_panics(|| {
        let _ = provider.toggle();
    });
}
