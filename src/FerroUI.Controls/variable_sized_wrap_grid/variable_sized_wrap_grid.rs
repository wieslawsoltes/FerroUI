//! WinUI's `VariableSizedWrapGrid` (`dxaml/xcp/dxaml/lib/VariableSizedWrapGrid_Partial.cpp` and
//! `.h`, with `WrapGrid::ComputeAlignmentOffsets` of `WrapGrid_Partial.cpp`, of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::occupancy_map::OccupancyMap;
use crate::navigable_containers::register_navigable_container;
use crate::{Control, ControlImpl, Panel, PanelImpl};
use ferroui_base::input::{INavigableContainer, InputElement, InputElementImpl, NavigationDirection};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, FerroObjectImpl, FerroProperty,
    Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::{Cell, RefCell};

/// A grid of tiles of one cell size where each child spans a number of rows and columns
/// (`VariableSizedWrapGrid.RowSpan`, `VariableSizedWrapGrid.ColumnSpan`): the children fill the
/// cells of a line in the `Orientation` (top to bottom for `Vertical`, the default) and wrap to
/// the next line at the panel's edge or after `MaximumRowsOrColumns` cells. The cell size is
/// `ItemWidth` by `ItemHeight`, or the desired size of the first child where they are not set.
/// A child whose tile does not fit any more is not shown.
#[repr(C)]
pub struct VariableSizedWrapGrid {
    base: Panel,
    map: RefCell<OccupancyMap>,
    item_size: Cell<Size>,
    items_per_line: Cell<usize>,
    line_count: Cell<usize>,
}

ferro_class!(VariableSizedWrapGrid: Panel);
ferroui_base::ferro_class_info!(VariableSizedWrapGrid {
    new: VariableSizedWrapGrid::new
});
ferro_impl_classes!(
    VariableSizedWrapGrid: StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

ferroui_base::ferro_impl_classes!(VariableSizedWrapGrid: FerroObjectImpl);

/// The component of a size along the orientation (WinUI's `OrientedSize::get_Direct`).
fn direct(size: Size, horizontal: bool) -> f64 {
    if horizontal {
        size.width
    } else {
        size.height
    }
}

/// The component of a size across the orientation (`OrientedSize::get_Indirect`).
fn indirect(size: Size, horizontal: bool) -> f64 {
    if horizontal {
        size.height
    } else {
        size.width
    }
}

/// A cell length times a number of cells, zero for no cells (the length of an empty panel can be
/// infinite).
fn times(length: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        length * count as f64
    }
}

/// The alignments of `WrapGrid::ComputeAlignmentOffsets`, whose horizontal and vertical values
/// are the same numbers in WinUI.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ChildrenAlignment {
    Start,
    Center,
    End,
    Stretch,
}

impl From<HorizontalAlignment> for ChildrenAlignment {
    fn from(value: HorizontalAlignment) -> Self {
        match value {
            HorizontalAlignment::Left => Self::Start,
            HorizontalAlignment::Center => Self::Center,
            HorizontalAlignment::Right => Self::End,
            HorizontalAlignment::Stretch => Self::Stretch,
        }
    }
}

impl From<VerticalAlignment> for ChildrenAlignment {
    fn from(value: VerticalAlignment) -> Self {
        match value {
            VerticalAlignment::Top => Self::Start,
            VerticalAlignment::Center => Self::Center,
            VerticalAlignment::Bottom => Self::End,
            VerticalAlignment::Stretch => Self::Stretch,
        }
    }
}

/// `WrapGrid::ComputeAlignmentOffsets`: the offset before the first line and the space between
/// the lines that align `total_lines` lines of `required` length in `available`.
fn alignment_offsets(alignment: ChildrenAlignment, available: f64, required: f64, total_lines: usize) -> (f64, f64) {
    if available.is_infinite() {
        return (0.0, 0.0);
    }
    match alignment {
        ChildrenAlignment::Start => (0.0, 0.0),
        ChildrenAlignment::Center => (((available - required) / 2.0).max(0.0), 0.0),
        ChildrenAlignment::End => ((available - required).max(0.0), 0.0),
        ChildrenAlignment::Stretch => (0.0, ((available - required) / (total_lines as f64 + 1.0)).max(0.0)),
    }
}

impl LayoutableImpl for VariableSizedWrapGrid {
    /// `MeasureOverride`: finds the cell size (`ComputeBounds`), places every child in the map
    /// (`CalculateOccupancyMap`), measures each child at the size of its tile and returns the
    /// size of the occupied cells with the alignment offsets.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let horizontal = this.orientation() == Orientation::Horizontal;
        let children = this.children().snapshot();
        let item_size = this.compute_item_size(available_size, &children);
        this.item_size.set(item_size);
        let spans: Vec<(usize, usize)> = children.iter().map(|child| Self::spans_of(child)).collect();
        let (items_per_line, line_count) = this.calculate_occupancy_map(&spans, horizontal, item_size, available_size);
        this.items_per_line.set(items_per_line);
        this.line_count.set(line_count);

        // The first child is measured by `compute_item_size`.
        for (child, (row_span, column_span)) in children.iter().zip(&spans).skip(1) {
            child.measure(Size::new(
                item_size.width * *column_span as f64,
                item_size.height * *row_span as f64,
            ));
        }

        let (starting, justification) = this.alignment(available_size);
        let total_direct = times(direct(item_size, horizontal), items_per_line)
            + direct(justification, horizontal) * (items_per_line as f64 + 1.0)
            + direct(starting, horizontal) * 2.0;
        let total_indirect = times(indirect(item_size, horizontal), line_count)
            + indirect(justification, horizontal) * (line_count as f64 + 1.0)
            + indirect(starting, horizontal) * 2.0;
        if horizontal {
            Size::new(total_direct, total_indirect)
        } else {
            Size::new(total_indirect, total_direct)
        }
    }

    /// `ArrangeOverride`: arranges each child at the bounds of its tile in the map, moved by the
    /// alignment offsets. As in WinUI, the children the map could not place (the panel is full)
    /// are not arranged, so a child that never fitted is not shown.
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let children = this.children().snapshot();
        if children.is_empty() {
            return final_size;
        }
        let horizontal = this.orientation() == Orientation::Horizontal;
        let item_size = this.item_size.get();
        let items_per_line = this.items_per_line.get().max(1);
        let (starting, justification) = this.alignment(final_size);
        let mut total_justification = Size::new(0.0, 0.0);
        let mut map = this.map.borrow_mut();
        for (index, child) in children.iter().enumerate() {
            // The map places the children in order, so the first child it could not place ends
            // the ones it placed.
            if !map.set_current_item(index) {
                break;
            }
            let mut rect = map.current_item_rect();

            // A tile that spans more cells than a line has keeps its own length when it starts a
            // line, up to the panel's: the map cut it to the line.
            if rect.y == 0.0 || rect.x == 0.0 {
                let (row_span, column_span) = Self::spans_of(child);
                if !horizontal && rect.y == 0.0 && row_span >= items_per_line {
                    let height = (row_span as f64 * item_size.height).min(final_size.height);
                    rect = rect.with_height(rect.height.max(height));
                } else if horizontal && rect.x == 0.0 && column_span >= items_per_line {
                    let width = (column_span as f64 * item_size.width).min(final_size.width);
                    rect = rect.with_width(rect.width.max(width));
                }
            }

            // As WinUI, the justification counts the children by index, not by line.
            if index % items_per_line == 0 {
                let lines = (index / items_per_line + 1) as f64;
                total_justification = if horizontal {
                    Size::new(0.0, justification.height * lines)
                } else {
                    Size::new(justification.width * lines, 0.0)
                };
            }
            if horizontal {
                total_justification = Size::new(
                    total_justification.width + justification.width,
                    total_justification.height,
                );
            } else {
                total_justification = Size::new(
                    total_justification.width,
                    total_justification.height + justification.height,
                );
            }
            child.arrange(Rect::new(
                rect.x + starting.width + total_justification.width,
                rect.y + starting.height + total_justification.height,
                rect.width,
                rect.height,
            ));
        }
        final_size
    }
}

ferroui_base::ferro_properties! { impl VariableSizedWrapGrid {
    ferro_property!(
        /// Defines the `ItemHeight` property: the height of a cell, or NaN (the default) for the
        /// desired height of the first child.
        pub fn item_height_property() -> StyledProperty<f64> {
            FerroProperty::register::<VariableSizedWrapGrid, _>("ItemHeight", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `ItemWidth` property: the width of a cell, or NaN (the default) for the
        /// desired width of the first child.
        pub fn item_width_property() -> StyledProperty<f64> {
            FerroProperty::register::<VariableSizedWrapGrid, _>("ItemWidth", f64::NAN)
        }
    );

    ferro_property!(
        /// Defines the `Orientation` property: the direction the cells of a line follow
        /// (`Vertical` by default).
        pub fn orientation_property() -> StyledProperty<Orientation> {
            FerroProperty::register::<VariableSizedWrapGrid, _>("Orientation", Orientation::Vertical)
        }
    );

    ferro_property!(
        /// Defines the `HorizontalChildrenAlignment` property (`Left` by default).
        pub fn horizontal_children_alignment_property() -> StyledProperty<HorizontalAlignment> {
            FerroProperty::register::<VariableSizedWrapGrid, _>(
                "HorizontalChildrenAlignment",
                HorizontalAlignment::Left,
            )
        }
    );

    ferro_property!(
        /// Defines the `VerticalChildrenAlignment` property (`Top` by default).
        pub fn vertical_children_alignment_property() -> StyledProperty<VerticalAlignment> {
            FerroProperty::register::<VariableSizedWrapGrid, _>("VerticalChildrenAlignment", VerticalAlignment::Top)
        }
    );

    ferro_property!(
        /// Defines the `MaximumRowsOrColumns` property: the most cells of a line, or a value of
        /// zero or less (-1 by default) for as many as fit.
        pub fn maximum_rows_or_columns_property() -> StyledProperty<i32> {
            FerroProperty::register::<VariableSizedWrapGrid, _>("MaximumRowsOrColumns", -1)
        }
    );

    ferro_property!(
        /// Defines the `RowSpan` attached property: the number of rows a child's tile spans (a
        /// value of zero or less is one; 1 by default).
        pub fn row_span_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<VariableSizedWrapGrid, Control, _>("RowSpan", 1)
        }
    );

    ferro_property!(
        /// Defines the `ColumnSpan` attached property: the number of columns a child's tile
        /// spans (a value of zero or less is one; 1 by default).
        pub fn column_span_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<VariableSizedWrapGrid, Control, _>("ColumnSpan", 1)
        }
    );
} }

impl VariableSizedWrapGrid {
    fn static_constructor() {
        register_navigable_container::<VariableSizedWrapGrid>();
        Panel::affects_parent_measure::<VariableSizedWrapGrid>(&[
            Self::row_span_property().as_property(),
            Self::column_span_property().as_property(),
        ]);
        Layoutable::affects_measure::<VariableSizedWrapGrid>(&[
            Self::item_height_property().as_property(),
            Self::item_width_property().as_property(),
            Self::orientation_property().as_property(),
            Self::horizontal_children_alignment_property().as_property(),
            Self::vertical_children_alignment_property().as_property(),
            Self::maximum_rows_or_columns_property().as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            map: RefCell::new(OccupancyMap::default()),
            item_size: Cell::new(Size::new(0.0, 0.0)),
            items_per_line: Cell::new(0),
            line_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The row and column spans of a child, at least one each (`GetRowAndColumnSpan`).
    fn spans_of(child: &Control) -> (usize, usize) {
        let row_span = Self::get_row_span(child).max(1) as usize;
        let column_span = Self::get_column_span(child).max(1) as usize;
        (row_span, column_span)
    }

    /// `ComputeBounds`: the cell size, measuring the first child at the size of its tile where
    /// `ItemWidth` or `ItemHeight` is not set.
    fn compute_item_size(&self, available_size: Size, children: &[Ref<Control>]) -> Size {
        let item_width = self.item_width();
        let item_height = self.item_height();
        let has_fixed_width = !item_width.is_nan();
        let has_fixed_height = !item_height.is_nan();
        let mut item_size = Size::new(
            if has_fixed_width {
                item_width
            } else {
                available_size.width
            },
            if has_fixed_height {
                item_height
            } else {
                available_size.height
            },
        );
        if let Some(first) = children.first() {
            let (row_span, column_span) = Self::spans_of(first);
            first.measure(Size::new(
                if has_fixed_width {
                    item_size.width * column_span as f64
                } else {
                    item_size.width
                },
                if has_fixed_height {
                    item_size.height * row_span as f64
                } else {
                    item_size.height
                },
            ));
            let desired = first.desired_size();
            if !has_fixed_width {
                item_size = item_size.with_width(desired.width);
            }
            if !has_fixed_height {
                item_size = item_size.with_height(desired.height);
            }
        }
        item_size
    }

    /// `CalculateOccupancyMap`: places the children in a new map and returns the cells per line
    /// and the lines it uses.
    fn calculate_occupancy_map(
        &self,
        spans: &[(usize, usize)],
        horizontal: bool,
        item_size: Size,
        maximum_size: Size,
    ) -> (usize, usize) {
        let mut map = self.map.borrow_mut();
        if spans.is_empty() {
            *map = OccupancyMap::default();
            return (0, 0);
        }
        let row_tile_count: usize = spans.iter().map(|(row_span, _)| row_span).sum();
        let column_tile_count: usize = spans.iter().map(|(_, column_span)| column_span).sum();
        let direct_tile_count = if horizontal { column_tile_count } else { row_tile_count };

        // The cells that fit in a line, capped by the tiles' cells along it (which every child
        // fits in, so the cap changes no placement and keeps a zero cell size bounded).
        let maximum_direct = direct(maximum_size, horizontal);
        let mut items_per_line = if maximum_direct.is_infinite() {
            direct_tile_count
        } else {
            ((maximum_direct / direct(item_size, horizontal)) as usize).min(direct_tile_count)
        };
        items_per_line = items_per_line.max(1);
        let maximum_items_per_line = self.maximum_rows_or_columns();
        if maximum_items_per_line > 0 {
            items_per_line = items_per_line.min(maximum_items_per_line as usize);
        }

        let mut max_column = if horizontal { row_tile_count } else { column_tile_count };
        let maximum_indirect = indirect(maximum_size, horizontal);
        if !maximum_indirect.is_infinite() {
            max_column = ((maximum_indirect / indirect(item_size, horizontal)).floor() as usize).min(max_column);
        }
        let max_column = max_column.max(1);

        *map = OccupancyMap::new(items_per_line, max_column, horizontal, item_size, spans.len());
        for (index, (row_span, column_span)) in spans.iter().enumerate() {
            if !map.find_and_fill_next_available_rect(*row_span, *column_span, index as u32 + 1) {
                map.update_element_count(index);
                break;
            }
        }
        let (line_count, items_per_line) = map.find_used_grid_size();
        (items_per_line, line_count)
    }

    /// `ComputeHorizontalAndVerticalAlignment`: the offset before the first cell and the space
    /// between the cells, as unoriented sizes. WinUI reads the orientation after it decides which
    /// counts go with which axis, so the counts are those of a vertical grid in both
    /// orientations; kept for the same layout.
    fn alignment(&self, available_size: Size) -> (Size, Size) {
        let item_size = self.item_size.get();
        let items_per_line = self.items_per_line.get();
        let line_count = self.line_count.get();
        let (start_x, justify_x) = alignment_offsets(
            self.horizontal_children_alignment().into(),
            available_size.width,
            times(item_size.width, line_count),
            line_count,
        );
        let (start_y, justify_y) = alignment_offsets(
            self.vertical_children_alignment().into(),
            available_size.height,
            times(item_size.height, items_per_line),
            items_per_line,
        );
        (Size::new(start_x, start_y), Size::new(justify_x, justify_y))
    }

    /// Gets the `RowSpan` of a child.
    pub fn get_row_span(element: &Control) -> i32 {
        element.get_value(Self::row_span_property())
    }

    /// Sets the `RowSpan` of a child.
    pub fn set_row_span(element: &Control, value: i32) {
        element.set_value(Self::row_span_property(), value)
    }

    /// Gets the `ColumnSpan` of a child.
    pub fn get_column_span(element: &Control) -> i32 {
        element.get_value(Self::column_span_property())
    }

    /// Sets the `ColumnSpan` of a child.
    pub fn set_column_span(element: &Control, value: i32) {
        element.set_value(Self::column_span_property(), value)
    }

    /// The height of a cell; NaN for the desired height of the first child.
    pub fn item_height(&self) -> f64 {
        self.get_value(Self::item_height_property())
    }

    pub fn set_item_height(&self, value: f64) {
        self.set_value(Self::item_height_property(), value)
    }

    /// The width of a cell; NaN for the desired width of the first child.
    pub fn item_width(&self) -> f64 {
        self.get_value(Self::item_width_property())
    }

    pub fn set_item_width(&self, value: f64) {
        self.set_value(Self::item_width_property(), value)
    }

    /// The direction the cells of a line follow.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }

    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }

    /// How the cells are aligned horizontally in the panel.
    pub fn horizontal_children_alignment(&self) -> HorizontalAlignment {
        self.get_value(Self::horizontal_children_alignment_property())
    }

    pub fn set_horizontal_children_alignment(&self, value: HorizontalAlignment) {
        self.set_value(Self::horizontal_children_alignment_property(), value)
    }

    /// How the cells are aligned vertically in the panel.
    pub fn vertical_children_alignment(&self) -> VerticalAlignment {
        self.get_value(Self::vertical_children_alignment_property())
    }

    pub fn set_vertical_children_alignment(&self, value: VerticalAlignment) {
        self.set_value(Self::vertical_children_alignment_property(), value)
    }

    /// The most cells of a line; zero or less for as many as fit.
    pub fn maximum_rows_or_columns(&self) -> i32 {
        self.get_value(Self::maximum_rows_or_columns_property())
    }

    pub fn set_maximum_rows_or_columns(&self, value: i32) {
        self.set_value(Self::maximum_rows_or_columns_property(), value)
    }
}

impl INavigableContainer for VariableSizedWrapGrid {
    /// The child the arrow keys reach over the map of the last measure
    /// (`GetTargetIndexFromNavigationAction`); `Next` and `Previous` follow the order of the
    /// children, as WinUI's selector does.
    fn get_control(
        &self,
        direction: NavigationDirection,
        from: Option<&Ref<InputElement>>,
        _wrap: bool,
    ) -> Option<Ref<InputElement>> {
        let children = self.children();
        let index = from
            .and_then(|from| from.cast::<Control>())
            .and_then(|from| children.index_of(&from));
        let target = match (direction, index) {
            (NavigationDirection::Next, None) | (NavigationDirection::First, None) => Some(0),
            (NavigationDirection::Last, None) => self.map.borrow().element_count().checked_sub(1),
            (NavigationDirection::Next, Some(index)) => Some(index + 1),
            (NavigationDirection::Previous, Some(index)) => index.checked_sub(1),
            (_, Some(index)) => self.map.borrow_mut().target_index(index, direction),
            _ => None,
        };
        target
            .filter(|target| *target < children.count())
            .map(|target| children.get(target).upcast())
    }
}
