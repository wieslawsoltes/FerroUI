//! The typed operations the untyped control needs of a source and of its selection: creating a
//! row or cell selection for a source (the reference's `ITreeDataGridSelectionFactory`, which its
//! classic sources implement), reading a cell selection, and updating the expander of a
//! hierarchical row.
//!
//! Not a file of the reference: the reference's control reaches these through the generic types
//! of its sources and selection models (`source.Selection is ITreeDataGridCellSelectionModel<T>`,
//! `new TreeDataGridCellSelectionModel<T>(source)`). The port's control holds a source as
//! `Rc<dyn ITreeDataGridSource>` (a value of a property, set from markup or a binding), and a trait
//! object cannot be asked for the model type it was made with. Each model type therefore registers
//! its typed operations once, keyed by the concrete types of its sources, selections and rows;
//! [`register_tree_data_grid_model`] does it for the sources of the core, and the handles that
//! carry a typed source to the control ([`TreeDataGridSourceRef`](crate::TreeDataGridSourceRef))
//! call it.

use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{as_hierarchical_row, RowRef};
use ferroui_treedatagrid_core::selection::{
    downcast_selection, ITreeDataGridSelection, ITreeSelectionModel, TreeDataGridCellSelectionModel,
    TreeDataGridRowSelectionModel,
};
use ferroui_treedatagrid_core::{
    CellIndex, FlatTreeDataGridSource, HierarchicalTreeDataGridSource, ITreeDataGridSource, ITreeDataGridSourceOf,
};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

/// A row selection of a source, untyped (`ITreeDataGridRowSelectionModel`): the tree selection
/// model of its rows and `StateChanged`.
pub trait IUntypedRowSelection {
    /// The tree selection model of the rows.
    fn model(&self) -> Rc<dyn ITreeSelectionModel>;
    /// `StateChanged`: the selection, its indexes or its source changed, raised once a change of
    /// the source's collections has finished.
    fn add_state_changed(&self, handler: Rc<dyn Fn()>) -> u64;
    fn remove_state_changed(&self, token: u64);
}

impl<M: TreeModel> IUntypedRowSelection for TreeDataGridRowSelectionModel<M> {
    fn model(&self) -> Rc<dyn ITreeSelectionModel> {
        self.base().clone() as Rc<dyn ITreeSelectionModel>
    }

    fn add_state_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        TreeDataGridRowSelectionModel::add_state_changed(self, handler)
    }

    fn remove_state_changed(&self, token: u64) {
        TreeDataGridRowSelectionModel::remove_state_changed(self, token);
    }
}

/// A cell selection of a source, untyped (`ITreeDataGridCellSelectionModel<T>` without its model
/// type).
pub trait IUntypedCellSelection {
    fn single_select(&self) -> bool;
    fn set_single_select(&self, value: bool);
    fn count(&self) -> i32;
    fn selected_index(&self) -> CellIndex;
    fn set_selected_index(&self, value: CellIndex);
    fn anchor_index(&self) -> CellIndex;
    fn range_anchor_index(&self) -> CellIndex;
    /// The selected cells, row by row.
    fn selected_indexes(&self) -> Vec<CellIndex>;
    fn is_selected(&self, index: &CellIndex) -> bool;
    fn clear(&self);
    /// Selects `column_count` columns and `row_count` rows from `start` (negative counts go back).
    fn set_selected_range(&self, start: &CellIndex, column_count: i32, row_count: i32);
    /// `SelectionChanged`.
    fn add_selection_changed(&self, handler: Rc<dyn Fn()>) -> u64;
    fn remove_selection_changed(&self, token: u64);
    /// The tree selection model of the rows of the cell selection.
    fn row_selection(&self) -> Rc<dyn ITreeSelectionModel>;
}

impl<M: TreeModel> IUntypedCellSelection for TreeDataGridCellSelectionModel<M> {
    fn single_select(&self) -> bool {
        TreeDataGridCellSelectionModel::single_select(self)
    }

    fn set_single_select(&self, value: bool) {
        TreeDataGridCellSelectionModel::set_single_select(self, value);
    }

    fn count(&self) -> i32 {
        TreeDataGridCellSelectionModel::count(self)
    }

    fn selected_index(&self) -> CellIndex {
        TreeDataGridCellSelectionModel::selected_index(self)
    }

    fn set_selected_index(&self, value: CellIndex) {
        TreeDataGridCellSelectionModel::set_selected_index(self, value);
    }

    fn anchor_index(&self) -> CellIndex {
        TreeDataGridCellSelectionModel::anchor_index(self)
    }

    fn range_anchor_index(&self) -> CellIndex {
        TreeDataGridCellSelectionModel::range_anchor_index(self)
    }

    fn selected_indexes(&self) -> Vec<CellIndex> {
        TreeDataGridCellSelectionModel::selected_indexes(self)
    }

    fn is_selected(&self, index: &CellIndex) -> bool {
        TreeDataGridCellSelectionModel::is_selected(self, index)
    }

    fn clear(&self) {
        TreeDataGridCellSelectionModel::clear(self);
    }

    fn set_selected_range(&self, start: &CellIndex, column_count: i32, row_count: i32) {
        TreeDataGridCellSelectionModel::set_selected_range(self, start, column_count, row_count);
    }

    fn add_selection_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        TreeDataGridCellSelectionModel::add_selection_changed(
            self,
            Rc::new(move |_: &ferroui_treedatagrid_core::selection::TreeDataGridCellSelectionChangedEventArgs| handler()),
        )
    }

    fn remove_selection_changed(&self, token: u64) {
        TreeDataGridCellSelectionModel::remove_selection_changed(self, token);
    }

    fn row_selection(&self) -> Rc<dyn ITreeSelectionModel> {
        self.row_selection().base().clone() as Rc<dyn ITreeSelectionModel>
    }
}

/// The selections a source can create (`ITreeDataGridSelectionFactory`).
pub trait ITreeDataGridSelectionFactory {
    /// A row selection of `source`, or `None` when `source` is not of the factory's source type.
    fn create_row_selection_model(&self, source: &Rc<dyn ITreeDataGridSource>) -> Option<Rc<dyn ITreeDataGridSelection>>;
    /// A cell selection of `source`, or `None` when `source` is not of the factory's source type.
    fn create_cell_selection_model(&self, source: &Rc<dyn ITreeDataGridSource>) -> Option<Rc<dyn ITreeDataGridSelection>>;
}

/// The typed operations on a source type: the selections it can create and the changes of its
/// column list, which the untyped source contract does not expose (`model.Columns as
/// INotifyCollectionChanged`).
pub trait ITreeDataGridSourceSupport: ITreeDataGridSelectionFactory {
    /// Subscribes `handler` to the changes of the columns of `source`; `None` when `source` is not
    /// of the type.
    fn add_columns_changed(&self, source: &Rc<dyn ITreeDataGridSource>, handler: Rc<dyn Fn()>) -> Option<u64>;
    /// Ends a subscription of [`add_columns_changed`](Self::add_columns_changed).
    fn remove_columns_changed(&self, source: &Rc<dyn ITreeDataGridSource>, token: u64);
}

/// The typed operations on the selections and rows of a model type.
pub trait ITreeDataGridModelSupport {
    /// The selection as a row selection of the model type.
    fn as_row_selection(&self, selection: &Rc<dyn ITreeDataGridSelection>) -> Option<Rc<dyn IUntypedRowSelection>>;
    /// The selection as a cell selection of the model type.
    fn as_cell_selection(&self, selection: &Rc<dyn ITreeDataGridSelection>) -> Option<Rc<dyn IUntypedCellSelection>>;
    /// Sets whether a hierarchical row of the model type shows its expander
    /// (`IExpanderRow<T>.UpdateShowExpander`); false when the row is not one.
    fn update_show_expander(&self, row: &RowRef, value: bool) -> bool;
    /// Forgets the explicit expander state of a hierarchical row (`HierarchicalRow<T>.ClearShowExpander`).
    fn clear_show_expander(&self, row: &RowRef);
}

struct SourceSupport<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static>(PhantomData<(Rc<M>, Rc<S>)>);

impl<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static> SourceSupport<M, S> {
    fn typed(source: &Rc<dyn ITreeDataGridSource>) -> Option<Rc<dyn ITreeDataGridSourceOf<M>>> {
        let any: Rc<dyn Any> = source.clone();
        let typed: Rc<S> = any.downcast::<S>().ok()?;
        Some(typed as Rc<dyn ITreeDataGridSourceOf<M>>)
    }
}

impl<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static> ITreeDataGridSelectionFactory for SourceSupport<M, S> {
    fn create_row_selection_model(&self, source: &Rc<dyn ITreeDataGridSource>) -> Option<Rc<dyn ITreeDataGridSelection>> {
        let typed = Self::typed(source)?;
        Some(TreeDataGridRowSelectionModel::new(&typed) as Rc<dyn ITreeDataGridSelection>)
    }

    fn create_cell_selection_model(&self, source: &Rc<dyn ITreeDataGridSource>) -> Option<Rc<dyn ITreeDataGridSelection>> {
        let typed = Self::typed(source)?;
        Some(TreeDataGridCellSelectionModel::new(&typed) as Rc<dyn ITreeDataGridSelection>)
    }
}

impl<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static> ITreeDataGridSourceSupport for SourceSupport<M, S> {
    fn add_columns_changed(&self, source: &Rc<dyn ITreeDataGridSource>, handler: Rc<dyn Fn()>) -> Option<u64> {
        let typed = Self::typed(source)?;
        let token = typed.columns().list().add_collection_changed(Rc::new(
            move |_: &ferroui_base::collections::NotifyCollectionChangedEventArgs<'_, ferroui_treedatagrid_core::models::ColumnRef<M>>| {
                handler()
            },
        ));
        Some(token)
    }

    fn remove_columns_changed(&self, source: &Rc<dyn ITreeDataGridSource>, token: u64) {
        if let Some(typed) = Self::typed(source) {
            typed.columns().list().remove_collection_changed(token);
        }
    }
}

struct ModelSupport<M: TreeModel>(PhantomData<Rc<M>>);

impl<M: TreeModel> ITreeDataGridModelSupport for ModelSupport<M> {
    fn as_row_selection(&self, selection: &Rc<dyn ITreeDataGridSelection>) -> Option<Rc<dyn IUntypedRowSelection>> {
        downcast_selection::<TreeDataGridRowSelectionModel<M>>(selection).map(|s| s as Rc<dyn IUntypedRowSelection>)
    }

    fn as_cell_selection(&self, selection: &Rc<dyn ITreeDataGridSelection>) -> Option<Rc<dyn IUntypedCellSelection>> {
        downcast_selection::<TreeDataGridCellSelectionModel<M>>(selection).map(|s| s as Rc<dyn IUntypedCellSelection>)
    }

    fn update_show_expander(&self, row: &RowRef, value: bool) -> bool {
        match as_hierarchical_row::<M>(row) {
            Some(row) => {
                row.update_show_expander(value);
                true
            }
            None => false,
        }
    }

    fn clear_show_expander(&self, row: &RowRef) {
        if let Some(row) = as_hierarchical_row::<M>(row) {
            row.clear_show_expander();
        }
    }
}

#[derive(Default)]
struct Registry {
    // By the type of a source.
    sources: HashMap<TypeId, Rc<dyn ITreeDataGridSourceSupport>>,
    // By the type of a selection or of a row.
    models: HashMap<TypeId, Rc<dyn ITreeDataGridModelSupport>>,
    // The model types registered.
    model_types: HashMap<TypeId, ()>,
}

ferroui_base::ferro_thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// Registers the typed operations of the model type `M` and of the core's sources of it
/// (`FlatTreeDataGridSource<M>`, `HierarchicalTreeDataGridSource<M>`). Registering a type again does
/// nothing.
pub fn register_tree_data_grid_model<M: TreeModel>() {
    let first = REGISTRY.with(|registry| registry.borrow_mut().model_types.insert(TypeId::of::<M>(), ()).is_none());
    if !first {
        return;
    }
    let support: Rc<dyn ITreeDataGridModelSupport> = Rc::new(ModelSupport::<M>(PhantomData));
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        registry.models.insert(TypeId::of::<TreeDataGridRowSelectionModel<M>>(), support.clone());
        registry.models.insert(TypeId::of::<TreeDataGridCellSelectionModel<M>>(), support.clone());
        registry.models.insert(TypeId::of::<ferroui_treedatagrid_core::models::HierarchicalRow<M>>(), support.clone());
        registry.models.insert(TypeId::of::<ferroui_treedatagrid_core::models::AnonymousRow<M>>(), support);
    });
    register_tree_data_grid_source::<M, FlatTreeDataGridSource<M>>();
    register_tree_data_grid_source::<M, HierarchicalTreeDataGridSource<M>>();
}

/// Registers a source type `S` of models of `M` (a source of the application's own that implements
/// [`ITreeDataGridSourceOf<M>`]), so that the control can create row and cell selections of it.
pub fn register_tree_data_grid_source<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static>() {
    register_model_only::<M>();
    let factory: Rc<dyn ITreeDataGridSourceSupport> = Rc::new(SourceSupport::<M, S>(PhantomData));
    REGISTRY.with(|registry| {
        registry.borrow_mut().sources.entry(TypeId::of::<S>()).or_insert(factory);
    });
}

fn register_model_only<M: TreeModel>() {
    let known = REGISTRY.with(|registry| registry.borrow().model_types.contains_key(&TypeId::of::<M>()));
    if !known {
        register_tree_data_grid_model::<M>();
    }
}

/// The typed operations of the type of `source`, if the type is registered.
pub fn source_support(source: &Rc<dyn ITreeDataGridSource>) -> Option<Rc<dyn ITreeDataGridSourceSupport>> {
    let id = source.as_any().type_id();
    REGISTRY.with(|registry| registry.borrow().sources.get(&id).cloned())
}

/// Whether the type of `source` is registered.
pub fn is_source_registered(source: &Rc<dyn ITreeDataGridSource>) -> bool {
    source_support(source).is_some()
}

fn model_support(id: TypeId) -> Option<Rc<dyn ITreeDataGridModelSupport>> {
    REGISTRY.with(|registry| registry.borrow().models.get(&id).cloned())
}

fn selection_type(selection: &Rc<dyn ITreeDataGridSelection>) -> TypeId {
    let any: &dyn Any = &**selection;
    any.type_id()
}

/// The selection as a row selection, if it is the row selection of a registered model type.
pub fn as_row_selection(selection: &Rc<dyn ITreeDataGridSelection>) -> Option<Rc<dyn IUntypedRowSelection>> {
    model_support(selection_type(selection)).and_then(|support| support.as_row_selection(selection))
}

/// The selection as a cell selection, if it is the cell selection of a registered model type.
pub fn as_cell_selection(selection: &Rc<dyn ITreeDataGridSelection>) -> Option<Rc<dyn IUntypedCellSelection>> {
    model_support(selection_type(selection)).and_then(|support| support.as_cell_selection(selection))
}

/// Sets whether `row` shows its expander; false when the row is not a hierarchical row of a
/// registered model type.
pub fn update_show_expander(row: &RowRef, value: bool) -> bool {
    model_support(row.as_any().type_id()).is_some_and(|support| support.update_show_expander(row, value))
}

/// Forgets the explicit expander state of `row`, if it is a hierarchical row of a registered model
/// type.
pub fn clear_show_expander(row: &RowRef) {
    if let Some(support) = model_support(row.as_any().type_id()) {
        support.clear_show_expander(row);
    }
}
