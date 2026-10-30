// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::PinnedDockWindow;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::{
    peers::{
        AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
        ControlAutomationPeerImpl, WindowAutomationPeer,
    },
    provider::IInvokeProvider,
    ElementNotEnabledException,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct PinnedDockWindowAutomationPeer {
    base: WindowAutomationPeer,
}
ferro_class!(PinnedDockWindowAutomationPeer:WindowAutomationPeer);
ferroui_base::ferro_class_info!(PinnedDockWindowAutomationPeer {interfaces:[Rc<dyn IInvokeProvider> => PinnedDockWindowAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(PinnedDockWindowAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for PinnedDockWindowAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "PinnedDockWindow".into()
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner(),
            "Pinned dock window",
            None,
        ))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        Some(Helper::format_state(&[
            ("WindowState", Some(Rc::new(owner.window_state()))),
            ("Visible", Some(Rc::new(owner.is_visible()))),
            ("ShowInTaskbar", Some(Rc::new(owner.show_in_taskbar()))),
        ]))
    }
}
impl PinnedDockWindowAutomationPeer {
    pub(crate) fn new(owner: &PinnedDockWindow) -> Ref<Self> {
        instantiate(Self {
            base: WindowAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<PinnedDockWindow> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("pinned-dock window peer owner")
    }
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(InvokeProvider(this))
    }
}
struct InvokeProvider(Ref<PinnedDockWindowAutomationPeer>);
impl IInvokeProvider for InvokeProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner();
        if !owner.is_visible() {
            owner.show();
        }
        owner.activate();
        owner.focus();
        Ok(())
    }
}
