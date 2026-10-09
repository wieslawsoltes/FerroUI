use crate::{ControlImpl, Panel, PanelImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl};
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::Cell;

/// A panel which lays out its children in a grid with all cells having the
/// same size.
#[repr(C)]
pub struct UniformGrid {
    base: Panel,
    rows: Cell<i32>,
    columns: Cell<i32>,
}

ferro_class!(UniformGrid: Panel);
ferroui_base::ferro_class_info!(UniformGrid { new: UniformGrid::new });
ferro_impl_classes!(
    UniformGrid: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferroui_base::ferro_impl_classes!(UniformGrid: FerroObjectImpl);

impl LayoutableImpl for UniformGrid {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        this.update_rows_and_columns();

        let rows = this.rows.get() as f64;
        let columns = this.columns.get() as f64;
        let mut max_width = 0.0;
        let mut max_height = 0.0;

        let child_available_size = Size::new(
            (available_size.width - (columns - 1.0) * this.column_spacing()) / columns,
            (available_size.height - (rows - 1.0) * this.row_spacing()) / rows,
        );

        for child in this.children().snapshot().iter() {
            child.measure(child_available_size);

            if child.desired_size().width > max_width {
                max_width = child.desired_size().width;
            }

            if child.desired_size().height > max_height {
                max_height = child.desired_size().height;
            }
        }

        if this.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(this);
            max_width = LayoutHelper::round_layout_value(max_width, scale);
            max_height = LayoutHelper::round_layout_value(max_height, scale);
        }

        let total_width = max_width * columns + this.column_spacing() * (columns - 1.0);
        let total_height = max_height * rows + this.row_spacing() * (rows - 1.0);

        Size::new(max(total_width, 0.0), max(total_height, 0.0))
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut x = this.first_column();
        let mut y = 0;

        let column_spacing = this.column_spacing();
        let row_spacing = this.row_spacing();
        let rows = this.rows.get();
        let columns = this.columns.get();

        let mut width = max((final_size.width - (columns - 1) as f64 * column_spacing) / columns as f64, 0.0);
        let mut height = max((final_size.height - (rows - 1) as f64 * row_spacing) / rows as f64, 0.0);

        // If layout rounding is enabled, round the per-cell unit size to
        // integral device units.
        if this.use_layout_rounding() {
            let scale = LayoutHelper::get_layout_scale(this);
            width = LayoutHelper::round_layout_value(width, scale);
            height = LayoutHelper::round_layout_value(height, scale);
        }

        for child in this.children().snapshot().iter() {
            if !child.is_visible() {
                continue;
            }

            let rect = Rect::new(x as f64 * (width + column_spacing), y as f64 * (height + row_spacing), width, height);

            child.arrange(rect);

            x += 1;

            if x >= columns {
                x = 0;
                y += 1;
            }
        }

        final_size
    }
}

ferroui_base::ferro_properties! { impl UniformGrid {
    ferro_property!(
        /// Defines the `Rows` property.
        pub fn rows_property() -> StyledProperty<i32> {
            FerroProperty::register::<UniformGrid, _>("Rows", 0)
        }
    );

    ferro_property!(
        /// Defines the `Columns` property.
        pub fn columns_property() -> StyledProperty<i32> {
            FerroProperty::register::<UniformGrid, _>("Columns", 0)
        }
    );

    ferro_property!(
        /// Defines the `FirstColumn` property.
        pub fn first_column_property() -> StyledProperty<i32> {
            FerroProperty::register::<UniformGrid, _>("FirstColumn", 0)
        }
    );

    ferro_property!(
        /// Defines the `RowSpacing` property.
        pub fn row_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<UniformGrid, _>("RowSpacing", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `ColumnSpacing` property.
        pub fn column_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<UniformGrid, _>("ColumnSpacing", 0.0)
        }
    );
} }

impl UniformGrid {
    fn static_constructor() {
        Layoutable::affects_measure::<UniformGrid>(&[
            Self::rows_property().as_property(),
            Self::columns_property().as_property(),
            Self::first_column_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Panel::construct(), rows: Cell::new(0), columns: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The number of rows in each column. A value of zero indicates that the
    /// number of rows is computed.
    pub fn rows(&self) -> i32 {
        self.get_value(Self::rows_property())
    }

    pub fn set_rows(&self, value: i32) {
        self.set_value(Self::rows_property(), value)
    }

    /// The number of columns in each row. A value of zero indicates that the
    /// number of columns is computed.
    pub fn columns(&self) -> i32 {
        self.get_value(Self::columns_property())
    }

    pub fn set_columns(&self, value: i32) {
        self.set_value(Self::columns_property(), value)
    }

    /// The number of columns that are left blank in the first row of the
    /// grid.
    pub fn first_column(&self) -> i32 {
        self.get_value(Self::first_column_property())
    }

    pub fn set_first_column(&self, value: i32) {
        self.set_value(Self::first_column_property(), value)
    }

    /// The spacing between rows.
    pub fn row_spacing(&self) -> f64 {
        self.get_value(Self::row_spacing_property())
    }

    pub fn set_row_spacing(&self, value: f64) {
        self.set_value(Self::row_spacing_property(), value)
    }

    /// The spacing between columns.
    pub fn column_spacing(&self) -> f64 {
        self.get_value(Self::column_spacing_property())
    }

    pub fn set_column_spacing(&self, value: f64) {
        self.set_value(Self::column_spacing_property(), value)
    }

    fn update_rows_and_columns(&self) {
        let mut rows = self.rows();
        let mut columns = self.columns();

        if self.first_column() >= columns {
            self.set_current_value(Self::first_column_property(), 0);
        }

        let mut item_count = self.first_column();

        for child in self.children().snapshot().iter() {
            if child.is_visible() {
                item_count += 1;
            }
        }

        if rows == 0 {
            if columns == 0 {
                rows = (item_count as f64).sqrt().ceil() as i32;
                columns = rows;
            } else {
                rows = item_count / columns;

                if item_count % columns != 0 {
                    rows += 1;
                }
            }
        } else if columns == 0 {
            columns = item_count / rows;

            if item_count % rows != 0 {
                columns += 1;
            }
        }

        self.rows.set(rows);
        self.columns.set(columns);
    }
}
