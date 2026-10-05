use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IValueProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::date_time_pickers::DatePicker;
use ferroui_base::utilities::{CultureInfo, DateTimeOffset};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`DatePicker`].
#[repr(C)]
pub struct DatePickerAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(DatePickerAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(DatePickerAutomationPeer {
    interfaces: [Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider]
});

impl FerroObjectImpl for DatePickerAutomationPeer {}
impl ControlAutomationPeerImpl for DatePickerAutomationPeer {}

impl AutomationPeerImpl for DatePickerAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
}

impl IValueProvider for ProviderAdapter<DatePickerAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }

    fn value(&self) -> Option<String> {
        self.0.value()
    }

    fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        self.0.set_value(value)
    }
}

impl DatePickerAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &DatePicker) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &DatePicker) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets a value that indicates whether the value is read-only (the
    /// value provider contract).
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Gets the owning date picker.
    pub fn owner(&self) -> Ref<DatePicker> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a date picker.")
    }

    /// Gets the selected date as text (the value provider contract).
    pub fn value(&self) -> Option<String> {
        self.owner().selected_date().map(|selected_date| selected_date.to_string())
    }

    /// Sets the selected date from text, if the text is a date (the value
    /// provider contract).
    pub fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        if let Some(result) =
            value.and_then(|value| DateTimeOffset::try_parse(value, &CultureInfo::current_culture()))
        {
            self.owner().set_selected_date(Some(result));
        }

        Ok(())
    }
}
