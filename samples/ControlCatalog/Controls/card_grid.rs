//! Port of `Controls/CardGrid.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl};
use std::cell::{Cell, RefCell};

/// A panel that lays out its children in equal-width columns, choosing the
/// column count from the available width and `MinItemWidth`. Rows are as
/// tall as their tallest child. Used for every card grid in the catalog so
/// the home page and the sample galleries reflow identically.
#[repr(C)]
pub struct CardGrid {
    base: Panel,
    columns: Cell<i32>,
    row_heights: RefCell<Vec<f64>>,
}

ferro_class!(CardGrid: Panel);
ferro_class_info!(CardGrid { new: CardGrid::new });
ferro_impl_classes!(
    CardGrid: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferro_properties! {
    impl CardGrid {
        pub fn min_item_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<CardGrid, _>("MinItemWidth", 248.0)
        }

        pub fn column_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<CardGrid, _>("ColumnSpacing", 12.0)
        }

        pub fn row_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<CardGrid, _>("RowSpacing", 12.0)
        }
    }
}

impl LayoutableImpl for CardGrid {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let count = this.visible_count();
        let column_spacing = this.column_spacing().max(0.0);
        let row_spacing = this.row_spacing().max(0.0);

        let columns = this.compute_columns(available_size.width, count);
        this.columns.set(columns);

        let width = if available_size.width.is_infinite() {
            f64::from(columns) * this.min_item_width().max(1.0) + column_spacing * f64::from(columns - 1)
        } else {
            available_size.width
        };

        let column_width = ((width - column_spacing * f64::from(columns - 1)) / f64::from(columns)).max(0.0);

        let rows = if count == 0 { 0 } else { (count + columns - 1) / columns };
        let mut row_heights = vec![0.0_f64; rows as usize];

        let mut index = 0;
        for child in this.children().snapshot().iter() {
            if !child.is_visible() {
                continue;
            }

            child.measure(Size::new(column_width, f64::INFINITY));
            let row = (index / columns) as usize;
            row_heights[row] = row_heights[row].max(child.desired_size().height);
            index += 1;
        }

        let mut height = 0.0;
        for row_height in &row_heights {
            height += row_height;
        }

        if rows > 1 {
            height += row_spacing * f64::from(rows - 1);
        }

        *this.row_heights.borrow_mut() = row_heights;

        Size::new(width, height)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let column_spacing = this.column_spacing().max(0.0);
        let row_spacing = this.row_spacing().max(0.0);
        let columns = this.columns.get();
        let column_width = if columns == 0 {
            final_size.width
        } else {
            ((final_size.width - column_spacing * f64::from(columns - 1)) / f64::from(columns)).max(0.0)
        };
        let row_heights = this.row_heights.borrow().clone();

        let mut index = 0;
        let mut y = 0.0;
        for child in this.children().snapshot().iter() {
            if !child.is_visible() {
                continue;
            }

            let row = (index / columns) as usize;
            let column = index % columns;

            if column == 0 && index > 0 {
                y += row_heights[row - 1] + row_spacing;
            }

            let x = f64::from(column) * (column_width + column_spacing);
            let row_height = if row < row_heights.len() { row_heights[row] } else { child.desired_size().height };
            child.arrange(Rect::new(x, y, column_width, row_height));
            index += 1;
        }

        final_size
    }
}

impl CardGrid {
    fn static_constructor() {
        Layoutable::affects_measure::<CardGrid>(&[
            Self::min_item_width_property().as_property(),
            Self::column_spacing_property().as_property(),
            Self::row_spacing_property().as_property(),
        ]);
    }

    pub fn construct() -> Self {
        Self { base: Panel::construct(), columns: Cell::new(1), row_heights: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The narrowest a column may be. The column count is the largest
    /// number of columns that keeps every column at least this wide.
    pub fn min_item_width(&self) -> f64 {
        self.get_value(Self::min_item_width_property())
    }

    pub fn set_min_item_width(&self, value: f64) {
        self.set_value(Self::min_item_width_property(), value)
    }

    pub fn column_spacing(&self) -> f64 {
        self.get_value(Self::column_spacing_property())
    }

    pub fn set_column_spacing(&self, value: f64) {
        self.set_value(Self::column_spacing_property(), value)
    }

    pub fn row_spacing(&self) -> f64 {
        self.get_value(Self::row_spacing_property())
    }

    pub fn set_row_spacing(&self, value: f64) {
        self.set_value(Self::row_spacing_property(), value)
    }

    fn visible_count(&self) -> i32 {
        let mut count = 0;
        for child in self.children().snapshot().iter() {
            if child.is_visible() {
                count += 1;
            }
        }
        count
    }

    fn compute_columns(&self, available_width: f64, count: i32) -> i32 {
        if count == 0 {
            return 1;
        }

        let min_width = self.min_item_width().max(1.0);
        let spacing = self.column_spacing().max(0.0);

        let columns = if available_width.is_infinite() {
            count.max(1)
        } else {
            // The saturating conversion of `as` stands for the unchecked cast of the original.
            ((available_width + spacing) / (min_width + spacing)).floor() as i32
        };

        // Column width depends only on the available width, not on how many children there are, so a
        // group with one card gets the same card width as a group with six.
        columns.max(1)
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_base::layout::Layoutable;
    use ferroui_controls::Border;

    fn card(height: f64) -> Ref<Border> {
        let border = Border::new();
        border.set_min_height(height);
        border
    }

    #[test]
    fn columns_follow_the_available_width() {
        let grid = CardGrid::new();
        for height in [10.0, 30.0, 20.0, 40.0, 5.0] {
            grid.children().add(card(height));
        }

        // (520 + 12) / (248 + 12) = 2.04: two columns of 254.
        grid.measure(Size::new(520.0, f64::INFINITY));
        assert_eq!(Size::new(520.0, 30.0 + 12.0 + 40.0 + 12.0 + 5.0), grid.desired_size());

        grid.arrange(Rect::new(0.0, 0.0, 520.0, 99.0));
        let children = grid.children().snapshot();
        let layoutable = |index: usize| -> &Layoutable { &children[index] };
        assert_eq!(Rect::new(0.0, 0.0, 254.0, 30.0), layoutable(0).bounds());
        assert_eq!(Rect::new(266.0, 0.0, 254.0, 30.0), layoutable(1).bounds());
        assert_eq!(Rect::new(0.0, 42.0, 254.0, 40.0), layoutable(2).bounds());
        assert_eq!(Rect::new(0.0, 94.0, 254.0, 5.0), layoutable(4).bounds());
    }

    #[test]
    fn an_unconstrained_width_gives_one_row_of_minimum_width_columns() {
        let grid = CardGrid::new();
        grid.set_min_item_width(100.0);
        grid.set_column_spacing(10.0);
        grid.children().add(card(10.0));
        grid.children().add(card(20.0));
        let hidden = card(90.0);
        hidden.set_is_visible(false);
        grid.children().add(hidden);

        grid.measure(Size::new(f64::INFINITY, f64::INFINITY));
        assert_eq!(Size::new(210.0, 20.0), grid.desired_size());
    }

    #[test]
    fn an_empty_grid_measures_to_the_available_width_and_no_height() {
        let grid = CardGrid::new();
        grid.measure(Size::new(300.0, 200.0));
        assert_eq!(Size::new(300.0, 0.0), grid.desired_size());
    }
}
