//! `Automation/Peers/TreeDataGridCheckBoxCellAutomationPeer.cs`: the automation peer of a check
//! box cell, which toggles the cell's value.

use super::TreeDataGridCellAutomationPeer;
use crate::primitives::TreeDataGridCheckBoxCell;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{IToggleProvider, ToggleState};
use ferroui_controls::automation::ElementNotEnabledException;
use std::rc::Rc;

/// An automation peer which represents a [`TreeDataGridCheckBoxCell`].
#[repr(C)]
pub struct TreeDataGridCheckBoxCellAutomationPeer {
    base: TreeDataGridCellAutomationPeer,
}

ferro_class!(TreeDataGridCheckBoxCellAutomationPeer: TreeDataGridCellAutomationPeer);
ferroui_base::ferro_class_info!(TreeDataGridCheckBoxCellAutomationPeer {
    interfaces: [Rc<dyn IToggleProvider> => TreeDataGridCheckBoxCellAutomationPeer::as_toggle_provider]
});

impl FerroObjectImpl for TreeDataGridCheckBoxCellAutomationPeer {}

impl ControlAutomationPeerImpl for TreeDataGridCheckBoxCellAutomationPeer {}

impl AutomationPeerImpl for TreeDataGridCheckBoxCellAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::CheckBox
    }
}

/// The handle through which the peer implements [`IToggleProvider`].
struct ToggleProviderAdapter(Ref<TreeDataGridCheckBoxCellAutomationPeer>);

impl IToggleProvider for ToggleProviderAdapter {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn toggle_state(&self) -> ToggleState {
        self.0.toggle_state()
    }

    fn toggle(&self) -> Result<(), ElementNotEnabledException> {
        self.0.toggle()
    }
}

impl TreeDataGridCheckBoxCellAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeDataGridCheckBoxCell) -> Self {
        Self { base: TreeDataGridCellAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeDataGridCheckBoxCell) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn as_toggle_provider(peer: Ref<Self>) -> Rc<dyn IToggleProvider> {
        Rc::new(ToggleProviderAdapter(peer))
    }

    /// The owning check box cell.
    pub fn owner(&self) -> Ref<TreeDataGridCheckBoxCell> {
        TreeDataGridCellAutomationPeer::owner(self).cast().expect("The owner of the peer is a check box cell.")
    }

    /// The state of the check box (`ToggleState`).
    pub fn toggle_state(&self) -> ToggleState {
        match self.owner().value() {
            Some(true) => ToggleState::On,
            Some(false) => ToggleState::Off,
            None => ToggleState::Indeterminate,
        }
    }

    /// Moves the check box to its next state (`Toggle`).
    ///
    /// # Panics
    /// When the cell is read only (`NotSupportedException`).
    pub fn toggle(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let owner = self.owner();
        if owner.is_read_only() {
            panic!("TreeDataGrid checkbox cells are read-only.");
        }

        owner.set_value(Self::next_value(owner.value(), owner.is_three_state()));
        Ok(())
    }

    fn next_value(value: Option<bool>, is_three_state: bool) -> Option<bool> {
        match value {
            Some(true) => {
                if is_three_state {
                    None
                } else {
                    Some(false)
                }
            }
            Some(false) => Some(true),
            None => Some(false),
        }
    }
}
