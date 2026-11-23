use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
    ItemsControlAutomationPeer, ItemsControlAutomationPeerImpl,
};
use crate::automation::provider::{
    IExpandCollapseProvider, ISelectionItemProvider, ISelectionProvider, ProviderAdapter,
};
use crate::automation::{AutomationError, ExpandCollapsePatternIdentifiers, ExpandCollapseState};
use crate::{ItemsControl, TreeViewItem};
use ferroui_base::{
    ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref,
};
use std::cell::Cell;
use std::rc::Rc;

/// An automation peer which represents a [`TreeViewItem`].
#[repr(C)]
pub struct TreeViewItemAutomationPeer {
    base: ItemsControlAutomationPeer,
    last_state: Cell<ExpandCollapseState>,
}

ferro_class!(TreeViewItemAutomationPeer: ItemsControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeViewItemAutomationPeer { interfaces: [
    Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider,
    Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider] });

impl FerroObjectImpl for TreeViewItemAutomationPeer {
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
impl ControlAutomationPeerImpl for TreeViewItemAutomationPeer {}
impl ItemsControlAutomationPeerImpl for TreeViewItemAutomationPeer {}

impl AutomationPeerImpl for TreeViewItemAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TreeItem
    }
}

impl ISelectionItemProvider for ProviderAdapter<TreeViewItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }

    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.selection_container()
    }

    fn add_to_selection(&self) -> Result<(), AutomationError> {
        self.0.add_to_selection()
    }

    fn remove_from_selection(&self) -> Result<(), AutomationError> {
        self.0.remove_from_selection()
    }

    fn select(&self) -> Result<(), AutomationError> {
        self.0.select()
    }
}

impl IExpandCollapseProvider for ProviderAdapter<TreeViewItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn expand_collapse_state(&self) -> ExpandCollapseState {
        self.0.expand_collapse_state()
    }

    fn shows_menu(&self) -> bool {
        false
    }

    fn expand(&self) -> Result<(), AutomationError> {
        self.0.set_expanded(true)
    }

    fn collapse(&self) -> Result<(), AutomationError> {
        self.0.set_expanded(false)
    }
}

impl TreeViewItemAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeViewItem) -> Self {
        Self {
            base: ItemsControlAutomationPeer::construct(owner),
            last_state: Cell::new(Self::to_state(owner.is_expanded(), owner.item_count())),
        }
    }

    fn tree_item(&self) -> Ref<TreeViewItem> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a tree view item.")
    }

    /// The state of the item (the expand and collapse provider contract).
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        let item = self.tree_item();
        Self::to_state(item.is_expanded(), item.item_count())
    }

    fn set_expanded(&self, expanded: bool) -> Result<(), AutomationError> {
        self.ensure_enabled()?;
        let item = self.tree_item();
        if item.item_count() == 0 {
            return Err(AutomationError::InvalidOperation("A leaf node cannot expand or collapse.".to_owned()));
        }
        item.set_current_value(TreeViewItem::is_expanded_property(), expanded);
        Ok(())
    }

    fn owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() != TreeViewItem::is_expanded_property().as_property()
            && e.property() != ItemsControl::item_count_property().as_property()
        {
            return;
        }

        // Compared with the last reported state, not the values of the event: the expanded and
        // collapsed events are raised before this handler, and one of their handlers may change
        // the item count, which notifies again from inside.
        let new_state = self.expand_collapse_state();
        let old_state = self.last_state.replace(new_state);
        if old_state != new_state {
            self.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(old_state) as BoxedValue),
                Some(Rc::new(new_state) as BoxedValue),
            );
        }
    }

    fn to_state(is_expanded: bool, item_count: i32) -> ExpandCollapseState {
        match (item_count, is_expanded) {
            (0, _) => ExpandCollapseState::LeafNode,
            (_, true) => ExpandCollapseState::Expanded,
            (_, false) => ExpandCollapseState::Collapsed,
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeViewItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets a value that indicates whether the item is selected (the
    /// selection item provider contract).
    pub fn is_selected(&self) -> bool {
        self.owner().get_value(TreeViewItem::is_selected_property())
    }

    /// Gets the selection provider of the tree view of the item (the
    /// selection item provider contract).
    pub fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        if let Some(tree_view) = self.owner().cast::<TreeViewItem>().and_then(|item| item.tree_view_owner()) {
            let parent_peer = self.get_or_create(&tree_view);
            return parent_peer.get_provider::<dyn ISelectionProvider>();
        }

        None
    }

    /// Clears any existing selection and then selects the item (the
    /// selection item provider contract).
    pub fn select(&self) -> Result<(), AutomationError> {
        self.ensure_enabled()?;

        if let Some(item) = self.owner().cast::<TreeViewItem>() {
            if let Some(tree_view) = item.tree_view_owner() {
                tree_view.selected_items().clear();
            }
            item.set_current_value(TreeViewItem::is_selected_property(), true);
        }

        Ok(())
    }

    // The reference implements the two members below explicitly: they are
    // members of the selection item provider contract only.
    fn add_to_selection(&self) -> Result<(), AutomationError> {
        self.ensure_enabled()?;

        if let Some(item) = self.owner().cast::<TreeViewItem>() {
            item.set_current_value(TreeViewItem::is_selected_property(), true);
        }

        Ok(())
    }

    fn remove_from_selection(&self) -> Result<(), AutomationError> {
        self.ensure_enabled()?;

        if let Some(item) = self.owner().cast::<TreeViewItem>() {
            item.set_current_value(TreeViewItem::is_selected_property(), false);
        }

        Ok(())
    }
}
