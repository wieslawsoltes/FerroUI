//! `Automation/Peers/TreeDataGridColumnHeaderAutomationPeer.cs`: the automation peer of a column
//! header.

use crate::primitives::TreeDataGridColumnHeader;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, ContentControlAutomationPeer, ControlAutomationPeerImpl,
};

/// An automation peer which represents a [`TreeDataGridColumnHeader`].
#[repr(C)]
pub struct TreeDataGridColumnHeaderAutomationPeer {
    base: ContentControlAutomationPeer,
}

ferro_class!(TreeDataGridColumnHeaderAutomationPeer: ContentControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeDataGridColumnHeaderAutomationPeer {});

impl FerroObjectImpl for TreeDataGridColumnHeaderAutomationPeer {}

impl ControlAutomationPeerImpl for TreeDataGridColumnHeaderAutomationPeer {}

impl AutomationPeerImpl for TreeDataGridColumnHeaderAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::HeaderItem
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl TreeDataGridColumnHeaderAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeDataGridColumnHeader) -> Self {
        Self { base: ContentControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeDataGridColumnHeader) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// The owning column header.
    pub fn owner(&self) -> Ref<TreeDataGridColumnHeader> {
        ContentControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a column header.")
    }
}
