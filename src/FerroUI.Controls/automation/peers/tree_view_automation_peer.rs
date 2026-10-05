use super::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeerImpl, ItemsControlAutomationPeer,
    ItemsControlAutomationPeerImpl,
};
use crate::TreeView;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`TreeView`].
#[repr(C)]
pub struct TreeViewAutomationPeer {
    base: ItemsControlAutomationPeer,
}

ferro_class!(TreeViewAutomationPeer: ItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeViewAutomationPeer {});

impl FerroObjectImpl for TreeViewAutomationPeer {}
impl ControlAutomationPeerImpl for TreeViewAutomationPeer {}
impl ItemsControlAutomationPeerImpl for TreeViewAutomationPeer {}

impl AutomationPeerImpl for TreeViewAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Tree
    }
}

impl TreeViewAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeView) -> Self {
        Self { base: ItemsControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeView) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
