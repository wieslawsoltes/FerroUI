// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::{HostWindow, HostWindowTitleBar};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::{
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::IInvokeProvider,
    ElementNotEnabledException,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct HostWindowTitleBarAutomationPeer {
    base: ControlAutomationPeer,
    owner: Ref<HostWindowTitleBar>,
}
ferro_class!(HostWindowTitleBarAutomationPeer:ControlAutomationPeer);
ferro_class_info!(HostWindowTitleBarAutomationPeer{interfaces:[Rc<dyn IInvokeProvider> =>HostWindowTitleBarAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(HostWindowTitleBarAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for HostWindowTitleBarAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "HostWindowTitleBar".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TitleBar
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner,
            "Host window title bar",
            None,
        ))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        Some(Helper::format_state(&[
            (
                "HasBackground",
                Some(Rc::new(this.owner.background_control().is_some())),
            ),
            ("Focusable", Some(Rc::new(this.owner.focusable()))),
            ("Visible", Some(Rc::new(this.owner.is_visible()))),
        ]))
    }
}
impl HostWindowTitleBarAutomationPeer {
    pub(crate) fn new(owner: &HostWindowTitleBar) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(Provider(this))
    }
}
struct Provider(Ref<HostWindowTitleBarAutomationPeer>);
impl IInvokeProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        if let Some(host) = self
            .0
            .owner
            .visual_root()
            .and_then(|root| root.cast::<HostWindow>())
        {
            host.activate();
            host.focus();
        } else {
            self.0.owner.focus();
        }
        Ok(())
    }
}
