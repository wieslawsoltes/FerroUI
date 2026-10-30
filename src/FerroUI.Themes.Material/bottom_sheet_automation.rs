// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: MIT
//! Accessible expand/collapse actions for the Material sheet's semantic states.

use crate::{MaterialBottomSheetHost, MaterialSheetState};
use ferroui_base::{BoxedValue, FerroObjectImpl, Ref, ferro_class, ferro_class_info, instantiate};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::IExpandCollapseProvider;
use ferroui_controls::automation::{
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
};
use std::cell::Cell;
use std::rc::Rc;

/// A sheet remains a pane, with native children and explicit expansion actions.
#[repr(C)]
pub struct MaterialBottomSheetAutomationPeer {
    base: ControlAutomationPeer,
    previous: Cell<ExpandCollapseState>,
}

ferro_class!(MaterialBottomSheetAutomationPeer: ControlAutomationPeer);
ferro_class_info!(MaterialBottomSheetAutomationPeer {
    interfaces: [Rc<dyn IExpandCollapseProvider> => MaterialBottomSheetAutomationPeer::provider]
});

impl FerroObjectImpl for MaterialBottomSheetAutomationPeer {}
impl ControlAutomationPeerImpl for MaterialBottomSheetAutomationPeer {}
impl AutomationPeerImpl for MaterialBottomSheetAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
}

impl MaterialBottomSheetAutomationPeer {
    pub fn new(owner: &MaterialBottomSheetHost) -> Ref<Self> {
        let this = instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            previous: Cell::new(Self::state(owner.current_state())),
        });
        let weak = this.downgrade();
        owner.settled(move |_, _| {
            if let Some(this) = weak.upgrade() {
                let state = Self::state(this.owner().current_state());
                let previous = this.previous.replace(state);
                if previous != state {
                    this.raise_property_changed_event(
                        ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                        Some(Rc::new(previous) as BoxedValue),
                        Some(Rc::new(state) as BoxedValue),
                    );
                }
            }
        });
        this
    }

    fn owner(&self) -> Ref<MaterialBottomSheetHost> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("Material sheet peer owner")
    }

    fn state(state: MaterialSheetState) -> ExpandCollapseState {
        match state {
            MaterialSheetState::Hidden => ExpandCollapseState::Collapsed,
            MaterialSheetState::PartiallyExpanded => ExpandCollapseState::PartiallyExpanded,
            MaterialSheetState::Expanded => ExpandCollapseState::Expanded,
        }
    }

    fn provider(this: Ref<Self>) -> Rc<dyn IExpandCollapseProvider> {
        Rc::new(SheetExpandCollapseProvider(this))
    }
}

struct SheetExpandCollapseProvider(Ref<MaterialBottomSheetAutomationPeer>);

impl IExpandCollapseProvider for SheetExpandCollapseProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        MaterialBottomSheetAutomationPeer::state(self.0.owner().current_state())
    }
    fn shows_menu(&self) -> bool {
        false
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        if self.0.owner().expand() {
            Ok(())
        } else {
            Err(ElementNotEnabledException::with_message(
                "Sheet expansion was declined.",
            ))
        }
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        let owner = self.0.owner();
        if owner.is_dismiss_enabled() && owner.hide() {
            Ok(())
        } else {
            Err(ElementNotEnabledException::with_message(
                "Sheet dismissal is disabled or was declined.",
            ))
        }
    }
}
