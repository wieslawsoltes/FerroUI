// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{controls::ToolControl, model_projection};
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
pub(crate) struct ToolControlAutomationPeer {
    base: ControlAutomationPeer,
    owner: Ref<ToolControl>,
}
ferro_class!(ToolControlAutomationPeer:ControlAutomationPeer);
ferro_class_info!(ToolControlAutomationPeer{interfaces:[Rc<dyn IInvokeProvider> => ToolControlAutomationPeer::as_invoke_provider]});
ferro_impl_classes!(ToolControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for ToolControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ToolControl".into()
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
            "Tool host",
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
        let tool = dock.as_ref().and_then(|d| d.clone().as_tool_dock());
        Some(Helper::format_state(&[
            (
                "VisibleDockables",
                Some(Rc::new(
                    dock.as_ref()
                        .and_then(|d| d.visible_dockables())
                        .map_or(0_i32, |v| v.count() as i32),
                )),
            ),
            (
                "Expanded",
                Some(Rc::new(tool.as_ref().is_some_and(|d| d.is_expanded()))),
            ),
            (
                "AutoHide",
                Some(Rc::new(tool.as_ref().is_some_and(|d| d.auto_hide()))),
            ),
            (
                "Alignment",
                tool.as_ref().map(|d| Rc::new(d.alignment()) as BoxedValue),
            ),
            (
                "GripMode",
                tool.map(|d| Rc::new(d.grip_mode()) as BoxedValue),
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
impl ToolControlAutomationPeer {
    pub(crate) fn new(owner: &ToolControl) -> Ref<Self> {
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
struct Provider(Ref<ToolControlAutomationPeer>);
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
