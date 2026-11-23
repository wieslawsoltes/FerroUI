//! Media groups expose their native button/slider peers as children.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};

#[repr(C)]
pub struct CameraCaptureUIAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(CameraCaptureUIAutomationPeer:ControlAutomationPeer);
ferro_class_info!(CameraCaptureUIAutomationPeer {});
ferro_impl_classes!(CameraCaptureUIAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for CameraCaptureUIAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
    fn get_class_name_core(_: &Self) -> String {
        "CameraCaptureUI".into()
    }
}
impl CameraCaptureUIAutomationPeer {
    pub fn new(owner: &crate::CameraCaptureUI) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
}
