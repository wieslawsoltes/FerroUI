// This source file is adapted from the Windows Presentation Foundation project.
// (https://github.com/dotnet/wpf/)

use crate::{
    ColumnDefinition, ColumnDefinitions, Control, ControlImpl, DefinitionBase, GridUnitType, Panel, PanelImpl,
    PanelImplExt, RowDefinition, RowDefinitions,
};
use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutHelper, Layoutable, LayoutableImpl};
use ferroui_base::utilities::math_utilities::{max as math_max, min as math_min, MathUtilities};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, AttachedProperty, FerroObjectImpl, FerroProperty, FerroPropertyChangedEventArgs, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, VisualImpl,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::cmp::Ordering;
use std::ops::{BitOr, BitOrAssign, Index};
use std::rc::Rc;

//
//  The grid validity / property caches dirtiness flags.
//
//  The following flags let the grid track dirtiness in a more granular
//  manner:
//  * Valid???Structure flags indicate that elements were added or removed.
//  * Valid???Layout flags indicate that the layout time portion of the
//    information stored on the objects should be updated.
//
#[allow(dead_code)]
const FLAG_VALID_DEFINITIONS_U_STRUCTURE: u32 = 0x0000_0001;
#[allow(dead_code)]
const FLAG_VALID_DEFINITIONS_V_STRUCTURE: u32 = 0x0000_0002;
const FLAG_VALID_CELLS_STRUCTURE: u32 = 0x0000_0004;

//  Boolean properties state.
/// Show grid lines?
const FLAG_SHOW_GRID_LINES_PROPERTY_VALUE: u32 = 0x0000_0100;

//  Boolean flags.
/// "0" when all notifications are ignored.
const FLAG_LISTEN_TO_NOTIFICATIONS: u32 = 0x0000_1000;
/// "1" if calculating to content in the U direction.
const FLAG_SIZE_TO_CONTENT_U: u32 = 0x0000_2000;
/// "1" if calculating to content in the V direction.
const FLAG_SIZE_TO_CONTENT_V: u32 = 0x0000_4000;
/// "1" if at least one cell belongs to a Star column.
const FLAG_HAS_STAR_CELLS_U: u32 = 0x0000_8000;
/// "1" if at least one cell belongs to a Star row.
const FLAG_HAS_STAR_CELLS_V: u32 = 0x0001_0000;
/// "1" if at least one cell of group 3 belongs to an Auto row.
const FLAG_HAS_GROUP3_CELLS_IN_AUTO_ROWS: u32 = 0x0002_0000;
/// "1" while in the context of the measure override.
const FLAG_MEASURE_OVERRIDE_IN_PROGRESS: u32 = 0x0004_0000;
/// "1" while in the context of the arrange override.
const FLAG_ARRANGE_OVERRIDE_IN_PROGRESS: u32 = 0x0008_0000;

/// 5 is an arbitrary constant chosen to end the measure loop.
const LAYOUT_LOOP_MAX_COUNT: i32 = 5;

/// The index that terminates a chain of cells.
const NO_CELL: usize = i32::MAX as usize;

thread_local! {
    /// The temporary array used during layout for various purposes. It holds
    /// indices into the definitions being processed, or -1.
    static TEMP_DEFINITIONS: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
}

/// Used internally; reflects the layout-time size type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LayoutTimeSizeType(u8);

impl LayoutTimeSizeType {
    pub(crate) const NONE: Self = Self(0x00);
    pub(crate) const PIXEL: Self = Self(0x01);
    pub(crate) const AUTO: Self = Self(0x02);
    pub(crate) const STAR: Self = Self(0x04);

    #[inline]
    fn has_all_flags(self, flags: Self) -> bool {
        (self.0 & flags.0) == flags.0
    }
}

impl BitOr for LayoutTimeSizeType {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for LayoutTimeSizeType {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Stores the calculated values of
/// 1. the attached cell positioning properties;
/// 2. the size type;
/// 3. the index of the next cell in the group.
#[derive(Clone, Copy, Default)]
struct CellCache {
    column_index: usize,
    row_index: usize,
    column_span: usize,
    row_span: usize,
    size_type_u: LayoutTimeSizeType,
    size_type_v: LayoutTimeSizeType,
    next: usize,
}

impl CellCache {
    #[inline]
    fn is_star_u(&self) -> bool {
        self.size_type_u.has_all_flags(LayoutTimeSizeType::STAR)
    }

    #[inline]
    fn is_auto_u(&self) -> bool {
        self.size_type_u.has_all_flags(LayoutTimeSizeType::AUTO)
    }

    #[inline]
    fn is_star_v(&self) -> bool {
        self.size_type_v.has_all_flags(LayoutTimeSizeType::STAR)
    }

    #[inline]
    fn is_auto_v(&self) -> bool {
        self.size_type_v.has_all_flags(LayoutTimeSizeType::AUTO)
    }
}

/// The key of a span in the span store.
#[derive(Clone, Copy, PartialEq, Eq)]
struct SpanKey {
    /// The starting index of the span.
    start: usize,
    /// The span count.
    count: usize,
    /// True for columns; false for rows.
    u: bool,
}

/// The collection of definitions used during the calculations: either the
/// collection supplied by the user or a default collection of one element.
#[derive(Clone)]
enum DefinitionsSource {
    Columns(ColumnDefinitions),
    Rows(RowDefinitions),
    Single(Ref<DefinitionBase>),
}

impl DefinitionsSource {
    fn definitions(&self) -> Definitions {
        match self {
            DefinitionsSource::Columns(definitions) => Definitions::Columns(definitions.snapshot()),
            DefinitionsSource::Rows(definitions) => Definitions::Rows(definitions.snapshot()),
            DefinitionsSource::Single(definition) => Definitions::Single(definition.clone()),
        }
    }
}

/// A read-only list of definitions.
enum Definitions {
    Columns(Rc<Vec<Ref<ColumnDefinition>>>),
    Rows(Rc<Vec<Ref<RowDefinition>>>),
    Single(Ref<DefinitionBase>),
}

impl Definitions {
    #[inline]
    fn count(&self) -> usize {
        match self {
            Definitions::Columns(definitions) => definitions.len(),
            Definitions::Rows(definitions) => definitions.len(),
            Definitions::Single(_) => 1,
        }
    }
}

impl Index<usize> for Definitions {
    type Output = DefinitionBase;

    #[inline]
    fn index(&self, index: usize) -> &DefinitionBase {
        match self {
            Definitions::Columns(definitions) => {
                let definition: &ColumnDefinition = &definitions[index];
                definition
            }
            Definitions::Rows(definitions) => {
                let definition: &RowDefinition = &definitions[index];
                definition
            }
            Definitions::Single(definition) => {
                assert!(index == 0, "index out of range");
                definition
            }
        }
    }
}

/// The extended data instantiated on demand, when the grid handles a
/// non-trivial case.
struct ExtendedData {
    /// The collection of column definitions (logical tree support).
    column_definitions: RefCell<Option<ColumnDefinitions>>,
    /// The collection of row definitions (logical tree support).
    row_definitions: RefCell<Option<RowDefinitions>>,
    /// The collection of column definitions used during the calculations.
    definitions_u: RefCell<Option<DefinitionsSource>>,
    /// The collection of row definitions used during the calculations.
    definitions_v: RefCell<Option<DefinitionsSource>>,
    /// The backing store for the logical children.
    cell_caches_collection: RefCell<Vec<CellCache>>,
    /// The index of the first cell in the first cell group.
    cell_group1: Cell<usize>,
    /// The index of the first cell in the second cell group.
    cell_group2: Cell<usize>,
    /// The index of the first cell in the third cell group.
    cell_group3: Cell<usize>,
    /// The index of the first cell in the fourth cell group.
    cell_group4: Cell<usize>,
}

impl ExtendedData {
    fn new() -> Self {
        Self {
            column_definitions: RefCell::new(None),
            row_definitions: RefCell::new(None),
            definitions_u: RefCell::new(None),
            definitions_v: RefCell::new(None),
            cell_caches_collection: RefCell::new(Vec::new()),
            cell_group1: Cell::new(0),
            cell_group2: Cell::new(0),
            cell_group3: Cell::new(0),
            cell_group4: Cell::new(0),
        }
    }
}

/// Clears a flag of a grid when dropped.
struct FlagGuard<'a>(&'a Grid, u32);

impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.0.set_flags(false, self.1);
    }
}

/// Compares two doubles: NaN is smaller than everything else and equal to
/// itself.
fn compare_to(x: f64, y: f64) -> Ordering {
    x.partial_cmp(&y).unwrap_or_else(|| match (x.is_nan(), y.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        _ => Ordering::Greater,
    })
}

/// The logarithm of `a` in base 2, computed as the quotient of natural
/// logarithms.
#[inline]
fn log2(a: f64) -> f64 {
    a.ln() / 2.0_f64.ln()
}

/// Defines a flexible grid area that consists of columns and rows.
#[repr(C)]
pub struct Grid {
    base: Panel,
    /// The extended data instantiated on demand, for the handling of the
    /// non-trivial case only.
    ext_data: OnceCell<ExtendedData>,
    /// The grid validity / property caches dirtiness flags.
    flags: Cell<u32>,
    /// Keeps track of definition indices.
    definition_indices: RefCell<Option<Vec<i32>>>,
    /// Stores unrounded values and rounding errors during layout rounding.
    rounding_errors: RefCell<Option<Vec<f64>>>,
    /// The visual that draws the grid lines, while they are shown.
    grid_lines_renderer: RefCell<Option<Ref<GridLinesRenderer>>>,
}

ferro_class!(Grid: Panel);
ferroui_base::ferro_class_info!(Grid { new: Grid::new });
ferro_impl_classes!(Grid: StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

ferroui_base::ferro_impl_classes!(Grid: FerroObjectImpl);

impl LayoutableImpl for Grid {
    /// Content measurement.
    fn measure_override(this: &Self, constraint: Size) -> Size {
        let grid_desired_size;
        let ext_data = this.ext_data.get();

        this.set_listen_to_notifications(true);
        this.set_flags(true, FLAG_MEASURE_OVERRIDE_IN_PROGRESS);
        let _in_progress = FlagGuard(this, FLAG_MEASURE_OVERRIDE_IN_PROGRESS);

        match ext_data {
            None => {
                let mut desired_size = Size::default();
                let children = this.children().snapshot();

                for child in children.iter() {
                    child.measure(constraint);
                    let child_desired_size = child.desired_size();
                    desired_size = Size::new(
                        math_max(desired_size.width, child_desired_size.width),
                        math_max(desired_size.height, child_desired_size.height),
                    );
                }
                grid_desired_size = desired_size;
            }
            Some(ext_data) => {
                {
                    let size_to_content_u = constraint.width == f64::INFINITY;
                    let size_to_content_v = constraint.height == f64::INFINITY;

                    // Clear index information and rounding errors.
                    if this.row_definitions_dirty() || this.column_definitions_dirty() {
                        *this.definition_indices.borrow_mut() = None;

                        if this.use_layout_rounding() {
                            *this.rounding_errors.borrow_mut() = None;
                        }
                    }

                    this.validate_definitions_u_structure();
                    this.validate_definitions_layout(&this.definitions_u(), size_to_content_u);

                    this.validate_definitions_v_structure();
                    this.validate_definitions_layout(&this.definitions_v(), size_to_content_v);

                    if this.size_to_content_u() != size_to_content_u || this.size_to_content_v() != size_to_content_v {
                        this.set_cells_structure_dirty(true);
                    }

                    this.set_flags(size_to_content_u, FLAG_SIZE_TO_CONTENT_U);
                    this.set_flags(size_to_content_v, FLAG_SIZE_TO_CONTENT_V);
                }

                this.validate_cells();

                let definitions_u = this.definitions_u();
                let definitions_v = this.definitions_v();

                debug_assert!(definitions_u.count() > 0 && definitions_v.count() > 0);

                //  Grid classifies cells into four groups depending on
                //  the column / row type a cell belongs to (number corresponds to
                //  group number):
                //
                //                   Px      Auto     Star
                //               +--------+--------+--------+
                //               |        |        |        |
                //            Px |    1   |    1   |    3   |
                //               |        |        |        |
                //               +--------+--------+--------+
                //               |        |        |        |
                //          Auto |    1   |    1   |    3   |
                //               |        |        |        |
                //               +--------+--------+--------+
                //               |        |        |        |
                //          Star |    4   |    2   |    4   |
                //               |        |        |        |
                //               +--------+--------+--------+
                //
                //  The group number indicates the order in which cells are measured.
                //  Certain order is necessary to be able to dynamically resolve star
                //  columns / rows sizes which are used as input for measuring of
                //  the cells belonging to them.
                //
                //  However, there are cases when topology of a grid causes cyclical
                //  size dependencies. For example:
                //
                //
                //                         column width="Auto"      column width="*"
                //                      +----------------------+----------------------+
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //  row height="Auto"   |                      |      cell 1 2        |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      +----------------------+----------------------+
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //  row height="*"      |       cell 2 1       |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      |                      |                      |
                //                      +----------------------+----------------------+
                //
                //  In order to accurately calculate constraint width for "cell 1 2"
                //  (which is the remaining of grid's available width and calculated
                //  value of Auto column), "cell 2 1" needs to be calculated first,
                //  as it contributes to the Auto column's calculated value.
                //  At the same time in order to accurately calculate constraint
                //  height for "cell 2 1", "cell 1 2" needs to be calculated first,
                //  as it contributes to Auto row height, which is used in the
                //  computation of Star row resolved height.
                //
                //  to "break" this cyclical dependency we are making (arbitrary)
                //  decision to treat cells like "cell 2 1" as if they appear in Auto
                //  rows. And then recalculate them one more time when star row
                //  heights are resolved.
                //
                //  (Or more strictly) the code below implement the following logic:
                //
                //                       +---------+
                //                       |  enter  |
                //                       +---------+
                //                            |
                //                            V
                //                    +----------------+
                //                    | Measure Group1 |
                //                    +----------------+
                //                            |
                //                            V
                //                          / - \
                //                        /       \
                //                  Y   /    Can    \    N
                //            +--------|   Resolve   |-----------+
                //            |         \  StarsV?  /            |
                //            |           \       /              |
                //            |             \ - /                |
                //            V                                  V
                //    +----------------+                       / - \
                //    | Resolve StarsV |                     /       \
                //    +----------------+               Y   /    Can    \    N
                //            |                      +----|   Resolve   |------+
                //            V                      |     \  StarsU?  /       |
                //    +----------------+             |       \       /         |
                //    | Measure Group2 |             |         \ - /           |
                //    +----------------+             |                         V
                //            |                      |                 +-----------------+
                //            V                      |                 | Measure Group2' |
                //    +----------------+             |                 +-----------------+
                //    | Resolve StarsU |             |                         |
                //    +----------------+             V                         V
                //            |              +----------------+        +----------------+
                //            V              | Resolve StarsU |        | Resolve StarsU |
                //    +----------------+     +----------------+        +----------------+
                //    | Measure Group3 |             |                         |
                //    +----------------+             V                         V
                //            |              +----------------+        +----------------+
                //            |              | Measure Group3 |        | Measure Group3 |
                //            |              +----------------+        +----------------+
                //            |                      |                         |
                //            |                      V                         V
                //            |              +----------------+        +----------------+
                //            |              | Resolve StarsV |        | Resolve StarsV |
                //            |              +----------------+        +----------------+
                //            |                      |                         |
                //            |                      |                         V
                //            |                      |                +------------------+
                //            |                      |                | Measure Group2'' |
                //            |                      |                +------------------+
                //            |                      |                         |
                //            +----------------------+-------------------------+
                //                                   |
                //                                   V
                //                           +----------------+
                //                           | Measure Group4 |
                //                           +----------------+
                //                                   |
                //                                   V
                //                               +--------+
                //                               |  exit  |
                //                               +--------+
                //
                //  where:
                //  *   all [Measure GroupN] - regular children measure process -
                //      each cell is measured given constraint size as an input
                //      and each cell's desired size is accumulated on the
                //      corresponding column / row;
                //  *   [Measure Group2'] - is when each cell is measured with
                //      infinite height as a constraint and a cell's desired
                //      height is ignored;
                //  *   [Measure Groups''] - is when each cell is measured (second
                //      time during single measure override) regularly but its
                //      returned width is ignored;
                //
                //  This algorithm is believed to be as close to ideal as possible.
                //  It has the following drawbacks:
                //  *   cells belonging to Group2 can be called to measure twice;
                //  *   iff during second measure a cell belonging to Group2 returns
                //      desired width greater than desired width returned the first
                //      time, such a cell is going to be clipped, even though it
                //      appears in Auto column.
                //

                this.measure_cells_group(ext_data.cell_group1.get(), false, false);
                let combined_row_spacing = this.row_spacing() * (definitions_v.count() as f64 - 1.0);
                let combined_column_spacing = this.column_spacing() * (definitions_u.count() as f64 - 1.0);
                let inner_available_size = Size::new(
                    constraint.width - combined_column_spacing,
                    constraint.height - combined_row_spacing,
                );
                {
                    //  After Group1 is measured, only Group3 may have cells
                    //  belonging to Auto rows.
                    let can_resolve_stars_v = !this.has_group3_cells_in_auto_rows();

                    if can_resolve_stars_v {
                        if this.has_star_cells_v() {
                            this.resolve_star(&definitions_v, inner_available_size.height);
                        }
                        this.measure_cells_group(ext_data.cell_group2.get(), false, false);
                        if this.has_star_cells_u() {
                            this.resolve_star(&definitions_u, inner_available_size.width);
                        }
                        this.measure_cells_group(ext_data.cell_group3.get(), false, false);
                    } else {
                        //  If at least one cell exists in Group2, it must be
                        //  measured before StarsU can be resolved.
                        let can_resolve_stars_u = ext_data.cell_group2.get() > this.private_cells_len();
                        if can_resolve_stars_u {
                            if this.has_star_cells_u() {
                                this.resolve_star(&definitions_u, inner_available_size.width);
                            }
                            this.measure_cells_group(ext_data.cell_group3.get(), false, false);
                            if this.has_star_cells_v() {
                                this.resolve_star(&definitions_v, inner_available_size.height);
                            }
                        } else {
                            // This is a revision to the algorithm employed for
                            // the cyclic dependency case described above. We
                            // now repeatedly measure Group3 and Group2 until
                            // their sizes settle. We also use a count
                            // heuristic to break a loop in case of one.

                            let mut has_desired_size_u_changed = false;
                            let mut cnt = 0;

                            // Cache Group2MinWidths & Group3MinHeights.
                            let group2_min_sizes = this.cache_min_sizes(ext_data.cell_group2.get(), false);
                            let group3_min_sizes = this.cache_min_sizes(ext_data.cell_group3.get(), true);

                            this.measure_cells_group(ext_data.cell_group2.get(), false, true);

                            loop {
                                if has_desired_size_u_changed {
                                    // Reset cached Group3Heights.
                                    this.apply_cached_min_sizes(&group3_min_sizes, true);
                                }

                                if this.has_star_cells_u() {
                                    this.resolve_star(&definitions_u, inner_available_size.width);
                                }
                                this.measure_cells_group(ext_data.cell_group3.get(), false, false);

                                // Reset cached Group2Widths.
                                this.apply_cached_min_sizes(&group2_min_sizes, false);

                                if this.has_star_cells_v() {
                                    this.resolve_star(&definitions_v, inner_available_size.height);
                                }
                                has_desired_size_u_changed = this.measure_cells_group(
                                    ext_data.cell_group2.get(),
                                    cnt == LAYOUT_LOOP_MAX_COUNT,
                                    false,
                                );

                                if !has_desired_size_u_changed {
                                    break;
                                }
                                cnt += 1;
                                if cnt > LAYOUT_LOOP_MAX_COUNT {
                                    break;
                                }
                            }
                        }
                    }
                }

                this.measure_cells_group(ext_data.cell_group4.get(), false, false);

                grid_desired_size = Size::new(
                    Self::calculate_desired_size(&definitions_u)
                        + this.column_spacing() * (definitions_u.count() as f64 - 1.0),
                    Self::calculate_desired_size(&definitions_v)
                        + this.row_spacing() * (definitions_v.count() as f64 - 1.0),
                );
            }
        }

        grid_desired_size
    }

    /// Content arrangement.
    fn arrange_override(this: &Self, arrange_size: Size) -> Size {
        this.set_flags(true, FLAG_ARRANGE_OVERRIDE_IN_PROGRESS);
        let _in_progress = FlagGuard(this, FLAG_ARRANGE_OVERRIDE_IN_PROGRESS);

        if this.ext_data.get().is_none() {
            let children = this.children().snapshot();

            for child in children.iter() {
                child.arrange(Rect::from_size(arrange_size));
            }
        } else {
            let definitions_u = this.definitions_u();
            let definitions_v = this.definitions_v();

            debug_assert!(definitions_u.count() > 0 && definitions_v.count() > 0);
            let row_spacing = this.row_spacing();
            let column_spacing = this.column_spacing();
            let combined_row_spacing = row_spacing * (definitions_v.count() as f64 - 1.0);
            let combined_column_spacing = column_spacing * (definitions_u.count() as f64 - 1.0);
            this.set_final_size(&definitions_u, arrange_size.width - combined_column_spacing, true);
            this.set_final_size(&definitions_v, arrange_size.height - combined_row_spacing, false);

            let children = this.children().snapshot();

            for current_cell in 0..this.private_cells_len() {
                let cell = &children[current_cell];
                let cache = this.private_cell(current_cell);

                let column_index = cache.column_index;
                let row_index = cache.row_index;
                let column_span = cache.column_span;
                let row_span = cache.row_span;

                let cell_rect = Rect::new(
                    if column_index == 0 {
                        0.0
                    } else {
                        definitions_u[column_index].final_offset()
                    },
                    if row_index == 0 {
                        0.0
                    } else {
                        definitions_v[row_index].final_offset()
                    },
                    Self::get_final_size_for_range(&definitions_u, column_index, column_span, column_spacing),
                    Self::get_final_size_for_range(&definitions_v, row_index, row_span, row_spacing),
                );

                cell.arrange(cell_rect);
            }

            // Update the render bound on the grid lines renderer visual.
            if let Some(grid_lines_renderer) = this.ensure_grid_lines_renderer() {
                grid_lines_renderer.update_render_bounds(arrange_size);
            }
        }

        arrange_size
    }
}

impl PanelImpl for Grid {
    fn children_changed(this: &Self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        this.set_cells_structure_dirty(true);
        Self::parent_children_changed(this, e);
    }
}

ferroui_base::ferro_properties! { impl Grid {
    ferro_property!(
        /// Defines the `ShowGridLines` property. This property is used mostly
        /// for the simplification of visual debugging. When it is set to
        /// true, grid lines are drawn to visualize the location of the grid
        /// lines.
        pub fn show_grid_lines_property() -> StyledProperty<bool> {
            FerroProperty::register::<Grid, _>("ShowGridLines", false)
        }
    );

    ferro_property!(
        /// Defines the `RowSpacing` property.
        pub fn row_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<Grid, _>("RowSpacing", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `ColumnSpacing` property.
        pub fn column_spacing_property() -> StyledProperty<f64> {
            FerroProperty::register::<Grid, _>("ColumnSpacing", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `Column` attached property, which can be set on any
        /// element treated as a cell. It specifies the position of the child
        /// with respect to the columns.
        ///
        /// Columns are 0-based: in order to appear in the first column, an
        /// element should have the property set to 0, which is the default
        /// value.
        pub fn column_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Grid, Control, _>(
                "Column",
                StyledPropertyOptions::new(0).validate(|v| *v >= 0),
            )
        }
    );

    ferro_property!(
        /// Defines the `Row` attached property, which can be set on any
        /// element treated as a cell. It specifies the position of the child
        /// with respect to the rows.
        ///
        /// Rows are 0-based: in order to appear in the first row, an element
        /// should have the property set to 0, which is the default value.
        pub fn row_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Grid, Control, _>(
                "Row",
                StyledPropertyOptions::new(0).validate(|v| *v >= 0),
            )
        }
    );

    ferro_property!(
        /// Defines the `ColumnSpan` attached property, which can be set on
        /// any element treated as a cell. It specifies the width of the child
        /// with respect to the columns: a value of 2 means that the child
        /// spans across two columns.
        ///
        /// The default value is 1.
        pub fn column_span_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Grid, Control, _>(
                "ColumnSpan",
                StyledPropertyOptions::new(1).validate(|v| *v > 0),
            )
        }
    );

    ferro_property!(
        /// Defines the `RowSpan` attached property, which can be set on any
        /// element treated as a cell. It specifies the height of the child
        /// with respect to the row grid lines: a value of 3 means that the
        /// child spans across three rows.
        ///
        /// The default value is 1.
        pub fn row_span_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached_with::<Grid, Control, _>(
                "RowSpan",
                StyledPropertyOptions::new(1).validate(|v| *v > 0),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsSharedSizeScope` attached property, which marks the
        /// scoping element for shared sizes.
        pub fn is_shared_size_scope_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<Grid, Control, _>("IsSharedSizeScope", false)
        }
    );
} }

impl Grid {
    fn static_constructor() {
        let show_grid_lines = Self::show_grid_lines_property();
        let column_spacing = Self::column_spacing_property();
        let row_spacing = Self::row_spacing_property();
        let is_shared_size_scope = Self::is_shared_size_scope_property();
        let column = Self::column_property();
        let column_span = Self::column_span_property();
        let row = Self::row_property();
        let row_span = Self::row_span_property();

        show_grid_lines
            .changed()
            .add_class_handler::<Grid>(Self::on_show_grid_lines_property_changed);
        column_spacing
            .changed()
            .add_class_handler::<Grid>(Self::on_spacing_property_changed);
        row_spacing
            .changed()
            .add_class_handler::<Grid>(Self::on_spacing_property_changed);

        is_shared_size_scope
            .changed()
            .add_class_handler::<Control>(DefinitionBase::on_is_shared_size_scope_property_changed);
        column
            .changed()
            .add_class_handler::<Control>(Self::on_cell_attached_property_changed);
        column_span
            .changed()
            .add_class_handler::<Control>(Self::on_cell_attached_property_changed);
        row.changed()
            .add_class_handler::<Control>(Self::on_cell_attached_property_changed);
        row_span
            .changed()
            .add_class_handler::<Control>(Self::on_cell_attached_property_changed);

        Layoutable::affects_measure::<Grid>(&[column_spacing.as_property(), row_spacing.as_property()]);
        Panel::affects_parent_measure::<Grid>(&[
            column.as_property(),
            column_span.as_property(),
            row.as_property(),
            row_span.as_property(),
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            ext_data: OnceCell::new(),
            flags: Cell::new(0),
            definition_indices: RefCell::new(None),
            rounding_errors: RefCell::new(None),
            grid_lines_renderer: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Sets the `Column` property on a control.
    pub fn set_column(element: &Control, value: i32) {
        element.set_value(Self::column_property(), value);
    }

    /// Reads the `Column` property from a control.
    pub fn get_column(element: &Control) -> i32 {
        element.get_value(Self::column_property())
    }

    /// Sets the `Row` property on a control.
    pub fn set_row(element: &Control, value: i32) {
        element.set_value(Self::row_property(), value);
    }

    /// Reads the `Row` property from a control.
    pub fn get_row(element: &Control) -> i32 {
        element.get_value(Self::row_property())
    }

    /// Sets the `ColumnSpan` property on a control.
    pub fn set_column_span(element: &Control, value: i32) {
        element.set_value(Self::column_span_property(), value);
    }

    /// Reads the `ColumnSpan` property from a control.
    pub fn get_column_span(element: &Control) -> i32 {
        element.get_value(Self::column_span_property())
    }

    /// Sets the `RowSpan` property on a control.
    pub fn set_row_span(element: &Control, value: i32) {
        element.set_value(Self::row_span_property(), value);
    }

    /// Reads the `RowSpan` property from a control.
    pub fn get_row_span(element: &Control) -> i32 {
        element.get_value(Self::row_span_property())
    }

    /// Sets the `IsSharedSizeScope` property on a control.
    pub fn set_is_shared_size_scope(element: &Control, value: bool) {
        element.set_value(Self::is_shared_size_scope_property(), value);
    }

    /// Reads the `IsSharedSizeScope` property from a control.
    pub fn get_is_shared_size_scope(element: &Control) -> bool {
        element.get_value(Self::is_shared_size_scope_property())
    }

    /// Whether the grid lines are shown.
    pub fn show_grid_lines(&self) -> bool {
        self.get_value(Self::show_grid_lines_property())
    }

    pub fn set_show_grid_lines(&self, value: bool) {
        self.set_value(Self::show_grid_lines_property(), value)
    }

    /// The size of the spacing to place between grid rows.
    pub fn row_spacing(&self) -> f64 {
        self.get_value(Self::row_spacing_property())
    }

    pub fn set_row_spacing(&self, value: f64) {
        self.set_value(Self::row_spacing_property(), value)
    }

    /// The size of the spacing to place between grid columns.
    pub fn column_spacing(&self) -> f64 {
        self.get_value(Self::column_spacing_property())
    }

    pub fn set_column_spacing(&self, value: f64) {
        self.set_value(Self::column_spacing_property(), value)
    }

    /// The collection of column definitions.
    pub fn column_definitions(&self) -> ColumnDefinitions {
        let ext_data = self.ext();
        if let Some(column_definitions) = ext_data.column_definitions.borrow().as_ref() {
            return column_definitions.clone();
        }

        let column_definitions = ColumnDefinitions::new();
        column_definitions.set_parent(Some(&self.to_ref()));
        *ext_data.column_definitions.borrow_mut() = Some(column_definitions.clone());
        column_definitions
    }

    pub fn set_column_definitions(&self, value: ColumnDefinitions) {
        let ext_data = self.ext();
        //  Otherwise the outgoing definitions stay registered with their
        //  shared size group and keep contributing to its minimum.
        let old_definitions = ext_data.column_definitions.borrow().clone();
        if let Some(old_definitions) = old_definitions {
            if old_definitions != value {
                old_definitions.set_parent(None);
            }
        }
        *ext_data.column_definitions.borrow_mut() = Some(value.clone());
        value.set_parent(Some(&self.to_ref()));
        self.invalidate_measure();
    }

    /// The collection of row definitions.
    pub fn row_definitions(&self) -> RowDefinitions {
        let ext_data = self.ext();
        if let Some(row_definitions) = ext_data.row_definitions.borrow().as_ref() {
            return row_definitions.clone();
        }

        let row_definitions = RowDefinitions::new();
        row_definitions.set_parent(Some(&self.to_ref()));
        *ext_data.row_definitions.borrow_mut() = Some(row_definitions.clone());
        row_definitions
    }

    pub fn set_row_definitions(&self, value: RowDefinitions) {
        let ext_data = self.ext();
        //  Otherwise the outgoing definitions stay registered with their
        //  shared size group and keep contributing to its minimum.
        let old_definitions = ext_data.row_definitions.borrow().clone();
        if let Some(old_definitions) = old_definitions {
            if old_definitions != value {
                old_definitions.set_parent(None);
            }
        }
        *ext_data.row_definitions.borrow_mut() = Some(value.clone());
        value.set_parent(Some(&self.to_ref()));
        self.invalidate_measure();
    }

    /// Invalidates the grid caches and makes the grid dirty for measure.
    pub(crate) fn invalidate(&self) {
        self.set_cells_structure_dirty(true);
        self.invalidate_measure();
    }

    /// Returns the final width of a column.
    ///
    /// Used from the actual width of a column definition. Calculates the
    /// final width using the offset data.
    pub(crate) fn get_final_column_definition_width(&self, column_index: i32) -> f64 {
        let mut value = 0.0;

        //  Actual value calculations require the structure to be up-to-date.
        if !self.column_definitions_dirty() {
            let definitions = self.definitions_u();
            let column_index = column_index as usize;
            value = definitions[(column_index + 1) % definitions.count()].final_offset();
            if column_index != 0 {
                value -= definitions[column_index].final_offset();
            }
        }
        value
    }

    /// Returns the final height of a row.
    ///
    /// Used from the actual height of a row definition. Calculates the final
    /// height using the offset data.
    pub(crate) fn get_final_row_definition_height(&self, row_index: i32) -> f64 {
        let mut value = 0.0;

        //  Actual value calculations require the structure to be up-to-date.
        if !self.row_definitions_dirty() {
            let definitions = self.definitions_v();
            let row_index = row_index as usize;
            value = definitions[(row_index + 1) % definitions.count()].final_offset();
            if row_index != 0 {
                value -= definitions[row_index].final_offset();
            }
        }
        value
    }

    /// Whether the measure override is in progress.
    #[allow(dead_code)]
    pub(crate) fn measure_override_in_progress(&self) -> bool {
        self.check_flags(FLAG_MEASURE_OVERRIDE_IN_PROGRESS)
    }

    /// Whether the arrange override is in progress.
    #[allow(dead_code)]
    pub(crate) fn arrange_override_in_progress(&self) -> bool {
        self.check_flags(FLAG_ARRANGE_OVERRIDE_IN_PROGRESS)
    }

    /// Whether the structure of the column definitions changed.
    pub(crate) fn column_definitions_dirty(&self) -> bool {
        self.column_definitions().is_dirty()
    }

    pub(crate) fn set_column_definitions_dirty(&self, value: bool) {
        self.column_definitions().set_is_dirty(value)
    }

    /// Whether the structure of the row definitions changed.
    pub(crate) fn row_definitions_dirty(&self) -> bool {
        self.row_definitions().is_dirty()
    }

    pub(crate) fn set_row_definitions_dirty(&self, value: bool) {
        self.row_definitions().set_is_dirty(value)
    }

    /// The extended data, created on demand.
    fn ext(&self) -> &ExtendedData {
        self.ext_data.get_or_init(ExtendedData::new)
    }

    /// Lays out cells according to rows and columns, and creates lookup
    /// grids.
    fn validate_cells(&self) {
        if self.cells_structure_dirty() {
            self.validate_cells_core();
            self.set_cells_structure_dirty(false);
        }
    }

    fn validate_cells_core(&self) {
        let children = self.children().snapshot();
        let ext_data = self.ext();
        let definitions_u = self.definitions_u();
        let definitions_v = self.definitions_v();

        let mut cells = vec![CellCache::default(); children.len()];
        let mut cell_group1 = NO_CELL;
        let mut cell_group2 = NO_CELL;
        let mut cell_group3 = NO_CELL;
        let mut cell_group4 = NO_CELL;

        let mut has_star_cells_u = false;
        let mut has_star_cells_v = false;
        let mut has_group3_cells_in_auto_rows = false;

        for i in (0..cells.len()).rev() {
            let child = &children[i];

            let mut cell = CellCache::default();

            //  Read indices from the corresponding properties:
            //      clamp to value < number_of_columns
            //      column >= 0 is guaranteed by the property value validation callback
            cell.column_index = (Self::get_column(child) as usize).min(definitions_u.count() - 1);
            //      clamp to value < number_of_rows
            //      row >= 0 is guaranteed by the property value validation callback
            cell.row_index = (Self::get_row(child) as usize).min(definitions_v.count() - 1);

            //  Read span properties:
            //      clamp to not exceed beyond the right side of the grid
            //      column_span > 0 is guaranteed by the property value validation callback
            cell.column_span = (Self::get_column_span(child) as usize).min(definitions_u.count() - cell.column_index);

            //      clamp to not exceed beyond the bottom side of the grid
            //      row_span > 0 is guaranteed by the property value validation callback
            cell.row_span = (Self::get_row_span(child) as usize).min(definitions_v.count() - cell.row_index);

            debug_assert!(cell.column_index < definitions_u.count());
            debug_assert!(cell.row_index < definitions_v.count());

            //  Calculate and cache the length types for the child.

            cell.size_type_u = Self::get_length_type_for_range(&definitions_u, cell.column_index, cell.column_span);
            cell.size_type_v = Self::get_length_type_for_range(&definitions_v, cell.row_index, cell.row_span);

            has_star_cells_u |= cell.is_star_u();
            has_star_cells_v |= cell.is_star_v();

            //  Distribute cells into four groups.

            if !cell.is_star_v() {
                if !cell.is_star_u() {
                    cell.next = cell_group1;
                    cell_group1 = i;
                } else {
                    cell.next = cell_group3;
                    cell_group3 = i;

                    //  Remember if this cell belongs to an auto row.
                    has_group3_cells_in_auto_rows |= cell.is_auto_v();
                }
            } else if cell.is_auto_u()
                //  Note below: if it spans through a Star column it is NOT Auto.
                && !cell.is_star_u()
            {
                cell.next = cell_group2;
                cell_group2 = i;
            } else {
                cell.next = cell_group4;
                cell_group4 = i;
            }

            cells[i] = cell;
        }

        *ext_data.cell_caches_collection.borrow_mut() = cells;
        ext_data.cell_group1.set(cell_group1);
        ext_data.cell_group2.set(cell_group2);
        ext_data.cell_group3.set(cell_group3);
        ext_data.cell_group4.set(cell_group4);

        self.set_flags(has_star_cells_u, FLAG_HAS_STAR_CELLS_U);
        self.set_flags(has_star_cells_v, FLAG_HAS_STAR_CELLS_V);
        self.set_flags(has_group3_cells_in_auto_rows, FLAG_HAS_GROUP3_CELLS_IN_AUTO_ROWS);
    }

    /// Initializes the U definitions either to the user supplied collection
    /// of column definitions or to a default single element collection.
    ///
    /// This is one of two methods where the column definitions and the U
    /// definitions are directly accessed. All the rest of the measure /
    /// arrange code must use the U definitions.
    fn validate_definitions_u_structure(&self) {
        if self.column_definitions_dirty() {
            let ext_data = self.ext();
            let column_definitions = ext_data.column_definitions.borrow().clone();

            match column_definitions {
                None => {
                    if ext_data.definitions_u.borrow().is_none() {
                        *ext_data.definitions_u.borrow_mut() = Some(self.mockup_column());
                    }
                }
                Some(column_definitions) => {
                    if column_definitions.count() == 0 {
                        //  If the column definitions collection is empty,
                        //  mockup an array with one column.
                        *ext_data.definitions_u.borrow_mut() = Some(self.mockup_column());
                    } else {
                        *ext_data.definitions_u.borrow_mut() = Some(DefinitionsSource::Columns(column_definitions));
                    }
                }
            }

            self.set_column_definitions_dirty(false);
        }

        debug_assert!(self.definitions_u().count() > 0);
    }

    /// Initializes the V definitions either to the user supplied collection
    /// of row definitions or to a default single element collection.
    ///
    /// This is one of two methods where the row definitions and the V
    /// definitions are directly accessed. All the rest of the measure /
    /// arrange code must use the V definitions.
    fn validate_definitions_v_structure(&self) {
        if self.row_definitions_dirty() {
            let ext_data = self.ext();
            let row_definitions = ext_data.row_definitions.borrow().clone();

            match row_definitions {
                None => {
                    if ext_data.definitions_v.borrow().is_none() {
                        *ext_data.definitions_v.borrow_mut() = Some(self.mockup_row());
                    }
                }
                Some(row_definitions) => {
                    if row_definitions.count() == 0 {
                        //  If the row definitions collection is empty,
                        //  mockup an array with one row.
                        *ext_data.definitions_v.borrow_mut() = Some(self.mockup_row());
                    } else {
                        *ext_data.definitions_v.borrow_mut() = Some(DefinitionsSource::Rows(row_definitions));
                    }
                }
            }

            self.set_row_definitions_dirty(false);
        }

        debug_assert!(self.definitions_v().count() > 0);
    }

    fn mockup_column(&self) -> DefinitionsSource {
        let definition = ColumnDefinition::new();
        definition.set_parent(Some(&self.to_ref()));
        DefinitionsSource::Single(definition.upcast())
    }

    fn mockup_row(&self) -> DefinitionsSource {
        let definition = RowDefinition::new();
        definition.set_parent(Some(&self.to_ref()));
        DefinitionsSource::Single(definition.upcast())
    }

    /// Validates the layout time size type information on the given array of
    /// definitions. Sets the minimum size and the measure sizes.
    ///
    /// If `treat_star_as_auto` is true then star definitions are treated as
    /// Auto.
    fn validate_definitions_layout(&self, definitions: &Definitions, treat_star_as_auto: bool) {
        for i in 0..definitions.count() {
            let definition = &definitions[i];
            definition.on_before_layout(self);

            let mut user_min_size = definition.user_min_size();
            let user_max_size = definition.user_max_size();
            let user_size;

            let definition_user_size = definition.user_size();
            match definition_user_size.grid_unit_type() {
                GridUnitType::Pixel => {
                    definition.set_size_type(LayoutTimeSizeType::PIXEL);
                    user_size = definition_user_size.value();
                    // This was brought with NewLayout and defeats the squishy
                    // behavior.
                    user_min_size = math_max(user_min_size, math_min(user_size, user_max_size));
                }
                GridUnitType::Auto => {
                    definition.set_size_type(LayoutTimeSizeType::AUTO);
                    user_size = f64::INFINITY;
                }
                GridUnitType::Star => {
                    if treat_star_as_auto {
                        definition.set_size_type(LayoutTimeSizeType::AUTO);
                        user_size = f64::INFINITY;
                    } else {
                        definition.set_size_type(LayoutTimeSizeType::STAR);
                        user_size = f64::INFINITY;
                    }
                }
            }

            definition.update_min_size(user_min_size);
            definition.set_measure_size(math_max(user_min_size, math_min(user_size, user_max_size)));
        }
    }

    fn cache_min_sizes(&self, cells_head: usize, is_rows: bool) -> Vec<f64> {
        let definitions = if is_rows {
            self.definitions_v()
        } else {
            self.definitions_u()
        };
        let mut min_sizes = vec![-1.0; definitions.count()];

        let cells_len = self.private_cells_len();
        let mut i = cells_head;
        loop {
            let cell = self.private_cell(i);
            if is_rows {
                min_sizes[cell.row_index] = definitions[cell.row_index].raw_min_size();
            } else {
                min_sizes[cell.column_index] = definitions[cell.column_index].raw_min_size();
            }

            i = cell.next;
            if i >= cells_len {
                break;
            }
        }

        min_sizes
    }

    fn apply_cached_min_sizes(&self, min_sizes: &[f64], is_rows: bool) {
        let definitions = if is_rows {
            self.definitions_v()
        } else {
            self.definitions_u()
        };
        for (i, min_size) in min_sizes.iter().enumerate() {
            if MathUtilities::greater_than_or_close(*min_size, 0.0) {
                definitions[i].set_min_size(*min_size);
            }
        }
    }

    /// Measures one group of cells.
    ///
    /// * `cells_head`: the head index of the cells chain.
    /// * `ignore_desired_size_u`: when true the desired width of the cells is
    ///   not registered in the columns.
    /// * `force_infinity_v`: passed through to `measure_cell`. When true the
    ///   desired height of the cells is not registered in the rows.
    ///
    /// Returns whether the desired width of a cell has changed.
    fn measure_cells_group(&self, cells_head: usize, ignore_desired_size_u: bool, force_infinity_v: bool) -> bool {
        let mut has_desired_size_u_changed = false;

        let cells_len = self.private_cells_len();
        if cells_head >= cells_len {
            return has_desired_size_u_changed;
        }

        let children = self.children().snapshot();
        let definitions_u = self.definitions_u();
        let definitions_v = self.definitions_v();
        let mut span_store: Option<Vec<(SpanKey, f64)>> = None;
        let ignore_desired_size_v = force_infinity_v;

        let mut i = cells_head;
        loop {
            let child = &children[i];
            let cell = self.private_cell(i);
            let old_width = child.desired_size().width;

            self.measure_cell(&definitions_u, &definitions_v, child, &cell, force_infinity_v);

            let desired_size = child.desired_size();
            has_desired_size_u_changed |= !MathUtilities::are_close(old_width, desired_size.width);

            if !ignore_desired_size_u {
                if cell.column_span == 1 {
                    let definition = &definitions_u[cell.column_index];
                    definition.update_min_size(math_min(desired_size.width, definition.user_max_size()));
                } else {
                    Self::register_span(
                        &mut span_store,
                        cell.column_index,
                        cell.column_span,
                        true,
                        desired_size.width,
                    );
                }
            }

            if !ignore_desired_size_v {
                if cell.row_span == 1 {
                    let definition = &definitions_v[cell.row_index];
                    definition.update_min_size(math_min(desired_size.height, definition.user_max_size()));
                } else {
                    Self::register_span(
                        &mut span_store,
                        cell.row_index,
                        cell.row_span,
                        false,
                        desired_size.height,
                    );
                }
            }

            i = cell.next;
            if i >= cells_len {
                break;
            }
        }

        if let Some(span_store) = span_store {
            for (key, desired_size) in span_store {
                self.ensure_min_size_in_definition_range(
                    if key.u { &definitions_u } else { &definitions_v },
                    key.start,
                    key.count,
                    if key.u {
                        self.column_spacing()
                    } else {
                        self.row_spacing()
                    },
                    desired_size,
                );
            }
        }

        has_desired_size_u_changed
    }

    /// Registers a span information for delayed processing. If an entry
    /// already exists the biggest value is stored.
    ///
    /// `u` is true if this is a column span and false if this is a row span.
    fn register_span(store: &mut Option<Vec<(SpanKey, f64)>>, start: usize, count: usize, u: bool, value: f64) {
        let store = store.get_or_insert_with(Vec::new);

        let key = SpanKey { start, count, u };
        match store.iter_mut().find(|(k, _)| *k == key) {
            Some((_, o)) => {
                if value > *o {
                    *o = value;
                }
            }
            None => store.push((key, value)),
        }
    }

    /// Takes care of measuring a single cell.
    ///
    /// If `force_infinity_v` is true then the cell is always calculated to
    /// an infinite height.
    fn measure_cell(
        &self,
        definitions_u: &Definitions,
        definitions_v: &Definitions,
        child: &Control,
        cell: &CellCache,
        force_infinity_v: bool,
    ) {
        let cell_measure_width = if cell.is_auto_u() && !cell.is_star_u() {
            //  If the cell belongs to at least one Auto column and not a
            //  single Star column then it should be calculated "to content",
            //  thus it is possible to "shortcut" the calculations and simply
            //  assign infinity here.
            f64::INFINITY
        } else {
            //  Otherwise...
            Self::get_measure_size_for_range(
                definitions_u,
                cell.column_index,
                cell.column_span,
                self.column_spacing(),
            )
        };

        let cell_measure_height = if force_infinity_v {
            f64::INFINITY
        } else if cell.is_auto_v() && !cell.is_star_v() {
            //  If the cell belongs to at least one Auto row and not a single
            //  Star row then it should be calculated "to content", thus it is
            //  possible to "shortcut" the calculations and simply assign
            //  infinity here.
            f64::INFINITY
        } else {
            Self::get_measure_size_for_range(definitions_v, cell.row_index, cell.row_span, self.row_spacing())
        };

        let child_constraint = Size::new(cell_measure_width, cell_measure_height);
        child.measure(child_constraint);
    }

    /// Calculates the one dimensional measure size of the given range of
    /// definitions.
    ///
    /// For "Auto" definitions the minimum size is used in place of the
    /// preferred size.
    fn get_measure_size_for_range(definitions: &Definitions, start: usize, count: usize, spacing: f64) -> f64 {
        debug_assert!(0 < count && (start + count) <= definitions.count());

        let mut measure_size = -spacing;

        for i in (start..start + count).rev() {
            let definition = &definitions[i];
            measure_size += spacing
                + if definition.size_type() == LayoutTimeSizeType::AUTO {
                    definition.min_size()
                } else {
                    definition.measure_size()
                };
        }

        measure_size
    }

    /// Accumulates the length type information of the given range of
    /// definitions.
    fn get_length_type_for_range(definitions: &Definitions, start: usize, count: usize) -> LayoutTimeSizeType {
        debug_assert!(0 < count && (start + count) <= definitions.count());

        let mut length_type = LayoutTimeSizeType::NONE;

        for i in (start..start + count).rev() {
            length_type |= definitions[i].size_type();
        }

        length_type
    }

    /// Distributes a minimum size back to a range of an array of
    /// definitions.
    ///
    /// `desired_size` is the minimum size that should "fit" into the
    /// definitions range.
    fn ensure_min_size_in_definition_range(
        &self,
        definitions: &Definitions,
        start: usize,
        count: usize,
        spacing: f64,
        desired_size: f64,
    ) {
        debug_assert!(1 < count && (start + count) <= definitions.count());

        // The spacing between the definitions that this element spans through
        // must not be distributed.
        let requested_size = math_max(desired_size - spacing * (count as f64 - 1.0), 0.0);

        //  Avoid processing when asked to distribute "0".
        if MathUtilities::is_zero(requested_size) {
            return;
        }

        //  The temp array is used to remember definitions for sorting.
        let mut temp_definitions = Self::take_temp_definitions(count);
        let end = start + count;
        let mut auto_definitions_count = 0;
        let mut range_min_size = 0.0;
        let mut range_preferred_size = 0.0;
        let mut range_max_size = 0.0;
        let mut max_max_size = 0.0; //  maximum of maximum sizes

        //  First accumulate the necessary information:
        //  a) sum up the sizes in the range;
        //  b) count the number of auto definitions in the range;
        //  c) initialize the temp array
        //  d) cache the maximum size into the size cache
        //  e) accumulate the max of max sizes
        for i in start..end {
            let definition = &definitions[i];
            let min_size = definition.min_size();
            let preferred_size = definition.preferred_size();
            let max_size = math_max(definition.user_max_size(), min_size);

            range_min_size += min_size;
            range_preferred_size += preferred_size;
            range_max_size += max_size;

            definition.set_size_cache(max_size);

            //  Sanity check: no matter what, but the min size must always be
            //  the smaller; the max size must be the biggest; and the
            //  preferred should be in between.
            debug_assert!(
                min_size <= preferred_size
                    && preferred_size <= max_size
                    && range_min_size <= range_preferred_size
                    && range_preferred_size <= range_max_size
            );

            if max_max_size < max_size {
                max_max_size = max_size;
            }
            if definition.user_size().is_auto() {
                auto_definitions_count += 1;
            }
            temp_definitions[i - start] = i as i32;
        }

        //  Avoid processing if the range is already big enough.
        if requested_size > range_min_size {
            if requested_size <= range_preferred_size {
                //
                //  The requested size fits into the preferred size of the range.
                //  Distribute according to the following logic:
                //  * do not distribute into auto definitions - they should
                //    continue to stay "tight";
                //  * for all non-auto definitions distribute to equi-size min
                //    sizes, without exceeding the preferred size.
                //
                //  In order to achieve that, definitions are sorted in a way
                //  that all auto definitions are first, then definitions
                //  follow ascending order with the preferred size as the key
                //  of sorting.
                //
                temp_definitions[..count].sort_by(|x, y| {
                    Self::span_preferred_distribution_order(&definitions[*x as usize], &definitions[*y as usize])
                });

                let mut size_to_distribute = requested_size;
                let mut i = 0;
                while i < auto_definitions_count {
                    let temp_definition = &definitions[temp_definitions[i] as usize];

                    //  Sanity check: only auto definitions allowed in this loop.
                    debug_assert!(temp_definition.user_size().is_auto());

                    //  Adjust the size to distribute by subtracting the auto
                    //  definition min size.
                    size_to_distribute -= temp_definition.min_size();
                    i += 1;
                }

                while i < count {
                    let temp_definition = &definitions[temp_definitions[i] as usize];

                    //  Sanity check: no auto definitions allowed in this loop.
                    debug_assert!(!temp_definition.user_size().is_auto());

                    let new_min_size = math_min(
                        size_to_distribute / (count - i) as f64,
                        temp_definition.preferred_size(),
                    );
                    if new_min_size > temp_definition.min_size() {
                        temp_definition.update_min_size(new_min_size);
                    }
                    size_to_distribute -= new_min_size;
                    i += 1;
                }

                //  Sanity check: the requested size must all be distributed.
                debug_assert!(MathUtilities::is_zero(size_to_distribute));
            } else if requested_size <= range_max_size {
                //
                //  The requested size is bigger than the preferred size, but
                //  fits into the max size of the range. Distribute according
                //  to the following logic:
                //  * do not distribute into auto definitions, if possible -
                //    they should continue to stay "tight";
                //  * for all non-auto definitions distribute to equi-size min
                //    sizes, without exceeding the max size.
                //
                //  In order to achieve that, definitions are sorted in a way
                //  that all non-auto definitions are last, then definitions
                //  follow ascending order with the max size as the key of
                //  sorting.
                //
                temp_definitions[..count].sort_by(|x, y| {
                    Self::span_max_distribution_order(&definitions[*x as usize], &definitions[*y as usize])
                });

                let mut size_to_distribute = requested_size - range_preferred_size;
                let mut i = 0;
                while i < count - auto_definitions_count {
                    let temp_definition = &definitions[temp_definitions[i] as usize];

                    //  Sanity check: no auto definitions allowed in this loop.
                    debug_assert!(!temp_definition.user_size().is_auto());

                    let preferred_size = temp_definition.preferred_size();
                    let new_min_size =
                        preferred_size + size_to_distribute / (count - auto_definitions_count - i) as f64;
                    temp_definition.update_min_size(math_min(new_min_size, temp_definition.size_cache()));
                    size_to_distribute -= temp_definition.min_size() - preferred_size;
                    i += 1;
                }

                while i < count {
                    let temp_definition = &definitions[temp_definitions[i] as usize];

                    //  Sanity check: only auto definitions allowed in this loop.
                    debug_assert!(temp_definition.user_size().is_auto());

                    let preferred_size = temp_definition.min_size();
                    let new_min_size = preferred_size + size_to_distribute / (count - i) as f64;
                    temp_definition.update_min_size(math_min(new_min_size, temp_definition.size_cache()));
                    size_to_distribute -= temp_definition.min_size() - preferred_size;
                    i += 1;
                }

                //  Sanity check: the requested size must all be distributed.
                debug_assert!(MathUtilities::is_zero(size_to_distribute));
            } else {
                //
                //  The requested size is bigger than the max size of the range.
                //  Distribute according to the following logic:
                //  * for all definitions distribute to equi-size min sizes.
                //
                let equal_size = requested_size / count as f64;

                if equal_size < max_max_size && !MathUtilities::are_close(equal_size, max_max_size) {
                    //  The equi-size is less than the maximum of max sizes.
                    //  In this case distribute so that smaller definitions
                    //  grow faster than bigger ones.
                    let total_remaining_size = max_max_size * count as f64 - range_max_size;
                    let size_to_distribute = requested_size - range_max_size;

                    //  Sanity check: the total remaining size and the size to
                    //  distribute must be real positive numbers.
                    debug_assert!(
                        total_remaining_size.is_finite()
                            && total_remaining_size > 0.0
                            && size_to_distribute.is_finite()
                            && size_to_distribute > 0.0
                    );

                    for temp_definition in temp_definitions[..count].iter() {
                        let temp_definition = &definitions[*temp_definition as usize];

                        let delta_size =
                            (max_max_size - temp_definition.size_cache()) * size_to_distribute / total_remaining_size;
                        temp_definition.update_min_size(temp_definition.size_cache() + delta_size);
                    }
                } else {
                    //
                    //  The equi-size is greater or equal to the maximum of max
                    //  sizes. All definitions receive the equal size as their
                    //  min sizes.
                    //
                    for temp_definition in temp_definitions[..count].iter() {
                        definitions[*temp_definition as usize].update_min_size(equal_size);
                    }
                }
            }
        }

        Self::return_temp_definitions(temp_definitions);
    }

    /// Resolves the stars of the given array of definitions.
    ///
    /// Must initialize the layout size for all Star entries in the given
    /// array of definitions.
    fn resolve_star(&self, definitions: &Definitions, available_size: f64) {
        self.resolve_star_max_discrepancy(definitions, available_size);
    }

    // New implementation as of 4.7. Several improvements:
    // 1. Allocate to *-defs hitting their min or max constraints, before allocating
    //      to other *-defs.  A def that hits its min uses more space than its
    //      proportional share, reducing the space available to everyone else.
    //      The legacy algorithm deducted this space only from defs processed
    //      after the min;  the new algorithm deducts it proportionally from all
    //      defs.   This avoids the "*-defs exceed available space" problem,
    //      and other related problems where *-defs don't receive proportional
    //      allocations even though no constraints are preventing it.
    // 2. When multiple defs hit min or max, resolve the one with maximum
    //      discrepancy (defined below).   This avoids discontinuities - small
    //      change in available space resulting in large change to one def's allocation.
    // 3. Correct handling of large *-values, including Infinity.
    fn resolve_star_max_discrepancy(&self, definitions: &Definitions, available_size: f64) {
        let def_count = definitions.count();
        let mut temp_definitions = Self::take_temp_definitions(def_count * 2);
        let mut min_count;
        let mut max_count;
        let mut taken_size = 0.0;
        let mut total_star_weight;
        let mut star_count = 0; // number of unresolved *-definitions
        let mut scale = 1.0; // scale factor applied to each *-weight;  negative means "Infinity is present"

        // Phase 1.  Determine the maximum *-weight and prepare to adjust *-weights
        let mut max_star = 0.0;
        for i in 0..def_count {
            let def = &definitions[i];

            if def.size_type() == LayoutTimeSizeType::STAR {
                star_count += 1;
                def.set_measure_size(1.0); // meaning "not yet resolved in phase 3"
                let value = def.user_size().value();
                if value > max_star {
                    max_star = value;
                }
            }
        }

        if max_star == f64::INFINITY {
            // negative scale means one or more of the weights was Infinity
            scale = -1.0;
        } else if star_count > 0 {
            // if maxStar * starCount > Double.Max, summing all the weights could cause
            // floating-point overflow.  To avoid that, scale the weights by a factor to keep
            // the sum within limits.  Choose a power of 2, to preserve precision.
            let power = log2(f64::MAX / max_star / star_count as f64).floor();
            if power < 0.0 {
                scale = 2.0_f64.powf(power - 4.0); // -4 is just for paranoia
            }
        }

        // normally Phases 2 and 3 execute only once.  But certain unusual combinations of weights
        // and constraints can defeat the algorithm, in which case we repeat Phases 2 and 3.
        // More explanation below...
        let mut run_phase_2_and_3 = true;
        while run_phase_2_and_3 {
            // Phase 2.   Compute total *-weight W and available space S.
            // For *-items that have Min or Max constraints, compute the ratios used to decide
            // whether proportional space is too big or too small and add the item to the
            // corresponding list.  (The "min" list is in the first half of the temp definitions,
            // the "max" list in the second half.  The temp definitions have capacity at least
            // 2*defCount, so there's room for both lists.)
            total_star_weight = 0.0;
            taken_size = 0.0;
            min_count = 0;
            max_count = 0;

            for i in 0..def_count {
                let def = &definitions[i];

                let size_type = def.size_type();
                if size_type == LayoutTimeSizeType::AUTO {
                    taken_size += def.min_size();
                } else if size_type == LayoutTimeSizeType::PIXEL {
                    taken_size += def.measure_size();
                } else if size_type == LayoutTimeSizeType::STAR {
                    if def.measure_size() < 0.0 {
                        taken_size += -def.measure_size(); // already resolved
                    } else {
                        let star_weight = Self::star_weight(def, scale);
                        total_star_weight += star_weight;

                        if def.min_size() > 0.0 {
                            // store ratio w/min in MeasureSize (for now)
                            temp_definitions[min_count] = i as i32;
                            min_count += 1;
                            def.set_measure_size(star_weight / def.min_size());
                        }

                        let effective_max_size = math_max(def.min_size(), def.user_max_size());
                        if effective_max_size != f64::INFINITY {
                            // store ratio w/max in SizeCache (for now)
                            temp_definitions[def_count + max_count] = i as i32;
                            max_count += 1;
                            def.set_size_cache(star_weight / effective_max_size);
                        }
                    }
                }
            }

            // Phase 3.  Resolve *-items whose proportional sizes are too big or too small.
            let min_count_phase2 = min_count;
            let max_count_phase2 = max_count;
            let mut taken_star_weight = 0.0;
            let mut remaining_available_size = available_size - taken_size;
            let mut remaining_star_weight = total_star_weight - taken_star_weight;
            // Sort by w/min (stored in MeasureSize), descending. The list is
            // queried from the back, i.e. in ascending order of w/min.
            temp_definitions[..min_count].sort_by(|x, y| {
                compare_to(
                    definitions[*y as usize].measure_size(),
                    definitions[*x as usize].measure_size(),
                )
            });
            // Sort by w/max (stored in SizeCache), ascending. The list is
            // queried from the back, i.e. in descending order of w/max.
            temp_definitions[def_count..def_count + max_count].sort_by(|x, y| {
                compare_to(
                    definitions[*x as usize].size_cache(),
                    definitions[*y as usize].size_cache(),
                )
            });

            while min_count + max_count > 0 && remaining_available_size > 0.0 {
                // the calculation
                //            remainingStarWeight = totalStarWeight - takenStarWeight
                // is subject to catastrophic cancellation if the two terms are nearly equal,
                // which leads to meaningless results.   Check for that, and recompute from
                // the remaining definitions.   [This leads to quadratic behavior in really
                // pathological cases - but they'd never arise in practice.]
                const STAR_FACTOR: f64 = 1.0 / 256.0; // lose more than 8 bits of precision -> recalculate
                if remaining_star_weight < total_star_weight * STAR_FACTOR {
                    taken_star_weight = 0.0;
                    total_star_weight = 0.0;

                    for i in 0..def_count {
                        let def = &definitions[i];
                        if def.size_type() == LayoutTimeSizeType::STAR && def.measure_size() > 0.0 {
                            total_star_weight += Self::star_weight(def, scale);
                        }
                    }

                    remaining_star_weight = total_star_weight - taken_star_weight;
                }

                let min_ratio = if min_count > 0 {
                    definitions[temp_definitions[min_count - 1] as usize].measure_size()
                } else {
                    f64::INFINITY
                };
                let max_ratio = if max_count > 0 {
                    definitions[temp_definitions[def_count + max_count - 1] as usize].size_cache()
                } else {
                    -1.0
                };

                // choose the def with larger ratio to the current proportion ("max discrepancy")
                let proportion = remaining_star_weight / remaining_available_size;
                let choose_min = Self::choose(min_ratio, max_ratio, proportion);

                // if no def was chosen, advance to phase 4;  the current proportion doesn't
                // conflict with any min or max values.
                let Some(choose_min) = choose_min else {
                    break;
                };

                // get the chosen definition and its resolved size
                let resolved_def;
                let resolved_size;
                if choose_min {
                    resolved_def = &definitions[temp_definitions[min_count - 1] as usize];
                    resolved_size = resolved_def.min_size();
                    min_count -= 1;
                } else {
                    resolved_def = &definitions[temp_definitions[def_count + max_count - 1] as usize];
                    resolved_size = math_max(resolved_def.min_size(), resolved_def.user_max_size());
                    max_count -= 1;
                }

                // resolve the chosen def, deduct its contributions from W and S.
                // Defs resolved in phase 3 are marked by storing the negative of their resolved
                // size in MeasureSize, to distinguish them from a pending def.
                taken_size += resolved_size;
                resolved_def.set_measure_size(-resolved_size);
                taken_star_weight += Self::star_weight(resolved_def, scale);
                star_count -= 1;

                remaining_available_size = available_size - taken_size;
                remaining_star_weight = total_star_weight - taken_star_weight;

                // advance to the next candidate defs, removing ones that have been resolved.
                // Both counts are advanced, as a def might appear in both lists.
                while min_count > 0 && definitions[temp_definitions[min_count - 1] as usize].measure_size() < 0.0 {
                    min_count -= 1;
                    temp_definitions[min_count] = -1;
                }
                while max_count > 0
                    && definitions[temp_definitions[def_count + max_count - 1] as usize].measure_size() < 0.0
                {
                    max_count -= 1;
                    temp_definitions[def_count + max_count] = -1;
                }
            }

            // decide whether to run Phase2 and Phase3 again.  There are 3 cases:
            // 1. There is space available, and *-defs remaining.  This is the
            //      normal case - move on to Phase 4 to allocate the remaining
            //      space proportionally to the remaining *-defs.
            // 2. There is space available, but no *-defs.  This implies at least one
            //      def was resolved as 'max', taking less space than its proportion.
            //      If there are also 'min' defs, reconsider them - we can give
            //      them more space.   If not, all the *-defs are 'max', so there's
            //      no way to use all the available space.
            // 3. We allocated too much space.   This implies at least one def was
            //      resolved as 'min'.  If there are also 'max' defs, reconsider
            //      them, otherwise the over-allocation is an inevitable consequence
            //      of the given min constraints.
            // Note that if we return to Phase2, at least one *-def will have been
            // resolved.  This guarantees we don't run Phase2+3 infinitely often.
            run_phase_2_and_3 = false;
            if star_count == 0 && taken_size < available_size {
                // if no *-defs remain and we haven't allocated all the space, reconsider the defs
                // resolved as 'min'.   Their allocation can be increased to make up the gap.
                for i in min_count..min_count_phase2 {
                    if temp_definitions[i] >= 0 {
                        let def = &definitions[temp_definitions[i] as usize];
                        def.set_measure_size(1.0); // mark as 'not yet resolved'
                        star_count += 1;
                        run_phase_2_and_3 = true; // found a candidate, so re-run Phases 2 and 3
                    }
                }
            }

            if taken_size > available_size {
                // if we've allocated too much space, reconsider the defs
                // resolved as 'max'.   Their allocation can be decreased to make up the gap.
                for i in max_count..max_count_phase2 {
                    if temp_definitions[def_count + i] >= 0 {
                        let def = &definitions[temp_definitions[def_count + i] as usize];
                        def.set_measure_size(1.0); // mark as 'not yet resolved'
                        star_count += 1;
                        run_phase_2_and_3 = true; // found a candidate, so re-run Phases 2 and 3
                    }
                }
            }
        }

        // Phase 4.  Resolve the remaining defs proportionally.
        let mut star_count = 0;
        for i in 0..def_count {
            let def = &definitions[i];

            if def.size_type() == LayoutTimeSizeType::STAR {
                if def.measure_size() < 0.0 {
                    // this def was resolved in phase 3 - fix up its measure size
                    def.set_measure_size(-def.measure_size());
                } else {
                    // this def needs resolution, add it to the list, sorted by *-weight
                    temp_definitions[star_count] = i as i32;
                    star_count += 1;
                    def.set_measure_size(Self::star_weight(def, scale));
                }
            }
        }

        if star_count > 0 {
            // Sort by *-weight (stored in MeasureSize), ascending.
            temp_definitions[..star_count].sort_by(|x, y| {
                compare_to(
                    definitions[*x as usize].measure_size(),
                    definitions[*y as usize].measure_size(),
                )
            });

            // compute the partial sums of *-weight, in increasing order of weight
            // for minimal loss of precision.
            let mut total_star_weight = 0.0;
            for i in 0..star_count {
                let def = &definitions[temp_definitions[i] as usize];
                total_star_weight += def.measure_size();
                def.set_size_cache(total_star_weight);
            }

            // resolve the defs, in decreasing order of weight
            for i in (0..star_count).rev() {
                let def = &definitions[temp_definitions[i] as usize];
                let mut resolved_size = if def.measure_size() > 0.0 {
                    math_max(available_size - taken_size, 0.0) * (def.measure_size() / def.size_cache())
                } else {
                    0.0
                };

                // min and max should have no effect by now, but just in case...
                resolved_size = math_min(resolved_size, def.user_max_size());
                resolved_size = math_max(def.min_size(), resolved_size);

                def.set_measure_size(resolved_size);
                taken_size += resolved_size;
            }
        }

        Self::return_temp_definitions(temp_definitions);
    }

    /// Calculates the desired size of the given array of definitions.
    fn calculate_desired_size(definitions: &Definitions) -> f64 {
        let mut desired_size = 0.0;

        for i in 0..definitions.count() {
            desired_size += definitions[i].min_size();
        }

        desired_size
    }

    /// Calculates and sets the final size of all definitions in the given
    /// array.
    ///
    /// `columns` is true if sizing column definitions and false for rows.
    fn set_final_size(&self, definitions: &Definitions, final_size: f64, columns: bool) {
        self.set_final_size_max_discrepancy(definitions, final_size, columns);
    }

    // new implementation, as of 4.7.  This incorporates the same algorithm
    // as in ResolveStarMaxDiscrepancy.  It differs in the same way that SetFinalSizeLegacy
    // differs from ResolveStarLegacy, namely (a) leaves results in def.SizeCache
    // instead of def.MeasureSize, (b) implements LayoutRounding if requested,
    // (c) stores intermediate results differently.
    // The LayoutRounding logic is improved:
    // 1. Use pre-rounded values during proportional allocation.  This avoids the
    //      same kind of problems arising from interaction with min/max that
    //      motivated the new algorithm in the first place.
    // 2. Use correct "nudge" amount when distributing roundoff space.   This
    //      comes into play at high DPI - greater than 134.
    // 3. Applies rounding only to real pixel values (not to ratios)
    fn set_final_size_max_discrepancy(&self, definitions: &Definitions, final_size: f64, columns: bool) {
        let def_count = definitions.count();
        let mut definition_indices = self.take_definition_indices();
        let mut min_count;
        let mut max_count;
        let mut taken_size = 0.0;
        let mut total_star_weight;
        let mut star_count = 0; // number of unresolved *-definitions
        let mut scale = 1.0; // scale factor applied to each *-weight;  negative means "Infinity is present"

        // Phase 1.  Determine the maximum *-weight and prepare to adjust *-weights
        let mut max_star = 0.0;
        for i in 0..def_count {
            let def = &definitions[i];

            let user_size = def.user_size();
            if user_size.is_star() {
                star_count += 1;
                def.set_measure_size(1.0); // meaning "not yet resolved in phase 3"
                if user_size.value() > max_star {
                    max_star = user_size.value();
                }
            }
        }

        if max_star == f64::INFINITY {
            // negative scale means one or more of the weights was Infinity
            scale = -1.0;
        } else if star_count > 0 {
            // if maxStar * starCount > Double.Max, summing all the weights could cause
            // floating-point overflow.  To avoid that, scale the weights by a factor to keep
            // the sum within limits.  Choose a power of 2, to preserve precision.
            let power = log2(f64::MAX / max_star / star_count as f64).floor();
            if power < 0.0 {
                scale = 2.0_f64.powf(power - 4.0); // -4 is just for paranoia
            }
        }

        // normally Phases 2 and 3 execute only once.  But certain unusual combinations of weights
        // and constraints can defeat the algorithm, in which case we repeat Phases 2 and 3.
        // More explanation below...
        let mut run_phase_2_and_3 = true;
        while run_phase_2_and_3 {
            // Phase 2.   Compute total *-weight W and available space S.
            // For *-items that have Min or Max constraints, compute the ratios used to decide
            // whether proportional space is too big or too small and add the item to the
            // corresponding list.  (The "min" list is in the first half of definitionIndices,
            // the "max" list in the second half.  DefinitionIndices has capacity at least
            // 2*defCount, so there's room for both lists.)
            total_star_weight = 0.0;
            taken_size = 0.0;
            min_count = 0;
            max_count = 0;

            for i in 0..def_count {
                let def = &definitions[i];

                let def_user_size = def.user_size();
                if def_user_size.is_star() {
                    debug_assert!(!def.is_shared(), "*-defs cannot be shared");

                    if def.measure_size() < 0.0 {
                        taken_size += -def.measure_size(); // already resolved
                    } else {
                        let star_weight = Self::star_weight(def, scale);
                        total_star_weight += star_weight;

                        if def.min_size_for_arrange() > 0.0 {
                            // store ratio w/min in MeasureSize (for now)
                            definition_indices[min_count] = i as i32;
                            min_count += 1;
                            def.set_measure_size(star_weight / def.min_size_for_arrange());
                        }

                        let effective_max_size = math_max(def.min_size_for_arrange(), def.user_max_size());
                        if effective_max_size != f64::INFINITY {
                            // store ratio w/max in SizeCache (for now)
                            definition_indices[def_count + max_count] = i as i32;
                            max_count += 1;
                            def.set_size_cache(star_weight / effective_max_size);
                        }
                    }
                } else {
                    let user_size = match def_user_size.grid_unit_type() {
                        GridUnitType::Pixel => def_user_size.value(),
                        GridUnitType::Auto => def.min_size_for_arrange(),
                        GridUnitType::Star => 0.0,
                    };

                    let user_max_size = if def.is_shared() {
                        //  overriding userMaxSize effectively prevents squishy-ness.
                        //  this is a "solution" to avoid shared definitions from been sized to
                        //  different final size at arrange time, if / when different grids receive
                        //  different final sizes.
                        user_size
                    } else {
                        def.user_max_size()
                    };

                    def.set_size_cache(math_max(def.min_size_for_arrange(), math_min(user_size, user_max_size)));
                    taken_size += def.size_cache();
                }
            }

            // Phase 3.  Resolve *-items whose proportional sizes are too big or too small.
            let min_count_phase2 = min_count;
            let max_count_phase2 = max_count;
            let mut taken_star_weight = 0.0;
            let mut remaining_available_size = final_size - taken_size;
            let mut remaining_star_weight = total_star_weight - taken_star_weight;

            definition_indices[..min_count].sort_by(|x, y| {
                compare_to(
                    definitions[*y as usize].measure_size(),
                    definitions[*x as usize].measure_size(),
                )
            });
            definition_indices[def_count..def_count + max_count].sort_by(|x, y| {
                compare_to(
                    definitions[*x as usize].size_cache(),
                    definitions[*y as usize].size_cache(),
                )
            });

            while min_count + max_count > 0 && remaining_available_size > 0.0 {
                // the calculation
                //            remainingStarWeight = totalStarWeight - takenStarWeight
                // is subject to catastrophic cancellation if the two terms are nearly equal,
                // which leads to meaningless results.   Check for that, and recompute from
                // the remaining definitions.   [This leads to quadratic behavior in really
                // pathological cases - but they'd never arise in practice.]
                const STAR_FACTOR: f64 = 1.0 / 256.0; // lose more than 8 bits of precision -> recalculate
                if remaining_star_weight < total_star_weight * STAR_FACTOR {
                    taken_star_weight = 0.0;
                    total_star_weight = 0.0;

                    for i in 0..def_count {
                        let def = &definitions[i];
                        if def.user_size().is_star() && def.measure_size() > 0.0 {
                            total_star_weight += Self::star_weight(def, scale);
                        }
                    }

                    remaining_star_weight = total_star_weight - taken_star_weight;
                }

                let min_ratio = if min_count > 0 {
                    definitions[definition_indices[min_count - 1] as usize].measure_size()
                } else {
                    f64::INFINITY
                };
                let max_ratio = if max_count > 0 {
                    definitions[definition_indices[def_count + max_count - 1] as usize].size_cache()
                } else {
                    -1.0
                };

                // choose the def with larger ratio to the current proportion ("max discrepancy")
                let proportion = remaining_star_weight / remaining_available_size;
                let choose_min = Self::choose(min_ratio, max_ratio, proportion);

                // if no def was chosen, advance to phase 4;  the current proportion doesn't
                // conflict with any min or max values.
                let Some(choose_min) = choose_min else {
                    break;
                };

                // get the chosen definition and its resolved size
                let resolved_index;
                let resolved_def;
                let resolved_size;
                if choose_min {
                    resolved_index = definition_indices[min_count - 1] as usize;
                    resolved_def = &definitions[resolved_index];
                    resolved_size = resolved_def.min_size_for_arrange();
                    min_count -= 1;
                } else {
                    resolved_index = definition_indices[def_count + max_count - 1] as usize;
                    resolved_def = &definitions[resolved_index];
                    resolved_size = math_max(resolved_def.min_size_for_arrange(), resolved_def.user_max_size());
                    max_count -= 1;
                }

                // resolve the chosen def, deduct its contributions from W and S.
                // Defs resolved in phase 3 are marked by storing the negative of their resolved
                // size in MeasureSize, to distinguish them from a pending def.
                taken_size += resolved_size;
                resolved_def.set_measure_size(-resolved_size);
                taken_star_weight += Self::star_weight(resolved_def, scale);
                star_count -= 1;

                remaining_available_size = final_size - taken_size;
                remaining_star_weight = total_star_weight - taken_star_weight;

                // advance to the next candidate defs, removing ones that have been resolved.
                // Both counts are advanced, as a def might appear in both lists.
                while min_count > 0 && definitions[definition_indices[min_count - 1] as usize].measure_size() < 0.0 {
                    min_count -= 1;
                    definition_indices[min_count] = -1;
                }
                while max_count > 0
                    && definitions[definition_indices[def_count + max_count - 1] as usize].measure_size() < 0.0
                {
                    max_count -= 1;
                    definition_indices[def_count + max_count] = -1;
                }
            }

            // decide whether to run Phase2 and Phase3 again.  There are 3 cases:
            // 1. There is space available, and *-defs remaining.  This is the
            //      normal case - move on to Phase 4 to allocate the remaining
            //      space proportionally to the remaining *-defs.
            // 2. There is space available, but no *-defs.  This implies at least one
            //      def was resolved as 'max', taking less space than its proportion.
            //      If there are also 'min' defs, reconsider them - we can give
            //      them more space.   If not, all the *-defs are 'max', so there's
            //      no way to use all the available space.
            // 3. We allocated too much space.   This implies at least one def was
            //      resolved as 'min'.  If there are also 'max' defs, reconsider
            //      them, otherwise the over-allocation is an inevitable consequence
            //      of the given min constraints.
            // Note that if we return to Phase2, at least one *-def will have been
            // resolved.  This guarantees we don't run Phase2+3 infinitely often.
            run_phase_2_and_3 = false;
            if star_count == 0 && taken_size < final_size {
                // if no *-defs remain and we haven't allocated all the space, reconsider the defs
                // resolved as 'min'.   Their allocation can be increased to make up the gap.
                for i in min_count..min_count_phase2 {
                    if definition_indices[i] >= 0 {
                        let def = &definitions[definition_indices[i] as usize];
                        def.set_measure_size(1.0); // mark as 'not yet resolved'
                        star_count += 1;
                        run_phase_2_and_3 = true; // found a candidate, so re-run Phases 2 and 3
                    }
                }
            }

            if taken_size > final_size {
                // if we've allocated too much space, reconsider the defs
                // resolved as 'max'.   Their allocation can be decreased to make up the gap.
                for i in max_count..max_count_phase2 {
                    if definition_indices[def_count + i] >= 0 {
                        let def = &definitions[definition_indices[def_count + i] as usize];
                        def.set_measure_size(1.0); // mark as 'not yet resolved'
                        star_count += 1;
                        run_phase_2_and_3 = true; // found a candidate, so re-run Phases 2 and 3
                    }
                }
            }
        }

        // Phase 4.  Resolve the remaining defs proportionally.
        let mut star_count = 0;
        for i in 0..def_count {
            let def = &definitions[i];

            if def.user_size().is_star() {
                if def.measure_size() < 0.0 {
                    // this def was resolved in phase 3 - fix up its size
                    def.set_size_cache(-def.measure_size());
                } else {
                    // this def needs resolution, add it to the list, sorted by *-weight
                    definition_indices[star_count] = i as i32;
                    star_count += 1;
                    def.set_measure_size(Self::star_weight(def, scale));
                }
            }
        }

        if star_count > 0 {
            definition_indices[..star_count].sort_by(|x, y| {
                compare_to(
                    definitions[*x as usize].measure_size(),
                    definitions[*y as usize].measure_size(),
                )
            });

            // compute the partial sums of *-weight, in increasing order of weight
            // for minimal loss of precision.
            let mut total_star_weight = 0.0;
            for i in 0..star_count {
                let def = &definitions[definition_indices[i] as usize];
                total_star_weight += def.measure_size();
                def.set_size_cache(total_star_weight);
            }

            // resolve the defs, in decreasing order of weight.
            for i in (0..star_count).rev() {
                let def = &definitions[definition_indices[i] as usize];
                let mut resolved_size = if def.measure_size() > 0.0 {
                    math_max(final_size - taken_size, 0.0) * (def.measure_size() / def.size_cache())
                } else {
                    0.0
                };

                // min and max should have no effect by now, but just in case...
                resolved_size = math_min(resolved_size, def.user_max_size());
                resolved_size = math_max(def.min_size_for_arrange(), resolved_size);

                // Use the raw (unrounded) sizes to update takenSize, so that
                // proportions are computed in the same terms as in phase 3;
                // this avoids errors arising from min/max constraints.
                taken_size += resolved_size;
                def.set_size_cache(resolved_size);
            }
        }

        // Phase 5.  Apply layout rounding.  We do this after fully allocating
        // unrounded sizes, to avoid breaking assumptions in the previous phases
        if self.use_layout_rounding() {
            let dpi = self.get_layout_root().map_or(1.0, |root| root.layout_scaling());
            let mut rounding_errors = self.take_rounding_errors();
            let mut rounded_taken_size = 0.0;

            // round each of the allocated sizes, keeping track of the deltas
            for i in 0..def_count {
                let def = &definitions[i];
                let rounded_size = LayoutHelper::round_layout_value(def.size_cache(), dpi);
                rounding_errors[i] = rounded_size - def.size_cache();
                def.set_size_cache(rounded_size);
                rounded_taken_size += rounded_size;
            }

            // The total allocation might differ from finalSize due to rounding
            // effects.  Tweak the allocations accordingly.

            // Theoretical and historical note.  The problem at hand - allocating
            // space to columns (or rows) with *-weights, min and max constraints,
            // and layout rounding - has a long history.  Especially the special
            // case of 50 columns with min=1 and available space=435 - allocating
            // seats in the U.S. House of Representatives to the 50 states in
            // proportion to their population.  There are numerous algorithms
            // and papers dating back to the 1700's, including the book:
            // Balinski, M. and H. Young, Fair Representation, Yale University Press, New Haven, 1982.
            //
            // One surprising result of all this research is that *any* algorithm
            // will suffer from one or more undesirable features such as the
            // "population paradox" or the "Alabama paradox", where (to use our terminology)
            // increasing the available space by one pixel might actually decrease
            // the space allocated to a given column, or increasing the weight of
            // a column might decrease its allocation.   This is worth knowing
            // in case someone complains about this behavior;  it's not a bug so
            // much as something inherent to the problem.  Cite the book mentioned
            // above or one of the 100s of references, and resolve as WontFix.
            //
            // Fortunately, our scenarios tend to have a small number of columns (~10 or fewer)
            // each being allocated a large number of pixels (~50 or greater), and
            // people don't even notice the kind of 1-pixel anomalies that are
            // theoretically inevitable, or don't care if they do.  At least they shouldn't
            // care - no one should be using the results of the grid layout to make
            // quantitative decisions; its job is to produce a reasonable display, not
            // to allocate seats in Congress.
            //
            // Our algorithm is more susceptible to paradox than the one currently
            // used for Congressional allocation ("Huntington-Hill" algorithm), but
            // it is faster to run:  O(N log N) vs. O(S * N), where N=number of
            // definitions, S = number of available pixels.  And it produces
            // adequate results in practice, as mentioned above.
            //
            // To reiterate one point:  all this only applies when layout rounding
            // is in effect.  When fractional sizes are allowed, the algorithm
            // behaves as well as possible, subject to the min/max constraints
            // and precision of floating-point computation.  (However, the resulting
            // display is subject to anti-aliasing problems.   TANSTAAFL.)

            if !MathUtilities::are_close(rounded_taken_size, final_size) {
                // Compute deltas
                for (i, index) in definition_indices[..def_count].iter_mut().enumerate() {
                    *index = i as i32;
                }

                // Sort rounding errors
                definition_indices[..def_count]
                    .sort_by(|x, y| compare_to(rounding_errors[*x as usize], rounding_errors[*y as usize]));
                let mut adjusted_size = rounded_taken_size;
                let dpi_increment = 1.0 / dpi;

                if rounded_taken_size > final_size {
                    let mut i = def_count;
                    while (adjusted_size > final_size && !MathUtilities::are_close(adjusted_size, final_size)) && i > 0
                    {
                        i -= 1;
                        let definition = &definitions[definition_indices[i] as usize];
                        let mut final_ = definition.size_cache() - dpi_increment;
                        final_ = math_max(final_, definition.min_size_for_arrange());
                        if final_ < definition.size_cache() {
                            adjusted_size -= dpi_increment;
                        }
                        definition.set_size_cache(final_);
                    }
                } else if rounded_taken_size < final_size {
                    let mut i = 0;
                    while (adjusted_size < final_size && !MathUtilities::are_close(adjusted_size, final_size))
                        && i < def_count
                    {
                        let definition = &definitions[definition_indices[i] as usize];
                        let mut final_ = definition.size_cache() + dpi_increment;
                        final_ = math_max(final_, definition.min_size_for_arrange());
                        if final_ > definition.size_cache() {
                            adjusted_size += dpi_increment;
                        }
                        definition.set_size_cache(final_);
                        i += 1;
                    }
                }
            }

            *self.rounding_errors.borrow_mut() = Some(rounding_errors);
        }

        let spacing = if columns {
            self.column_spacing()
        } else {
            self.row_spacing()
        };
        // Phase 6.  Compute final offsets
        definitions[0].set_final_offset(0.0);
        for i in 0..def_count {
            definitions[(i + 1) % def_count]
                .set_final_offset(definitions[i].final_offset() + definitions[i].size_cache() + spacing);
        }

        *self.definition_indices.borrow_mut() = Some(definition_indices);
    }

    /// Chooses the ratio with the maximum discrepancy from the current
    /// proportion. Returns:
    ///
    /// * `Some(true)` if the proportion fails a min constraint but not a
    ///   max, or if the min constraint has the higher discrepancy;
    /// * `Some(false)` if the proportion fails a max constraint but not a
    ///   min, or if the max constraint has the higher discrepancy;
    /// * `None` if the proportion doesn't fail a min or max constraint.
    ///
    /// The discrepancy is the ratio of the proportion to the max- or
    /// min-ratio. When both ratios hit the constraint,
    /// `min_ratio < proportion < max_ratio`, and the min ratio has the higher
    /// discrepancy if `(proportion / min_ratio) > (max_ratio / proportion)`.
    fn choose(min_ratio: f64, max_ratio: f64, proportion: f64) -> Option<bool> {
        if min_ratio < proportion {
            if max_ratio > proportion {
                // compare proportion/minRatio : maxRatio/proportion, but
                // do it carefully to avoid floating-point overflow or underflow
                // and divide-by-0.
                let min_power = log2(min_ratio).floor();
                let max_power = log2(max_ratio).floor();
                let f = 2.0_f64.powf(((min_power + max_power) / 2.0).floor());
                Some((proportion / f) * (proportion / f) > (min_ratio / f) * (max_ratio / f))
            } else {
                Some(true)
            }
        } else if max_ratio > proportion {
            Some(false)
        } else {
            None
        }
    }

    /// Calculates the final (aka arrange) size of the given range.
    fn get_final_size_for_range(definitions: &Definitions, start: usize, count: usize, spacing: f64) -> f64 {
        let mut size = -spacing;

        for i in (start..start + count).rev() {
            size += spacing + definitions[i].size_cache();
        }

        size
    }

    /// Sets or unsets one or multiple flags on the object.
    #[inline]
    fn set_flags(&self, value: bool, flags: u32) {
        self.flags.set(if value {
            self.flags.get() | flags
        } else {
            self.flags.get() & !flags
        });
    }

    /// Returns true if all the flags in the given bitmask are set on the
    /// object.
    #[inline]
    fn check_flags(&self, flags: u32) -> bool {
        (self.flags.get() & flags) == flags
    }

    /// Synchronizes the grid lines renderer with the `ShowGridLines`
    /// property by adding / removing the renderer visual. Returns the
    /// renderer visual, if the lines are shown.
    fn ensure_grid_lines_renderer(&self) -> Option<Ref<GridLinesRenderer>> {
        let show_grid_lines = self.show_grid_lines();
        let current = self.grid_lines_renderer.borrow().clone();

        if show_grid_lines && current.is_none() {
            let renderer = GridLinesRenderer::new();
            *self.grid_lines_renderer.borrow_mut() = Some(renderer.clone());
            self.visual_children().add(renderer.upcast());
        }

        if !show_grid_lines {
            if let Some(renderer) = current {
                self.visual_children().remove(&renderer.upcast());
                *self.grid_lines_renderer.borrow_mut() = None;
            }
        }

        self.grid_lines_renderer.borrow().clone()
    }

    fn on_show_grid_lines_property_changed(grid: &Grid, e: &FerroPropertyChangedEventArgs<'_>) {
        // A trivial grid is 1 by 1: there are no grid lines anyway.
        if grid.ext_data.get().is_some() && grid.listen_to_notifications() {
            grid.invalidate_arrange();
        }

        grid.set_flags(e.get_new_value::<bool>(), FLAG_SHOW_GRID_LINES_PROPERTY_VALUE);
    }

    fn on_spacing_property_changed(grid: &Grid, _e: &FerroPropertyChangedEventArgs<'_>) {
        if grid.ext_data.get().is_some() && grid.listen_to_notifications() {
            grid.set_cells_structure_dirty(true);
        }
    }

    fn on_cell_attached_property_changed(child: &Control, _e: &FerroPropertyChangedEventArgs<'_>) {
        let parent = child.visual_parent();
        if let Some(grid) = parent.as_ref().and_then(|p| p.downcast_ref::<Grid>()) {
            if grid.ext_data.get().is_some() && grid.listen_to_notifications() {
                grid.set_cells_structure_dirty(true);
            }
        }
    }

    /// The array of column definitions used during the calculations.
    fn definitions_u(&self) -> Definitions {
        self.ext()
            .definitions_u
            .borrow()
            .as_ref()
            .expect("the column definitions structure has not been validated")
            .definitions()
    }

    /// The array of row definitions used during the calculations.
    fn definitions_v(&self) -> Definitions {
        self.ext()
            .definitions_v
            .borrow()
            .as_ref()
            .expect("the row definitions structure has not been validated")
            .definitions()
    }

    /// Takes the layout time array of definition indices out of its thread
    /// slot, with at least `required_length` entries reset to -1.
    fn take_temp_definitions(required_length: usize) -> Vec<i32> {
        let mut temp_definitions = TEMP_DEFINITIONS.take();
        temp_definitions.clear();
        temp_definitions.resize(required_length, -1);
        temp_definitions
    }

    fn return_temp_definitions(temp_definitions: Vec<i32>) {
        TEMP_DEFINITIONS.set(temp_definitions);
    }

    /// Takes the array of definition indices out of the grid; it is put back
    /// when the final sizes have been set.
    fn take_definition_indices(&self) -> Vec<i32> {
        let required_length = self.definitions_u().count().max(self.definitions_v().count()).max(1) * 2;

        match self.definition_indices.borrow_mut().take() {
            Some(definition_indices) if definition_indices.len() >= required_length => definition_indices,
            _ => vec![0; required_length],
        }
    }

    /// Takes the array of rounding errors out of the grid; it is put back
    /// when the layout rounding has been applied.
    fn take_rounding_errors(&self) -> Vec<f64> {
        let required_length = self.definitions_u().count().max(self.definitions_v().count());

        match self.rounding_errors.borrow_mut().take() {
            Some(rounding_errors) if rounding_errors.len() >= required_length => rounding_errors,
            None if required_length == 0 => vec![0.0; 1],
            _ => vec![0.0; required_length],
        }
    }

    /// The number of cells.
    #[inline]
    fn private_cells_len(&self) -> usize {
        self.ext().cell_caches_collection.borrow().len()
    }

    /// The cached values of a cell.
    #[inline]
    fn private_cell(&self, index: usize) -> CellCache {
        self.ext().cell_caches_collection.borrow()[index]
    }

    #[inline]
    fn cells_structure_dirty(&self) -> bool {
        !self.check_flags(FLAG_VALID_CELLS_STRUCTURE)
    }

    #[inline]
    fn set_cells_structure_dirty(&self, value: bool) {
        self.set_flags(!value, FLAG_VALID_CELLS_STRUCTURE)
    }

    #[inline]
    fn listen_to_notifications(&self) -> bool {
        self.check_flags(FLAG_LISTEN_TO_NOTIFICATIONS)
    }

    #[inline]
    fn set_listen_to_notifications(&self, value: bool) {
        self.set_flags(value, FLAG_LISTEN_TO_NOTIFICATIONS)
    }

    #[inline]
    fn size_to_content_u(&self) -> bool {
        self.check_flags(FLAG_SIZE_TO_CONTENT_U)
    }

    #[inline]
    fn size_to_content_v(&self) -> bool {
        self.check_flags(FLAG_SIZE_TO_CONTENT_V)
    }

    #[inline]
    fn has_star_cells_u(&self) -> bool {
        self.check_flags(FLAG_HAS_STAR_CELLS_U)
    }

    #[inline]
    fn has_star_cells_v(&self) -> bool {
        self.check_flags(FLAG_HAS_STAR_CELLS_V)
    }

    #[inline]
    fn has_group3_cells_in_auto_rows(&self) -> bool {
        self.check_flags(FLAG_HAS_GROUP3_CELLS_IN_AUTO_ROWS)
    }

    /// Returns the *-weight, adjusted for the scale computed during Phase 1.
    fn star_weight(def: &DefinitionBase, scale: f64) -> f64 {
        if scale < 0.0 {
            // if one of the *-weights is Infinity, adjust the weights by mapping
            // Infinity to 1.0 and everything else to 0.0:  the infinite items share the
            // available space equally, everyone else gets nothing.
            if def.user_size().value() == f64::INFINITY {
                1.0
            } else {
                0.0
            }
        } else {
            def.user_size().value() * scale
        }
    }

    /// Orders definitions for the distribution of a span that fits into the
    /// preferred size of its range.
    fn span_preferred_distribution_order(definition_x: &DefinitionBase, definition_y: &DefinitionBase) -> Ordering {
        if definition_x.user_size().is_auto() {
            if definition_y.user_size().is_auto() {
                compare_to(definition_x.min_size(), definition_y.min_size())
            } else {
                Ordering::Less
            }
        } else if definition_y.user_size().is_auto() {
            Ordering::Greater
        } else {
            compare_to(definition_x.preferred_size(), definition_y.preferred_size())
        }
    }

    /// Orders definitions for the distribution of a span that fits into the
    /// maximum size of its range.
    fn span_max_distribution_order(definition_x: &DefinitionBase, definition_y: &DefinitionBase) -> Ordering {
        if definition_x.user_size().is_auto() {
            if definition_y.user_size().is_auto() {
                compare_to(definition_x.size_cache(), definition_y.size_cache())
            } else {
                Ordering::Greater
            }
        } else if definition_y.user_size().is_auto() {
            Ordering::Less
        } else {
            compare_to(definition_x.size_cache(), definition_y.size_cache())
        }
    }
}

const GRID_LINES_DASH_LENGTH: f64 = 4.0;
const GRID_LINES_PEN_WIDTH: f64 = 1.0;

thread_local! {
    /// The size of the last arranged grid that shows its lines; shared by
    /// all the renderers, as in the reference.
    static GRID_LINES_LAST_ARRANGE_SIZE: Cell<Size> = const { Cell::new(Size::new(0.0, 0.0)) };
    /// The first pen to draw a dash.
    static GRID_LINES_ODD_DASH_PEN: Rc<dyn ferroui_base::media::IPen> = grid_lines_pen(ferroui_base::media::Brushes::blue(), 0.0);
    /// The second pen to draw a dash.
    static GRID_LINES_EVEN_DASH_PEN: Rc<dyn ferroui_base::media::IPen> =
        grid_lines_pen(ferroui_base::media::Brushes::yellow(), GRID_LINES_DASH_LENGTH);
}

fn grid_lines_pen(brush: Rc<dyn ferroui_base::media::IBrush>, dash_offset: f64) -> Rc<dyn ferroui_base::media::IPen> {
    use ferroui_base::media::{DashStyle, Pen, PenLineCap, PenLineJoin};

    let dash_style = DashStyle::with_dashes(Some(&[GRID_LINES_DASH_LENGTH, GRID_LINES_DASH_LENGTH]), dash_offset);
    Pen::with_all(Some(brush), GRID_LINES_PEN_WIDTH, Some(dash_style.into()), PenLineCap::Flat, PenLineJoin::Miter, 10.0)
        .into()
}

/// Helper to render grid lines.
#[repr(C)]
pub(crate) struct GridLinesRenderer {
    base: Control,
}

ferro_class!(GridLinesRenderer: Control);
ferro_impl_classes!(
    GridLinesRenderer: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl VisualImpl for GridLinesRenderer {
    fn render(this: &Self, drawing_context: &mut ferroui_base::media::DrawingContext) {
        let Some(grid) = this.get_visual_parent_of_type::<Grid>() else { return };

        if !grid.show_grid_lines() {
            return;
        }

        let last_arrange_size = GRID_LINES_LAST_ARRANGE_SIZE.get();

        let column_definitions = grid.column_definitions();
        let column_spacing = grid.column_spacing();
        for i in 1..column_definitions.count() {
            let final_offset = column_definitions.get(i).final_offset();
            Self::draw_grid_line(drawing_context, final_offset, 0.0, final_offset, last_arrange_size.height);

            if column_spacing != 0.0 {
                Self::draw_grid_line(
                    drawing_context,
                    final_offset - column_spacing,
                    0.0,
                    final_offset - column_spacing,
                    last_arrange_size.height,
                );
            }
        }

        let row_definitions = grid.row_definitions();
        let row_spacing = grid.row_spacing();
        for i in 1..row_definitions.count() {
            let final_offset = row_definitions.get(i).final_offset();
            Self::draw_grid_line(drawing_context, 0.0, final_offset, last_arrange_size.width, final_offset);

            if row_spacing != 0.0 {
                Self::draw_grid_line(
                    drawing_context,
                    0.0,
                    final_offset - row_spacing,
                    last_arrange_size.width,
                    final_offset - row_spacing,
                );
            }
        }
    }
}

impl GridLinesRenderer {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct() })
    }

    /// Draws a single grid line.
    fn draw_grid_line(
        drawing_context: &mut ferroui_base::media::DrawingContext,
        start_x: f64,
        start_y: f64,
        end_x: f64,
        end_y: f64,
    ) {
        let start = ferroui_base::Point::new(start_x, start_y);
        let end = ferroui_base::Point::new(end_x, end_y);
        GRID_LINES_ODD_DASH_PEN.with(|pen| drawing_context.draw_line(pen, start, end));
        GRID_LINES_EVEN_DASH_PEN.with(|pen| drawing_context.draw_line(pen, start, end));
    }

    pub(crate) fn update_render_bounds(&self, arrange_size: Size) {
        GRID_LINES_LAST_ARRANGE_SIZE.set(arrange_size);
        self.invalidate_visual();
    }
}
