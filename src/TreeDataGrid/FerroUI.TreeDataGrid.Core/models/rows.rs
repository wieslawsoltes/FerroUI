//! The rows of a source: `Models/IRow.cs`, `IRow\`1.cs`, `IRows.cs`, `IIndentedRow.cs`,
//! `IModelIndexableRow.cs`, `IExpander.cs`, `IExpanderRow.cs`, `IExpanderRowController.cs`,
//! `RowEventArgs.cs` and `AnonymousRow.cs`.

use crate::accessor::TreeModel;
use crate::model_collection::Models;
use crate::{GridLength, IndexPath};
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A row of a source (`IRow`).
pub trait IRow: Any {
    /// The row header.
    fn header(&self) -> Option<BoxedValue>;
    /// The height of the row.
    fn height(&self) -> GridLength;
    fn set_height(&self, value: GridLength);
    /// The untyped model of the row.
    fn model(&self) -> Option<BoxedValue>;
    fn as_any(&self) -> &dyn Any;

    /// The row as an indented row (`is IIndentedRow`).
    fn as_indented(&self) -> Option<&dyn IIndentedRow> {
        None
    }
    /// The row as a row of an indexed source (`is IModelIndexableRow`).
    fn as_model_indexable(&self) -> Option<&dyn IModelIndexableRow> {
        None
    }
    /// The row as an expander (`is IExpander`).
    fn as_expander(&self) -> Option<&dyn IExpander> {
        None
    }
    /// The row's property change notification, if it has one.
    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        None
    }
}

/// A shared row.
pub type RowRef = Rc<dyn IRow>;

/// A row of models of `M` (`IRow<TModel>`).
pub trait IRowOf<M: TreeModel>: IRow {
    /// The model.
    fn model_of(&self) -> Rc<M>;
    /// Updates the model index after a change of the data (`UpdateModelIndex`).
    fn update_model_index(&self, delta: i32);
}

/// A row that is indented to show nesting (`IIndentedRow`).
pub trait IIndentedRow: IRow {
    fn indent(&self) -> i32;
}

/// A row of an integer-indexed source (`IModelIndexableRow`).
pub trait IModelIndexableRow: IRow {
    /// The index of the model in its parent collection.
    fn model_index(&self) -> i32;
    /// The index of the model from the root collection.
    fn model_index_path(&self) -> IndexPath;
}

/// An element that can expand (`IExpander`).
pub trait IExpander {
    fn is_expanded(&self) -> bool;
    fn set_is_expanded(&self, value: bool);
    /// Whether the expander is shown.
    fn show_expander(&self) -> bool;
}

/// A row that expands to reveal nested data (`IExpanderRow<TModel>`).
pub trait IExpanderRow<M: TreeModel>: IRowOf<M> + IExpander + INotifyPropertyChanged {
    /// Updates whether the expander is shown.
    fn update_show_expander(&self, value: bool);
}

/// A controller notified of the state of expander rows (`IExpanderRowController<TModel>`).
pub trait IExpanderRowController<M: TreeModel> {
    /// Before a row starts expanding or collapsing.
    fn on_begin_expand_collapse(&self, row: &dyn IExpanderRow<M>);
    /// When a row finishes expanding or collapsing (only after a matching begin).
    fn on_end_expand_collapse(&self, row: &dyn IExpanderRow<M>);
    /// When the children of a row change: a change of its child rows (the reference passes the
    /// untyped arguments of the child rows, or a reset).
    fn on_child_collection_changed(&self, row: &dyn IExpanderRow<M>, e: &RowsChangedArgs<'_>);
    /// When the collection of children of a row is replaced (`IChildCollectionReplacementController`).
    fn on_child_collection_replaced(&self, _row: &dyn IExpanderRow<M>, _children: Option<&Models<M>>) {}
    /// Whether the controller replaces child collections (`is IChildCollectionReplacementController`).
    fn replaces_child_collections(&self) -> bool {
        false
    }
}

/// A change of the rows of a source.
pub type RowsChangedArgs<'a> = NotifyCollectionChangedEventArgs<'a, RowRef>;

/// A handler of [`IRows::add_collection_changed`].
pub type RowsChangedHandler = dyn for<'a> Fn(&RowsChangedArgs<'a>);

/// The rows of a source (`IRows`).
pub trait IRows {
    fn count(&self) -> usize;
    fn get(&self, index: usize) -> RowRef;
    /// The index of the row of the model at `model_index`, or -1.
    fn model_index_to_row_index(&self, model_index: &IndexPath) -> i32;
    /// The model index of the row at `row_index` (the empty path for none).
    fn row_index_to_model_index(&self, row_index: i32) -> IndexPath;
    fn add_collection_changed(&self, handler: Rc<RowsChangedHandler>) -> u64;
    fn remove_collection_changed(&self, token: u64);

    /// The rows, in order.
    fn to_vec(&self) -> Vec<RowRef> {
        (0..self.count()).map(|i| self.get(i)).collect()
    }
}

/// The handlers of the change of a list of rows.
#[derive(Default)]
pub struct RowsChangedEvent {
    handlers: RefCell<Vec<(u64, Rc<RowsChangedHandler>)>>,
    next_token: Cell<u64>,
}

impl RowsChangedEvent {
    pub fn add(&self, handler: Rc<RowsChangedHandler>) -> u64 {
        let token = self.next_token.get() + 1;
        self.next_token.set(token);
        self.handlers.borrow_mut().push((token, handler));
        token
    }

    pub fn remove(&self, token: u64) {
        self.handlers.borrow_mut().retain(|(t, _)| *t != token);
    }

    pub fn has_handlers(&self) -> bool {
        !self.handlers.borrow().is_empty()
    }

    pub fn raise(&self, args: &RowsChangedArgs<'_>) {
        let handlers: Vec<Rc<RowsChangedHandler>> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler(args);
        }
    }
}

/// The arguments of the row events of a source (`RowEventArgs<TRow>`).
pub struct RowEventArgs<R> {
    pub row: R,
}

impl<R> RowEventArgs<R> {
    pub fn new(row: R) -> Self {
        Self { row }
    }
}

/// `AnonymousRow<TModel>`: a row of a flat source that is updated and reused for every index, as
/// the rows of a flat grid whose rows are not resized keep no state.
pub struct AnonymousRow<M: TreeModel> {
    model_index: Cell<i32>,
    model: RefCell<Option<Rc<M>>>,
}

impl<M: TreeModel> Default for AnonymousRow<M> {
    fn default() -> Self {
        Self { model_index: Cell::new(0), model: RefCell::new(None) }
    }
}

impl<M: TreeModel> AnonymousRow<M> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// A row of the given model and index.
    pub fn of(model_index: i32, model: Rc<M>) -> Rc<Self> {
        let row = Self::new();
        row.update(model_index, model);
        row
    }

    /// Points the row at another model.
    pub fn update(&self, model_index: i32, model: Rc<M>) {
        self.model_index.set(model_index);
        *self.model.borrow_mut() = Some(model);
    }
}

impl<M: TreeModel> IRow for AnonymousRow<M> {
    fn header(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.model_index.get()) as BoxedValue)
    }

    fn height(&self) -> GridLength {
        GridLength::AUTO
    }

    fn set_height(&self, _value: GridLength) {}

    fn model(&self) -> Option<BoxedValue> {
        self.model.borrow().clone().map(|m| m as BoxedValue)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_model_indexable(&self) -> Option<&dyn IModelIndexableRow> {
        Some(self)
    }
}

impl<M: TreeModel> IRowOf<M> for AnonymousRow<M> {
    fn model_of(&self) -> Rc<M> {
        self.model.borrow().clone().expect("an anonymous row that was updated")
    }

    /// # Panics
    /// Always: an anonymous row has no index of its own (`NotSupportedException`).
    fn update_model_index(&self, _delta: i32) {
        panic!("Specified method is not supported.");
    }
}

impl<M: TreeModel> IModelIndexableRow for AnonymousRow<M> {
    fn model_index(&self) -> i32 {
        self.model_index.get()
    }

    fn model_index_path(&self) -> IndexPath {
        IndexPath::new(self.model_index.get())
    }
}
