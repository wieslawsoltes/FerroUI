//! `Models/TreeDataGrid/ColumnBase\`2.cs`: the base of the view columns that select a value from
//! the model (`ColumnBase<TModel, TValue>`): the value selector, the binding of the cells and the
//! comparisons that sort by the value.
//!
//! The reference's getter expression and setter are an [`Accessor`]: its getter selects the value,
//! and the binding of a cell reads, writes and observes the value through it.

use super::{AccessorBinding, ColumnBase, IColumnOptions};
use ferroui_base::BoxedValue;
use ferroui_controls::GridLength;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models::{Comparison, ListSortDirection, ValueComparer};
use ferroui_treedatagrid_core::{Accessor, PropertyPathObserver};
use std::ops::Deref;
use std::rc::Rc;

/// `ColumnBase<TModel, TValue>`.
pub struct ValueColumnBase<M: TreeModel, V: Clone + 'static> {
    base: ColumnBase<M>,
    accessor: Accessor<M, V>,
    observer: Option<Rc<PropertyPathObserver<M>>>,
    can_user_sort: bool,
    sort_ascending: Option<Comparison<M>>,
    sort_descending: Option<Comparison<M>>,
}

impl<M: TreeModel, V: Clone + 'static> ValueColumnBase<M, V> {
    /// The base of a column of the value `accessor` reads, sorted by `comparer` (`None`: the values
    /// do not compare, and the column sorts only with the comparisons of its options).
    pub fn new(
        header: Option<BoxedValue>,
        accessor: Accessor<M, V>,
        comparer: Option<ValueComparer<V>>,
        width: Option<GridLength>,
        options: Rc<dyn IColumnOptions<M>>,
    ) -> Self {
        let base = ColumnBase::new(header, width, options);
        let column_options = base.options();
        let can_user_sort = column_options.can_user_sort_column().unwrap_or(true);
        let (sort_ascending, sort_descending) = if can_user_sort {
            (
                column_options
                    .compare_ascending()
                    .or_else(|| default_sort(&accessor, comparer.clone(), ListSortDirection::Ascending)),
                column_options
                    .compare_descending()
                    .or_else(|| default_sort(&accessor, comparer.clone(), ListSortDirection::Descending)),
            )
        } else {
            (None, None)
        };
        let observer = PropertyPathObserver::create(&accessor);
        Self { base, accessor, observer, can_user_sort, sort_ascending, sort_descending }
    }

    /// The state every column has.
    pub fn column_base(&self) -> &ColumnBase<M> {
        &self.base
    }

    /// The function that selects the value of the column from a model (`ValueSelector`).
    pub fn value_selector(&self) -> &Rc<dyn Fn(&Rc<M>) -> V> {
        self.accessor.getter()
    }

    /// The accessor of the value (`Binding`: its getter, setter and observed path).
    pub fn accessor(&self) -> &Accessor<M, V> {
        &self.accessor
    }

    /// Whether the cells of the column write their values back (`Binding.Write is not null`).
    pub fn can_write(&self) -> bool {
        self.accessor.setter().is_some()
    }

    /// The comparison that sorts the models by the column in `direction`, or `None`.
    pub fn get_comparison(&self, direction: ListSortDirection) -> Option<Comparison<M>> {
        if !self.can_user_sort {
            return None;
        }

        match direction {
            ListSortDirection::Ascending => self.sort_ascending.clone(),
            ListSortDirection::Descending => self.sort_descending.clone(),
        }
    }

    /// The binding of a cell of `model` (`CreateBindingExpression`).
    pub fn create_binding_expression(&self, model: &Rc<M>) -> Rc<AccessorBinding<M, V>> {
        AccessorBinding::with_observer(self.accessor.clone(), self.observer.clone(), Some(model.clone()))
    }
}

impl<M: TreeModel, V: Clone + 'static> Deref for ValueColumnBase<M, V> {
    type Target = ColumnBase<M>;

    fn deref(&self) -> &ColumnBase<M> {
        &self.base
    }
}

// `DefaultSortAscending` and `DefaultSortDescending`: the values of the models compared with the
// default comparer of the value type.
fn default_sort<M: TreeModel, V: 'static>(
    accessor: &Accessor<M, V>,
    comparer: Option<ValueComparer<V>>,
    direction: ListSortDirection,
) -> Option<Comparison<M>> {
    let comparer = comparer?;
    let getter = accessor.getter().clone();
    Some(Rc::new(move |x: &Rc<M>, y: &Rc<M>| {
        let a = getter(x);
        let b = getter(y);
        match direction {
            ListSortDirection::Ascending => comparer(&a, &b),
            ListSortDirection::Descending => comparer(&b, &a),
        }
    }))
}
