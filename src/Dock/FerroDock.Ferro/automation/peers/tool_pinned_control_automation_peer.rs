// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{controls::ToolPinnedControl, model_projection};
use ferrodock_model::core::IDockable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::{
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::ISelectionProvider,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct ToolPinnedControlAutomationPeer {
    base: ControlAutomationPeer,
    owner: Ref<ToolPinnedControl>,
}
ferro_class!(ToolPinnedControlAutomationPeer:ControlAutomationPeer);
ferro_class_info!(ToolPinnedControlAutomationPeer {interfaces:[Rc<dyn ISelectionProvider> => ToolPinnedControlAutomationPeer::as_selection_provider]});
ferro_impl_classes!(ToolPinnedControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for ToolPinnedControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ToolPinnedControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Tab
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(&this.owner, "Pinned tool tabs", None))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        Some(Helper::format_state(&[
            ("Orientation", Some(Rc::new(this.owner.orientation()))),
            ("Items", Some(Rc::new(this.owner.items().count()))),
            (
                "HasSelection",
                Some(Rc::new(this.selected_dockable().is_some())),
            ),
        ]))
    }
}
impl ToolPinnedControlAutomationPeer {
    pub(crate) fn new(owner: &ToolPinnedControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn selected_dockable(&self) -> Option<Rc<dyn IDockable>> {
        let mut i = 0;
        while i < self.owner.items().count() {
            let item = self.owner.items().get_at(i);
            i += 1;
            let Some(owner) = model_projection::dockable(item.as_ref())
                .and_then(|d| d.owner())
                .and_then(|o| o.as_dock())
            else {
                continue;
            };
            let Some(active) = owner.active_dockable() else {
                continue;
            };
            let mut index = 0;
            while index < self.owner.items().count() {
                let candidate =
                    model_projection::dockable(self.owner.items().get_at(index).as_ref());
                index += 1;
                if candidate.is_some_and(|c| Rc::ptr_eq(&c, &active)) {
                    return Some(active);
                }
            }
        }
        None
    }
    fn as_selection_provider(this: Ref<Self>) -> Rc<dyn ISelectionProvider> {
        Rc::new(Provider(this))
    }
}
struct Provider(Ref<ToolPinnedControlAutomationPeer>);
impl ISelectionProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn can_select_multiple(&self) -> bool {
        false
    }
    fn is_selection_required(&self) -> bool {
        false
    }
    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        let selected = self.0.selected_dockable();
        let peer = selected
            .map(model_projection::to_dockable_value)
            .and_then(|v| Helper::try_get_container_peer(&self.0.owner, Some(&v)));
        Helper::to_selection_list(peer)
    }
}
