use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
    ToggleButtonAutomationPeer,
};
use crate::automation::provider::{ISelectionItemProvider, ISelectionProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, SelectionItemPatternIdentifiers};
use crate::primitives::ToggleButton;
use crate::RadioButton;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`RadioButton`].
#[repr(C)]
pub struct RadioButtonAutomationPeer {
    base: ToggleButtonAutomationPeer,
}

ferro_class! {
    RadioButtonAutomationPeer: ToggleButtonAutomationPeer,
    virtuals RadioButtonAutomationPeerImpl: ControlAutomationPeerImpl {
        /// Raises the property changed event of the selection state for a
        /// change of the checked state of the radio button (not for use
        /// outside the framework).
        fn raise_toggle_state_property_changed_event(this, old_value: Option<bool>, new_value: Option<bool>);
    }
}
ferroui_base::ferro_class_info!(RadioButtonAutomationPeer {
    interfaces: [Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider]
});

impl FerroObjectImpl for RadioButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        ControlAutomationPeer::owner(this).property_changed(move |e| {
            if e.property() == ToggleButton::is_checked_property().as_property() {
                let Some(this) = weak.upgrade() else { return };
                let (old_value, new_value) = e.get_old_and_new_value::<Option<bool>>();
                this.raise_toggle_state_property_changed_event(old_value, new_value);
            }
        });
    }
}

impl ControlAutomationPeerImpl for RadioButtonAutomationPeer {}

impl AutomationPeerImpl for RadioButtonAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "RadioButton".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::RadioButton
    }
}

impl RadioButtonAutomationPeerImpl for RadioButtonAutomationPeer {
    fn raise_toggle_state_property_changed_event(this: &Self, old_value: Option<bool>, new_value: Option<bool>) {
        this.raise_property_changed_event(
            SelectionItemPatternIdentifiers::is_selected_property(),
            Some(Rc::new(old_value == Some(true)) as BoxedValue),
            Some(Rc::new(new_value == Some(true)) as BoxedValue),
        );
    }
}

impl ISelectionItemProvider for ProviderAdapter<RadioButtonAutomationPeer> {
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
        self.0.add_to_selection()
    }

    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.remove_from_selection()
    }

    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.select()
    }
}

impl RadioButtonAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &RadioButton) -> Self {
        Self { base: ToggleButtonAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &RadioButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn radio_button(&self) -> Ref<RadioButton> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a radio button.")
    }

    /// Whether the radio button is checked (the selection item provider
    /// contract).
    pub fn is_selected(&self) -> bool {
        self.radio_button().is_checked() == Some(true)
    }

    /// The container of the selection: none (the selection item provider
    /// contract).
    pub fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        None
    }

    /// Adds the radio button to the selection (the selection item provider
    /// contract).
    ///
    /// # Panics
    ///
    /// Panics if the radio button is not checked.
    pub fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        if self.radio_button().is_checked() != Some(true) {
            panic!("Operation cannot be performed");
        }
        Ok(())
    }

    /// Removes the radio button from the selection (the selection item
    /// provider contract).
    ///
    /// # Panics
    ///
    /// Panics if the radio button is checked.
    pub fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        if self.radio_button().is_checked() == Some(true) {
            panic!("Operation cannot be performed");
        }
        Ok(())
    }

    /// Checks the radio button (the selection item provider contract).
    ///
    /// # Panics
    ///
    /// Panics if the radio button is disabled.
    pub fn select(&self) -> Result<(), ElementNotEnabledException> {
        if !self.is_enabled() {
            panic!("Element is disabled thus it cannot be selected");
        }

        self.radio_button().set_is_checked(Some(true));
        Ok(())
    }
}
