// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use super::dock_tab_strip_item_automation_peer::define_dock_tab_strip_item_peer;
use crate::{controls::DocumentTabStripItem, model_projection};
use ferrodock_model::{
    core::{DockCapability, DockingWindowState, IDockable},
    DockCapabilityResolver,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::{
    automation::{
        peers::{
            AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
            ControlAutomationPeer, ControlAutomationPeerImpl, ListItemAutomationPeer,
        },
        provider::{IInvokeProvider, ISelectionItemProvider, ISelectionProvider},
        ElementNotEnabledException, SelectionItemPatternIdentifiers,
    },
    primitives::TabStrip,
};
use std::rc::Rc;
define_dock_tab_strip_item_peer!(
    DocumentDockTabStripItemAutomationPeer,
    DocumentDockTabStripItemAutomationPeerImpl,
    DocumentTabStripItem,
    DocumentTabStripItemProvider
);
#[repr(C)]
pub(crate) struct DocumentTabStripItemAutomationPeer {
    base: DocumentDockTabStripItemAutomationPeer,
}
ferro_class!(DocumentTabStripItemAutomationPeer:DocumentDockTabStripItemAutomationPeer);
ferro_class_info!(DocumentTabStripItemAutomationPeer {});
ferro_impl_classes!(DocumentTabStripItemAutomationPeer:FerroObjectImpl,AutomationPeerImpl,ControlAutomationPeerImpl);
impl DocumentDockTabStripItemAutomationPeerImpl for DocumentTabStripItemAutomationPeer {
    fn class_name(_this: &Self) -> String {
        "DocumentTabStripItem".into()
    }
    fn fallback_name(_this: &Self) -> String {
        "Document tab".into()
    }
    fn build_state_text(this: &Self, dockable: Option<Rc<dyn IDockable>>) -> String {
        let owner = this.owner_control();
        Helper::format_state(&[
            ("Selected", Some(Rc::new(owner.is_selected()))),
            ("Active", Some(Rc::new(owner.is_active()))),
            (
                "CanClose",
                Some(Rc::new(capability(dockable.clone(), DockCapability::Close))),
            ),
            (
                "CanFloat",
                Some(Rc::new(capability(dockable.clone(), DockCapability::Float))),
            ),
            (
                "DockingState",
                Some(Rc::new(
                    dockable.map_or(DockingWindowState::NONE, |d| d.docking_state()),
                )),
            ),
        ])
    }
}
impl DocumentTabStripItemAutomationPeer {
    pub(crate) fn new(owner: &DocumentTabStripItem) -> Ref<Self> {
        instantiate(Self {
            base: DocumentDockTabStripItemAutomationPeer::construct(owner),
        })
    }
}
fn capability(dockable: Option<Rc<dyn IDockable>>, kind: DockCapability) -> bool {
    dockable.is_some_and(|d| {
        DockCapabilityResolver::is_enabled(
            Some(d.clone()),
            kind,
            DockCapabilityResolver::resolve_operation_dock(Some(d)),
        )
    })
}
