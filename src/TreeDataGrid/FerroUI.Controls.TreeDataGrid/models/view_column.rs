//! The view columns of models of a type (`IColumn<TModel>` of the control) and the handle a view
//! column gives the core.
//!
//! Not a file of the reference: the reference's classic sources hold the view columns directly,
//! while the port's sources are the core's, which hold core columns. A view column is both: it
//! lays itself out and creates its cells (the view contracts of this module), and it is a core
//! column of models of `M` whose `IColumn::view` is a [`ViewColumnHandle`], which the
//! presentation downcasts to present the column itself.

use super::{ColumnRef, IColumn, ICellColumn, IUpdateColumnLayout};
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::models as core_models;
use std::rc::Rc;

/// The view of a core column that presents itself: the answer of a view column to the core's
/// `IColumn::view`.
pub struct ViewColumnHandle(pub ColumnRef);

/// The view column of a core column, if it is a view column of this crate (the core column's
/// `view()` is a [`ViewColumnHandle`]).
pub fn view_column_of(column: &dyn core_models::IColumn) -> Option<ColumnRef> {
    let view = column.view()?;
    view.downcast::<ViewColumnHandle>().ok().map(|handle| handle.0.clone())
}

/// A view column of models of `M` (`IColumn<TModel>` of the control): a view column that lays
/// itself out and creates its cells, and a core column of models of `M`.
///
/// The view and the core contracts name some members alike (`header`, `width`, `sort_direction`,
/// `tag`, `as_any`); a caller names the contract it means (`IColumn::header(&*column)`).
pub trait IViewColumn<M: TreeModel>: IColumn + IUpdateColumnLayout + ICellColumn + core_models::IColumnOf<M> {}

impl<M: TreeModel, T: IColumn + IUpdateColumnLayout + ICellColumn + core_models::IColumnOf<M> + ?Sized> IViewColumn<M> for T {}

/// A view column of models of `M` whose cells show an expander (`IExpanderColumn<TModel>` of the
/// control).
pub trait IViewExpanderColumn<M: TreeModel>: IViewColumn<M> + core_models::IExpanderColumn<M> {}

impl<M: TreeModel, T: IViewColumn<M> + core_models::IExpanderColumn<M> + ?Sized> IViewExpanderColumn<M> for T {}

/// The untyped form of a model: the model itself for a model that is already an untyped value
/// (the models of the declarative columns), the model boxed otherwise.
pub(crate) fn boxed_model<M: TreeModel>(model: &Rc<M>) -> ferroui_base::BoxedValue {
    let any: &dyn std::any::Any = &**model;
    match any.downcast_ref::<ferroui_base::BoxedValue>() {
        Some(inner) => inner.clone(),
        None => model.clone() as ferroui_base::BoxedValue,
    }
}

/// An untyped model with the box of a model that is already an untyped value removed (a row's
/// model of the declarative columns is a boxed `BoxedValue`).
pub(crate) fn unwrap_boxed_model(value: ferroui_base::BoxedValue) -> ferroui_base::BoxedValue {
    match value.downcast_ref::<ferroui_base::BoxedValue>() {
        Some(inner) => inner.clone(),
        None => value,
    }
}

/// The model of `row` as a model of `M` (the reference's `IRow<TModel>.Model`).
///
/// # Panics
/// When the row has no model or its model is not of type `M` (the reference's invalid cast).
pub(crate) fn row_model<M: 'static>(row: &core_models::RowRef) -> Rc<M> {
    let boxed = row.model().expect("a row with a model");
    let any: Rc<dyn std::any::Any> = boxed;
    match any.downcast::<M>() {
        Ok(model) => model,
        Err(_) => panic!("Unable to cast the model of the row to {}.", std::any::type_name::<M>()),
    }
}
