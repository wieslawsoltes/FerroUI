//! `Presentation/CellColumnBase.cs`: the layout state of a presented column.
//!
//! Layout algorithm derived from TreeDataGrid `ColumnBase<TModel>` (MIT), Copyright (c) .NET
//! Foundation and Contributors. This base owns view layout only; sorting and row identity belong
//! to the core.
//!
//! The reference's abstract class becomes a field: a presented column holds a `CellColumnBase` and
//! implements the view contracts ([`IColumn`](crate::models::IColumn),
//! [`IUpdateColumnLayout`](crate::models::IUpdateColumnLayout)) by delegating to it, with
//! [`impl_cell_column_base!`](crate::impl_cell_column_base).

use super::CellColumnOptions;
use ferroui_base::BoxedValue;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core::models::{ListSortDirection, NotifyingBase};
use std::cell::{Cell, RefCell};

/// `MathUtilities.AreClose` of the reference (`Utils/DoubleUtils.cs`).
pub(crate) fn are_close(value1: f64, value2: f64) -> bool {
    // In case they are infinities (then epsilon check does not work).
    if value1 == value2 {
        return true;
    }
    const DOUBLE_EPSILON: f64 = 2.220_446_049_250_313e-16;
    let eps = (value1.abs() + value2.abs() + 10.0) * DOUBLE_EPSILON;
    let delta = value1 - value2;
    -eps < delta && eps > delta
}

/// `DoubleUtils.IsZero`.
pub(crate) fn is_zero(value: f64) -> bool {
    const DOUBLE_EPSILON: f64 = 2.220_446_049_250_313e-16;
    value.abs() < 10.0 * DOUBLE_EPSILON
}

fn non_nan(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v
    }
}

/// The header, width, sort direction and layout state of a presented column (`CellColumnBase<TModel>`).
pub struct CellColumnBase {
    notifying: NotifyingBase,
    actual_width: Cell<f64>,
    width: Cell<GridLength>,
    auto_width: Cell<f64>,
    star_width: Cell<f64>,
    star_width_was_constrained: Cell<bool>,
    header: RefCell<Option<BoxedValue>>,
    sort_direction: Cell<Option<ListSortDirection>>,
    tag: RefCell<Option<BoxedValue>>,
    options: CellColumnOptions,
}

impl CellColumnBase {
    pub fn new(header: Option<BoxedValue>, width: Option<GridLength>, options: CellColumnOptions) -> Self {
        let this = Self {
            notifying: NotifyingBase::new(),
            actual_width: Cell::new(f64::NAN),
            width: Cell::new(GridLength::AUTO),
            auto_width: Cell::new(f64::NAN),
            star_width: Cell::new(f64::NAN),
            star_width_was_constrained: Cell::new(false),
            header: RefCell::new(header),
            sort_direction: Cell::new(None),
            tag: RefCell::new(None),
            options,
        };
        this.set_width(width.unwrap_or(GridLength::AUTO));
        this
    }

    /// The `PropertyChanged` of the column.
    pub fn notifying(&self) -> &NotifyingBase {
        &self.notifying
    }

    pub fn options(&self) -> &CellColumnOptions {
        &self.options
    }

    /// The width the column is laid out at.
    pub fn actual_width(&self) -> f64 {
        self.actual_width.get()
    }

    fn set_actual_width(&self, value: f64) {
        // `RaiseAndSetIfChanged` compares with `EqualityComparer<double>`: NaN equals NaN.
        let old = self.actual_width.get();
        if old == value || (old.is_nan() && value.is_nan()) {
            return;
        }
        self.actual_width.set(value);
        self.notifying.raise_property_changed("ActualWidth");
    }

    pub fn width(&self) -> GridLength {
        self.width.get()
    }

    fn set_width_property(&self, value: GridLength) {
        self.notifying.raise_and_set_cell_if_changed(&self.width, value, "Width");
    }

    pub fn header(&self) -> Option<BoxedValue> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        let same = match (&*self.header.borrow(), &value) {
            (Some(a), Some(b)) => **a == **b,
            (None, None) => true,
            _ => false,
        };
        if !same {
            *self.header.borrow_mut() = value;
            self.notifying.raise_property_changed("Header");
        }
    }

    pub fn sort_direction(&self) -> Option<ListSortDirection> {
        self.sort_direction.get()
    }

    pub fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.notifying.raise_and_set_cell_if_changed(&self.sort_direction, value, "SortDirection");
    }

    pub fn tag(&self) -> Option<BoxedValue> {
        self.tag.borrow().clone()
    }

    pub fn set_tag(&self, value: Option<BoxedValue>) {
        *self.tag.borrow_mut() = value;
    }

    /// `IColumn.CanUserResize`.
    pub fn can_user_resize(&self) -> Option<bool> {
        self.options.can_user_resize_column
    }

    /// `IUpdateColumnLayout.MinActualWidth`.
    pub fn min_actual_width(&self) -> f64 {
        self.coerce_actual_width(0.0)
    }

    /// `IUpdateColumnLayout.MaxActualWidth`.
    pub fn max_actual_width(&self) -> f64 {
        self.coerce_actual_width(f64::INFINITY)
    }

    /// `IUpdateColumnLayout.StarWidthWasConstrained`.
    pub fn star_width_was_constrained(&self) -> bool {
        self.star_width_was_constrained.get()
    }

    /// `IColumnMeasurementOptions.RequiresUnconstrainedWidthMeasurement`.
    pub fn requires_unconstrained_width_measurement(&self) -> bool {
        self.width().is_auto() || self.options.min_width.is_auto() || self.options.max_width.is_some_and(|w| w.is_auto())
    }

    /// `IUpdateColumnLayout.CellMeasured`.
    pub fn cell_measured(&self, width: f64, _row_index: i32) -> f64 {
        self.auto_width.set(non_nan(self.auto_width.get()).max(self.coerce_measured_width(width)));

        if self.width().grid_unit_type() == GridUnitType::Auto {
            return self.auto_width.get();
        }

        if !self.actual_width().is_nan() {
            return self.actual_width();
        }

        // If we're measuring a star column before its actual width has been calculated,
        // return the minimum width so we can continue measuring the remaining columns.
        if self.width().is_star() {
            return self.min_actual_width();
        }

        self.auto_width.get()
    }

    /// `IUpdateColumnLayout.CalculateStarWidth`.
    ///
    /// # Panics
    /// When the column is not a star column (`InvalidOperationException`).
    pub fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
        if !self.width().is_star() {
            panic!("Attempt to calculate star width on a non-star column.");
        }

        let width = (available_width / total_stars) * self.width().value();
        let star_width = self.coerce_actual_width(width);
        self.star_width.set(star_width);
        self.star_width_was_constrained.set(!are_close(star_width, width));
    }

    /// `IUpdateColumnLayout.CommitActualWidth`.
    pub fn commit_actual_width(&self) -> bool {
        let width = match self.width().grid_unit_type() {
            GridUnitType::Auto => {
                if self.auto_width.get().is_nan() {
                    self.coerce_actual_width(0.0)
                } else {
                    self.auto_width.get()
                }
            }
            GridUnitType::Pixel => self.coerce_actual_width(self.width().value()),
            GridUnitType::Star => self.star_width.get(),
        };

        let old_width = self.actual_width();
        self.set_actual_width(width);
        self.star_width_was_constrained.set(false);

        // `AreClose` would report true for this condition. If the user has auto columns that
        // are not yet realized, then the auto width will remain NaN. This would lead to an
        // endless layout cycle causing the whole UI to have degraded performance, until all
        // columns have an actual value set for the auto width.
        if old_width.is_nan() && self.actual_width().is_nan() {
            return false;
        }

        !are_close(old_width, self.actual_width())
    }

    /// `IUpdateColumnLayout.SetWidth`.
    pub fn set_width(&self, width: GridLength) {
        self.set_width_property(width);

        if width.is_absolute() {
            self.set_actual_width(width.value());
        }
    }

    fn coerce_actual_width(&self, width: f64) -> f64 {
        let width = match self.options.min_width.grid_unit_type() {
            GridUnitType::Auto => width.max(self.auto_width.get()),
            GridUnitType::Pixel => width.max(self.options.min_width.value()),
            GridUnitType::Star => panic!("The method or operation is not implemented."),
        };

        match self.options.max_width.map(|w| w.grid_unit_type()) {
            Some(GridUnitType::Auto) => width.min(self.auto_width.get()),
            Some(GridUnitType::Pixel) => width.min(self.options.max_width.map_or(f64::INFINITY, |w| w.value())),
            Some(GridUnitType::Star) => panic!("The method or operation is not implemented."),
            None => width,
        }
    }

    fn coerce_measured_width(&self, width: f64) -> f64 {
        // Auto min/max constraints are derived from the measured width itself. Applying them
        // while the auto width is still NaN creates a circular NaN that can never initialize.
        let width = match self.options.min_width.grid_unit_type() {
            GridUnitType::Auto => width,
            GridUnitType::Pixel => width.max(self.options.min_width.value()),
            GridUnitType::Star => panic!("The method or operation is not implemented."),
        };

        match self.options.max_width.map(|w| w.grid_unit_type()) {
            Some(GridUnitType::Auto) => width,
            Some(GridUnitType::Pixel) => width.min(self.options.max_width.map_or(f64::INFINITY, |w| w.value())),
            Some(GridUnitType::Star) => panic!("The method or operation is not implemented."),
            None => width,
        }
    }
}

/// Implements [`INotifyPropertyChanged`](ferroui_base::data::model::INotifyPropertyChanged), the
/// layout members of [`IColumn`](crate::models::IColumn) and
/// [`IUpdateColumnLayout`](crate::models::IUpdateColumnLayout) for a type that holds a
/// [`CellColumnBase`] in the field `base`. The type implements the rest of `IColumn` (`as_any`,
/// `as_cell_column`, `core_column`) inside the `IColumn` block it passes.
macro_rules! impl_cell_column_base {
    ($ty:ty { $($column_items:tt)* }) => {
        impl ::ferroui_base::data::model::INotifyPropertyChanged for $ty {
            fn property_changed(&self) -> &::ferroui_base::data::model::Event<str> {
                self.base.notifying().property_changed()
            }
        }

        impl $crate::models::IColumn for $ty {
            fn actual_width(&self) -> f64 {
                self.base.actual_width()
            }
            fn can_user_resize(&self) -> Option<bool> {
                self.base.can_user_resize()
            }
            fn header(&self) -> Option<::ferroui_base::BoxedValue> {
                self.base.header()
            }
            fn width(&self) -> ::ferroui_controls::GridLength {
                self.base.width()
            }
            fn sort_direction(&self) -> Option<::ferroui_treedatagrid_core::models::ListSortDirection> {
                self.base.sort_direction()
            }
            fn set_sort_direction(&self, value: Option<::ferroui_treedatagrid_core::models::ListSortDirection>) {
                self.base.set_sort_direction(value)
            }
            fn tag(&self) -> Option<::ferroui_base::BoxedValue> {
                self.base.tag()
            }
            fn set_tag(&self, value: Option<::ferroui_base::BoxedValue>) {
                self.base.set_tag(value)
            }
            fn as_any(&self) -> &dyn ::std::any::Any {
                self
            }
            fn as_update_column_layout(&self) -> Option<&dyn $crate::models::IUpdateColumnLayout> {
                Some(self)
            }
            fn requires_unconstrained_width_measurement(&self) -> bool {
                self.base.requires_unconstrained_width_measurement()
            }
            $($column_items)*
        }

        impl $crate::models::IUpdateColumnLayout for $ty {
            fn min_actual_width(&self) -> f64 {
                self.base.min_actual_width()
            }
            fn max_actual_width(&self) -> f64 {
                self.base.max_actual_width()
            }
            fn star_width_was_constrained(&self) -> bool {
                self.base.star_width_was_constrained()
            }
            fn cell_measured(&self, width: f64, row_index: i32) -> f64 {
                self.base.cell_measured(width, row_index)
            }
            fn calculate_star_width(&self, available_width: f64, total_stars: f64) {
                self.base.calculate_star_width(available_width, total_stars)
            }
            fn commit_actual_width(&self) -> bool {
                self.base.commit_actual_width()
            }
            fn set_width(&self, width: ::ferroui_controls::GridLength) {
                self.base.set_width(width)
            }
        }
    };
}

pub(crate) use impl_cell_column_base;
