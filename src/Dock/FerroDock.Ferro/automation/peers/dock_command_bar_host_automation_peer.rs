// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::DockCommandBarHost;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct DockCommandBarHostAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(DockCommandBarHostAutomationPeer:ControlAutomationPeer);
ferroui_base::ferro_class_info!(DockCommandBarHostAutomationPeer {});
ferro_impl_classes!(DockCommandBarHostAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for DockCommandBarHostAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "DockCommandBarHost".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ToolBar
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|v| !v.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner(),
            "Dock command bars",
            None,
        ))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        Some(Helper::format_state(&[
            ("Visible", Some(Rc::new(owner.is_visible()))),
            (
                "MenuBars",
                Some(Rc::new(owner.menu_bars().map_or(0, |v| v.count()) as i32)),
            ),
            (
                "ToolBars",
                Some(Rc::new(owner.tool_bars().map_or(0, |v| v.count()) as i32)),
            ),
            (
                "RibbonBars",
                Some(Rc::new(owner.ribbon_bars().map_or(0, |v| v.count()) as i32)),
            ),
            (
                "BaseDefinitions",
                Some(Rc::new(
                    owner.base_command_bars().map_or(0, |v| v.count()) as i32
                )),
            ),
        ]))
    }
}
impl DockCommandBarHostAutomationPeer {
    pub(crate) fn new(owner: &DockCommandBarHost) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<DockCommandBarHost> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("command-bar host owner")
    }
}
