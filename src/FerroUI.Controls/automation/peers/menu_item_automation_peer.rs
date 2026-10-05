use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, IToggleProvider, ProviderAdapter, ToggleState,
};
use crate::automation::{
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState, TogglePatternIdentifiers,
};
use crate::i_menu_element::as_menu_element;
use crate::primitives::AccessText;
use crate::{MenuItem, MenuItemToggleType};
use ferroui_base::data::core::ValueType;
use ferroui_base::{
    ferro_class, instantiate, AnyValue, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents a [`MenuItem`].
#[repr(C)]
pub struct MenuItemAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(MenuItemAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(MenuItemAutomationPeer {
    interfaces: [
        Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider,
        Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider,
        Rc<dyn IToggleProvider> => ProviderAdapter::as_toggle_provider
    ]
});

impl FerroObjectImpl for MenuItemAutomationPeer {
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

impl ControlAutomationPeerImpl for MenuItemAutomationPeer {}

impl AutomationPeerImpl for MenuItemAutomationPeer {
    fn get_access_key_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_access_key_core(this);

        if is_null_or_white_space(&result) {
            let access_text = this
                .owner()
                .header_presenter()
                .and_then(|presenter| presenter.child())
                .and_then(|child| child.cast::<AccessText>());
            if let Some(access_text) = access_text {
                result = access_text.access_key();
            }
        }

        result
    }

    fn get_accelerator_key_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_accelerator_key_core(this);

        if is_null_or_white_space(&result) {
            result = this.owner().input_gesture().map(|gesture| gesture.to_string());
        }

        result
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::MenuItem
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_name_core(this);

        if result.is_none() {
            let header = this.owner().header();
            let header = header.as_ref().and_then(|header| {
                let header: &dyn AnyValue = &**header;
                header.downcast_ref::<String>()
            });
            if let Some(header) = header {
                result = AccessText::remove_access_key_marker(Some(header));
            }
        }

        result
    }

    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        let owner = this.owner();

        if provider_type == ValueType::of::<Rc<dyn IExpandCollapseProvider>>() && !owner.has_sub_menu() {
            return None;
        }
        if provider_type == ValueType::of::<Rc<dyn IInvokeProvider>>() && owner.has_sub_menu() {
            return None;
        }
        if provider_type == ValueType::of::<Rc<dyn IToggleProvider>>()
            && owner.toggle_type() == MenuItemToggleType::None
        {
            return None;
        }

        Self::parent_get_provider_core(this, provider_type)
    }
}

fn is_null_or_white_space(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|value| value.chars().all(char::is_whitespace))
}

impl IExpandCollapseProvider for ProviderAdapter<MenuItemAutomationPeer> {
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

impl IInvokeProvider for ProviderAdapter<MenuItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.invoke()
    }
}

impl IToggleProvider for ProviderAdapter<MenuItemAutomationPeer> {
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

impl MenuItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &MenuItem) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &MenuItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning menu item.
    pub fn owner(&self) -> Ref<MenuItem> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a menu item.")
    }

    // The reference implements the toggle provider contract explicitly: its
    // two members are members of the contract only.
    fn toggle_state(&self) -> ToggleState {
        if self.owner().is_checked() {
            ToggleState::On
        } else {
            ToggleState::Off
        }
    }

    /// Gets the state, expanded or collapsed, of the submenu of the item
    /// (the expand/collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        let owner = self.owner();
        if !owner.has_sub_menu() {
            return ExpandCollapseState::LeafNode;
        }
        if owner.is_sub_menu_open() {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }

    /// Gets a value indicating whether expanding the item shows a menu
    /// (the expand/collapse provider contract).
    pub fn shows_menu(&self) -> bool {
        self.owner().has_sub_menu()
    }

    /// Sends a request to activate the menu item (the invoke provider
    /// contract).
    pub fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let owner = self.owner();
        let (command, command_parameter) = (owner.command(), owner.command_parameter());
        if command.is_some_and(|command| !command.can_execute(command_parameter.as_ref())) {
            return Err(ElementNotEnabledException::new());
        }

        // This feels like a bit of a hack: ideally we'd add a new method to the menu item or
        // to the menu interaction handler contract for invoking a menu item, but given that
        // we only need it here and adding a new method would involve an API review, let's
        // KISS for now.
        let handler = owner.menu_interaction_handler();
        match handler.as_ref().and_then(|handler| handler.as_default_menu_interaction_handler()) {
            Some(handler) => handler.click(&owner.to_menu_item()),
            None => owner.to_menu_item().raise_click(),
        }

        Ok(())
    }

    /// Opens the submenu of the item (the expand/collapse provider
    /// contract).
    ///
    /// # Panics
    ///
    /// Panics if the item has no submenu.
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;
        let owner = self.owner();
        if !owner.has_sub_menu() {
            panic!("Operation is not valid due to the current state of the object.");
        }
        owner.open();
        Ok(())
    }

    /// Closes the submenu of the item (the expand/collapse provider
    /// contract).
    ///
    /// # Panics
    ///
    /// Panics if the item has no submenu.
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;
        let owner = self.owner();
        if !owner.has_sub_menu() {
            panic!("Operation is not valid due to the current state of the object.");
        }
        if !owner.is_sub_menu_open() {
            return Ok(());
        }

        // Match clicking an open top-level header, which closes the whole menu rather than
        // leaving the menu bar active with no submenu open.
        let main_menu = if owner.is_top_level() {
            owner.parent().and_then(|parent| as_menu_element(&parent)).and_then(|parent| parent.as_main_menu())
        } else {
            None
        };
        match main_menu {
            Some(main_menu) => main_menu.close(),
            None => owner.close(),
        }

        Ok(())
    }

    fn toggle(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        let owner = self.owner();
        if owner.has_sub_menu() {
            return Ok(());
        }

        match owner.toggle_type() {
            MenuItemToggleType::CheckBox => {
                owner.set_current_value(MenuItem::is_checked_property(), !owner.is_checked());
            }
            MenuItemToggleType::Radio => {
                if !owner.is_checked() {
                    owner.set_current_value(MenuItem::is_checked_property(), true);
                }
            }
            MenuItemToggleType::None => {}
        }

        Ok(())
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let owner = self.owner();

        if e.property() == MenuItem::is_checked_property().as_property()
            && owner.toggle_type() != MenuItemToggleType::None
        {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            self.raise_property_changed_event(
                TogglePatternIdentifiers::toggle_state_property(),
                Some(Rc::new(Self::to_state(old_value)) as BoxedValue),
                Some(Rc::new(Self::to_state(new_value)) as BoxedValue),
            );
        } else if e.property() == MenuItem::is_sub_menu_open_property().as_property() && owner.has_sub_menu() {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            let state = |value: bool| {
                if value {
                    ExpandCollapseState::Expanded
                } else {
                    ExpandCollapseState::Collapsed
                }
            };
            self.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(state(old_value)) as BoxedValue),
                Some(Rc::new(state(new_value)) as BoxedValue),
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
