use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IExpandCollapseProvider, IValueProvider, ProviderAdapter};
use crate::automation::{
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState, ValuePatternIdentifiers,
};
use crate::AutoCompleteBox;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents an [`AutoCompleteBox`].
#[repr(C)]
pub struct AutoCompleteBoxAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(AutoCompleteBoxAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(AutoCompleteBoxAutomationPeer {
    interfaces: [
        Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider,
        Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider
    ]
});

impl FerroObjectImpl for AutoCompleteBoxAutomationPeer {
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

impl ControlAutomationPeerImpl for AutoCompleteBoxAutomationPeer {}

impl AutomationPeerImpl for AutoCompleteBoxAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }

    fn get_class_name_core(_this: &Self) -> String {
        "AutoCompleteBox".to_owned()
    }
}

impl IExpandCollapseProvider for ProviderAdapter<AutoCompleteBoxAutomationPeer> {
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

impl IValueProvider for ProviderAdapter<AutoCompleteBoxAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }

    fn value(&self) -> Option<String> {
        self.0.value()
    }

    /// The member of the contract is implemented apart from the setter of
    /// the `Value` property of the class: it sets the text unconditionally.
    fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        self.0.owner().set_text(value);
        Ok(())
    }
}

impl AutoCompleteBoxAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &AutoCompleteBox) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &AutoCompleteBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning auto-complete box.
    pub fn owner(&self) -> Ref<AutoCompleteBox> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is an auto-complete box.")
    }

    /// The state, expanded or collapsed, of the drop-down (the expand and
    /// collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        Self::to_state(self.owner().is_drop_down_open())
    }

    /// Whether expanding the element shows a menu (the expand and collapse
    /// provider contract).
    pub fn shows_menu(&self) -> bool {
        true
    }

    /// Closes the drop-down (the expand and collapse provider contract).
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_drop_down_open(false);
        Ok(())
    }

    /// Opens the drop-down (the expand and collapse provider contract).
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_drop_down_open(true);
        Ok(())
    }

    /// Whether the value is read-only (the value provider contract).
    pub fn is_read_only(&self) -> bool {
        false
    }

    /// The text of the auto-complete box.
    pub fn value(&self) -> Option<String> {
        self.owner().text()
    }

    /// Sets the text of the auto-complete box unless it is the text
    /// already.
    pub fn set_value(&self, value: Option<&str>) {
        let owner = self.owner();

        if value == owner.text().as_deref() {
            return;
        }

        owner.set_text(value);
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let property = e.property();

        if property == AutoCompleteBox::is_drop_down_open_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            self.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(Self::to_state(old_value)) as BoxedValue),
                Some(Rc::new(Self::to_state(new_value)) as BoxedValue),
            );
        } else if property == AutoCompleteBox::text_property().as_property() {
            self.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                e.get_old_value::<Option<String>>().flatten().map(|value| Rc::new(value) as BoxedValue),
                e.get_new_value::<Option<String>>().map(|value| Rc::new(value) as BoxedValue),
            );
        }
    }

    fn to_state(value: bool) -> ExpandCollapseState {
        if value {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
}
