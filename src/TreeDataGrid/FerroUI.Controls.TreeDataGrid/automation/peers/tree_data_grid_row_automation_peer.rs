//! `Automation/Peers/TreeDataGridRowAutomationPeer.cs`: the automation peer of a row: its
//! expansion (toggle and expand/collapse patterns), its model as a read-only value and its
//! selection.

use crate::primitives::TreeDataGridRow;
use crate::presentation::PresentationRef;
use crate::TreeDataGrid;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, ISelectionItemProvider, ISelectionProvider, IToggleProvider, IValueProvider, ProviderError,
    ToggleState,
};
use ferroui_controls::automation::{ElementNotEnabledException, ExpandCollapseState};
use ferroui_treedatagrid_core::models::RowRef;
use ferroui_treedatagrid_core::IndexPath;
use std::rc::Rc;

/// An automation peer which represents a [`TreeDataGridRow`].
#[repr(C)]
pub struct TreeDataGridRowAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TreeDataGridRowAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TreeDataGridRowAutomationPeer {
    interfaces: [
        Rc<dyn IToggleProvider> => TreeDataGridRowAutomationPeer::as_toggle_provider,
        Rc<dyn IValueProvider> => TreeDataGridRowAutomationPeer::as_value_provider,
        Rc<dyn IExpandCollapseProvider> => TreeDataGridRowAutomationPeer::as_expand_collapse_provider,
        Rc<dyn ISelectionItemProvider> => TreeDataGridRowAutomationPeer::as_selection_item_provider,
    ]
});

impl FerroObjectImpl for TreeDataGridRowAutomationPeer {}

impl ControlAutomationPeerImpl for TreeDataGridRowAutomationPeer {}

impl AutomationPeerImpl for TreeDataGridRowAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TreeItem
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }

    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if (provider_type == ValueType::of::<Rc<dyn IToggleProvider>>()
            || provider_type == ValueType::of::<Rc<dyn IExpandCollapseProvider>>())
            && this.toggle_expander().is_none()
        {
            return None;
        }

        if provider_type == ValueType::of::<Rc<dyn ISelectionItemProvider>>()
            && (this.try_get_model_index().is_none() || this.row_selection_model().is_none())
        {
            return None;
        }

        Self::parent_get_provider_core(this, provider_type)
    }
}

struct ToggleProviderAdapter(Ref<TreeDataGridRowAutomationPeer>);

impl IToggleProvider for ToggleProviderAdapter {
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

struct ValueProviderAdapter(Ref<TreeDataGridRowAutomationPeer>);

impl IValueProvider for ValueProviderAdapter {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }

    fn value(&self) -> Option<String> {
        self.0.value()
    }

    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        self.0.set_value(value);
        Ok(())
    }
}

struct ExpandCollapseProviderAdapter(Ref<TreeDataGridRowAutomationPeer>);

impl IExpandCollapseProvider for ExpandCollapseProviderAdapter {
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

struct SelectionItemProviderAdapter(Ref<TreeDataGridRowAutomationPeer>);

impl ISelectionItemProvider for SelectionItemProviderAdapter {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }

    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.selection_container()
    }

    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.add_to_selection()
    }

    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.remove_from_selection()
    }

    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.select()
    }
}

impl TreeDataGridRowAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TreeDataGridRow) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TreeDataGridRow) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn as_toggle_provider(peer: Ref<Self>) -> Rc<dyn IToggleProvider> {
        Rc::new(ToggleProviderAdapter(peer))
    }

    fn as_value_provider(peer: Ref<Self>) -> Rc<dyn IValueProvider> {
        Rc::new(ValueProviderAdapter(peer))
    }

    fn as_expand_collapse_provider(peer: Ref<Self>) -> Rc<dyn IExpandCollapseProvider> {
        Rc::new(ExpandCollapseProviderAdapter(peer))
    }

    fn as_selection_item_provider(peer: Ref<Self>) -> Rc<dyn ISelectionItemProvider> {
        Rc::new(SelectionItemProviderAdapter(peer))
    }

    /// The owning row.
    pub fn owner(&self) -> Ref<TreeDataGridRow> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a row.")
    }

    /// Whether the row is expanded (`ToggleState`); indeterminate for a row without an expander.
    pub fn toggle_state(&self) -> ToggleState {
        match self.toggle_expander() {
            Some(row) => {
                if row.as_expander().is_some_and(|e| e.is_expanded()) {
                    ToggleState::On
                } else {
                    ToggleState::Off
                }
            }
            None => ToggleState::Indeterminate,
        }
    }

    /// The model is read only (`IsReadOnly`).
    pub fn is_read_only(&self) -> bool {
        true
    }

    /// `ExpandCollapseState`.
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        match self.toggle_expander() {
            Some(row) => {
                if row.as_expander().is_some_and(|e| e.is_expanded()) {
                    ExpandCollapseState::Expanded
                } else {
                    ExpandCollapseState::Collapsed
                }
            }
            None => ExpandCollapseState::LeafNode,
        }
    }

    /// `ShowsMenu`.
    pub fn shows_menu(&self) -> bool {
        false
    }

    /// Whether the row is selected (`IsSelected`).
    pub fn is_selected(&self) -> bool {
        match (self.try_get_model_index(), self.row_selection_model()) {
            (Some(model_index), Some(selection)) => selection.is_selected(&model_index),
            _ => false,
        }
    }

    /// The selection provider of the owning grid (`SelectionContainer`).
    pub fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        let owner = self.owning_grid()?;
        self.get_or_create(&owner).get_provider::<dyn ISelectionProvider>()
    }

    /// The text of the model (`Value`).
    pub fn value(&self) -> Option<String> {
        self.owner().model().map(|model| ValueTypes::to_display_string(Some(&model)))
    }

    /// Expands or collapses the row (`Toggle`).
    pub fn toggle(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let Some(row) = self.toggle_expander() {
            if let Some(expander) = row.as_expander() {
                expander.set_is_expanded(!expander.is_expanded());
            }
        }
        Ok(())
    }

    /// Expands the row (`Expand`).
    pub fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let Some(row) = self.toggle_expander() {
            if let Some(expander) = row.as_expander() {
                expander.set_is_expanded(true);
            }
        }
        Ok(())
    }

    /// Collapses the row (`Collapse`).
    pub fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let Some(row) = self.toggle_expander() {
            if let Some(expander) = row.as_expander() {
                expander.set_is_expanded(false);
            }
        }
        Ok(())
    }

    /// # Panics
    /// Always: rows are read only (`NotSupportedException`).
    pub fn set_value(&self, _value: Option<&str>) {
        panic!("TreeDataGrid rows are read-only.");
    }

    /// Adds the row to the selection (`AddToSelection`).
    pub fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let (Some(model_index), Some(selection)) = (self.try_get_model_index(), self.row_selection_model()) {
            selection.select(&model_index, false);
        }
        Ok(())
    }

    /// Removes the row from the selection (`RemoveFromSelection`).
    pub fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let (Some(model_index), Some(selection)) = (self.try_get_model_index(), self.row_selection_model()) {
            selection.deselect(&model_index);
        }
        Ok(())
    }

    /// Selects the row alone (`Select`).
    pub fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;

        if let (Some(model_index), Some(selection)) = (self.try_get_model_index(), self.row_selection_model()) {
            selection.select(&model_index, true);
        }
        Ok(())
    }

    fn owning_grid(&self) -> Option<Ref<TreeDataGrid>> {
        self.owner().find_ancestor_of_type::<TreeDataGrid>(false)
    }

    fn row_selection_model(&self) -> Option<PresentationRef> {
        let presentation = self.owning_grid()?.presentation()?;
        if presentation.selected_indexes().is_some() {
            Some(presentation)
        } else {
            None
        }
    }

    fn try_get_model_index(&self) -> Option<IndexPath> {
        let owner = self.owner();
        let rows = owner.rows()?;
        let row_index = owner.row_index();

        if row_index < 0 || row_index as usize >= rows.count() {
            return None;
        }

        let model_index = rows.row_index_to_model_index(row_index);
        if model_index.count() > 0 {
            Some(model_index)
        } else {
            None
        }
    }

    // The row of the owner when it is an expander that shows its expander (`GetToggleExpanderOrNull`).
    fn toggle_expander(&self) -> Option<RowRef> {
        let owner = self.owner();
        let rows = owner.rows()?;
        let row_index = owner.row_index();

        if row_index < 0 || row_index as usize >= rows.count() {
            return None;
        }

        let row = rows.get(row_index as usize);
        if row.as_expander().is_some_and(|expander| expander.show_expander()) {
            Some(row)
        } else {
            None
        }
    }
}
