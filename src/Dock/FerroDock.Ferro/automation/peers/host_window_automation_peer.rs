// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::HostWindow;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::{
    peers::{
        AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeerImpl,
        WindowAutomationPeer,
    },
    provider::IInvokeProvider,
    ElementNotEnabledException,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct HostWindowAutomationPeer {
    base: WindowAutomationPeer,
    // This is the original readonly strong owner, in addition to the native base's weak owner.
    owner: Ref<HostWindow>,
}
ferro_class!(HostWindowAutomationPeer:WindowAutomationPeer);
ferro_class_info!(HostWindowAutomationPeer {interfaces:[Rc<dyn IInvokeProvider> => HostWindowAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(HostWindowAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for HostWindowAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "HostWindow".into()
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|n| !n.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner,
            "Dock host window",
            this.owner.window().and_then(|w| w.layout()).map(|r| r as _),
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let id = Helper::resolve_automation_id(
            &this.owner,
            this.owner.window().and_then(|w| w.layout()).map(|r| r as _),
        );
        Some(if !id.trim().is_empty() {
            id
        } else {
            Self::parent_get_automation_id_core(this).unwrap_or_default()
        })
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        Some(Helper::format_state(&[
            ("Tracked", Some(Rc::new(this.owner.is_tracked()))),
            ("ToolWindow", Some(Rc::new(this.owner.is_tool_window()))),
            ("WindowState", Some(Rc::new(this.owner.window_state()))),
            (
                "HasDockWindow",
                Some(Rc::new(this.owner.window().is_some())),
            ),
            (
                "DockContent",
                Some(Rc::new(
                    this.owner
                        .window()
                        .and_then(|w| w.layout())
                        .map_or_else(|| "none".into(), |r| r.id()),
                )),
            ),
        ]))
    }
}
impl HostWindowAutomationPeer {
    pub(crate) fn new(owner: &HostWindow) -> Ref<Self> {
        instantiate(Self {
            base: WindowAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(InvokeProvider(this))
    }
}
struct InvokeProvider(Ref<HostWindowAutomationPeer>);
impl IInvokeProvider for InvokeProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner.clone();
        if !owner.is_visible() {
            owner.show();
        }
        owner.activate();
        owner.focus();
        Ok(())
    }
}
