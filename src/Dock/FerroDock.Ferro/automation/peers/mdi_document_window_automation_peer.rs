// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{controls::MdiDocumentWindow, model_projection};
use ferrodock_model::{
    core::{DockCapability, IDockable},
    DockCapabilityResolver,
};
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
pub(crate) struct MdiDocumentWindowAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(MdiDocumentWindowAutomationPeer:ControlAutomationPeer);
ferro_class_info!(MdiDocumentWindowAutomationPeer {interfaces:[Rc<dyn IInvokeProvider> => MdiDocumentWindowAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(MdiDocumentWindowAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for MdiDocumentWindowAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "MdiDocumentWindow".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Window
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|v| !v.trim().is_empty()) {
            return base;
        }
        let owner = this.owner();
        Some(Helper::resolve_name(
            &owner,
            "MDI document window",
            model_projection::dockable(owner.data_context().as_ref()),
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let id = Helper::resolve_automation_id(
            &owner,
            model_projection::dockable(owner.data_context().as_ref()),
        );
        if !id.trim().is_empty() {
            Some(id)
        } else {
            Some(Self::parent_get_automation_id_core(this).unwrap_or_default())
        }
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let dockable = model_projection::dockable(owner.data_context().as_ref());
        let state = model_projection::mdi_document(owner.data_context().as_ref())
            .map(|d| d.mdi_state())
            .unwrap_or_else(|| owner.mdi_state());
        Some(Helper::format_state(&[
            ("Active", Some(Rc::new(owner.is_active()))),
            ("MdiState", Some(Rc::new(state))),
            (
                "CanClose",
                Some(Rc::new(enabled(dockable.clone(), DockCapability::Close))),
            ),
            (
                "CanDrag",
                Some(Rc::new(enabled(dockable, DockCapability::Drag))),
            ),
        ]))
    }
}
fn enabled(dockable: Option<Rc<dyn IDockable>>, capability: DockCapability) -> bool {
    dockable.is_some_and(|d| {
        DockCapabilityResolver::is_enabled(
            Some(d.clone()),
            capability,
            DockCapabilityResolver::resolve_operation_dock(Some(d)),
        )
    })
}
impl MdiDocumentWindowAutomationPeer {
    pub(crate) fn new(owner: &MdiDocumentWindow) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<MdiDocumentWindow> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("native MDI window peer owner")
    }
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(InvokeProvider(this))
    }
}
struct InvokeProvider(Ref<MdiDocumentWindowAutomationPeer>);
impl IInvokeProvider for InvokeProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner();
        if !Helper::try_activate_dockable(model_projection::dockable(owner.data_context().as_ref()))
        {
            owner.focus();
        }
        Ok(())
    }
}
