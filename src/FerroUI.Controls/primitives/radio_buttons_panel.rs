//! The items panel of WinUI's `RadioButtons`: a structural port of
//! `ColumnMajorUniformToLargestGridLayout` (`controls/dev/RadioButtons/
//! ColumnMajorUniformToLargestGridLayout.cpp` of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::{ControlImpl, Panel, PanelImpl, RadioButtons};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Rect, Ref, Size,
    StyledElementImpl, StyledProperty, StyledPropertyOptions, VisualImpl,
};
use std::cell::Cell;

/// Lays out the radio buttons of a [`RadioButtons`] in columns of equal cells as large as the
/// largest one, filling each column from the top, with as many columns as the available width
/// and the `MaxColumns` of the group allow (WinUI's `ColumnMajorUniformToLargestGridLayout`).
///
/// Deviation (docs/porting/winui-controls.md, Input): WinUI lays the buttons out with a layout
/// of an `ItemsRepeater`; FerroUI's `RadioButtons` is an items control, and this panel is its
/// items panel, set by the control theme of the WinUI theme with the spacing of the resources
/// `RadioButtonsColumnSpacing` and `RadioButtonsRowSpacing`. `MaxColumns` is read from the group
/// the panel is the items panel of (its templated parent); a panel of another control has one
/// column.
#[repr(C)]
pub struct RadioButtonsPanel {
    base: Panel,
    largest_child_size: Cell<Size>,
    actual_column_count: Cell<usize>,
}

ferro_class!(RadioButtonsPanel: Panel);
ferroui_base::ferro_class_info!(RadioButtonsPanel { new: RadioButtonsPanel::new });
ferro_impl_classes!(RadioButtonsPanel: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);
ferroui_base::ferro_impl_classes!(RadioButtonsPanel: FerroObjectImpl);

impl LayoutableImpl for RadioButtonsPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let children = this.children().snapshot();
        if children.is_empty() {
            this.actual_column_count.set(1);
            return Size::new(0.0, 0.0);
        }
        let mut largest = Size::new(0.0, 0.0);
        for child in children.iter() {
            child.measure(available_size);
            let desired = child.desired_size();
            largest.width = largest.width.max(desired.width);
            largest.height = largest.height.max(desired.height);
        }
        this.largest_child_size.set(largest);
        let columns = this.calculate_columns(children.len(), largest.width, available_size.width);
        this.actual_column_count.set(columns);
        let max_items_per_column = children.len().div_ceil(columns);
        Size::new(
            largest.width * columns as f64 + this.column_spacing() * (columns - 1) as f64,
            largest.height * max_items_per_column as f64 + this.row_spacing() * (max_items_per_column - 1) as f64,
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        let columns = this.actual_column_count.get().max(1);
        let item_count = children.len();
        let min_items_per_column = item_count / columns;
        let number_of_columns_with_extra_elements = item_count % columns;
        let largest = this.largest_child_size.get();
        let (column_spacing, row_spacing) = (this.column_spacing(), this.row_spacing());
        let mut index = 0;
        for column in 0..columns {
            let rows = min_items_per_column + usize::from(column < number_of_columns_with_extra_elements);
            for row in 0..rows {
                let Some(child) = children.get(index) else { return final_size };
                let desired = child.desired_size();
                child.arrange(Rect::new(
                    column as f64 * (largest.width + column_spacing),
                    row as f64 * (largest.height + row_spacing),
                    desired.width,
                    desired.height,
                ));
                index += 1;
            }
        }
        final_size
    }
}

ferroui_base::ferro_properties! { impl RadioButtonsPanel {
    ferro_property!(
        /// Defines the `ColumnSpacing` property: the space between the columns. 0 by default; the
        /// WinUI theme sets `RadioButtonsColumnSpacing` (7).
        pub fn column_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<RadioButtonsPanel, _>(
                "ColumnSpacing",
                StyledPropertyOptions::new(0.0).coerce(|_, value: f64| if value.is_finite() { value } else { 0.0 }),
            )
        }
    );

    ferro_property!(
        /// Defines the `RowSpacing` property: the space between the rows. 0 by default; the WinUI
        /// theme sets `RadioButtonsRowSpacing` (8).
        pub fn row_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<RadioButtonsPanel, _>(
                "RowSpacing",
                StyledPropertyOptions::new(0.0).coerce(|_, value: f64| if value.is_finite() { value } else { 0.0 }),
            )
        }
    );
} }

impl RadioButtonsPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<RadioButtonsPanel>(&[
            Self::column_spacing_property().as_property(),
            Self::row_spacing_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct(), largest_child_size: Cell::new(Size::new(0.0, 0.0)), actual_column_count: Cell::new(1) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the space between the columns.
    pub fn column_spacing(&self) -> f64 {
        self.get_value(Self::column_spacing_property())
    }

    /// Sets the space between the columns.
    pub fn set_column_spacing(&self, value: f64) {
        self.set_value(Self::column_spacing_property(), value);
    }

    /// Gets the space between the rows.
    pub fn row_spacing(&self) -> f64 {
        self.get_value(Self::row_spacing_property())
    }

    /// Sets the space between the rows.
    pub fn set_row_spacing(&self, value: f64) {
        self.set_value(Self::row_spacing_property(), value);
    }

    /// The `MaxColumns` of the group the panel lays out.
    fn max_columns(&self) -> usize {
        self.templated_parent()
            .and_then(|parent| parent.cast::<RadioButtons>())
            .map_or(1, |owner| owner.max_columns().max(1) as usize)
    }

    /// The number of columns (ColumnMajorUniformToLargestGridLayout::CalculateColumns): every
    /// column after the first takes the spacing and the width of the largest item.
    fn calculate_columns(&self, child_count: usize, max_item_width: f64, available_width: f64) -> usize {
        let width_needed_for_extra_column = self.column_spacing() + max_item_width;
        // The number of columns from data and api ignoring available space.
        let requested_column_count = self.max_columns().min(child_count);
        // If columns can be added with effectively 0 extra space return as many columns as needed.
        if width_needed_for_extra_column < f64::from(f32::EPSILON) {
            return requested_column_count.max(1);
        }
        let extra_width_after_first_column = available_width - max_item_width;
        let max_extra_columns = (extra_width_after_first_column / width_needed_for_extra_column).floor().max(0.0);
        // The smaller of the number of columns from data and api and the number of columns the
        // available space can support; 1 even if there isn't any data.
        let effective_column_count = (requested_column_count as f64).min(max_extra_columns + 1.0);
        (effective_column_count as usize).max(1)
    }
}
