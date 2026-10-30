// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{controls::PinnedDockControl, model_projection};
use ferrodock_model::controls::IToolDock;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::{
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::IExpandCollapseProvider,
    AutomationProperties, ElementNotEnabledException, ExpandCollapseState,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct PinnedDockControlAutomationPeer {
    base: ControlAutomationPeer,
    owner: Ref<PinnedDockControl>,
}
ferro_class!(PinnedDockControlAutomationPeer:ControlAutomationPeer);
ferro_class_info!(PinnedDockControlAutomationPeer{interfaces:[Rc<dyn IExpandCollapseProvider> => PinnedDockControlAutomationPeer::as_expand_collapse_provider]});
ferro_impl_classes!(PinnedDockControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for PinnedDockControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "PinnedDockControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        let pinned = this.get_pinned_dock();
        let name = AutomationProperties::get_name(&this.owner);
        if name.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return name;
        }
        let title = pinned.and_then(|d| d.active_dockable()).map(|d| d.title());
        Some(
            title
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "Pinned dock host".into()),
        )
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let id = AutomationProperties::get_automation_id(&this.owner);
        if id.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return id;
        }
        if let Some(pinned) = this.get_pinned_dock() {
            let id = pinned.id();
            if !id.trim().is_empty() {
                return Some(id);
            }
        }
        Some(Self::parent_get_automation_id_core(this).unwrap_or_default())
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let pinned = this.get_pinned_dock();
        Some(Helper::format_state(&[
            (
                "Alignment",
                Some(Rc::new(this.owner.pinned_dock_alignment())),
            ),
            (
                "DisplayMode",
                Some(Rc::new(this.owner.pinned_dock_display_mode())),
            ),
            ("HasPinnedDock", Some(Rc::new(pinned.is_some()))),
            (
                "Expanded",
                Some(Rc::new(pinned.as_ref().is_some_and(|d| d.is_expanded()))),
            ),
            (
                "VisibleDockables",
                Some(Rc::new(
                    pinned
                        .as_ref()
                        .and_then(|d| d.visible_dockables())
                        .map_or(0_i32, |l| l.count() as i32),
                )),
            ),
            (
                "ActiveDockable",
                Some(Rc::new(
                    pinned
                        .and_then(|d| d.active_dockable())
                        .map_or_else(|| "none".into(), |d| d.title()),
                )),
            ),
        ]))
    }
}
impl PinnedDockControlAutomationPeer {
    pub(crate) fn new(owner: &PinnedDockControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn get_pinned_dock(&self) -> Option<Rc<dyn IToolDock>> {
        model_projection::root(self.owner.data_context().as_ref()).and_then(|r| r.pinned_dock())
    }
    fn as_expand_collapse_provider(this: Ref<Self>) -> Rc<dyn IExpandCollapseProvider> {
        Rc::new(Provider(this))
    }
}
struct Provider(Ref<PinnedDockControlAutomationPeer>);
impl IExpandCollapseProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        let Some(pinned) = self.0.get_pinned_dock() else {
            return ExpandCollapseState::Collapsed;
        };
        if pinned.visible_dockables().is_none()
            || pinned
                .visible_dockables()
                .expect("source visible list")
                .count()
                == 0
        {
            return ExpandCollapseState::Collapsed;
        }
        if pinned.is_expanded() {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
    fn shows_menu(&self) -> bool {
        false
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        if let Some(pinned) = self.0.get_pinned_dock() {
            pinned.set_is_expanded(true);
        }
        Ok(())
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        if let Some(pinned) = self.0.get_pinned_dock() {
            pinned.set_is_expanded(false);
        }
        Ok(())
    }
}
