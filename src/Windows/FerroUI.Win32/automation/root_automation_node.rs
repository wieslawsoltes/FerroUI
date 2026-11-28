//! The node of the root of the tree of a window (the port of
//! `RootAutomationNode.cs`).
//!
//! The class of the reference derives from the node and adds the
//! interface of a fragment root. Here a node of the kind
//! [`NodeKind::Root`](super::automation_node::NodeKind) is made with that
//! interface, and what the class overrides and adds is in this file.

use super::automation_node::{AutomationNode, SendProvider};
use super::interop::{
    IRawElementProviderFragment, IRawElementProviderFragmentRoot, IRawElementProviderSimple, UiaCoreProviderApi,
};
use ferroui_base::{input::KeyboardDevice, reactive::IDisposable};
use ferroui_base::{PixelPoint, Ref};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::IRootProvider;
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::{Control, TopLevel, primitives::PopupRoot};
use ferroui_microcom::{ComPtr, HResult};
use std::rc::Rc;

/// What a node of a window does beyond a node.
pub(crate) struct RootAutomationNode;

impl RootAutomationNode {
    /// The constructor of the reference: the subscription to the focus of
    /// the root, for the node with the number `id`.
    ///
    /// # Panics
    /// Panics when the peer does not provide the root contract (the
    /// internal exception of the reference).
    pub fn new(peer: &Ref<AutomationPeer>, id: i32) -> Vec<Rc<dyn IDisposable>> {
        let root = peer
            .get_provider::<dyn IRootProvider>()
            .expect("Attempt to create RootAutomationNode from peer which does not implement IRootProvider.");
        vec![root.focus_changed(Rc::new(move || Self::on_root_focus_changed(id)))]
    }

    /// The node itself.
    pub fn get_fragment_root(node: &AutomationNode) -> Option<ComPtr<IRawElementProviderFragmentRoot>> {
        node.this().cast::<IRawElementProviderFragmentRoot>().ok()
    }

    /// `Peer`: the root contract of the peer, on the UI thread.
    pub fn peer(peer: &Ref<AutomationPeer>) -> Option<Rc<dyn IRootProvider>> {
        peer.get_provider::<dyn IRootProvider>()
    }

    /// `WindowImpl`: the window of the root, on the UI thread. The
    /// reference narrows the platform implementation to a window base;
    /// what it uses of it (the handle and the conversion of a point) is of
    /// every top level here.
    pub fn window_impl(peer: &Ref<AutomationPeer>) -> Option<Rc<dyn ITopLevelImpl>> {
        Self::peer(peer)?.platform_impl()
    }

    /// The node at a point of the screen.
    ///
    /// The reference converts the point and makes the node on the thread
    /// of the caller; both touch objects of the UI thread, and run there
    /// here with the rest.
    pub fn element_provider_from_point(
        node: &AutomationNode,
        x: f64,
        y: f64,
    ) -> Result<Option<ComPtr<IRawElementProviderFragment>>, HResult> {
        let result = node.invoke_sync(move |peer| {
            let window_impl = Self::window_impl(peer)?;
            let p = window_impl.point_to_client(PixelPoint::new(x as i32, y as i32));
            let found = Self::peer(peer)?.get_peer_from_point(p);
            AutomationNode::get_or_create(found.as_ref()).map(SendProvider)
        })?;
        Ok(result.and_then(|found| found.into_inner().cast::<IRawElementProviderFragment>().ok()))
    }

    /// The node of the element that has the focus.
    pub fn get_focus(node: &AutomationNode) -> Result<Option<ComPtr<IRawElementProviderFragment>>, HResult> {
        let result = node.invoke_sync(|peer| {
            let focus = Self::peer(peer)?.get_focus().or_else(|| Self::owned_popup_focus(peer).map(|(_, focus)| focus));
            AutomationNode::get_or_create(focus.as_ref()).map(SendProvider)
        })?;
        Ok(result.and_then(|focus| focus.into_inner().cast::<IRawElementProviderFragment>().ok()))
    }

    // Win32 keeps the owner HWND active for a popup. The system asks that
    // HWND's provider for focus while FerroUI routes keyboard input to a
    // control in the separate popup tree.
    fn owned_popup_focus(peer: &Ref<AutomationPeer>) -> Option<(Ref<PopupRoot>, Ref<AutomationPeer>)> {
        let owner = peer.cast::<ControlAutomationPeer>()?.try_owner()?.cast::<TopLevel>()?;
        let focus = KeyboardDevice::instance()?.focused_element()?.cast::<Control>()?;
        let focus_root = TopLevel::get_top_level(Some(&focus))?.cast::<PopupRoot>()?;
        let mut popup = focus_root.clone();
        loop {
            if !popup.is_visible() {
                return None;
            }
            let parent = popup.parent_top_level()?;
            if parent.ptr_eq(&owner) {
                return Some((focus_root, ControlAutomationPeer::create_peer_for_element(&focus)));
            }
            popup = parent.cast::<PopupRoot>()?;
        }
    }

    /// The provider of the system for the window of the root: the node
    /// overrides what it answers of the window.
    pub fn get_host_raw_element_provider(
        node: &AutomationNode,
    ) -> Result<Option<ComPtr<IRawElementProviderSimple>>, HResult> {
        let handle = node.invoke_sync(|peer| {
            Self::window_impl(peer).and_then(|window_impl| window_impl.handle()).map_or(0, |handle| handle.handle())
        })?;
        if handle == 0 {
            return Ok(None);
        }
        UiaCoreProviderApi::uia_host_provider_from_hwnd(handle)
    }

    fn on_root_focus_changed(id: i32) {
        let Some(peer) = AutomationNode::peer_of(id) else {
            return;
        };
        let Some(root) = Self::peer(&peer) else { return };
        let mut focus = root.get_focus();
        if focus.is_none() {
            if let Some((popup, popup_focus)) = Self::owned_popup_focus(&peer) {
                let popup = ControlAutomationPeer::create_peer_for_element(&popup);
                if AutomationNode::existing(&popup).is_some() {
                    // The popup's own focus tracker reports the change.
                    return;
                }
                // Follow later changes inside the popup as well. Its tracker
                // starts with the current focus before the node subscribes.
                AutomationNode::get_or_create(Some(&popup));
                focus = Some(popup_focus);
            }
        }
        AutomationNode::raise_focus_changed(AutomationNode::get_or_create(focus.as_ref()).as_ref());
    }
}
