use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{IExpandCollapseProvider, ProviderAdapter};
use crate::automation::{ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState};
use crate::DrawerPage;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

/// An automation peer which represents a [`DrawerPage`].
#[repr(C)]
pub struct DrawerPageAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(DrawerPageAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(DrawerPageAutomationPeer {
    interfaces: [Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider]
});

impl FerroObjectImpl for DrawerPageAutomationPeer {
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

impl ControlAutomationPeerImpl for DrawerPageAutomationPeer {}

impl AutomationPeerImpl for DrawerPageAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_name_core(this);

        if result.as_deref().is_none_or(str::is_empty) {
            result = this.owner().header().map(|header| ValueTypes::to_display_string(Some(&header)));
        }

        result
    }
}

impl IExpandCollapseProvider for ProviderAdapter<DrawerPageAutomationPeer> {
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

impl DrawerPageAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &DrawerPage) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &DrawerPage) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning drawer page.
    pub fn owner(&self) -> Ref<DrawerPage> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a drawer page.")
    }

    /// The state, expanded or collapsed, of the drawer (the expand and
    /// collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        Self::to_state(self.owner().is_open())
    }

    /// Whether expanding the element shows a menu (the expand and collapse
    /// provider contract).
    pub fn shows_menu(&self) -> bool {
        false
    }

    /// Closes the drawer (the expand and collapse provider contract).
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_open(false);
        Ok(())
    }

    /// Opens the drawer (the expand and collapse provider contract).
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.owner().set_is_open(true);
        Ok(())
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == DrawerPage::is_open_property().as_property() {
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
