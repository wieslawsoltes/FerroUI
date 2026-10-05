use super::{AutomationPeer, AutomationPeerImpl, ButtonAutomationPeer, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::automation::provider::{ISelectionItemProvider, ISelectionProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::calendar::{date_of, Calendar, CalendarDayButton, CalendarSelectionMode};
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`CalendarDayButton`].
#[repr(C)]
pub struct CalendarDayButtonAutomationPeer {
    base: ButtonAutomationPeer,
}

ferro_class!(CalendarDayButtonAutomationPeer: ButtonAutomationPeer);
ferroui_base::ferro_class_info!(CalendarDayButtonAutomationPeer {
    interfaces: [Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider]
});

impl FerroObjectImpl for CalendarDayButtonAutomationPeer {}
impl ControlAutomationPeerImpl for CalendarDayButtonAutomationPeer {}

impl AutomationPeerImpl for CalendarDayButtonAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "CalendarDayButton".to_owned()
    }
}

impl ISelectionItemProvider for ProviderAdapter<CalendarDayButtonAutomationPeer> {
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

impl CalendarDayButtonAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &CalendarDayButton) -> Self {
        Self { base: ButtonAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &CalendarDayButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning day button.
    pub fn owner(&self) -> Ref<CalendarDayButton> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a calendar day button.")
    }

    /// Gets a value that indicates whether the day is selected (the
    /// selection item provider contract).
    pub fn is_selected(&self) -> bool {
        self.owner().is_selected()
    }

    /// Gets the selection provider of the calendar of the day button (the
    /// selection item provider contract).
    pub fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        match self.owner().owner() {
            Some(calendar) => self.get_or_create(&calendar).get_provider::<dyn ISelectionProvider>(),
            None => None,
        }
    }

    /// Clears any existing selection and then selects the day (the
    /// selection item provider contract).
    pub fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let Some((calendar, date)) = self.try_get_selectable_date() {
            calendar.set_selected_date(Some(date));
        }

        Ok(())
    }

    // The reference implements the two members below explicitly: they are
    // members of the selection item provider contract only.
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let Some((calendar, date)) = self.try_get_selectable_date() else {
            return Ok(());
        };
        if calendar.selected_dates().contains(date) {
            return Ok(());
        }

        if calendar.selection_mode() == CalendarSelectionMode::SingleDate {
            calendar.set_selected_date(Some(date));
        } else {
            calendar.selected_dates().add(date);
        }

        Ok(())
    }

    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let owner = self.owner();
        if let Some(calendar) = owner.owner().filter(|calendar| calendar.selection_mode() != CalendarSelectionMode::None)
        {
            if let Some(date) = date_of(&owner) {
                calendar.selected_dates().remove(date);
            }
        }

        Ok(())
    }

    fn try_get_selectable_date(&self) -> Option<(Ref<Calendar>, DateTime)> {
        let owner = self.owner();
        if let Some(calendar) = owner.owner().filter(|calendar| calendar.selection_mode() != CalendarSelectionMode::None)
        {
            if let Some(value) = date_of(&owner) {
                if !owner.is_blackout() {
                    return Some((calendar, value));
                }
            }
        }

        None
    }
}
