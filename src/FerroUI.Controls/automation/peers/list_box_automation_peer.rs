use super::{
    AutomationPeerImpl, ControlAutomationPeerImpl, ItemsControlAutomationPeerImpl,
    SelectingItemsControlAutomationPeer, SelectingItemsControlAutomationPeerImpl,
};
use crate::ListBox;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`ListBox`].
#[repr(C)]
pub struct ListBoxAutomationPeer {
    base: SelectingItemsControlAutomationPeer,
}

ferro_class!(ListBoxAutomationPeer: SelectingItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(ListBoxAutomationPeer {});

ferroui_base::ferro_impl_classes!(ListBoxAutomationPeer: FerroObjectImpl, AutomationPeerImpl, ControlAutomationPeerImpl, ItemsControlAutomationPeerImpl, SelectingItemsControlAutomationPeerImpl);

impl ListBoxAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ListBox) -> Self {
        Self { base: SelectingItemsControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ListBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
