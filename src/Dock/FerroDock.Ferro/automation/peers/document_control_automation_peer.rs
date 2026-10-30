// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{controls::DocumentControl, model_projection};
use ferrodock_model::core::{IDock, IDockable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    Ref,
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
pub(crate) struct DocumentControlAutomationPeer {
    base: ControlAutomationPeer,
    owner: Ref<DocumentControl>,
}
ferro_class!(DocumentControlAutomationPeer:ControlAutomationPeer);
ferro_class_info!(DocumentControlAutomationPeer{interfaces:[Rc<dyn IInvokeProvider> => DocumentControlAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(DocumentControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for DocumentControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "DocumentControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner,
            "Document host",
            this.get_active_dockable(),
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let dock = this.get_dock();
        let id = Helper::resolve_automation_id(&this.owner, dock.map(|d| d as Rc<dyn IDockable>));
        Some(if !id.trim().is_empty() {
            id
        } else {
            Self::parent_get_automation_id_core(this).unwrap_or_default()
        })
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let dock = this.get_dock();
        let document = dock.as_ref().and_then(|d| d.clone().as_document_dock());
        Some(Helper::format_state(&[
            ("Active", Some(Rc::new(this.owner.is_active()))),
            ("TabsLayout", Some(Rc::new(this.owner.tabs_layout()))),
            (
                "HasVisibleDockables",
                Some(Rc::new(this.owner.has_visible_dockables())),
            ),
            (
                "VisibleDockables",
                Some(Rc::new(
                    dock.as_ref()
                        .and_then(|d| d.visible_dockables())
                        .map_or(0_i32, |v| v.count() as i32),
                )),
            ),
            (
                "CanCreateDocument",
                Some(Rc::new(
                    document.as_ref().is_some_and(|d| d.can_create_document()),
                )),
            ),
            (
                "LayoutMode",
                document.map(|d| Rc::new(d.layout_mode()) as BoxedValue),
            ),
            (
                "ActiveDockable",
                Some(Rc::new(
                    dock.and_then(|d| d.active_dockable())
                        .map_or_else(|| "none".into(), |d| d.title()),
                )),
            ),
        ]))
    }
}
impl DocumentControlAutomationPeer {
    pub(crate) fn new(owner: &DocumentControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn get_dock(&self) -> Option<Rc<dyn IDock>> {
        model_projection::dockable(self.owner.data_context().as_ref()).and_then(|d| d.as_dock())
    }
    fn get_active_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.get_dock().and_then(|d| d.active_dockable())
    }
    fn resolve_dockable_to_activate(&self) -> Option<Rc<dyn IDockable>> {
        let dock = self.get_dock()?;
        if let Some(active) = dock.active_dockable() {
            return Some(active);
        }
        dock.visible_dockables()
            .and_then(|list| if list.count() > 0 { list.get(0) } else { None })
    }
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(Provider(this))
    }
}
struct Provider(Ref<DocumentControlAutomationPeer>);
impl IInvokeProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        if !Helper::try_activate_dockable(self.0.resolve_dockable_to_activate()) {
            self.0.owner.focus();
        }
        Ok(())
    }
}
