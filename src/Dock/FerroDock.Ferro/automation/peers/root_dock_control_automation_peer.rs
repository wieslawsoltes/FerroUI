// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::RootDockControl;
use crate::model_projection;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
#[repr(C)]
pub(crate) struct RootDockControlAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RootDockControlAutomationPeer:ControlAutomationPeer);
ferroui_base::ferro_class_info!(RootDockControlAutomationPeer {});
ferro_impl_classes!(RootDockControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for RootDockControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "RootDockControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|name| !name.trim().is_empty()) {
            return base;
        }
        let owner = this.owner();
        let root = model_projection::root(owner.data_context().as_ref());
        Some(Helper::resolve_name(
            &owner,
            "Root dock",
            root.map(|root| root as _),
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let root = model_projection::root(owner.data_context().as_ref());
        let id = Helper::resolve_automation_id(&owner, root.map(|root| root as _));
        if !id.trim().is_empty() {
            Some(id)
        } else {
            Some(Self::parent_get_automation_id_core(this).unwrap_or_default())
        }
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let Some(root) = model_projection::root(this.owner().data_context().as_ref()) else {
            return Some(Helper::format_state(&[(
                "HasRoot",
                Some(std::rc::Rc::new(false)),
            )]));
        };
        let pinned = root.pinned_dock();
        // Evaluate source getters left-to-right, preserving the pinned snapshot.
        Some(Helper::format_state(&[
            ("HasRoot", Some(std::rc::Rc::new(true))),
            (
                "FocusableRoot",
                Some(std::rc::Rc::new(root.is_focusable_root())),
            ),
            (
                "VisibleDockables",
                Some(std::rc::Rc::new(
                    root.visible_dockables().map_or(0, |v| v.count()),
                )),
            ),
            (
                "HiddenDockables",
                Some(std::rc::Rc::new(
                    root.hidden_dockables().map_or(0, |v| v.count()),
                )),
            ),
            (
                "PinnedLeft",
                Some(std::rc::Rc::new(
                    root.left_pinned_dockables().map_or(0, |v| v.count()),
                )),
            ),
            (
                "PinnedRight",
                Some(std::rc::Rc::new(
                    root.right_pinned_dockables().map_or(0, |v| v.count()),
                )),
            ),
            (
                "PinnedTop",
                Some(std::rc::Rc::new(
                    root.top_pinned_dockables().map_or(0, |v| v.count()),
                )),
            ),
            (
                "PinnedBottom",
                Some(std::rc::Rc::new(
                    root.bottom_pinned_dockables().map_or(0, |v| v.count()),
                )),
            ),
            (
                "PinnedVisible",
                Some(std::rc::Rc::new(
                    pinned
                        .and_then(|d| d.visible_dockables())
                        .map_or(0, |v| v.count()),
                )),
            ),
            (
                "ActiveDockable",
                Some(std::rc::Rc::new(
                    root.active_dockable()
                        .map_or_else(|| "none".into(), |d| d.title()),
                )),
            ),
        ]))
    }
}
impl RootDockControlAutomationPeer {
    pub(crate) fn new(owner: &RootDockControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<RootDockControl> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("root control owns this peer")
    }
}
