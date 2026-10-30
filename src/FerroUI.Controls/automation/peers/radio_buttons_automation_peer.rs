//! The automation peer of WinUI's `RadioButtons`
//! (`controls/dev/RadioButtons/RadioButtonsAutomationPeer.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl, ItemsControlAutomationPeerImpl, SelectingItemsControlAutomationPeer,
    SelectingItemsControlAutomationPeerImpl,
};
use crate::RadioButtons;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`RadioButtons`]: a group named by its header when it
/// has no name.
///
/// Deviation (docs/porting/winui-controls.md, Input): WinUI's peer is a plain group; this one is
/// the peer of a selecting items control, with the selection pattern of the group.
#[repr(C)]
pub struct RadioButtonsAutomationPeer {
    base: SelectingItemsControlAutomationPeer,
}

ferro_class!(RadioButtonsAutomationPeer: SelectingItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(RadioButtonsAutomationPeer {});
ferro_impl_classes!(
    RadioButtonsAutomationPeer: FerroObjectImpl,
    ControlAutomationPeerImpl,
    ItemsControlAutomationPeerImpl,
    SelectingItemsControlAutomationPeerImpl
);

impl AutomationPeerImpl for RadioButtonsAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "RadioButtons".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }

    fn get_name_core(this: &Self) -> Option<String> {
        // The name, or else the text of the header.
        Self::parent_get_name_core(this).filter(|name| !name.is_empty()).or_else(|| {
            ControlAutomationPeer::owner(this)
                .cast::<RadioButtons>()
                .and_then(|owner| owner.header())
                .and_then(|header| ValueTypes::try_to_string(&*header))
        })
    }
}

impl RadioButtonsAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &RadioButtons) -> Self {
        Self { base: SelectingItemsControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &RadioButtons) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
