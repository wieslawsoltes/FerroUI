//! `TreeDataGridColumnHeaderAutomationPeerTests.cs`: the peer of a column header is a header item
//! that is a control element but not a content element.

use super::automation_peer_test_helper::create_peer;
use super::support::start_app;
use crate::automation::peers::TreeDataGridColumnHeaderAutomationPeer;
use crate::primitives::TreeDataGridColumnHeader;
use ferroui_controls::automation::peers::AutomationControlType;

#[test]
fn should_create_header_item_peer() {
    let _app = start_app();
    let header = TreeDataGridColumnHeader::new();
    let peer = create_peer::<TreeDataGridColumnHeaderAutomationPeer>(&header);

    assert_eq!(AutomationControlType::HeaderItem, peer.get_automation_control_type());
    assert!(peer.is_control_element());
    assert!(!peer.is_content_element());
}
