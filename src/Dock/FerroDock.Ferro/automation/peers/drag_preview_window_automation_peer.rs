// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::DragPreviewWindow;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl, WindowAutomationPeer,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct DragPreviewWindowAutomationPeer {
    base: WindowAutomationPeer,
}
ferro_class!(DragPreviewWindowAutomationPeer:WindowAutomationPeer);
ferroui_base::ferro_class_info!(DragPreviewWindowAutomationPeer {});
ferro_impl_classes!(DragPreviewWindowAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for DragPreviewWindowAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "DragPreviewWindow".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|name| !name.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner(),
            "Drag preview window",
            None,
        ))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        Some(Helper::format_state(&[
            ("Visible", Some(Rc::new(owner.is_visible()))),
            ("WindowState", Some(Rc::new(owner.window_state()))),
        ]))
    }
    fn is_control_element_core(_this: &Self) -> bool {
        false
    }
    fn is_content_element_core(_this: &Self) -> bool {
        false
    }
}
impl DragPreviewWindowAutomationPeer {
    pub(crate) fn new(owner: &DragPreviewWindow) -> Ref<Self> {
        instantiate(Self {
            base: WindowAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<DragPreviewWindow> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("native drag-preview window peer owner")
    }
}
