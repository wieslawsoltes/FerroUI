// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::DockControl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct DockControlAutomationPeer {
    base: ControlAutomationPeer,
    owner: Ref<DockControl>,
}
ferro_class!(DockControlAutomationPeer:ControlAutomationPeer);
ferro_class_info!(DockControlAutomationPeer {});
ferro_impl_classes!(DockControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for DockControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "DockControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|n| !n.trim().is_empty()) {
            return base;
        }
        let owner = this.owner();
        Some(Helper::resolve_name(
            &owner,
            "Dock host",
            owner.layout().map(|l| l as _),
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let id = Helper::resolve_automation_id(&owner, owner.layout().map(|l| l as _));
        Some(if !id.trim().is_empty() {
            id
        } else {
            Self::parent_get_automation_id_core(this).unwrap_or_default()
        })
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let layout = owner.layout();
        Some(Helper::format_state(&[
            (
                "Layout",
                Some(Rc::new(
                    layout.as_ref().map_or_else(|| "none".into(), |l| l.id()),
                )),
            ),
            ("DockingEnabled", Some(Rc::new(owner.is_docking_enabled()))),
            ("Dragging", Some(Rc::new(owner.is_dragging_dock()))),
            ("SelectorOpen", Some(Rc::new(owner.is_open()))),
            (
                "ManagedWindowLayer",
                Some(Rc::new(owner.enable_managed_window_layer())),
            ),
            (
                "VisibleDockables",
                Some(Rc::new(
                    layout
                        .as_ref()
                        .and_then(|l| l.visible_dockables())
                        .map_or(0, |l| l.count()),
                )),
            ),
            (
                "GlobalDocking",
                Some(Rc::new(
                    layout.as_ref().is_some_and(|l| l.enable_global_docking()),
                )),
            ),
        ]))
    }
}
impl DockControlAutomationPeer {
    pub(crate) fn new(owner: &DockControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn owner(&self) -> Ref<DockControl> {
        self.owner.clone()
    }
}
