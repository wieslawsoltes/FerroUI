use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
    ItemsControlAutomationPeerImpl, SelectingItemsControlAutomationPeer, SelectingItemsControlAutomationPeerImpl,
    SelectingItemsControlAutomationPeerImplExt, UnrealizedElementAutomationPeer,
};
use crate::automation::provider::{IExpandCollapseProvider, IValueProvider, ProviderAdapter};
use crate::automation::{
    AutomationElementIdentifiers, AutomationProperties, ElementNotEnabledException, ExpandCollapsePatternIdentifiers,
    ExpandCollapseState, ValuePatternIdentifiers,
};
use crate::{ComboBox, ComboBoxItem, ContentControl, Control, TextBlock};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroPropertyChangedEventArgs, Ref, StaticType, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// An automation peer which represents a [`ComboBox`].
#[repr(C)]
pub struct ComboBoxAutomationPeer {
    base: SelectingItemsControlAutomationPeer,
    selection: RefCell<Option<Ref<UnrealizedSelectionPeer>>>,
}

ferro_class!(ComboBoxAutomationPeer: SelectingItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(ComboBoxAutomationPeer {
    interfaces: [
        Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider,
        Rc<dyn IValueProvider> => ProviderAdapter::as_value_provider
    ]
});

impl FerroObjectImpl for ComboBoxAutomationPeer {}
impl ControlAutomationPeerImpl for ComboBoxAutomationPeer {}
impl ItemsControlAutomationPeerImpl for ComboBoxAutomationPeer {}

impl AutomationPeerImpl for ComboBoxAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ComboBox
    }
}

impl SelectingItemsControlAutomationPeerImpl for ComboBoxAutomationPeer {
    fn get_selection_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        if this.expand_collapse_state() == ExpandCollapseState::Expanded {
            return Self::parent_get_selection_core(this);
        }

        // If the combo box is not open then we won't have an ItemsPresenter so the default
        // GetSelectionCore implementation won't work. For this case we create a separate
        // peer to represent the unrealized item.
        if let Some(selection) = this.owner().selected_item() {
            let existing = this.selection.borrow().clone();
            let peer = match existing {
                Some(peer) => peer,
                None => {
                    let peer = UnrealizedSelectionPeer::new(this);
                    *this.selection.borrow_mut() = Some(peer.clone());
                    peer
                }
            };
            peer.set_item(Some(selection));
            return Some(vec![peer.upcast()]);
        }

        None
    }

    fn owner_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_owner_property_changed(this, e);

        if e.property() == ComboBox::is_drop_down_open_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            this.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(Self::to_state(old_value)) as BoxedValue),
                Some(Rc::new(Self::to_state(new_value)) as BoxedValue),
            );
        } else if e.property() == ComboBox::text_property().as_property() && this.owner().is_editable() {
            let (old_value, new_value) = e.get_old_and_new_value::<Option<String>>();
            this.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                old_value.map(|value| Rc::new(value) as BoxedValue),
                new_value.map(|value| Rc::new(value) as BoxedValue),
            );
        } else if e.property() == ComboBox::is_editable_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            this.raise_property_changed_event(
                ValuePatternIdentifiers::is_read_only_property(),
                Some(Rc::new(!old_value) as BoxedValue),
                Some(Rc::new(!new_value) as BoxedValue),
            );

            // The Value pattern switches between SelectedItem name and Text when IsEditable changes.
            this.raise_property_changed_event(ValuePatternIdentifiers::value_property(), None, None);
        }
    }
}

impl IExpandCollapseProvider for ProviderAdapter<ComboBoxAutomationPeer> {
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

impl IValueProvider for ProviderAdapter<ComboBoxAutomationPeer> {
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

impl ComboBoxAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ComboBox) -> Self {
        Self { base: SelectingItemsControlAutomationPeer::construct(owner), selection: RefCell::new(None) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ComboBox) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning combo box.
    pub fn owner(&self) -> Ref<ComboBox> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a combo box.")
    }

    /// Gets the state, expanded or collapsed, of the drop-down of the combo
    /// box (the expand/collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        Self::to_state(self.owner().is_drop_down_open())
    }

    /// Gets a value indicating whether expanding the combo box shows a
    /// menu of items (the expand/collapse provider contract).
    pub fn shows_menu(&self) -> bool {
        true
    }

    /// Closes the drop-down (the expand/collapse provider contract).
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_drop_down_open(false);
        Ok(())
    }

    /// Opens the drop-down (the expand/collapse provider contract).
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_drop_down_open(true);
        Ok(())
    }

    // The reference implements the value provider contract explicitly: its
    // three members are members of the contract only.
    fn is_read_only(&self) -> bool {
        !self.owner().is_editable()
    }

    fn value(&self) -> Option<String> {
        let owner = self.owner();
        if owner.is_editable() {
            return owner.text();
        }

        let selection = self.get_selection();
        if selection.len() == 1 {
            Some(selection[0].get_name())
        } else {
            None
        }
    }

    fn set_value(&self, value: Option<&str>) -> Result<(), ElementNotEnabledException> {
        let owner = self.owner();
        if !owner.is_editable() {
            panic!("Cannot set the value of a non-editable ComboBox.");
        }

        owner.set_current_value(ComboBox::text_property(), value.map(str::to_owned));
        Ok(())
    }

    fn to_state(value: bool) -> ExpandCollapseState {
        if value {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
}

/// The peer of the selected item of a combo box the drop-down of which is
/// closed: the item has no container then.
#[repr(C)]
struct UnrealizedSelectionPeer {
    base: UnrealizedElementAutomationPeer,
    // The combo box peer owns this peer: the reference back is weak.
    owner: WeakRef<ComboBoxAutomationPeer>,
    item: RefCell<Option<BoxedValue>>,
}

ferro_class!(UnrealizedSelectionPeer: UnrealizedElementAutomationPeer);
ferroui_base::ferro_class_info!(UnrealizedSelectionPeer {});

impl FerroObjectImpl for UnrealizedSelectionPeer {}

impl AutomationPeerImpl for UnrealizedSelectionPeer {
    fn get_accelerator_key_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_access_key_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_automation_id_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_class_name_core(_this: &Self) -> String {
        <ComboBoxItem as StaticType>::TYPE.name().to_owned()
    }

    fn get_labeled_by_core(_this: &Self) -> Option<Ref<AutomationPeer>> {
        None
    }

    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.owner.upgrade().map(|owner| owner.upcast())
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ListItem
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let item = this.item.borrow().clone();

        if let Some(c) = item.as_ref().and_then(Control::from_boxed) {
            let mut result = AutomationProperties::get_name(&c);

            if result.is_none() {
                let text = c
                    .cast::<ContentControl>()
                    .and_then(|cc| cc.presenter())
                    .and_then(|presenter| presenter.child())
                    .and_then(|child| child.cast::<TextBlock>());
                if let Some(text) = text {
                    result = text.text();
                }
            }

            if result.is_none() {
                result = c
                    .get_value(ContentControl::content_property())
                    .map(|content| ValueTypes::to_display_string(Some(&content)));
            }

            return result;
        }

        item.map(|item| ValueTypes::to_display_string(Some(&item)))
    }
}

impl UnrealizedSelectionPeer {
    fn new(owner: &ComboBoxAutomationPeer) -> Ref<Self> {
        instantiate(Self {
            base: UnrealizedElementAutomationPeer::construct(),
            owner: owner.to_ref().downgrade(),
            item: RefCell::new(None),
        })
    }

    fn set_item(&self, value: Option<BoxedValue>) {
        let unchanged = reference_equals(&self.item.borrow(), &value);
        if !unchanged {
            let old_value = self.get_name_core();
            *self.item.borrow_mut() = value;
            self.raise_property_changed_event(
                AutomationElementIdentifiers::name_property(),
                old_value.map(|value| Rc::new(value) as BoxedValue),
                self.get_name_core().map(|value| Rc::new(value) as BoxedValue),
            );
        }
    }
}

/// Whether two untyped values are the same object: the same boxed value, or
/// the same control.
fn reference_equals(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            Rc::ptr_eq(a, b)
                || matches!((Control::from_boxed(a), Control::from_boxed(b)), (Some(a), Some(b)) if a == b)
        }
        _ => false,
    }
}
