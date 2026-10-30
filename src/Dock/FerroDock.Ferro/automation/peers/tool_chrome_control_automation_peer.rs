// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{controls::ToolChromeControl, model_projection};
use ferrodock_model::{
    core::{DockCapability, IDockable},
    DockCapabilityResolver,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::{
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ContentControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::{IExpandCollapseProvider, IInvokeProvider},
    ElementNotEnabledException, ExpandCollapseState,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct ToolChromeControlAutomationPeer {
    base: ContentControlAutomationPeer,
    owner: Ref<ToolChromeControl>,
}
ferro_class!(ToolChromeControlAutomationPeer:ContentControlAutomationPeer);
ferro_class_info!(ToolChromeControlAutomationPeer {interfaces:[Rc<dyn IInvokeProvider> => ToolChromeControlAutomationPeer::as_invoke_provider,Rc<dyn IExpandCollapseProvider> => ToolChromeControlAutomationPeer::as_expand_collapse_provider]});
ferro_impl_classes!(ToolChromeControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for ToolChromeControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ToolChromeControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TitleBar
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        let dockable = this.active_dockable();
        if this.owner.title().is_some_and(|t| !t.trim().is_empty()) {
            return this.owner.title();
        }
        Some(Helper::resolve_name(
            &this.owner,
            "Tool window chrome",
            dockable,
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let id = Helper::resolve_automation_id(&this.owner, this.active_dockable());
        Some(if !id.trim().is_empty() {
            id
        } else {
            Self::parent_get_automation_id_core(this).unwrap_or_default()
        })
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let dockable = this.active_dockable();
        Some(Helper::format_state(&[
            ("Active", Some(Rc::new(this.owner.is_active()))),
            ("Pinned", Some(Rc::new(this.owner.is_pinned()))),
            ("Floating", Some(Rc::new(this.owner.is_floating()))),
            ("Maximized", Some(Rc::new(this.owner.is_maximized()))),
            ("HasMenu", Some(Rc::new(this.owner.tool_flyout().is_some()))),
            (
                "CanClose",
                Some(Rc::new(Self::is_capability_enabled(
                    dockable,
                    DockCapability::Close,
                ))),
            ),
        ]))
    }
}
impl ToolChromeControlAutomationPeer {
    pub(crate) fn new(owner: &ToolChromeControl) -> Ref<Self> {
        instantiate(Self {
            base: ContentControlAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn active_dockable(&self) -> Option<Rc<dyn IDockable>> {
        model_projection::dockable(self.owner.data_context().as_ref())
            .and_then(|d| d.as_dock())
            .and_then(|d| d.active_dockable())
    }
    fn is_capability_enabled(
        dockable: Option<Rc<dyn IDockable>>,
        capability: DockCapability,
    ) -> bool {
        dockable.is_some_and(|d| {
            DockCapabilityResolver::is_enabled(
                Some(d.clone()),
                capability,
                DockCapabilityResolver::resolve_operation_dock(Some(d)),
            )
        })
    }
    fn as_invoke_provider(this: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(Provider(this))
    }
    fn as_expand_collapse_provider(this: Ref<Self>) -> Rc<dyn IExpandCollapseProvider> {
        Rc::new(Provider(this))
    }
}
struct Provider(Ref<ToolChromeControlAutomationPeer>);
impl IInvokeProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        if !Helper::try_activate_dockable(self.0.active_dockable()) {
            self.0.owner.focus();
        }
        Ok(())
    }
}
impl IExpandCollapseProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        if self.0.owner.tool_flyout().is_some_and(|f| f.is_open()) {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
    fn shows_menu(&self) -> bool {
        true
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        if let Some(flyout) = self.0.owner.tool_flyout().filter(|f| !f.is_open()) {
            flyout.show_at(&self.0.owner);
        }
        Ok(())
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        if let Some(flyout) = self.0.owner.tool_flyout().filter(|f| f.is_open()) {
            flyout.hide();
        }
        Ok(())
    }
}
