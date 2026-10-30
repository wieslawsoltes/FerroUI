// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{
    controls::{ToolPinItemControl, ToolPinnedControl},
    model_projection,
};
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
        ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::{IInvokeProvider, ISelectionItemProvider, ISelectionProvider},
    ElementNotEnabledException,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct ToolPinItemControlAutomationPeer {
    base: ControlAutomationPeer,
    owner: Ref<ToolPinItemControl>,
}
ferro_class!(ToolPinItemControlAutomationPeer:ControlAutomationPeer);
ferro_class_info!(ToolPinItemControlAutomationPeer {interfaces:[Rc<dyn IInvokeProvider> => ToolPinItemControlAutomationPeer::as_invoke_provider,Rc<dyn ISelectionItemProvider> => ToolPinItemControlAutomationPeer::as_selection_item_provider]});
ferro_impl_classes!(ToolPinItemControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for ToolPinItemControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ToolPinItemControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TabItem
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner,
            "Pinned tool",
            this.dockable(),
        ))
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        let id = Helper::resolve_automation_id(&this.owner, this.dockable());
        Some(if !id.trim().is_empty() {
            id
        } else {
            Self::parent_get_automation_id_core(this).unwrap_or_default()
        })
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let dockable = this.dockable();
        Some(Helper::format_state(&[
            ("Orientation", Some(Rc::new(this.owner.orientation()))),
            (
                "CanPin",
                Some(Rc::new(Self::is_capability_enabled(
                    dockable.clone(),
                    DockCapability::Pin,
                ))),
            ),
            (
                "CanClose",
                Some(Rc::new(Self::is_capability_enabled(
                    dockable.clone(),
                    DockCapability::Close,
                ))),
            ),
            (
                "CanFloat",
                Some(Rc::new(Self::is_capability_enabled(
                    dockable.clone(),
                    DockCapability::Float,
                ))),
            ),
            (
                "DockingState",
                dockable.map(|d| Rc::new(d.docking_state()) as _),
            ),
        ]))
    }
}
impl ToolPinItemControlAutomationPeer {
    pub(crate) fn new(owner: &ToolPinItemControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            owner: owner.to_ref(),
        })
    }
    fn dockable(&self) -> Option<Rc<dyn IDockable>> {
        model_projection::dockable(self.owner.data_context().as_ref())
    }
    fn invoke(&self) {
        if !Helper::try_activate_dockable(self.dockable()) {
            self.owner.focus();
        }
    }
    fn is_selected(&self) -> bool {
        self.dockable().is_some_and(|d| {
            d.owner()
                .and_then(|o| o.as_dock())
                .and_then(|o| o.active_dockable())
                .is_some_and(|a| Rc::ptr_eq(&a, &d))
        })
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        let container = self
            .owner
            .find_ancestor_of_type::<ToolPinnedControl>(false)?;
        ControlAutomationPeer::create_peer_for_element(&container)
            .get_provider::<dyn ISelectionProvider>()
    }
    fn remove_from_selection(&self) {
        if !self.is_selected() {
            return;
        }
        let Some((dockable, owner)) = self
            .dockable()
            .and_then(|d| d.owner().and_then(|o| o.as_dock()).map(|o| (d, o)))
        else {
            return;
        };
        let Some(visible) = owner.visible_dockables() else {
            return;
        };
        let mut index = 0;
        while index < visible.count() {
            let next = visible.get(index);
            index += 1;
            if next
                .as_ref()
                .is_some_and(|next| Rc::ptr_eq(next, &dockable))
            {
                continue;
            }
            Helper::try_activate_dockable(next);
            return;
        }
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
    fn as_selection_item_provider(this: Ref<Self>) -> Rc<dyn ISelectionItemProvider> {
        Rc::new(Provider(this))
    }
}
struct Provider(Ref<ToolPinItemControlAutomationPeer>);
impl IInvokeProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.invoke();
        Ok(())
    }
}
impl ISelectionItemProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.selection_container()
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.invoke();
        Ok(())
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.remove_from_selection();
        Ok(())
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.invoke();
        Ok(())
    }
}
