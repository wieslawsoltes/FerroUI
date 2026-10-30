// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::{DockTargetBase, GlobalDockTarget};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::IInvokeProvider;
use ferroui_controls::automation::ElementNotEnabledException;
use std::rc::Rc;
#[repr(C)]
pub(crate) struct DockTargetAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(DockTargetAutomationPeer:ControlAutomationPeer);
ferroui_base::ferro_class_info!(DockTargetAutomationPeer {interfaces:[Rc<dyn IInvokeProvider> => DockTargetAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(DockTargetAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for DockTargetAutomationPeer {
    fn get_class_name_core(this: &Self) -> String {
        if this.global() {
            "GlobalDockTarget".into()
        } else {
            "DockTarget".into()
        }
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|name| !name.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner(),
            if this.global() {
                "Global dock target"
            } else {
                "Dock target"
            },
            None,
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let id = Helper::resolve_automation_id(&this.owner(), None);
        if !id.trim().is_empty() {
            Some(id)
        } else {
            Some(Self::parent_get_automation_id_core(this).unwrap_or_default())
        }
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        Some(Helper::format_state(&[
            ("Horizontal", Some(Rc::new(owner.show_horizontal_targets()))),
            ("Vertical", Some(Rc::new(owner.show_vertical_targets()))),
            (
                "GlobalAvailable",
                Some(Rc::new(owner.is_global_dock_available())),
            ),
            ("GlobalActive", Some(Rc::new(owner.is_global_dock_active()))),
        ]))
    }
    fn is_control_element_core(this: &Self) -> bool {
        this.owner().is_visible()
    }
    fn is_content_element_core(this: &Self) -> bool {
        this.owner().is_visible()
    }
}
struct InvokeProvider(Ref<DockTargetAutomationPeer>);
impl IInvokeProvider for InvokeProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.owner().focus();
        Ok(())
    }
}
impl DockTargetAutomationPeer {
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(InvokeProvider(this.to_ref()))
    }

    pub(crate) fn new(owner: &DockTargetBase) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<DockTargetBase> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("native target peer owner")
    }
    fn global(&self) -> bool {
        self.owner().cast::<GlobalDockTarget>().is_some()
    }
}
