use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IValueProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::date_time_pickers::TimePicker;
use ferroui_base::animation::TimeSpan;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`TimePicker`].
#[repr(C)]
pub struct TimePickerAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TimePickerAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TimePickerAutomationPeer {
    interfaces: [Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider]
});

impl FerroObjectImpl for TimePickerAutomationPeer {}
impl ControlAutomationPeerImpl for TimePickerAutomationPeer {}

impl AutomationPeerImpl for TimePickerAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Custom
    }
}

impl IValueProvider for ProviderAdapter<TimePickerAutomationPeer> {
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

impl TimePickerAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TimePicker) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TimePicker) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets a value that indicates whether the value is read-only (the
    /// value provider contract).
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Gets the owning time picker.
    pub fn owner(&self) -> Ref<TimePicker> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a time picker.")
    }

    /// Gets the selected time as text (the value provider contract).
    pub fn value(&self) -> Option<String> {
        self.owner().selected_time().map(|selected_time| selected_time.to_string())
    }

    /// Sets the selected time from text, if the text is a time (the value
    /// provider contract).
    pub fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        if let Some(result) = value.and_then(|value| TimeSpan::parse(value).ok()) {
            self.owner().set_selected_time(Some(result));
        }

        Ok(())
    }
}
