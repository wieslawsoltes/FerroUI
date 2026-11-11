//! Media groups expose their native button/slider peers as children.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};

#[repr(C)]
pub struct MediaPlayerElementAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(MediaPlayerElementAutomationPeer:ControlAutomationPeer);
ferro_class_info!(MediaPlayerElementAutomationPeer {});
ferro_impl_classes!(MediaPlayerElementAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for MediaPlayerElementAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
    fn get_class_name_core(_: &Self) -> String {
        "MediaPlayerElement".into()
    }
}
impl MediaPlayerElementAutomationPeer {
    pub fn new(owner: &crate::MediaPlayerElement) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
}

#[repr(C)]
pub struct MediaTransportControlsAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(MediaTransportControlsAutomationPeer:ControlAutomationPeer);
ferro_class_info!(MediaTransportControlsAutomationPeer {});
ferro_impl_classes!(MediaTransportControlsAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for MediaTransportControlsAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }
    fn get_class_name_core(_: &Self) -> String {
        "MediaTransportControls".into()
    }
}
impl MediaTransportControlsAutomationPeer {
    pub fn new(owner: &crate::MediaTransportControls) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
}
