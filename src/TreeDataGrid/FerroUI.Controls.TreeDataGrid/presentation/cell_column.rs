//! `Presentation/CellColumn.cs`: the view columns that create cells directly from core rows,
//! without a row wrapper, for the neutral columns of a core source.
//!
//! The reference's `ICellColumn<TModel>` is the contract's [`ICellColumn`]. For a neutral text or
//! check box column its factory makes the classic typed columns (`TextColumn<TModel, TValue>`,
//! `CheckBoxColumn<TModel>`) over the core column's getter; the port's presentation holds models as
//! untyped values, so these are the untyped forms of those columns: [`CoreTextColumn`] and
//! [`CoreCheckBoxColumn`], whose cells (the crate's `TextCell` and `CheckBoxCell`) read and write
//! the value through the core's erased access to the column ([`CoreCellBinding`]).

use super::cell_binding::{check_box_conversion, untyped_conversion, CoreCellBinding};
use super::cell_column_base::impl_cell_column_base;
use super::core_conversions::to_view_length;
use super::{CellColumnBase, CellColumnOptions};
use crate::models::{CellRef, CheckBoxCell, ICellBinding, ICellColumn, ITextCellCultureOptions, TextCell, TextColumnOptions};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::models::{IColumn as CoreColumn, RowRef};
use std::rc::Rc;

/// The layout options of the view column of a neutral core column (`CopyOptions`): resizing and
/// the width limits of the core column. Sorting belongs to the core; these columns contain only
/// view binding and layout state.
pub(crate) fn copy_options(column: &dyn CoreColumn) -> CellColumnOptions {
    let mut options = CellColumnOptions::default();
    if let Some(value) = column.untyped_value() {
        let (can_user_resize, _can_user_sort, min_width, max_width) = value.layout_options();
        options.can_user_resize_column = can_user_resize;
        options.min_width = to_view_length(min_width);
        options.max_width = max_width.map(to_view_length);
    }
    options
}

/// The text a text column searches (`ITextSearchableColumn.SelectValue`: `ValueSelector(model)?.ToString()`).
fn select_text(column: &dyn CoreColumn, model: &BoxedValue) -> Option<String> {
    let value = column.untyped_value()?.get_value(model)?;
    Some(ValueTypes::to_display_string(Some(&value)))
}

/// The view column of a neutral text column (the reference's `TextColumn<TModel, TValue>` made by
/// `CellColumnFactory`): its cells show the value as text and write edits back.
pub struct CoreTextColumn {
    base: CellColumnBase,
    model: Rc<dyn CoreColumn>,
    text_options: Rc<TextColumnOptions<BoxedValue>>,
}

impl CoreTextColumn {
    /// The view column of `model`, a value column of the core.
    ///
    /// # Panics
    /// When `model` is not a value column (`untyped_value`).
    pub fn new(model: Rc<dyn CoreColumn>) -> Rc<Self> {
        assert!(model.untyped_value().is_some(), "The column is not a value column.");
        let options = copy_options(&*model);
        let text_options = TextColumnOptions::<BoxedValue>::new();
        text_options.set_can_user_resize_column(options.can_user_resize_column);
        text_options.set_can_user_sort_column(Some(false));
        text_options.set_min_width(options.min_width);
        text_options.set_max_width(options.max_width);
        Rc::new(Self {
            base: CellColumnBase::new(model.header(), Some(to_view_length(model.width())), options),
            model,
            text_options: Rc::new(text_options),
        })
    }

    /// The core column presented.
    pub fn model(&self) -> &Rc<dyn CoreColumn> {
        &self.model
    }

    /// The options of the column's cells.
    pub fn options(&self) -> &Rc<TextColumnOptions<BoxedValue>> {
        &self.text_options
    }

    fn is_read_only(&self) -> bool {
        !self.model.untyped_value().is_some_and(|value| value.can_set())
    }
}

impl_cell_column_base!(CoreTextColumn {
    fn as_cell_column(&self) -> Option<&dyn crate::models::ICellColumn> {
        Some(self)
    }

    fn core_column(&self) -> Option<Rc<dyn ferroui_treedatagrid_core::models::IColumn>> {
        Some(self.model.clone())
    }
});

impl ICellColumn for CoreTextColumn {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        let binding = CoreCellBinding::new(self.model.clone(), untyped_conversion(), row.model());
        let options: Rc<dyn ITextCellCultureOptions> = self.text_options.clone();
        TextCell::<Option<BoxedValue>>::new(binding as Rc<dyn ICellBinding<Option<BoxedValue>>>, self.is_read_only(), Some(options))
            as CellRef
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        cell.as_any()
            .downcast_ref::<TextCell<Option<BoxedValue>>>()
            .is_some_and(|cell| cell.try_set_source(row.model().as_ref()))
    }

    fn select_value(&self, model: &BoxedValue) -> Option<String> {
        select_text(&*self.model, model)
    }

    fn is_text_search_enabled(&self) -> bool {
        self.text_options.is_text_search_enabled()
    }
}

/// The view column of a neutral check box column (the reference's `CheckBoxColumn<TModel>` made by
/// `CellColumnFactory`).
pub struct CoreCheckBoxColumn {
    base: CellColumnBase,
    model: Rc<dyn CoreColumn>,
    is_three_state: bool,
}

impl CoreCheckBoxColumn {
    /// The view column of `model`, a check box column of the core.
    ///
    /// # Panics
    /// When `model` is not a value column (`untyped_value`).
    pub fn new(model: Rc<dyn CoreColumn>) -> Rc<Self> {
        let is_three_state = model.untyped_value().expect("The column is not a value column.").is_three_state();
        let options = copy_options(&*model);
        Rc::new(Self {
            base: CellColumnBase::new(model.header(), Some(to_view_length(model.width())), options),
            model,
            is_three_state,
        })
    }

    /// The core column presented.
    pub fn model(&self) -> &Rc<dyn CoreColumn> {
        &self.model
    }

    /// Whether the check boxes have three states (`IsThreeState`).
    pub fn is_three_state(&self) -> bool {
        self.is_three_state
    }

    fn is_read_only(&self) -> bool {
        !self.model.untyped_value().is_some_and(|value| value.can_set())
    }
}

impl_cell_column_base!(CoreCheckBoxColumn {
    fn as_cell_column(&self) -> Option<&dyn crate::models::ICellColumn> {
        Some(self)
    }

    fn core_column(&self) -> Option<Rc<dyn ferroui_treedatagrid_core::models::IColumn>> {
        Some(self.model.clone())
    }
});

impl ICellColumn for CoreCheckBoxColumn {
    fn create_cell(&self, row: &RowRef) -> CellRef {
        let binding = CoreCellBinding::new(self.model.clone(), check_box_conversion(), row.model());
        CheckBoxCell::new(binding as Rc<dyn ICellBinding<Option<bool>>>, self.is_read_only(), self.is_three_state) as CellRef
    }

    fn try_reuse_cell(&self, cell: &CellRef, row: &RowRef) -> bool {
        cell.as_any().downcast_ref::<CheckBoxCell>().is_some_and(|cell| cell.try_set_source(row.model().as_ref()))
    }
}
