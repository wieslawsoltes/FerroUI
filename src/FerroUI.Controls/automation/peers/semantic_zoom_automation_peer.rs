//! WinUI's `SemanticZoomAutomationPeer` (`dxaml/xcp/dxaml/lib/SemanticZoomAutomationPeer_Partial.cpp`
//! of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IToggleProvider, ProviderAdapter, ToggleState};
use crate::automation::{AutomationError, TogglePatternIdentifiers};
use crate::{Control, SemanticZoom};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`SemanticZoom`]: the toggle pattern changes the view
/// (on while the zoomed-in view is active), and the children are those of the active view.
///
/// WinUI's control type is `SemanticZoom`, a control type of UI Automation alone; the port reports
/// a group, which every platform's accessibility knows.
#[repr(C)]
pub struct SemanticZoomAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(SemanticZoomAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(SemanticZoomAutomationPeer {
    interfaces: [Rc<dyn IToggleProvider> => ProviderAdapter::as_toggle_provider]
});

impl FerroObjectImpl for SemanticZoomAutomationPeer {}

impl ControlAutomationPeerImpl for SemanticZoomAutomationPeer {
    /// WinUI's `GetChildrenCore`: the peer of the active view.
    fn get_children_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        let owner = this.owner();
        let view = if owner.is_zoomed_in_view_active() { owner.zoomed_in_view() } else { owner.zoomed_out_view() };
        let view: Ref<Control> = view?;
        Some(vec![this.get_or_create(&view)])
    }
}

impl AutomationPeerImpl for SemanticZoomAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }

    fn get_class_name_core(_this: &Self) -> String {
        "SemanticZoom".to_owned()
    }
}

impl IToggleProvider for ProviderAdapter<SemanticZoomAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn toggle_state(&self) -> ToggleState {
        SemanticZoomAutomationPeer::to_state(self.0.owner().is_zoomed_in_view_active())
    }

    /// WinUI's `AutomationSemanticZoomOnToggle`.
    fn toggle(&self) -> Result<(), AutomationError> {
        self.0.ensure_enabled()?;
        self.0.owner().toggle_active_view();
        Ok(())
    }
}

impl SemanticZoomAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &SemanticZoom) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &SemanticZoom) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning semantic zoom.
    pub fn owner(&self) -> Ref<SemanticZoom> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a semantic zoom.")
    }

    /// WinUI's `RaiseToggleStatePropertyChangedEvent`: the zoomed-in view became active when
    /// `is_zoomed_in_view_active`.
    pub fn raise_toggle_state_property_changed_event(&self, is_zoomed_in_view_active: bool) {
        self.raise_property_changed_event(
            TogglePatternIdentifiers::toggle_state_property(),
            Some(Rc::new(Self::to_state(!is_zoomed_in_view_active)) as BoxedValue),
            Some(Rc::new(Self::to_state(is_zoomed_in_view_active)) as BoxedValue),
        );
    }

    fn to_state(is_zoomed_in_view_active: bool) -> ToggleState {
        if is_zoomed_in_view_active {
            ToggleState::On
        } else {
            ToggleState::Off
        }
    }
}
