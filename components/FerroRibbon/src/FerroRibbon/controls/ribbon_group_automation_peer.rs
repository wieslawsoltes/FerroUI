//! Source RibbonGroupAutomationPeer from shared RibbonGroup.cs.
use super::RibbonGroup;
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, ferro_class_info, instantiate};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
#[repr(C)]
pub struct RibbonGroupAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RibbonGroupAutomationPeer:ControlAutomationPeer);
ferro_class_info!(RibbonGroupAutomationPeer{markup:{namespace:"FerroRibbon.Controls",constructors:[(Ref<RibbonGroup>)=>|owner:Ref<RibbonGroup>|RibbonGroupAutomationPeer::new(&owner)]}});
impl FerroObjectImpl for RibbonGroupAutomationPeer {}
impl ControlAutomationPeerImpl for RibbonGroupAutomationPeer {}
impl RibbonGroupAutomationPeer {
    pub fn new(owner: &RibbonGroup) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
}
impl AutomationPeerImpl for RibbonGroupAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
    fn get_class_name_core(_: &Self) -> String {
        "RibbonGroup".into()
    }
    fn get_name_core(this: &Self) -> Option<String> {
        ControlAutomationPeer::owner(this)
            .cast::<RibbonGroup>()
            .expect("group peer owner")
            .header()
            .or_else(|| Self::parent_get_name_core(this))
    }
}
