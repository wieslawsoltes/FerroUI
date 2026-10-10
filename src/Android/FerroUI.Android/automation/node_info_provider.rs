// Members of the class of the reference that nothing of the port calls yet are kept with it.
#![allow(dead_code)]

use super::i_node_info_provider::NodeActionError;
use crate::explore_by_touch_helper::IVirtualViewOwner;
use ferroui_base::reactive::IDisposable;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::automation::AutomationPropertyChangedEventArgs;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

/// What the node info provider of a provider contract `T` of a peer
/// shares: the helper that owns the virtual view, the peer and the number
/// of the virtual view.
///
/// The reference is an abstract class the providers derive from; here a
/// provider holds this value and implements the contract of a node info
/// provider itself.
pub(crate) struct NodeInfoProvider<T: ?Sized + 'static> {
    owner: Weak<dyn IVirtualViewOwner>,
    peer: Ref<AutomationPeer>,
    virtual_view_id: i32,
    /// The handler on the property changes of the peer.
    subscription: Rc<dyn IDisposable>,
    provider: PhantomData<fn() -> Rc<T>>,
}

/// What a provider does when a property of its peer changed: the override
/// of `PeerPropertyChanged` of the reference.
pub(crate) type PeerPropertyChanged = fn(&dyn IVirtualViewOwner, i32, &AutomationPropertyChangedEventArgs);

impl<T: ?Sized + 'static> NodeInfoProvider<T> {
    /// `peer_property_changed` is `None` for a provider that does not
    /// follow the properties of its peer.
    pub(crate) fn new(
        owner: Weak<dyn IVirtualViewOwner>,
        peer: Ref<AutomationPeer>,
        virtual_view_id: i32,
        peer_property_changed: Option<PeerPropertyChanged>,
    ) -> Self {
        let handler_owner = owner.clone();
        let subscription = peer.property_changed(move |e| {
            if let (Some(peer_property_changed), Some(owner)) = (peer_property_changed, handler_owner.upgrade()) {
                peer_property_changed(&*owner, virtual_view_id, e);
            }
        });
        Self { owner, peer, virtual_view_id, subscription, provider: PhantomData }
    }

    pub(crate) fn virtual_view_id(&self) -> i32 {
        self.virtual_view_id
    }

    pub(crate) fn invalidate_self(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.invalidate_virtual_view(self.virtual_view_id);
        }
    }

    pub(crate) fn invalidate_self_with(&self, change_types: i32) {
        if let Some(owner) = self.owner.upgrade() {
            owner.invalidate_virtual_view_with(self.virtual_view_id, change_types);
        }
    }

    /// The provider of the contract of the peer.
    pub(crate) fn get_provider(&self) -> Result<Rc<T>, NodeActionError> {
        self.peer.get_provider::<T>().ok_or_else(|| {
            NodeActionError::InvalidOperation(format!(
                "Peer instance does not implement {}.",
                std::any::type_name::<T>()
            ))
        })
    }
}

impl<T: ?Sized + 'static> Drop for NodeInfoProvider<T> {
    fn drop(&mut self) {
        // The reference never removes the handler; a peer outlives the
        // providers of its node here, so the handler goes with them.
        self.subscription.dispose();
    }
}
