// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use super::dock_tab_strip_automation_peer::define_dock_tab_strip_peer;
use crate::{controls::DocumentTabStrip, model_projection};
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
    DocumentDockTabStripAutomationPeer,
    DocumentDockTabStripAutomationPeerImpl,
    DocumentTabStrip
);
#[repr(C)]
pub(crate) struct DocumentTabStripAutomationPeer {
    base: DocumentDockTabStripAutomationPeer,
}
ferro_class!(DocumentTabStripAutomationPeer:DocumentDockTabStripAutomationPeer);
ferro_class_info!(DocumentTabStripAutomationPeer {});
ferro_impl_classes!(DocumentTabStripAutomationPeer:FerroObjectImpl,AutomationPeerImpl,ControlAutomationPeerImpl,ItemsControlAutomationPeerImpl,SelectingItemsControlAutomationPeerImpl);
impl DocumentDockTabStripAutomationPeerImpl for DocumentTabStripAutomationPeer {
    fn class_name(_this: &Self) -> String {
        "DocumentTabStrip".into()
    }
    fn fallback_name(_this: &Self) -> String {
        "Document tabs".into()
    }
    fn build_state_text(this: &Self, dock: Option<Rc<dyn IDock>>) -> String {
        let document = dock.and_then(|d| d.as_document_dock());
        let owner = this.owner_control();
        Helper::format_state(&[
            ("CanCreate", Some(Rc::new(owner.can_create_item()))),
            ("Active", Some(Rc::new(owner.is_active()))),
            (
                "EnableWindowDrag",
                Some(Rc::new(owner.enable_window_drag())),
            ),
            ("Orientation", Some(Rc::new(owner.orientation()))),
            ("SelectedIndex", Some(Rc::new(owner.selected_index()))),
            ("Items", Some(Rc::new(owner.items().count()))),
            (
                "CanCreateDocument",
                Some(Rc::new(
                    document.as_ref().is_some_and(|d| d.can_create_document()),
                )),
            ),
            (
                "LayoutMode",
                document.map(|d| Rc::new(d.layout_mode()) as _),
            ),
        ])
    }
}
impl DocumentTabStripAutomationPeer {
    pub(crate) fn new(owner: &DocumentTabStrip) -> Ref<Self> {
        instantiate(Self {
            base: DocumentDockTabStripAutomationPeer::construct(owner),
        })
    }
}
