use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
    SplitButtonAutomationPeer,
};
use crate::automation::provider::{IToggleProvider, ProviderAdapter, ToggleState};
use crate::automation::{ElementNotEnabledException, TogglePatternIdentifiers};
use crate::ToggleSplitButton;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents a [`ToggleSplitButton`].
#[repr(C)]
pub struct ToggleSplitButtonAutomationPeer {
    base: SplitButtonAutomationPeer,
}

ferro_class!(ToggleSplitButtonAutomationPeer: SplitButtonAutomationPeer);
ferroui_base::ferro_class_info!(ToggleSplitButtonAutomationPeer {
    interfaces: [Rc<dyn IToggleProvider> => ProviderAdapter::as_toggle_provider]
});

impl FerroObjectImpl for ToggleSplitButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.owner_property_changed(e);
            }
        });
    }
}

impl ControlAutomationPeerImpl for ToggleSplitButtonAutomationPeer {}

impl AutomationPeerImpl for ToggleSplitButtonAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::SplitButton
    }

    fn get_class_name_core(_this: &Self) -> String {
        "ToggleSplitButton".to_owned()
    }
}

impl IToggleProvider for ProviderAdapter<ToggleSplitButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn toggle_state(&self) -> ToggleState {
        ToggleSplitButtonAutomationPeer::to_state(self.0.owner().is_checked())
    }

    fn toggle(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        self.0.owner().toggle_for_automation();
        Ok(())
    }
}

impl ToggleSplitButtonAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ToggleSplitButton) -> Self {
        Self { base: SplitButtonAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ToggleSplitButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning toggle split button.
    pub fn owner(&self) -> Ref<ToggleSplitButton> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a toggle split button.")
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == ToggleSplitButton::is_checked_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            self.raise_property_changed_event(
                TogglePatternIdentifiers::toggle_state_property(),
                Some(Rc::new(Self::to_state(old_value)) as BoxedValue),
                Some(Rc::new(Self::to_state(new_value)) as BoxedValue),
            );
        }
    }

    fn to_state(value: bool) -> ToggleState {
        if value {
            ToggleState::On
        } else {
            ToggleState::Off
        }
    }
}
