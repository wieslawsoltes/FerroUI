//! `Automation/Peers/TreeDataGridAutomationPeer.cs`: the automation peer of the grid, which
//! provides the selection of its rows.

use crate::presentation::PresentationRef;
use crate::selection::ITreeDataGridSelectionInteraction;
use crate::TreeDataGrid;
use ferroui_base::data::core::ValueType;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::ISelectionProvider;
use ferroui_controls::automation::SelectionPatternIdentifiers;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An automation peer which represents a [`TreeDataGrid`].
#[repr(C)]
pub struct TreeDataGridAutomationPeer {
    base: ControlAutomationPeer,
    owner_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    source: RefCell<Option<PresentationRef>>,
    source_token: Cell<Option<u64>>,
    row_selection: RefCell<Option<Rc<dyn ITreeDataGridSelectionInteraction>>>,
    row_selection_token: Cell<Option<u64>>,
}

ferro_class!(TreeDataGridAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeDataGridAutomationPeer {
    interfaces: [Rc<dyn ISelectionProvider> => TreeDataGridAutomationPeer::as_selection_provider]
});

impl FerroObjectImpl for TreeDataGridAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        let subscription = this.owner().property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
            if let Some(this) = weak.upgrade() {
                this.on_owner_property_changed(e);
            }
        });
        *this.owner_subscription.borrow_mut() = Some(subscription);
        this.attach_source(this.owner().presentation());
    }
}

impl ControlAutomationPeerImpl for TreeDataGridAutomationPeer {}

impl AutomationPeerImpl for TreeDataGridAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::DataGrid
    }

    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if provider_type == ValueType::of::<Rc<dyn ISelectionProvider>>()
            && this.source.borrow().as_ref().and_then(|source| source.selected_indexes()).is_none()
        {
            return None;
        }

        Self::parent_get_provider_core(this, provider_type)
    }
}

/// The handle through which the peer implements [`ISelectionProvider`].
struct SelectionProviderAdapter(Ref<TreeDataGridAutomationPeer>);

impl ISelectionProvider for SelectionProviderAdapter {
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

impl TreeDataGridAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeDataGrid) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
            owner_subscription: RefCell::new(None),
            source: RefCell::new(None),
            source_token: Cell::new(None),
            row_selection: RefCell::new(None),
            row_selection_token: Cell::new(None),
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeDataGrid) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn as_selection_provider(peer: Ref<Self>) -> Rc<dyn ISelectionProvider> {
        Rc::new(SelectionProviderAdapter(peer))
    }

    /// The owning grid.
    pub fn owner(&self) -> Ref<TreeDataGrid> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a TreeDataGrid.")
    }

    /// Whether more than one row can be selected (`CanSelectMultiple`).
    pub fn can_select_multiple(&self) -> bool {
        self.source.borrow().as_ref().is_some_and(|source| source.can_select_multiple())
    }

    /// A selection is not required (`IsSelectionRequired`).
    pub fn is_selection_required(&self) -> bool {
        false
    }

    /// The peers of the selected rows that are realized (`GetSelection`).
    pub fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        let source = self.source.borrow().clone();
        let owner = self.owner();
        let (Some(selected_indexes), Some(rows)) = (source.and_then(|source| source.selected_indexes()), owner.rows()) else {
            return Vec::new();
        };

        let mut result = Vec::new();

        for model_index in &selected_indexes {
            let row_index = rows.model_index_to_row_index(model_index);

            if row_index >= 0 {
                if let Some(row) = owner.try_get_row(row_index) {
                    if row.is_attached_to_visual_tree() {
                        result.push(self.get_or_create(&row));
                    }
                }
            }
        }

        result
    }

    fn attach_source(&self, source: Option<PresentationRef>) {
        let previous = self.source.borrow_mut().take();
        if let (Some(previous), Some(token)) = (previous, self.source_token.take()) {
            previous.property_changed().remove(token);
        }

        if let Some(source) = &source {
            let weak = self.to_ref().downgrade();
            let token = source.property_changed().add(Rc::new(move |name: &str| {
                if let Some(this) = weak.upgrade() {
                    this.on_source_property_changed(name);
                }
            }));
            self.source_token.set(Some(token));
        }
        *self.source.borrow_mut() = source;

        self.attach_row_selection();
    }

    fn attach_row_selection(&self) {
        let previous = self.row_selection.borrow_mut().take();
        if let (Some(previous), Some(token)) = (previous, self.row_selection_token.take()) {
            previous.remove_selection_changed(token);
        }

        let row_selection = self.source.borrow().as_ref().and_then(|source| source.selection_interaction());

        if let Some(row_selection) = &row_selection {
            let weak = self.to_ref().downgrade();
            let token = row_selection.add_selection_changed(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.raise_selection_changed();
                }
            }));
            self.row_selection_token.set(Some(token));
        }
        *self.row_selection.borrow_mut() = row_selection;
    }

    fn raise_selection_changed(&self) {
        self.raise_property_changed_event(SelectionPatternIdentifiers::selection_property(), None, None);
    }

    fn on_owner_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == TreeDataGrid::presentation_property().as_property() {
            self.attach_source(self.owner().presentation());
            self.raise_selection_changed();
        }
    }

    fn on_source_property_changed(&self, name: &str) {
        if name == "Selection" || name == "SelectionInteraction" {
            self.attach_row_selection();
            self.raise_selection_changed();
        }
    }
}
