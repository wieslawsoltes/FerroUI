// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{controls::MdiDocumentControl, model_projection};
use ferrodock_model::core::{IDock, IDockable};
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
pub(crate) struct MdiDocumentControlAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(MdiDocumentControlAutomationPeer:ControlAutomationPeer);
ferro_class_info!(MdiDocumentControlAutomationPeer{interfaces:[Rc<dyn IInvokeProvider> => MdiDocumentControlAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(MdiDocumentControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for MdiDocumentControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "MdiDocumentControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let name = Self::parent_get_name_core(this);
        if name.as_ref().is_some_and(|n| !n.trim().is_empty()) {
            return name;
        }
        Some(Helper::resolve_name(
            &this.owner(),
            "MDI document host",
            this.get_active_dockable(),
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let id = Helper::resolve_automation_id(
            &this.owner(),
            this.get_dock().map(|d| d as Rc<dyn IDockable>),
        );
        if !id.trim().is_empty() {
            Some(id)
        } else {
            Some(Self::parent_get_automation_id_core(this).unwrap_or_default())
        }
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let dock = this.get_dock();
        Some(Helper::format_state(&[
            ("Active", Some(Rc::new(owner.is_active()))),
            (
                "HasVisibleDocuments",
                Some(Rc::new(owner.has_visible_documents())),
            ),
            (
                "VisibleDockables",
                Some(Rc::new(
                    dock.as_ref()
                        .and_then(|d| d.visible_dockables())
                        .map(|v| v.count())
                        .unwrap_or(0) as i32,
                )),
            ),
            (
                "LayoutManager",
                Some(Rc::new(
                    owner
                        .layout_manager()
                        .map(|m| m.runtime_type_name())
                        .unwrap_or("none")
                        .to_owned(),
                )),
            ),
            (
                "ActiveDockable",
                Some(Rc::new(
                    dock.and_then(|d| d.active_dockable())
                        .map(|d| d.title())
                        .unwrap_or_else(|| "none".into()),
                )),
            ),
        ]))
    }
}
impl MdiDocumentControlAutomationPeer {
    pub(crate) fn new(owner: &MdiDocumentControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<MdiDocumentControl> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("native MDI document host peer owner")
    }
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(InvokeProvider(this))
    }
    fn get_dock(&self) -> Option<Rc<dyn IDock>> {
        model_projection::dockable(self.owner().data_context().as_ref()).and_then(|d| d.as_dock())
    }
    fn get_active_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.get_dock()?.active_dockable()
    }
    fn resolve_dockable_to_activate(&self) -> Option<Rc<dyn IDockable>> {
        let dock = self.get_dock()?;
        if let Some(active) = dock.active_dockable() {
            return Some(active);
        }
        dock.visible_dockables()?.iter().next().flatten()
    }
}
struct InvokeProvider(Ref<MdiDocumentControlAutomationPeer>);
impl IInvokeProvider for InvokeProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        let dockable = self.0.resolve_dockable_to_activate();
        if dockable.is_none() || !Helper::try_activate_dockable(dockable) {
            self.0.owner().focus();
        }
        Ok(())
    }
}
