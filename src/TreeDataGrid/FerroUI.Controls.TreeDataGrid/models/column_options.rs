//! `Models/TreeDataGrid/ColumnOptions.cs`: the less commonly used options of a view column (the
//! options of the core's columns and the gestures that start editing a cell).
//!
//! The reference's options are a mutable object the column shares with the code that created it;
//! here the fields have interior mutability and the column holds the options in an `Rc`.

use super::{BeginEditGestures, ICellOptions};
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core::models::Comparison;
use std::cell::{Cell, RefCell};

/// `ColumnOptions<TModel>`.
pub struct ColumnOptions<M: 'static> {
    can_user_resize_column: Cell<Option<bool>>,
    can_user_sort_column: Cell<Option<bool>>,
    min_width: Cell<GridLength>,
    max_width: Cell<Option<GridLength>>,
    compare_ascending: RefCell<Option<Comparison<M>>>,
    compare_descending: RefCell<Option<Comparison<M>>>,
    begin_edit_gestures: Cell<BeginEditGestures>,
}

impl<M: 'static> Default for ColumnOptions<M> {
    fn default() -> Self {
        Self {
            can_user_resize_column: Cell::new(None),
            can_user_sort_column: Cell::new(None),
            min_width: Cell::new(GridLength::new(30.0, GridUnitType::Pixel)),
            max_width: Cell::new(None),
            compare_ascending: RefCell::new(None),
            compare_descending: RefCell::new(None),
            begin_edit_gestures: Cell::new(BeginEditGestures::DEFAULT),
        }
    }
}

impl<M: 'static> Clone for ColumnOptions<M> {
    fn clone(&self) -> Self {
        Self {
            can_user_resize_column: Cell::new(self.can_user_resize_column.get()),
            can_user_sort_column: Cell::new(self.can_user_sort_column.get()),
            min_width: Cell::new(self.min_width.get()),
            max_width: Cell::new(self.max_width.get()),
            compare_ascending: RefCell::new(self.compare_ascending.borrow().clone()),
            compare_descending: RefCell::new(self.compare_descending.borrow().clone()),
            begin_edit_gestures: Cell::new(self.begin_edit_gestures.get()),
        }
    }
}

impl<M: 'static> ColumnOptions<M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the user can resize the column by dragging; `None`: the control's
    /// `CanUserResizeColumns` applies.
    pub fn can_user_resize_column(&self) -> Option<bool> {
        self.can_user_resize_column.get()
    }

    pub fn set_can_user_resize_column(&self, value: Option<bool>) {
        self.can_user_resize_column.set(value);
    }

    /// Whether the user can sort the column by clicking; `None`: the control's
    /// `CanUserSortColumns` applies.
    pub fn can_user_sort_column(&self) -> Option<bool> {
        self.can_user_sort_column.get()
    }

    pub fn set_can_user_sort_column(&self, value: Option<bool>) {
        self.can_user_sort_column.set(value);
    }

    /// The minimum width of the column.
    pub fn min_width(&self) -> GridLength {
        self.min_width.get()
    }

    pub fn set_min_width(&self, value: GridLength) {
        self.min_width.set(value);
    }

    /// The maximum width of the column.
    pub fn max_width(&self) -> Option<GridLength> {
        self.max_width.get()
    }

    pub fn set_max_width(&self, value: Option<GridLength>) {
        self.max_width.set(value);
    }

    /// A custom comparison for the ascending order.
    pub fn compare_ascending(&self) -> Option<Comparison<M>> {
        self.compare_ascending.borrow().clone()
    }

    pub fn set_compare_ascending(&self, value: Option<Comparison<M>>) {
        *self.compare_ascending.borrow_mut() = value;
    }

    /// A custom comparison for the descending order.
    pub fn compare_descending(&self) -> Option<Comparison<M>> {
        self.compare_descending.borrow().clone()
    }

    pub fn set_compare_descending(&self, value: Option<Comparison<M>>) {
        *self.compare_descending.borrow_mut() = value;
    }

    /// The gestures that make a cell enter edit mode.
    pub fn begin_edit_gestures(&self) -> BeginEditGestures {
        self.begin_edit_gestures.get()
    }

    pub fn set_begin_edit_gestures(&self, value: BeginEditGestures) {
        self.begin_edit_gestures.set(value);
    }
}

/// Options of a view column: the column options they extend (the reference's options derive from
/// `ColumnOptions<TModel>`).
pub trait IColumnOptions<M: 'static>: 'static {
    fn column_options(&self) -> &ColumnOptions<M>;
}

impl<M: 'static> IColumnOptions<M> for ColumnOptions<M> {
    fn column_options(&self) -> &ColumnOptions<M> {
        self
    }
}

impl<M: 'static> ICellOptions for ColumnOptions<M> {
    fn begin_edit_gestures(&self) -> BeginEditGestures {
        ColumnOptions::begin_edit_gestures(self)
    }
}
