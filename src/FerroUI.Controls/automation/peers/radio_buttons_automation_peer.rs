// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../../NOTICE.md.

use crate::RadioButtons;
use crate::automation::peers::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeerImpl,
    ItemsControlAutomationPeerImpl, SelectingItemsControlAutomationPeer,
    SelectingItemsControlAutomationPeerImpl,
};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};

/// The native accessibility selection provider of a radio button group.
#[repr(C)]
pub struct RadioButtonsAutomationPeer { base: SelectingItemsControlAutomationPeer }
ferro_class!(RadioButtonsAutomationPeer: SelectingItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(RadioButtonsAutomationPeer {});
ferro_impl_classes!(RadioButtonsAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl, ItemsControlAutomationPeerImpl, SelectingItemsControlAutomationPeerImpl);
impl AutomationPeerImpl for RadioButtonsAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String { "RadioButtons".to_string() }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType { AutomationControlType::Group }
}
impl RadioButtonsAutomationPeer {
    pub fn construct(owner: &RadioButtons) -> Self { Self { base: SelectingItemsControlAutomationPeer::construct(owner) } }
    pub fn new(owner: &RadioButtons) -> Ref<Self> { instantiate(Self::construct(owner)) }
}
