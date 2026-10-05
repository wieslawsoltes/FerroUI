use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionProvider, ProviderAdapter};
use crate::automation::SelectionPatternIdentifiers;
use crate::{ListBox, PipsPager, PipsPagerSelectedIndexChangedEventArgs};
use ferroui_base::controls::NameScopeExtensions;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// An automation peer for [`PipsPager`].
#[repr(C)]
pub struct PipsPagerAutomationPeer {
    base: ControlAutomationPeer,
    pips_list: RefCell<Option<Ref<ListBox>>>,
}

ferro_class!(PipsPagerAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(PipsPagerAutomationPeer {
    interfaces: [Rc<dyn ISelectionProvider> => ProviderAdapter::as_selection_provider]
});

impl FerroObjectImpl for PipsPagerAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.owner().selected_index_changed(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(e);
            }
        });
    }
}

impl ControlAutomationPeerImpl for PipsPagerAutomationPeer {}

impl AutomationPeerImpl for PipsPagerAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::List
    }

    fn get_class_name_core(_this: &Self) -> String {
        "PipsPager".to_owned()
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let name = Self::parent_get_name_core(this);
        match name {
            Some(name) if !name.trim().is_empty() => Some(name),
            _ => Some("Pips Pager".to_owned()),
        }
    }
}

impl ISelectionProvider for ProviderAdapter<PipsPagerAutomationPeer> {
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

impl PipsPagerAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &PipsPager) -> Self {
        Self { base: ControlAutomationPeer::construct(owner), pips_list: RefCell::new(None) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &PipsPager) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owner as a [`PipsPager`].
    fn owner(&self) -> Ref<PipsPager> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a pips pager.")
    }

    /// Gets a value that indicates whether more than one pip can be selected
    /// at a time (the selection provider contract).
    pub fn can_select_multiple(&self) -> bool {
        false
    }

    /// Gets a value that indicates whether a pip has to be selected (the
    /// selection provider contract).
    pub fn is_selection_required(&self) -> bool {
        true
    }

    /// Gets the peer of the container of the selected pip (the selection
    /// provider contract).
    pub fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        let mut result = Vec::new();
        let owner = self.owner();

        if owner.selected_page_index() >= 0 && owner.selected_page_index() < owner.number_of_pages() {
            if self.pips_list.borrow().is_none() {
                let pips_list = NameScopeExtensions::find_name_scope(&owner)
                    .and_then(|scope| scope.find_as::<ListBox>("PART_PipsPagerList"));
                *self.pips_list.borrow_mut() = pips_list;
            }

            let pips_list = self.pips_list.borrow().clone();
            if let Some(pips_list) = pips_list {
                if let Some(container) = pips_list.container_from_index(owner.selected_page_index()) {
                    let peer = self.get_or_create(&container);
                    result.push(peer);
                }
            }
        }

        result
    }

    fn on_selection_changed(&self, e: &PipsPagerSelectedIndexChangedEventArgs) {
        self.raise_property_changed_event(
            SelectionPatternIdentifiers::selection_property(),
            Some(Rc::new(e.old_index()) as BoxedValue),
            Some(Rc::new(e.new_index()) as BoxedValue),
        );
    }
}
