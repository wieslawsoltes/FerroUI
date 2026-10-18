//! `Models/TreeDataGrid/ColumnBase\`1.cs`: the state and the layout every view column has: its
//! header, its width and the width it is laid out at (auto, pixel and star widths, constrained by
//! the minimum and maximum widths of its options), its sort direction and its tag.
//!
//! The reference's abstract base class becomes a field: a view column holds a `ColumnBase<M>` in
//! its field `base` and gets its contracts from [`impl_view_column_base!`].

use super::double_utils::{are_close, double_equals};
use super::{ColumnOptions, IColumnOptions};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::Event;
use ferroui_base::BoxedValue;
use ferroui_controls::{GridLength, GridUnitType};
use ferroui_treedatagrid_core::models::{ListSortDirection, NotifyingBase};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ColumnBase<TModel>`: the state of a view column of models of `M`.
pub struct ColumnBase<M: 'static> {
    notifying: NotifyingBase,
    actual_width: Cell<f64>,
    width: Cell<GridLength>,
    auto_width: Cell<f64>,
    star_width: Cell<f64>,
    star_width_was_constrained: Cell<bool>,
    header: RefCell<Option<BoxedValue>>,
    sort_direction: Cell<Option<ListSortDirection>>,
    tag: RefCell<Option<BoxedValue>>,
    options: Rc<dyn IColumnOptions<M>>,
    // What a core column has and the reference's view column does not: an identifier, a
    // visibility and a presentation key.
    id: String,
    is_visible: Cell<bool>,
    presentation_key: RefCell<Option<String>>,
}

impl<M: 'static> ColumnBase<M> {
    /// The state of a column with `header`, `width` (`None`: `Auto`) and `options`.
    pub fn new(header: Option<BoxedValue>, width: Option<GridLength>, options: Rc<dyn IColumnOptions<M>>) -> Self {
        let id = header.as_ref().map(|h| ValueTypes::to_display_string(Some(h))).unwrap_or_default();
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
            id,
            is_visible: Cell::new(true),
            presentation_key: RefCell::new(None),
        };
        this.set_width(width.unwrap_or(GridLength::AUTO));
        this
    }

    /// The `PropertyChanged` of the column.
    pub fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }

    /// The notification helpers of the column.
    pub fn notifying(&self) -> &NotifyingBase {
        &self.notifying
    }

    /// The width of the column after measurement.
    pub fn actual_width(&self) -> f64 {
        self.actual_width.get()
    }

    fn set_actual_width(&self, value: f64) {
        if !double_equals(self.actual_width.get(), value) {
            self.actual_width.set(value);
            self.notifying.raise_property_changed("ActualWidth");
        }
    }

    /// The width of the column; it is set through `IColumns::set_column_width`.
    pub fn width(&self) -> GridLength {
        self.width.get()
    }

    /// The column header.
    pub fn header(&self) -> Option<BoxedValue> {
        self.header.borrow().clone()
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.notifying.raise_and_set_if_changed(&self.header, value, "Header");
    }

    /// The options of the column.
    pub fn options(&self) -> &ColumnOptions<M> {
        self.options.column_options()
    }

    /// The options of the column as they were given.
    pub fn column_options(&self) -> &Rc<dyn IColumnOptions<M>> {
        &self.options
    }

    /// The sort direction indicator shown on the column. Setting it does not sort the data
    /// (`ITreeDataGridSource::sort_by` does).
    pub fn sort_direction(&self) -> Option<ListSortDirection> {
        self.sort_direction.get()
    }

    pub fn set_sort_direction(&self, value: Option<ListSortDirection>) {
        self.notifying.raise_and_set_cell_if_changed(&self.sort_direction, value, "SortDirection");
    }

    /// A user-defined object attached to the column.
    pub fn tag(&self) -> Option<BoxedValue> {
        self.tag.borrow().clone()
    }

    pub fn set_tag(&self, value: Option<BoxedValue>) {
        *self.tag.borrow_mut() = value;
    }

    /// The identifier of the column as a core column: its header's text.
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible.get()
    }

    pub fn set_is_visible(&self, value: bool) {
        self.notifying.raise_and_set_cell_if_changed(&self.is_visible, value, "IsVisible");
    }

    pub fn presentation_key(&self) -> Option<String> {
        self.presentation_key.borrow().clone()
    }

    pub fn set_presentation_key(&self, value: Option<String>) {
        self.notifying.raise_and_set_if_changed(&self.presentation_key, value, "PresentationKey");
    }

    /// `IColumn.CanUserResize`.
    pub fn can_user_resize(&self) -> Option<bool> {
        self.options().can_user_resize_column()
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
        let options = self.options();
        self.width().grid_unit_type() == GridUnitType::Auto
            || options.min_width().grid_unit_type() == GridUnitType::Auto
            || options.max_width().is_some_and(|max| max.grid_unit_type() == GridUnitType::Auto)
    }

    /// `IUpdateColumnLayout.CellMeasured`.
    pub fn cell_measured(&self, width: f64, _row_index: i32) -> f64 {
        self.auto_width.set(f64_max(non_nan(self.auto_width.get()), self.coerce_measured_width(width)));

        if self.width().grid_unit_type() == GridUnitType::Auto {
            return self.auto_width.get();
        }

        if !self.actual_width().is_nan() {
            return self.actual_width();
        }

        // If we're measuring a star column before its actual width has been calculated, return
        // the minimum width so we can continue measuring the remaining columns.
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

        !are_close(old_width, self.actual_width())
    }

    /// `IUpdateColumnLayout.SetWidth`: the preferred width; an absolute width is also the width
    /// the column is laid out at.
    pub fn set_width(&self, width: GridLength) {
        self.width.set(width);

        if width.is_absolute() {
            self.set_actual_width(width.value());
        }
    }

    fn coerce_actual_width(&self, width: f64) -> f64 {
        let options = self.options();
        let auto_width = self.auto_width.get();
        let min_width = options.min_width();
        let width = match min_width.grid_unit_type() {
            GridUnitType::Auto => f64_max(width, auto_width),
            GridUnitType::Pixel => f64_max(width, min_width.value()),
            GridUnitType::Star => panic!("The method or operation is not implemented."),
        };

        match options.max_width() {
            Some(max_width) => match max_width.grid_unit_type() {
                GridUnitType::Auto => f64_min(width, auto_width),
                GridUnitType::Pixel => f64_min(width, max_width.value()),
                GridUnitType::Star => panic!("The method or operation is not implemented."),
            },
            None => width,
        }
    }

    fn coerce_measured_width(&self, width: f64) -> f64 {
        // Auto min/max constraints are derived from the measured width itself. Applying them
        // while the auto width is still NaN creates a circular NaN that can never initialize.
        let options = self.options();
        let min_width = options.min_width();
        let width = match min_width.grid_unit_type() {
            GridUnitType::Auto => width,
            GridUnitType::Pixel => f64_max(width, min_width.value()),
            GridUnitType::Star => panic!("The method or operation is not implemented."),
        };

        match options.max_width() {
            Some(max_width) => match max_width.grid_unit_type() {
                GridUnitType::Auto => width,
                GridUnitType::Pixel => f64_min(width, max_width.value()),
                GridUnitType::Star => panic!("The method or operation is not implemented."),
            },
            None => width,
        }
    }
}

fn non_nan(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v
    }
}

// `Math.Max` and `Math.Min`: a NaN operand gives NaN (Rust's `f64::max` ignores it).
fn f64_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn f64_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// Implements the contracts every view column has on a type with a [`ColumnBase`] (or a type that
/// dereferences to one) in its field `base` and its own `Weak` in its field `weak_self`: the
/// framework's `INotifyPropertyChanged`, the view `IColumn` and `IUpdateColumnLayout`, and the
/// core's `IColumn` (with the column's [`ViewColumnHandle`](super::ViewColumnHandle) as its view).
/// The type implements `ICellColumn` and the core's `IColumnOf<M>` itself.
macro_rules! impl_view_column_base {
    (impl[$($gen:tt)*] $ty:ty, model $model:ty) => {
        impl<$($gen)*> ::ferroui_base::data::model::INotifyPropertyChanged for $ty {
            fn property_changed(&self) -> &::ferroui_base::data::model::Event<str> {
                self.base.property_changed()
            }
        }

        impl<$($gen)*> $crate::models::IColumn for $ty {
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
            fn as_cell_column(&self) -> Option<&dyn $crate::models::ICellColumn> {
                Some(self)
            }
            fn requires_unconstrained_width_measurement(&self) -> bool {
                self.base.requires_unconstrained_width_measurement()
            }
            fn core_column(&self) -> Option<::std::rc::Rc<dyn ::ferroui_treedatagrid_core::models::IColumn>> {
                self.weak_self
                    .upgrade()
                    .map(|column| column as ::std::rc::Rc<dyn ::ferroui_treedatagrid_core::models::IColumn>)
            }
        }

        impl<$($gen)*> $crate::models::IUpdateColumnLayout for $ty {
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

        impl<$($gen)*> ::ferroui_treedatagrid_core::models::IColumn for $ty {
            fn id(&self) -> String {
                self.base.id()
            }
            fn header(&self) -> Option<::ferroui_base::BoxedValue> {
                self.base.header()
            }
            fn width(&self) -> ::ferroui_treedatagrid_core::GridLength {
                $crate::models::to_core_grid_length(self.base.width())
            }
            fn set_width(&self, value: ::ferroui_treedatagrid_core::GridLength) {
                self.base.set_width($crate::models::from_core_grid_length(value))
            }
            fn is_visible(&self) -> bool {
                self.base.is_visible()
            }
            fn set_is_visible(&self, value: bool) {
                self.base.set_is_visible(value)
            }
            fn presentation_key(&self) -> Option<String> {
                self.base.presentation_key()
            }
            fn set_presentation_key(&self, value: Option<String>) {
                self.base.set_presentation_key(value)
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
            fn model_type(&self) -> ::std::any::TypeId {
                ::std::any::TypeId::of::<$model>()
            }
            fn view(&self) -> Option<::std::rc::Rc<dyn ::std::any::Any>> {
                let column = self.weak_self.upgrade()?;
                let view: $crate::models::ColumnRef = column;
                Some(::std::rc::Rc::new($crate::models::ViewColumnHandle(view)) as ::std::rc::Rc<dyn ::std::any::Any>)
            }
        }
    };
}

pub(crate) use impl_view_column_base;
