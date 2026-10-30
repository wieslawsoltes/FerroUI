// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use super::dock_tab_strip_item_automation_peer::define_dock_tab_strip_item_peer;
use crate::{controls::ToolTabStripItem, model_projection};
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
    ToolDockTabStripItemAutomationPeer,
    ToolDockTabStripItemAutomationPeerImpl,
    ToolTabStripItem,
    ToolTabStripItemProvider
);
#[repr(C)]
pub(crate) struct ToolTabStripItemAutomationPeer {
    base: ToolDockTabStripItemAutomationPeer,
}
ferro_class!(ToolTabStripItemAutomationPeer:ToolDockTabStripItemAutomationPeer);
ferro_class_info!(ToolTabStripItemAutomationPeer {});
ferro_impl_classes!(ToolTabStripItemAutomationPeer:FerroObjectImpl,AutomationPeerImpl,ControlAutomationPeerImpl);
impl ToolDockTabStripItemAutomationPeerImpl for ToolTabStripItemAutomationPeer {
    fn class_name(_this: &Self) -> String {
        "ToolTabStripItem".into()
    }
    fn fallback_name(_this: &Self) -> String {
        "Tool tab".into()
    }
    fn build_state_text(this: &Self, dockable: Option<Rc<dyn IDockable>>) -> String {
        let owner = this.owner_control();
        Helper::format_state(&[
            ("Selected", Some(Rc::new(owner.is_selected()))),
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
impl ToolTabStripItemAutomationPeer {
    pub(crate) fn new(owner: &ToolTabStripItem) -> Ref<Self> {
        instantiate(Self {
            base: ToolDockTabStripItemAutomationPeer::construct(owner),
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
