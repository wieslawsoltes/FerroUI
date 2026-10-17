//! `ITreeDataGridSource.cs`: the contract of a source of a tree data grid.

use crate::accessor::TreeModel;
use crate::model_collection::Models;
use crate::models::{ColumnList, IColumn, IRows, ListSortDirection};
use crate::selection::{ITreeDataGridSelection, UntypedModels};
use crate::IndexPath;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// Where a dragged row is dropped relative to the target row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RowDropPosition {
    #[default]
    None,
    Before,
    After,
    Inside,
}

/// The effects of a move of rows (`[Flags] RowMoveEffects`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RowMoveEffects(pub u32);

impl RowMoveEffects {
    pub const NONE: RowMoveEffects = RowMoveEffects(0);
    pub const COPY: RowMoveEffects = RowMoveEffects(1);
    pub const MOVE: RowMoveEffects = RowMoveEffects(2);
    pub const LINK: RowMoveEffects = RowMoveEffects(4);

    pub fn contains(self, other: RowMoveEffects) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for RowMoveEffects {
    type Output = RowMoveEffects;

    fn bitor(self, rhs: RowMoveEffects) -> RowMoveEffects {
        RowMoveEffects(self.0 | rhs.0)
    }
}

/// A source of a tree data grid, untyped (`ITreeDataGridSource`).
pub trait ITreeDataGridSource: INotifyPropertyChanged + Any {
    /// The number of columns.
    fn column_count(&self) -> usize;
    /// The column at `index`, untyped.
    fn column(&self, index: usize) -> Rc<dyn IColumn>;
    /// The columns, untyped.
    fn columns_untyped(&self) -> Vec<Rc<dyn IColumn>> {
        (0..self.column_count()).map(|i| self.column(i)).collect()
    }
    /// The rows.
    fn rows(&self) -> Rc<dyn IRows>;
    /// The selection (a row selection is created on first access unless one was set).
    fn selection(&self) -> Option<Rc<dyn ITreeDataGridSelection>>;
    /// Sets the selection, whose source must be the items of the source.
    fn set_selection(&self, value: Option<Rc<dyn ITreeDataGridSelection>>);
    fn is_hierarchical(&self) -> bool;
    fn is_sorted(&self) -> bool;
    /// The items, untyped.
    fn items_untyped(&self) -> UntypedModels;
    /// The child models of a model, untyped (`GetModelChildren(object)`).
    fn get_model_children_untyped(&self, model: &BoxedValue) -> Option<Vec<BoxedValue>>;
    /// Sorts the rows by `column` (a column of the source); false when the column is not one.
    fn sort_by(&self, column: Option<&dyn IColumn>, direction: ListSortDirection) -> bool;
    fn clear_sort(&self);
    /// Subscribes to `Sorted`.
    fn add_sorted(&self, handler: Rc<dyn Fn()>) -> u64;
    fn remove_sorted(&self, token: u64);
    /// Moves the rows at `indexes` of `source` to `target_index` (drag and drop).
    fn move_rows(
        &self,
        source: &dyn ITreeDataGridSource,
        indexes: &[IndexPath],
        target_index: &IndexPath,
        position: RowDropPosition,
        effects: RowMoveEffects,
    );
    fn as_any(&self) -> &dyn Any;
}

/// A source of models of `M` (`ITreeDataGridSource<TModel>`).
pub trait ITreeDataGridSourceOf<M: TreeModel>: ITreeDataGridSource {
    fn items(&self) -> Models<M>;
    fn set_items(&self, value: Models<M>);
    fn columns(&self) -> &ColumnList<M>;
    /// The child models of `model`.
    fn get_model_children(&self, model: &Rc<M>) -> Option<Models<M>>;
}

/// The source of a shared source as the concrete source type `T`.
pub fn downcast_source<T: 'static>(source: &Rc<dyn ITreeDataGridSource>) -> Option<Rc<T>> {
    let any: Rc<dyn Any> = source.clone();
    any.downcast::<T>().ok()
}
