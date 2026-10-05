use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ContentControlAutomationPeer, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{IToggleProvider, ProviderAdapter, ToggleState};
use crate::automation::{ElementNotEnabledException, TogglePatternIdentifiers};
use crate::primitives::ToggleButton;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`ToggleButton`].
#[repr(C)]
pub struct ToggleButtonAutomationPeer {
    base: ContentControlAutomationPeer,
}

ferro_class!(ToggleButtonAutomationPeer: ContentControlAutomationPeer);
ferroui_base::ferro_class_info!(ToggleButtonAutomationPeer { interfaces: [Rc<dyn IToggleProvider> => ProviderAdapter::as_toggle_provider] });

impl FerroObjectImpl for ToggleButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |e| {
            if e.property() == ToggleButton::is_checked_property().as_property() {
                let Some(this) = weak.upgrade() else { return };
                let (old_value, new_value) = e.get_old_and_new_value::<Option<bool>>();
                this.raise_property_changed_event(
                    TogglePatternIdentifiers::toggle_state_property(),
                    Some(Rc::new(Self::to_state(old_value)) as BoxedValue),
                    Some(Rc::new(Self::to_state(new_value)) as BoxedValue),
                );
            }
        });
    }
}

impl ControlAutomationPeerImpl for ToggleButtonAutomationPeer {}

impl AutomationPeerImpl for ToggleButtonAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl IToggleProvider for ProviderAdapter<ToggleButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn toggle_state(&self) -> ToggleState {
        self.0.toggle_state()
    }

    fn toggle(&self) -> Result<(), ElementNotEnabledException> {
        self.0.toggle()
    }
}

impl ToggleButtonAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ToggleButton) -> Self {
        Self { base: ContentControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ToggleButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning toggle button.
    pub fn owner(&self) -> Ref<ToggleButton> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a toggle button.")
    }

    fn to_state(value: Option<bool>) -> ToggleState {
        match value {
            Some(true) => ToggleState::On,
            Some(false) => ToggleState::Off,
            None => ToggleState::Indeterminate,
        }
    }

    /// The toggle state of the button (the toggle provider contract).
    pub fn toggle_state(&self) -> ToggleState {
        Self::to_state(self.owner().is_checked())
    }

    /// Cycles through the toggle states of the button (the toggle provider
    /// contract).
    pub fn toggle(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;
        self.owner().perform_click();
        Ok(())
    }
}
