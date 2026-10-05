use super::{AutomationPeer, ControlAutomationPeer};
use crate::automation::provider::{IEmbeddedRootProvider, IRootProvider};
use crate::embedding::EmbeddableControlRoot;
use crate::platform::ITopLevelImpl;
use crate::testing::{MockImplKind, MockWindowImpl, TestServices, UnitTestApplication};
use ferroui_base::Ref;
use std::rc::Rc;

// The reference takes the embeddable control root of its compositor test
// services; here the root is created over a mock top-level implementation.
fn create_top_level() -> Ref<EmbeddableControlRoot> {
    let platform_impl: Rc<dyn ITopLevelImpl> = MockWindowImpl::bare(MockImplKind::TopLevel);
    let top_level = EmbeddableControlRoot::with_impl(platform_impl);
    top_level.prepare();
    top_level
}

#[test]
fn peer_provides_i_root_provider() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let top_level = create_top_level();
    let peer: Ref<AutomationPeer> = ControlAutomationPeer::create_peer_for_element(&top_level);

    let root_provider = peer.get_provider::<dyn IRootProvider>();

    assert!(root_provider.is_some());
    assert!(root_provider.unwrap().peer() == peer);
}

#[test]
fn peer_still_provides_i_embedded_root_provider() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let top_level = create_top_level();
    let peer = ControlAutomationPeer::create_peer_for_element(&top_level);

    let embedded_root_provider = peer.get_provider::<dyn IEmbeddedRootProvider>();

    assert!(embedded_root_provider.is_some());
}

#[test]
fn i_root_provider_platform_impl_returns_owner_platform_impl() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let top_level = create_top_level();
    let peer = ControlAutomationPeer::create_peer_for_element(&top_level);

    let root_provider = peer.get_provider::<dyn IRootProvider>();

    assert!(root_provider.is_some());
    let expected = top_level.platform_impl().expect("the root has a platform implementation");
    let actual = root_provider.unwrap().platform_impl().expect("the provider has a platform implementation");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&expected), Rc::as_ptr(&actual)));
}
