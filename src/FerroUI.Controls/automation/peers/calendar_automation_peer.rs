use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionProvider, IValueProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, SelectionPatternIdentifiers, ValuePatternIdentifiers};
use crate::calendar::{Calendar, CalendarSelectionMode};
use crate::SelectionChangedEventArgs;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`Calendar`].
#[repr(C)]
pub struct CalendarAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(CalendarAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(CalendarAutomationPeer {
    interfaces: [
        Rc<dyn ISelectionProvider> => ProviderAdapter::as_selection_provider,
        Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider
    ]
});

impl FerroObjectImpl for CalendarAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.owner().selected_dates_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.owner_selected_dates_changed(e);
            }
        });
    }
}

impl ControlAutomationPeerImpl for CalendarAutomationPeer {}

impl AutomationPeerImpl for CalendarAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Calendar
    }

    fn get_class_name_core(_this: &Self) -> String {
        "Calendar".to_owned()
    }
}

impl ISelectionProvider for ProviderAdapter<CalendarAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn can_select_multiple(&self) -> bool {
        self.0.can_select_multiple()
    }

    fn is_selection_required(&self) -> bool {
        self.0.is_selection_required()
    }

    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        self.0.get_selection()
    }
}

impl IValueProvider for ProviderAdapter<CalendarAutomationPeer> {
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

impl CalendarAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Calendar) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Calendar) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning calendar.
    pub fn owner(&self) -> Ref<Calendar> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a calendar.")
    }

    /// Gets a value that indicates whether more than one day can be selected
    /// at a time (the selection provider contract).
    pub fn can_select_multiple(&self) -> bool {
        let selection_mode = self.owner().selection_mode();
        selection_mode == CalendarSelectionMode::SingleRange || selection_mode == CalendarSelectionMode::MultipleRange
    }

    /// Gets a value that indicates whether a day has to be selected (the
    /// selection provider contract).
    pub fn is_selection_required(&self) -> bool {
        false
    }

    /// Gets the peers of the buttons of the selected days that are
    /// displayed (the selection provider contract).
    pub fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        let owner = self.owner();
        owner
            .selected_dates()
            .to_vec()
            .into_iter()
            .filter_map(|date| owner.find_day_button_from_day(date))
            .map(|day_button| self.get_or_create(&day_button))
            .collect()
    }

    /// Gets a value that indicates whether the value is read-only (the
    /// value provider contract).
    pub fn is_read_only(&self) -> bool {
        true
    }

    /// Gets the selected dates, formatted with the current culture and
    /// joined with its list separator (the value provider contract).
    pub fn value(&self) -> Option<String> {
        let culture = CultureInfo::current_culture();
        let dates: Vec<String> =
            self.owner().selected_dates().to_vec().into_iter().map(|x| x.to_string_provider(&culture)).collect();
        Some(dates.join(culture.text_info().list_separator()))
    }

    /// Not supported: the value of a calendar is read-only (the value
    /// provider contract).
    pub fn set_value(&self, _value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        panic!("Specified method is not supported.");
    }

    fn owner_selected_dates_changed(&self, _e: &SelectionChangedEventArgs) {
        self.raise_property_changed_event(SelectionPatternIdentifiers::selection_property(), None, None);
        self.raise_property_changed_event(ValuePatternIdentifiers::value_property(), None, None);
    }
}
