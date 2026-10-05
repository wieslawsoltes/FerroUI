use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ContentControlAutomationPeer, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{IExpandCollapseProvider, IInvokeProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState};
use crate::SplitButton;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::cell::Cell;
use std::rc::Rc;

/// An automation peer which represents a [`SplitButton`].
#[repr(C)]
pub struct SplitButtonAutomationPeer {
    base: ContentControlAutomationPeer,
    expand_collapse_state: Cell<ExpandCollapseState>,
}

ferro_class!(SplitButtonAutomationPeer: ContentControlAutomationPeer);
ferroui_base::ferro_class_info!(SplitButtonAutomationPeer {
    interfaces: [
        Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider,
        Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider
    ]
});

impl FerroObjectImpl for SplitButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.owner().flyout_state_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.owner_flyout_state_changed();
            }
        });
    }
}

impl ControlAutomationPeerImpl for SplitButtonAutomationPeer {}

impl AutomationPeerImpl for SplitButtonAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::SplitButton
    }

    fn get_class_name_core(_this: &Self) -> String {
        "SplitButton".to_owned()
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl IInvokeProvider for ProviderAdapter<SplitButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        self.0.owner().invoke_primary();
        Ok(())
    }
}

impl IExpandCollapseProvider for ProviderAdapter<SplitButtonAutomationPeer> {
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

impl SplitButtonAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &SplitButton) -> Self {
        Self {
            base: ContentControlAutomationPeer::construct(owner),
            expand_collapse_state: Cell::new(Self::to_state(owner.is_flyout_open())),
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &SplitButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning split button.
    pub fn owner(&self) -> Ref<SplitButton> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a split button.")
    }

    /// The state, expanded or collapsed, of the flyout (the expand and
    /// collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        Self::to_state(self.owner().is_flyout_open())
    }

    /// Whether expanding the element shows a menu (the expand and collapse
    /// provider contract).
    pub fn shows_menu(&self) -> bool {
        true
    }

    /// Closes the flyout (the expand and collapse provider contract).
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().close_flyout_for_automation();
        Ok(())
    }

    /// Opens the flyout (the expand and collapse provider contract).
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().open_flyout_for_automation();
        Ok(())
    }

    fn owner_flyout_state_changed(&self) {
        let old_state = self.expand_collapse_state.get();
        let new_state = Self::to_state(self.owner().is_flyout_open());

        if old_state != new_state {
            self.expand_collapse_state.set(new_state);
            self.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(old_state) as BoxedValue),
                Some(Rc::new(new_state) as BoxedValue),
            );
        }
    }

    fn to_state(is_open: bool) -> ExpandCollapseState {
        if is_open {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
}
