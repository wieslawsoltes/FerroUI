use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IExpandCollapseProvider, IInvokeProvider, IValueProvider, ProviderError, ProviderAdapter};
use crate::automation::{
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState, ValuePatternIdentifiers,
};
use crate::calendar_date_picker::CalendarDatePicker;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents a [`CalendarDatePicker`].
#[repr(C)]
pub struct CalendarDatePickerAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(CalendarDatePickerAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(CalendarDatePickerAutomationPeer {
    interfaces: [
        Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider,
        Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider,
        Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider
    ]
});

impl FerroObjectImpl for CalendarDatePickerAutomationPeer {
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

impl ControlAutomationPeerImpl for CalendarDatePickerAutomationPeer {}

impl AutomationPeerImpl for CalendarDatePickerAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }

    fn get_class_name_core(_this: &Self) -> String {
        "CalendarDatePicker".to_owned()
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl IInvokeProvider for ProviderAdapter<CalendarDatePickerAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.invoke()
    }
}

impl IExpandCollapseProvider for ProviderAdapter<CalendarDatePickerAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn expand_collapse_state(&self) -> ExpandCollapseState {
        self.0.expand_collapse_state()
    }

    fn shows_menu(&self) -> bool {
        self.0.shows_menu()
    }

    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.0.expand()
    }

    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.0.collapse()
    }
}

impl IValueProvider for ProviderAdapter<CalendarDatePickerAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }

    fn value(&self) -> Option<String> {
        self.0.value()
    }

    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        self.0.set_value(value).map_err(Into::into)
    }
}

impl CalendarDatePickerAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &CalendarDatePicker) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &CalendarDatePicker) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning calendar date picker.
    pub fn owner(&self) -> Ref<CalendarDatePicker> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a calendar date picker.")
    }

    /// Gets the state, expanded or collapsed, of the drop-down of the date
    /// picker (the expand/collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        Self::to_state(self.owner().is_drop_down_open())
    }

    /// Gets a value indicating whether expanding the date picker shows a
    /// menu (the expand/collapse provider contract).
    pub fn shows_menu(&self) -> bool {
        true
    }

    /// Gets a value that indicates whether the value is read-only (the
    /// value provider contract).
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// Gets the text of the date picker (the value provider contract).
    pub fn value(&self) -> Option<String> {
        self.owner().text()
    }

    /// Opens the drop-down (the invoke provider contract).
    pub fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;
        self.owner().set_is_drop_down_open(true);
        Ok(())
    }

    /// Opens the drop-down (the expand/collapse provider contract).
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_drop_down_open(true);
        Ok(())
    }

    /// Closes the drop-down (the expand/collapse provider contract).
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_drop_down_open(false);
        Ok(())
    }

    /// Sets the text of the date picker (the value provider contract).
    pub fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        self.owner().set_text(value);
        Ok(())
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == CalendarDatePicker::text_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<String>>();
            self.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                old_value.map(|value| Rc::new(value) as BoxedValue),
                new_value.map(|value| Rc::new(value) as BoxedValue),
            );
        } else if e.property() == CalendarDatePicker::is_drop_down_open_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            self.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(Self::to_state(old_value)) as BoxedValue),
                Some(Rc::new(Self::to_state(new_value)) as BoxedValue),
            );
        }
    }

    fn to_state(is_expanded: bool) -> ExpandCollapseState {
        if is_expanded {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
}
