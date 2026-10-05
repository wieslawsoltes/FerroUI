use super::{
    AutomationPeer, AutomationPeerImpl, ControlAutomationPeerImpl, ItemsControlAutomationPeer,
    ItemsControlAutomationPeerImpl,
};
use crate::automation::provider::{ISelectionProvider, ProviderAdapter};
use crate::automation::SelectionPatternIdentifiers;
use crate::primitives::SelectingItemsControl;
use crate::selection::{ISelectionModel, SelectionModelSelectionChangedEventArgs};
use crate::{ListBox, SelectionMode};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// An automation peer which represents a [`SelectingItemsControl`].
#[repr(C)]
pub struct SelectingItemsControlAutomationPeer {
    base: ItemsControlAutomationPeer,
    selection: RefCell<Option<Rc<dyn ISelectionModel>>>,
    selection_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    SelectingItemsControlAutomationPeer: ItemsControlAutomationPeer, virtuals SelectingItemsControlAutomationPeerImpl: ItemsControlAutomationPeerImpl {
        /// Gets the peers of the selected items, or `None` if there are
        /// none.
        fn get_selection_core(this) -> Option<Vec<Ref<AutomationPeer>>>;
        /// Gets the selection mode of the owner.
        fn get_selection_mode_core(this) -> SelectionMode;
        /// Called when a property of the owner has changed.
        fn owner_property_changed(this, e: &FerroPropertyChangedEventArgs<'_>);
        /// Called when the selection of the owner has changed.
        fn owner_selection_changed(this, e: &dyn SelectionModelSelectionChangedEventArgs);
    }
}
ferroui_base::ferro_class_info!(SelectingItemsControlAutomationPeer { interfaces: [Rc<dyn ISelectionProvider> => ProviderAdapter::as_selection_provider] });

impl FerroObjectImpl for SelectingItemsControlAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let owner = this.owner();
        let selection = owner.get_direct_value(ListBox::selection_property());
        *this.selection.borrow_mut() = selection;
        this.subscribe_to_selection();

        let weak = this.to_ref().downgrade();
        owner.property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.owner_property_changed(e);
            }
        });
    }
}

impl AutomationPeerImpl for SelectingItemsControlAutomationPeer {}
impl ControlAutomationPeerImpl for SelectingItemsControlAutomationPeer {}
impl ItemsControlAutomationPeerImpl for SelectingItemsControlAutomationPeer {}

impl SelectingItemsControlAutomationPeerImpl for SelectingItemsControlAutomationPeer {
    fn get_selection_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        let mut result: Option<Vec<Ref<AutomationPeer>>> = None;

        if let Some(owner) = this.owner().cast::<SelectingItemsControl>() {
            let selection = this.owner().get_direct_value(ListBox::selection_property());
            let selected_indexes: Vec<i32> =
                selection.map(|selection| selection.selected_indexes().iter().collect()).unwrap_or_default();

            for i in selected_indexes {
                let container = owner.container_from_index(i);

                if let Some(c) = container.filter(|c| c.is_attached_to_visual_tree()) {
                    let peer = this.get_or_create(&c);
                    result.get_or_insert_with(Vec::new).push(peer);
                }
            }
        }

        result
    }

    fn get_selection_mode_core(this: &Self) -> SelectionMode {
        this.owner()
            .cast::<SelectingItemsControl>()
            .map(|owner| owner.get_value(ListBox::selection_mode_property()))
            .unwrap_or(SelectionMode::SINGLE)
    }

    fn owner_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == ListBox::selection_property().as_property() {
            this.unsubscribe_from_selection();
            let selection = this.owner().get_direct_value(ListBox::selection_property());
            *this.selection.borrow_mut() = selection;
            this.subscribe_to_selection();
            this.raise_selection_changed();
        }
    }

    fn owner_selection_changed(this: &Self, _e: &dyn SelectionModelSelectionChangedEventArgs) {
        this.raise_selection_changed();
    }
}

impl ISelectionProvider for ProviderAdapter<SelectingItemsControlAutomationPeer> {
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

impl SelectingItemsControlAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    /// The class is abstract: it is the base of the peers of the selecting
    /// items control classes.
    pub fn construct(owner: &SelectingItemsControl) -> Self {
        Self {
            base: ItemsControlAutomationPeer::construct(owner),
            selection: RefCell::new(None),
            selection_changed: RefCell::new(None),
        }
    }

    /// Gets a value that indicates whether more than one item can be
    /// selected concurrently (the selection provider contract).
    pub fn can_select_multiple(&self) -> bool {
        self.get_selection_mode_core().contains(SelectionMode::MULTIPLE)
    }

    /// Gets a value that indicates whether at least one item is required
    /// to be selected (the selection provider contract).
    pub fn is_selection_required(&self) -> bool {
        self.get_selection_mode_core().contains(SelectionMode::ALWAYS_SELECTED)
    }

    /// Retrieves the peer of each item that is selected (the selection
    /// provider contract).
    pub fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        self.get_selection_core().unwrap_or_default()
    }

    // The selection model holds the handler; the handler holds the peer
    // weakly.
    fn subscribe_to_selection(&self) {
        let selection = self.selection.borrow().clone();
        let subscription = selection.map(|selection| {
            let weak = self.to_ref().downgrade();
            selection.selection_changed(Rc::new(move |e| {
                if let Some(this) = weak.upgrade() {
                    this.owner_selection_changed(e);
                }
            }))
        });
        *self.selection_changed.borrow_mut() = subscription;
    }

    fn unsubscribe_from_selection(&self) {
        let subscription = self.selection_changed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn raise_selection_changed(&self) {
        self.raise_property_changed_event(SelectionPatternIdentifiers::selection_property(), None, None);
    }
}
