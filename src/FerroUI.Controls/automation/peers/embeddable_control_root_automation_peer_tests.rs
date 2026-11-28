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

/// The root peers resolve a hit (and a focus) through the resolver a library
/// registers (`ControlAutomationPeer::register_automation_target`): before
/// one is registered the hit control's own peer is returned, as on main; after,
/// the peer of the control the resolver names. The resolver of this test names
/// the parent of a control called `automation-target-inner` only, so the other
/// tests of the process are not affected.
#[test]
fn hit_testing_resolves_through_the_registered_automation_target() {
    use super::WindowBaseAutomationPeer;
    use crate::testing::MockWindowingPlatform;
    use crate::{Border, Control, Window};
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::media::Brushes;
    use ferroui_base::{Point, Thickness};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(400.0, 300.0));
    let outer = Border::new();
    outer.set_horizontal_alignment(HorizontalAlignment::Left);
    outer.set_vertical_alignment(VerticalAlignment::Top);
    outer.set_padding(Thickness::new(10.0, 10.0, 10.0, 10.0));
    let inner = Border::new();
    inner.set_name(Some("automation-target-inner".to_owned()));
    inner.set_width(40.0);
    inner.set_height(20.0);
    inner.set_background(Some(Brushes::red()));
    outer.set_child(Some(inner.clone().upcast()));
    window.set_content(Some(Control::boxed(outer.clone())));
    window.show();
    crate::native_control_host_tests::frame();

    let root = ControlAutomationPeer::create_peer_for_element(&window)
        .cast::<WindowBaseAutomationPeer>()
        .expect("the window's peer");
    let inner_peer = ControlAutomationPeer::create_peer_for_element(&inner);
    let outer_peer = ControlAutomationPeer::create_peer_for_element(&outer);
    let at_inner = Point::new(20.0, 15.0);
    assert!(root.get_peer_from_point(at_inner) == Some(inner_peer));

    fn to_parent(control: &Ref<Control>) -> Option<Ref<Control>> {
        if control.name().as_deref() != Some("automation-target-inner") {
            return None;
        }
        control.get_visual_parent()?.find_ancestor_of_type::<Control>(true)
    }
    assert!(ControlAutomationPeer::register_automation_target(to_parent));
    assert!(!ControlAutomationPeer::register_automation_target(to_parent), "the first registration stays");

    assert!(root.get_peer_from_point(at_inner) == Some(outer_peer));
    let outer_control: Ref<Control> = outer.clone().upcast();
    assert!(ControlAutomationPeer::automation_target(outer_control.clone()) == outer_control);
    assert!(ControlAutomationPeer::automation_target(inner.clone().upcast()) == outer_control);
    window.close();
}
