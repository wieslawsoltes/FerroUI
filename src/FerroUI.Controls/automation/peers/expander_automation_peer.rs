use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IExpandCollapseProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState};
use crate::{Control, Expander};
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents an [`Expander`].
#[repr(C)]
pub struct ExpanderAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ExpanderAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ExpanderAutomationPeer {
    interfaces: [Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider]
});

impl FerroObjectImpl for ExpanderAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        ControlAutomationPeer::owner(this).property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.owner_property_changed(e);
            }
        });
    }
}

impl ControlAutomationPeerImpl for ExpanderAutomationPeer {}

impl AutomationPeerImpl for ExpanderAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Expander
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl IExpandCollapseProvider for ProviderAdapter<ExpanderAutomationPeer> {
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

impl ExpanderAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Control) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Control) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning expander.
    pub fn owner(&self) -> Ref<Expander> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is an expander.")
    }

    /// The state, expanded or collapsed, of the expander (the expand and
    /// collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        Self::to_state(self.owner().is_expanded())
    }

    /// Whether expanding the element shows a menu (the expand and collapse
    /// provider contract).
    pub fn shows_menu(&self) -> bool {
        false
    }

    /// Collapses the expander (the expand and collapse provider contract).
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_expanded(false);
        Ok(())
    }

    /// Expands the expander (the expand and collapse provider contract).
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_expanded(true);
        Ok(())
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == Expander::is_expanded_property().as_property() {
            let (old_value, new_value) = e.get_old_and_new_value::<bool>();
            self.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(Self::to_state(old_value)) as BoxedValue),
                Some(Rc::new(Self::to_state(new_value)) as BoxedValue),
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
