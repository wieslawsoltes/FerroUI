// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use super::dock_tab_strip_automation_peer::define_dock_tab_strip_peer;
use crate::{controls::ToolTabStrip, model_projection};
use ferrodock_model::core::IDock;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeerImpl,
    ItemsControlAutomationPeerImpl, SelectingItemsControlAutomationPeer,
    SelectingItemsControlAutomationPeerImpl,
};
use std::rc::Rc;
define_dock_tab_strip_peer!(
    ToolDockTabStripAutomationPeer,
    ToolDockTabStripAutomationPeerImpl,
    ToolTabStrip
);
#[repr(C)]
pub(crate) struct ToolTabStripAutomationPeer {
    base: ToolDockTabStripAutomationPeer,
}
ferro_class!(ToolTabStripAutomationPeer:ToolDockTabStripAutomationPeer);
ferro_class_info!(ToolTabStripAutomationPeer {});
ferro_impl_classes!(ToolTabStripAutomationPeer:FerroObjectImpl,AutomationPeerImpl,ControlAutomationPeerImpl,ItemsControlAutomationPeerImpl,SelectingItemsControlAutomationPeerImpl);
impl ToolDockTabStripAutomationPeerImpl for ToolTabStripAutomationPeer {
    fn class_name(_this: &Self) -> String {
        "ToolTabStrip".into()
    }
    fn fallback_name(_this: &Self) -> String {
        "Tool tabs".into()
    }
    fn build_state_text(this: &Self, dock: Option<Rc<dyn IDock>>) -> String {
        let tool = dock.and_then(|d| d.as_tool_dock());
        let owner = this.owner_control();
        Helper::format_state(&[
            ("CanCreate", Some(Rc::new(owner.can_create_item()))),
            ("SelectedIndex", Some(Rc::new(owner.selected_index()))),
            ("Items", Some(Rc::new(owner.items().count()))),
            (
                "ScrollOrientation",
                Some(Rc::new(owner.mouse_wheel_scroll_orientation())),
            ),
            (
                "Alignment",
                tool.as_ref().map(|d| Rc::new(d.alignment()) as _),
            ),
            (
                "Expanded",
                Some(Rc::new(tool.as_ref().is_some_and(|d| d.is_expanded()))),
            ),
            (
                "AutoHide",
                Some(Rc::new(tool.as_ref().is_some_and(|d| d.auto_hide()))),
            ),
        ])
    }
}
impl ToolTabStripAutomationPeer {
    pub(crate) fn new(owner: &ToolTabStrip) -> Ref<Self> {
        instantiate(Self {
            base: ToolDockTabStripAutomationPeer::construct(owner),
        })
    }
}
