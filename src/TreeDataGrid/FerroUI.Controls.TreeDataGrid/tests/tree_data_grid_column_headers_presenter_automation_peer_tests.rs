//! `TreeDataGridColumnHeadersPresenterAutomationPeerTests.cs`: the peer of the presenter of the
//! column headers is a header that is a control element but not a content element.

use super::automation_peer_test_helper::create_peer;
use super::support::start_app;
use crate::automation::peers::TreeDataGridColumnHeadersPresenterAutomationPeer;
use crate::primitives::TreeDataGridColumnHeadersPresenter;
use ferroui_controls::automation::peers::AutomationControlType;

#[test]
fn should_create_header_peer() {
    let _app = start_app();
    let presenter = TreeDataGridColumnHeadersPresenter::new();
    let peer = create_peer::<TreeDataGridColumnHeadersPresenterAutomationPeer>(&presenter);

    assert_eq!(AutomationControlType::Header, peer.get_automation_control_type());
    assert!(peer.is_control_element());
    assert!(!peer.is_content_element());
}
