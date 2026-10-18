//! `TreeDataGridCellAutomationPeerTests.cs`: the peer of a cell, its control type and its name
//! (the value of a text cell, unless the automation name is set).

use super::automation_peer_test_helper::create_peer;
use super::support::start_app;
use crate::automation::peers::TreeDataGridCellAutomationPeer;
use crate::primitives::TreeDataGridTextCell;
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::AutomationProperties;

#[test]
fn should_create_custom_cell_peer() {
    let _app = start_app();
    let cell = TreeDataGridTextCell::new();
    let peer = create_peer::<TreeDataGridCellAutomationPeer>(&cell);

    assert_eq!(AutomationControlType::Custom, peer.get_automation_control_type());
    assert!(peer.is_control_element());
    assert!(peer.is_content_element());
}

#[test]
fn should_use_text_cell_value_as_accessible_name() {
    let _app = start_app();
    let cell = TreeDataGridTextCell::new();
    cell.set_value(Some("Row 1, Column 1".to_string()));
    let peer = create_peer::<TreeDataGridCellAutomationPeer>(&cell);

    assert_eq!("Row 1, Column 1", peer.get_name());
}

#[test]
fn should_prefer_explicit_automation_name() {
    let _app = start_app();
    let cell = TreeDataGridTextCell::new();
    cell.set_value(Some("Fallback value".to_string()));
    AutomationProperties::set_name(&cell, Some("Explicit name"));
    let peer = create_peer::<TreeDataGridCellAutomationPeer>(&cell);

    assert_eq!("Explicit name", peer.get_name());
}
