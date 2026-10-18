//! `Automation/Peers/TreeDataGridCellAutomationPeer.cs`: the automation peer of a cell.

use crate::primitives::{TreeDataGridCell, TreeDataGridTemplateCell, TreeDataGridTextCell};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};

/// An automation peer which represents a [`TreeDataGridCell`].
#[repr(C)]
pub struct TreeDataGridCellAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TreeDataGridCellAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeDataGridCellAutomationPeer {});

impl FerroObjectImpl for TreeDataGridCellAutomationPeer {}

impl ControlAutomationPeerImpl for TreeDataGridCellAutomationPeer {}

impl AutomationPeerImpl for TreeDataGridCellAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let name = Self::parent_get_name_core(this);
        if name.as_deref().is_some_and(|name| !name.is_empty()) {
            return name;
        }

        let owner = this.owner();
        if let Some(text_cell) = owner.clone().cast::<TreeDataGridTextCell>() {
            return text_cell.value();
        }
        if let Some(template_cell) = owner.clone().cast::<TreeDataGridTemplateCell>() {
            return template_cell.content().map(|content| ValueTypes::to_display_string(Some(&content)));
        }
        // `Owner.Model?.ToString()`: the text of the value of the cell's model.
        owner.model().and_then(|model| model.value()).map(|value| ValueTypes::to_display_string(Some(&value)))
    }
}

impl TreeDataGridCellAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeDataGridCell) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeDataGridCell) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// The owning cell.
    pub fn owner(&self) -> Ref<TreeDataGridCell> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a cell.")
    }
}
