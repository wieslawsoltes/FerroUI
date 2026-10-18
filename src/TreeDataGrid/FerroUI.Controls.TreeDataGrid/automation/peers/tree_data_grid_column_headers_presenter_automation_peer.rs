//! `Automation/Peers/TreeDataGridColumnHeadersPresenterAutomationPeer.cs`: the automation peer of
//! the presenter of the column headers.

use crate::primitives::TreeDataGridColumnHeadersPresenter;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};

/// An automation peer which represents a [`TreeDataGridColumnHeadersPresenter`].
#[repr(C)]
pub struct TreeDataGridColumnHeadersPresenterAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TreeDataGridColumnHeadersPresenterAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeDataGridColumnHeadersPresenterAutomationPeer {});

impl FerroObjectImpl for TreeDataGridColumnHeadersPresenterAutomationPeer {}

impl ControlAutomationPeerImpl for TreeDataGridColumnHeadersPresenterAutomationPeer {}

impl AutomationPeerImpl for TreeDataGridColumnHeadersPresenterAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Header
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl TreeDataGridColumnHeadersPresenterAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeDataGridColumnHeadersPresenter) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeDataGridColumnHeadersPresenter) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// The owning presenter.
    pub fn owner(&self) -> Ref<TreeDataGridColumnHeadersPresenter> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a column headers presenter.")
    }
}
