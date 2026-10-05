use crate::grid_mocks::{GridAssert, GridMock};
use crate::test_support::{test_scope, TestRoot};
use crate::{
    Border, Button, ColumnDefinition, ColumnDefinitions, Control, ControlImpl, Decorator, Grid, GridLength,
    GridUnitType, Panel, PanelImpl, RowDefinition, RowDefinitions, ScrollViewer, TextBlock,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, IntoRef, Point, Rect, Ref, Size, StyledElementImpl,
    Thickness, VisualImpl,
};
use std::cell::Cell;

/// A control with a fixed measured size.
#[repr(C)]
struct TestControl {
    base: Control,
    measure_size: Cell<Size>,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(TestControl: FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl LayoutableImpl for TestControl {
    fn measure_override(this: &Self, _available_size: Size) -> Size {
        this.measure_size.get()
    }
}

impl TestControl {
    fn new(measure_size: Size) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            measure_size: Cell::new(measure_size),
        })
    }
}

/// A panel that stacks its children; it stands in for the stack panel of the
/// reference tests.
#[repr(C)]
struct TestStackPanel {
    base: Panel,
    orientation: Cell<Orientation>,
}

ferro_class!(TestStackPanel: Panel);
ferro_impl_classes!(
    TestStackPanel: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    PanelImpl
);

impl LayoutableImpl for TestStackPanel {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let horizontal = this.orientation.get() == Orientation::Horizontal;
        let constraint = if horizontal {
            Size::new(f64::INFINITY, available_size.height)
        } else {
            Size::new(available_size.width, f64::INFINITY)
        };
        let mut result = Size::default();

        for child in this.children().snapshot().iter() {
            child.measure(constraint);
            let desired = child.desired_size();
            result = if horizontal {
                Size::new(result.width + desired.width, result.height.max(desired.height))
            } else {
                Size::new(result.width.max(desired.width), result.height + desired.height)
            };
        }

        result
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let horizontal = this.orientation.get() == Orientation::Horizontal;
        let mut offset = 0.0;

        for child in this.children().snapshot().iter() {
            let desired = child.desired_size();
            if horizontal {
                child.arrange(Rect::new(offset, 0.0, desired.width, final_size.height));
                offset += desired.width;
            } else {
                child.arrange(Rect::new(0.0, offset, final_size.width, desired.height));
                offset += desired.height;
            }
        }

        final_size
    }
}

impl TestStackPanel {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Panel::construct(),
            orientation: Cell::new(Orientation::Vertical),
        })
    }

    /// Creates a panel that is a shared size scope and has the given
    /// children.
    fn shared_size_scope(children: &[&Ref<Grid>]) -> Ref<Self> {
        let result = Self::new();
        Grid::set_is_shared_size_scope(&result, true);
        for child in children {
            result.children().add(*child);
        }
        result
    }
}

type Column<'a> = (Option<&'a str>, GridLength);

fn create_grid(columns: &[Column<'_>]) -> Ref<Grid> {
    let columns: Vec<_> = columns
        .iter()
        .map(|c| {
            (
                c.0,
                c.1,
                ColumnDefinition::min_width_property().get_default_value(ColumnDefinition::TYPE),
            )
        })
        .collect();
    create_grid_with_min_width(&columns)
}

fn create_grid_with_min_width(columns: &[(Option<&str>, GridLength, f64)]) -> Ref<Grid> {
    let columns: Vec<_> = columns
        .iter()
        .map(|c| {
            (
                c.0,
                c.1,
                c.2,
                ColumnDefinition::max_width_property().get_default_value(ColumnDefinition::TYPE),
            )
        })
        .collect();
    create_grid_with_min_max_width(&columns)
}

fn create_grid_with_min_max_width(columns: &[(Option<&str>, GridLength, f64, f64)]) -> Ref<Grid> {
    let grid = Grid::new();
    for c in columns {
        let k = ColumnDefinition::new();
        k.set_shared_size_group(c.0);
        k.set_width(c.1);
        k.set_min_width(c.2);
        k.set_max_width(c.3);
        grid.column_definitions().add(k);
    }

    grid
}

fn add_sizer(grid: &Grid, column: i32, size: f64) -> Ref<Control> {
    let ctrl = Control::new();
    ctrl.set_min_width(size);
    ctrl.set_min_height(size);
    ctrl.set_value(Grid::column_property(), column);
    grid.children().add(&ctrl);
    ctrl
}

fn column(width: GridLength, shared_size_group: Option<&str>) -> Ref<ColumnDefinition> {
    let result = ColumnDefinition::new();
    result.set_width(width);
    result.set_shared_size_group(shared_size_group);
    result
}

fn row(height: GridLength, shared_size_group: Option<&str>) -> Ref<RowDefinition> {
    let result = RowDefinition::new();
    result.set_height(height);
    result.set_shared_size_group(shared_size_group);
    result
}

fn border(width: Option<f64>, height: Option<f64>) -> Ref<Border> {
    let result = Border::new();
    if let Some(width) = width {
        result.set_width(width);
    }
    if let Some(height) = height {
        result.set_height(height);
    }
    result
}

fn cell(control: impl IntoRef<Control>, row: i32, column: i32) -> Ref<Control> {
    let control = control.into_ref();
    Grid::set_row(&control, row);
    Grid::set_column(&control, column);
    control
}

fn star(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Star)
}

fn pixels(value: f64) -> GridLength {
    GridLength::from_pixels(value)
}

fn auto(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Auto)
}

fn child_bounds(grid: &Grid, index: usize) -> Rect {
    grid.children().get(index).bounds()
}

/// Asserts the actual width of all the columns of a shared size group.
fn assert_group_width(grid: &Grid, shared_size_group: Option<&str>, expected: f64) {
    for cd in grid.column_definitions().snapshot().iter() {
        if cd.shared_size_group().as_deref() == shared_size_group {
            assert_eq!(expected, cd.actual_width());
        }
    }
}

/// Places the grid in a scope grid in a root grid, which is a shared size
/// scope or not.
fn root_with_scope(grid: &Ref<Grid>, is_shared_size_scope: bool) -> (Ref<Grid>, Ref<Grid>) {
    let scope = Grid::new();
    scope.children().add(grid);

    let root = Grid::new();
    root.set_use_layout_rounding(false);
    root.set_value(Grid::is_shared_size_scope_property(), is_shared_size_scope);
    root.children().add(&scope);

    (root, scope)
}

fn measure_and_arrange(grid: &Grid, size: f64) {
    grid.measure(Size::new(size, size));
    grid.arrange(Rect::from_points(Point::default(), Point::new(size, size)));
}

/// The body shared by the "same size" tests: lays out a grid with the given
/// columns at 200x200 and checks the actual width of each group.
fn same_size(is_shared_size_scope: bool, columns: &[Column<'_>], expected: &[(Option<&str>, f64)]) {
    let grid = create_grid(columns);
    let _root = root_with_scope(&grid, is_shared_size_scope);

    measure_and_arrange(&grid, 200.0);

    for (shared_size_group, width) in expected {
        assert_group_width(&grid, *shared_size_group, *width);
    }
}

/// Builds the columns of a "same size" test: an optional leading and trailing
/// column without a group around the given columns.
fn with_edges<'a>(first: bool, columns: &[Column<'a>], last: bool) -> Vec<Column<'a>> {
    let mut result = Vec::new();
    if first {
        result.push((None, GridLength::default()));
    }
    result.extend_from_slice(columns);
    if last {
        result.push((None, GridLength::default()));
    }
    result
}

fn one_group(length: GridLength) -> [Column<'static>; 4] {
    [(Some("A"), length); 4]
}

fn two_groups(a: GridLength, b: GridLength) -> [Column<'static>; 4] {
    [(Some("A"), a), (Some("B"), b), (Some("B"), b), (Some("A"), a)]
}

fn execute_layout_pass(root: &TestRoot) {
    root.layout_manager().execute_layout_pass();
}

/// Shared groups validate after layout and apply any resulting invalidation
/// on the next pass.
fn execute_shared_size_layout_pass(root: &TestRoot) {
    execute_layout_pass(root);
    execute_layout_pass(root);
}

#[test]
fn calculates_colspan_correctly() {
    let target = Grid::new();
    target.set_column_definitions(ColumnDefinitions::from_items([
        ColumnDefinition::with_width(GridLength::AUTO),
        ColumnDefinition::with_width(GridLength::new(4.0, GridUnitType::Pixel)),
        ColumnDefinition::with_width(GridLength::AUTO),
    ]));
    target.set_row_definitions(RowDefinitions::from_items([
        RowDefinition::with_height(GridLength::AUTO),
        RowDefinition::with_height(GridLength::AUTO),
    ]));
    let child = border(Some(100.0), Some(25.0));
    Grid::set_column_span(&child, 3);
    target.children().add(child);
    target.children().add(cell(border(Some(150.0), Some(25.0)), 1, 0));
    target.children().add(cell(border(Some(50.0), Some(25.0)), 1, 2));

    target.measure(Size::INFINITY);

    // Issue #25 only appears after a second measure
    target.invalidate_measure();
    target.measure(Size::INFINITY);

    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Size::new(204.0, 50.0), target.bounds().size());
    assert_eq!(150.0, target.column_definitions().get(0).actual_width());
    assert_eq!(4.0, target.column_definitions().get(1).actual_width());
    assert_eq!(50.0, target.column_definitions().get(2).actual_width());
    assert_eq!(Rect::new(52.0, 0.0, 100.0, 25.0), child_bounds(&target, 0));
    assert_eq!(Rect::new(0.0, 25.0, 150.0, 25.0), child_bounds(&target, 1));
    assert_eq!(Rect::new(154.0, 25.0, 50.0, 25.0), child_bounds(&target, 2));
}

#[test]
fn layout_empty_column_row_layout_like_a_normal_panel() {
    // Arrange & Action
    let grid = GridMock::new(None, Some(Size::new(600.0, 200.0)));

    // Assert
    GridAssert::children_width(&grid, &[600.0]);
    GridAssert::children_height(&grid, &[200.0]);
}

#[test]
fn layout_pixel_row_column_bounds_correct() {
    // Arrange & Action
    let row_grid = GridMock::new_rows(RowDefinitions::parse("100,200,300").unwrap(), 0.0, 0.0);
    let column_grid = GridMock::new_columns(ColumnDefinitions::parse("50,100,150").unwrap(), 0.0, 0.0);

    // Assert
    GridAssert::children_height(&row_grid, &[100.0, 200.0, 300.0]);
    GridAssert::children_width(&column_grid, &[50.0, 100.0, 150.0]);
}

#[test]
fn layout_star_row_column_bounds_correct() {
    // Arrange & Action
    let row_grid = GridMock::new_rows(RowDefinitions::parse("1*,2*,3*").unwrap(), 600.0, 0.0);
    let column_grid = GridMock::new_columns(ColumnDefinitions::parse("*,*,2*").unwrap(), 600.0, 0.0);

    // Assert
    GridAssert::children_height(&row_grid, &[100.0, 200.0, 300.0]);
    GridAssert::children_width(&column_grid, &[150.0, 150.0, 300.0]);
}

#[test]
fn layout_mix_pixel_star_row_column_bounds_correct() {
    // Arrange & Action
    let row_grid = GridMock::new_rows(RowDefinitions::parse("1*,2*,150").unwrap(), 600.0, 0.0);
    let column_grid = GridMock::new_columns(ColumnDefinitions::parse("1*,2*,150").unwrap(), 600.0, 0.0);

    // Assert
    GridAssert::children_height(&row_grid, &[150.0, 300.0, 150.0]);
    GridAssert::children_width(&column_grid, &[150.0, 300.0, 150.0]);
}

#[test]
fn layout_star_row_column_with_min_length_bounds_correct() {
    // Arrange & Action
    let first_row = RowDefinition::with_value(1.0, GridUnitType::Star);
    first_row.set_min_height(200.0);
    let row_grid = GridMock::new_rows(
        RowDefinitions::from_items([
            first_row,
            RowDefinition::with_value(1.0, GridUnitType::Star),
            RowDefinition::with_value(1.0, GridUnitType::Star),
        ]),
        300.0,
        0.0,
    );
    let first_column = ColumnDefinition::with_value(1.0, GridUnitType::Star);
    first_column.set_min_width(200.0);
    let column_grid = GridMock::new_columns(
        ColumnDefinitions::from_items([
            first_column,
            ColumnDefinition::with_value(1.0, GridUnitType::Star),
            ColumnDefinition::with_value(1.0, GridUnitType::Star),
        ]),
        300.0,
        0.0,
    );

    // Assert
    GridAssert::children_height(&row_grid, &[200.0, 50.0, 50.0]);
    GridAssert::children_width(&column_grid, &[200.0, 50.0, 50.0]);
}

#[test]
fn layout_star_row_column_with_max_length_bounds_correct() {
    // Arrange & Action
    let first_row = RowDefinition::with_value(1.0, GridUnitType::Star);
    first_row.set_max_height(200.0);
    let row_grid = GridMock::new_rows(
        RowDefinitions::from_items([
            first_row,
            RowDefinition::with_value(1.0, GridUnitType::Star),
            RowDefinition::with_value(1.0, GridUnitType::Star),
        ]),
        800.0,
        0.0,
    );
    let first_column = ColumnDefinition::with_value(1.0, GridUnitType::Star);
    first_column.set_max_width(200.0);
    let column_grid = GridMock::new_columns(
        ColumnDefinitions::from_items([
            first_column,
            ColumnDefinition::with_value(1.0, GridUnitType::Star),
            ColumnDefinition::with_value(1.0, GridUnitType::Star),
        ]),
        800.0,
        0.0,
    );

    // Assert
    GridAssert::children_height(&row_grid, &[200.0, 300.0, 300.0]);
    GridAssert::children_width(&column_grid, &[200.0, 300.0, 300.0]);
}

#[test]
fn changing_child_column_invalidates_measure() {
    let target = Grid::new();
    target.set_column_definitions(ColumnDefinitions::parse("*,*").unwrap());
    let child = Border::new();
    Grid::set_column(&child, 0);
    target.children().add(&child);

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
    assert!(target.is_measure_valid());

    Grid::set_column(&child, 1);

    assert!(!target.is_measure_valid());
}

#[test]
fn grid_grid_length_same_size_pixel_0() {
    same_size(false, &[(None, GridLength::default()); 4], &[(None, 0.0)]);
}

#[test]
fn grid_grid_length_same_size_pixel_50() {
    same_size(false, &[(None, pixels(50.0)); 4], &[(None, 50.0)]);
}

#[test]
fn grid_grid_length_same_size_auto() {
    same_size(false, &[(None, auto(0.0)); 4], &[(None, 0.0)]);
}

#[test]
fn grid_grid_length_same_size_star() {
    same_size(false, &[(None, star(1.0)); 4], &[(None, 50.0)]);
}

/// Declares the "same size" tests of one layout of the columns: with or
/// without a leading and a trailing column that is not in a group.
macro_rules! same_size_tests {
    ($first:expr, $last:expr, [$pixel_0:ident, $pixel_50:ident, $auto:ident, $star:ident],
     [$pixel_0_two:ident, $pixel_50_two:ident, $auto_two:ident, $star_two:ident]) => {
        #[test]
        fn $pixel_0() {
            let columns = with_edges($first, &one_group(GridLength::default()), $last);
            same_size(true, &columns, &[(Some("A"), 0.0)]);
        }

        #[test]
        fn $pixel_50() {
            let columns = with_edges($first, &one_group(pixels(50.0)), $last);
            same_size(true, &columns, &[(Some("A"), 50.0)]);
        }

        #[test]
        fn $auto() {
            let columns = with_edges($first, &one_group(auto(0.0)), $last);
            same_size(true, &columns, &[(Some("A"), 0.0)]);
        }

        #[test]
        fn $star() {
            // Star sizing is treated as Auto, 1 is ignored
            let columns = with_edges($first, &one_group(star(1.0)), $last);
            same_size(true, &columns, &[(Some("A"), 0.0)]);
        }

        #[test]
        fn $pixel_0_two() {
            let columns = with_edges(
                $first,
                &two_groups(GridLength::default(), GridLength::default()),
                $last,
            );
            same_size(true, &columns, &[(Some("A"), 0.0), (Some("B"), 0.0)]);
        }

        #[test]
        fn $pixel_50_two() {
            let columns = with_edges($first, &two_groups(pixels(25.0), pixels(75.0)), $last);
            same_size(true, &columns, &[(Some("A"), 25.0), (Some("B"), 75.0)]);
        }

        #[test]
        fn $auto_two() {
            let columns = with_edges($first, &two_groups(auto(0.0), auto(0.0)), $last);
            same_size(true, &columns, &[(Some("A"), 0.0), (Some("B"), 0.0)]);
        }

        #[test]
        fn $star_two() {
            // Star sizing is treated as Auto, 1 is ignored
            let columns = with_edges($first, &two_groups(star(1.0), star(1.0)), $last);
            same_size(true, &columns, &[(Some("A"), 0.0), (Some("B"), 0.0)]);
        }
    };
}

same_size_tests!(
    false,
    false,
    [
        shared_size_grid_grid_length_same_size_pixel_0,
        shared_size_grid_grid_length_same_size_pixel_50,
        shared_size_grid_grid_length_same_size_auto,
        shared_size_grid_grid_length_same_size_star
    ],
    [
        shared_size_grid_grid_length_same_size_pixel_0_two_groups,
        shared_size_grid_grid_length_same_size_pixel_50_two_groups,
        shared_size_grid_grid_length_same_size_auto_two_groups,
        shared_size_grid_grid_length_same_size_star_two_groups
    ]
);

same_size_tests!(
    true,
    false,
    [
        shared_size_grid_grid_length_same_size_pixel_0_first_column_0,
        shared_size_grid_grid_length_same_size_pixel_50_first_column_0,
        shared_size_grid_grid_length_same_size_auto_first_column_0,
        shared_size_grid_grid_length_same_size_star_first_column_0
    ],
    [
        shared_size_grid_grid_length_same_size_pixel_0_first_column_0_two_groups,
        shared_size_grid_grid_length_same_size_pixel_50_first_column_0_two_groups,
        shared_size_grid_grid_length_same_size_auto_first_column_0_two_groups,
        shared_size_grid_grid_length_same_size_star_first_column_0_two_groups
    ]
);

same_size_tests!(
    false,
    true,
    [
        shared_size_grid_grid_length_same_size_pixel_0_last_column_0,
        shared_size_grid_grid_length_same_size_pixel_50_last_column_0,
        shared_size_grid_grid_length_same_size_auto_last_column_0,
        shared_size_grid_grid_length_same_size_star_last_column_0
    ],
    [
        shared_size_grid_grid_length_same_size_pixel_0_last_column_0_two_groups,
        shared_size_grid_grid_length_same_size_pixel_50_last_column_0_two_groups,
        shared_size_grid_grid_length_same_size_auto_last_column_0_two_groups,
        shared_size_grid_grid_length_same_size_star_last_column_0_two_groups
    ]
);

same_size_tests!(
    true,
    true,
    [
        shared_size_grid_grid_length_same_size_pixel_0_first_and_last_column_0,
        shared_size_grid_grid_length_same_size_pixel_50_first_and_last_column_0,
        shared_size_grid_grid_length_same_size_auto_first_and_last_column_0,
        shared_size_grid_grid_length_same_size_star_first_and_last_column_0
    ],
    [
        shared_size_grid_grid_length_same_size_pixel_0_first_and_last_column_0_two_groups,
        shared_size_grid_grid_length_same_size_pixel_50_first_and_last_column_0_two_groups,
        shared_size_grid_grid_length_same_size_auto_first_and_last_column_0_two_groups,
        shared_size_grid_grid_length_same_size_star_first_and_last_column_0_two_groups
    ]
);

#[test]
fn size_propagation_is_constrained_to_innermost_scope() {
    let grids = [
        create_grid(&[(Some("A"), GridLength::default())]),
        create_grid(&[(Some("A"), pixels(30.0)), (None, GridLength::default())]),
    ];
    let inner_scope = Grid::new();

    for grid in &grids {
        inner_scope.children().add(grid);
    }

    inner_scope.set_value(Grid::is_shared_size_scope_property(), true);

    let outer_grid = create_grid(&[(Some("A"), pixels(0.0))]);
    let outer_scope = Grid::new();
    outer_scope.children().add(&outer_grid);
    outer_scope.children().add(&inner_scope);

    let root = Grid::new();
    root.set_use_layout_rounding(false);
    root.set_value(Grid::is_shared_size_scope_property(), true);
    root.children().add(&outer_scope);

    measure_and_arrange(&root, 50.0);
    assert_eq!(0.0, outer_grid.column_definitions().get(0).actual_width());
}

#[test]
fn size_group_changes_are_tracked() {
    let grids = [
        create_grid(&[(None, auto(0.0)), (None, GridLength::default())]),
        create_grid(&[(Some("A"), pixels(30.0)), (None, GridLength::default())]),
    ];
    let scope = Grid::new();
    for grid in &grids {
        scope.children().add(grid);
    }

    let root = Grid::new();
    root.set_use_layout_rounding(false);
    root.set_value(Grid::is_shared_size_scope_property(), true);
    root.children().add(&scope);

    measure_and_arrange(&root, 50.0);
    assert_eq!(0.0, grids[0].column_definitions().get(0).actual_width());

    grids[0].column_definitions().get(0).set_shared_size_group(Some("A"));

    measure_and_arrange(&root, 51.0);
    assert_eq!(30.0, grids[0].column_definitions().get(0).actual_width());

    grids[0].column_definitions().get(0).set_shared_size_group(None);

    measure_and_arrange(&root, 52.0);
    assert_eq!(0.0, grids[0].column_definitions().get(0).actual_width());
}

#[test]
fn size_group_definition_resizes_are_tracked() {
    let test_scope = test_scope();
    let grids = [
        create_grid(&[(Some("A"), pixels(5.0)), (None, GridLength::default())]),
        create_grid(&[(Some("A"), pixels(5.0)), (None, GridLength::default())]),
    ];
    let scope = Grid::new();
    for grid in &grids {
        scope.children().add(grid);
    }

    let root_grid = Grid::new();
    root_grid.set_use_layout_rounding(false);
    root_grid.set_value(Grid::is_shared_size_scope_property(), true);
    root_grid.children().add(&scope);

    let root = TestRoot::with_child(&root_grid);
    root.set_width(50.0);
    root.set_height(50.0);

    root.execute_initial_layout_pass();

    assert_eq!(5.0, grids[0].column_definitions().get(0).actual_width());
    assert_eq!(5.0, grids[1].column_definitions().get(0).actual_width());

    grids[0]
        .column_definitions()
        .get(0)
        .set_width(GridLength::new(10.0, GridUnitType::Pixel));

    for grid in &grids {
        measure_and_arrange(grid, 50.0);
    }

    assert_eq!(10.0, grids[0].column_definitions().get(0).actual_width());
    assert_eq!(10.0, grids[1].column_definitions().get(0).actual_width());

    // The layout pass queued by the change runs when the scope ends, which
    // has to happen while the root is alive.
    drop(test_scope);
}

#[test]
fn shared_size_group_shrinks_when_content_is_hidden() {
    let _scope = test_scope();
    let grids = [
        create_grid(&[(Some("A"), GridLength::AUTO)]),
        create_grid(&[(Some("A"), GridLength::AUTO)]),
    ];
    let content = border(Some(50.0), None);
    content.set_is_visible(false);
    grids[1].children().add(&content);

    let scope = TestStackPanel::shared_size_scope(&[&grids[0], &grids[1]]);
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    for grid in &grids {
        assert_eq!(0.0, grid.column_definitions().get(0).actual_width());
    }

    content.set_is_visible(true);
    execute_shared_size_layout_pass(&root);
    for grid in &grids {
        assert_eq!(50.0, grid.column_definitions().get(0).actual_width());
    }

    content.set_is_visible(false);
    execute_shared_size_layout_pass(&root);
    for grid in &grids {
        assert_eq!(0.0, grid.column_definitions().get(0).actual_width());
    }
}

#[test]
fn shared_size_group_shrinks_when_participant_uses_cyclic_measure_path() {
    // A grid mixing an auto-column/star-row cell with a star-column/auto-row cell cannot
    // resolve stars in either direction up front, so the grid measures it through its cyclic
    // dependency path. That path saves definition min sizes before the repeated measure and
    // restores them afterwards. Saving the effective min size folds the group minimum into
    // the definition's own contribution, which then reclassifies it as a long pole - and
    // long poles are deliberately never remeasured, so the group stays pinned open.
    let _scope = test_scope();
    let cyclic_grid = create_grid(&[(Some("A"), GridLength::AUTO), (None, star(1.0))]);
    cyclic_grid.set_height(100.0); // star rows collapse to auto under an infinite constraint.
    cyclic_grid.row_definitions().add(row(GridLength::AUTO, None));
    cyclic_grid.row_definitions().add(row(star(1.0), None));

    let auto_column_star_row_child = border(Some(10.0), Some(10.0));
    Grid::set_column(&auto_column_star_row_child, 0);
    Grid::set_row(&auto_column_star_row_child, 1);

    let star_column_auto_row_child = border(Some(10.0), Some(10.0));
    Grid::set_column(&star_column_auto_row_child, 1);
    Grid::set_row(&star_column_auto_row_child, 0);

    cyclic_grid.children().add(&auto_column_star_row_child);
    cyclic_grid.children().add(&star_column_auto_row_child);

    let plain_grid = create_grid(&[(Some("A"), GridLength::AUTO), (None, star(1.0))]);
    let wide_content = border(Some(50.0), Some(10.0));
    plain_grid.children().add(&wide_content);

    let scope = TestStackPanel::shared_size_scope(&[&cyclic_grid, &plain_grid]);
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    execute_shared_size_layout_pass(&root);
    assert_eq!(50.0, cyclic_grid.column_definitions().get(0).actual_width());
    assert_eq!(50.0, plain_grid.column_definitions().get(0).actual_width());

    wide_content.set_is_visible(false);
    execute_shared_size_layout_pass(&root);
    assert_eq!(10.0, cyclic_grid.column_definitions().get(0).actual_width());
    assert_eq!(10.0, plain_grid.column_definitions().get(0).actual_width());
}

/// Creates a grid with an auto column in group "A" followed by a star column.
fn shared_auto_and_star_grid() -> Ref<Grid> {
    let grid = Grid::new();
    grid.column_definitions().add(column(GridLength::AUTO, Some("A")));
    grid.column_definitions().add(column(star(1.0), None));
    grid
}

#[test]
fn shared_size_group_is_registered_for_definitions_assigned_as_a_collection() {
    // Definitions supplied through the column definitions setter - an object initializer, a
    // shared resource, or a parsed "Auto,*" - are already in the collection when the
    // grid claims it, so they never pass through the collection-changed handler that joins
    // them to the parent tree.
    let _scope = test_scope();
    let grids = [Grid::new(), Grid::new()];
    for grid in &grids {
        grid.set_column_definitions(ColumnDefinitions::from_items([
            column(GridLength::AUTO, Some("A")),
            column(star(1.0), None),
        ]));
    }
    grids[0].children().add(border(Some(50.0), Some(10.0)));

    let scope = TestStackPanel::shared_size_scope(&[&grids[0], &grids[1]]);
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    // Shared groups validate after layout and apply any resulting invalidation on the next pass.
    execute_layout_pass(&root);

    assert_eq!(50.0, grids[0].column_definitions().get(0).actual_width());
    assert_eq!(50.0, grids[1].column_definitions().get(0).actual_width());
}

#[test]
fn replacing_definition_collection_releases_its_shared_size_group() {
    // The outgoing definitions are no longer reachable from the grid, so nothing resets their
    // measured minimum. Left registered, they keep the group pinned at whatever size they
    // last contributed.
    let _scope = test_scope();
    let grid = shared_auto_and_star_grid();
    grid.children().add(border(Some(50.0), Some(10.0)));

    let other = shared_auto_and_star_grid();

    let scope = TestStackPanel::shared_size_scope(&[&grid, &other]);
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    // Shared groups validate after layout and apply any resulting invalidation on the next pass.
    execute_layout_pass(&root);
    assert_eq!(50.0, other.column_definitions().get(0).actual_width());

    grid.set_column_definitions(ColumnDefinitions::from_items([
        column(GridLength::AUTO, None),
        column(star(1.0), None),
    ]));
    execute_layout_pass(&root);
    execute_layout_pass(&root);

    assert_eq!(0.0, other.column_definitions().get(0).actual_width());
}

#[test]
fn removing_definition_detaches_it_from_the_grid() {
    let _scope = test_scope();
    let shared = column(GridLength::AUTO, Some("A"));
    let grid = Grid::new();
    grid.column_definitions().add(shared.clone());
    grid.column_definitions().add(column(star(1.0), None));
    grid.children().add(border(Some(50.0), Some(10.0)));

    let other = shared_auto_and_star_grid();

    let scope = TestStackPanel::shared_size_scope(&[&grid, &other]);
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    execute_layout_pass(&root);
    assert_eq!(50.0, other.column_definitions().get(0).actual_width());

    grid.column_definitions().remove(&shared);
    execute_layout_pass(&root);
    execute_layout_pass(&root);
    assert_eq!(0.0, other.column_definitions().get(0).actual_width());
    assert!(shared.parent().is_none());

    // A definition that has left the grid must no longer see the grid's scope.
    shared.set_shared_size_group(None);
    shared.set_shared_size_group(Some("A"));
    execute_layout_pass(&root);
    execute_layout_pass(&root);
    assert_eq!(0.0, other.column_definitions().get(0).actual_width());
}

#[test]
fn moving_definition_between_grids_moves_its_shared_size_registration() {
    let _scope = test_scope();
    let shared = column(GridLength::AUTO, Some("A"));
    let source = Grid::new();
    source.column_definitions().add(shared.clone());
    source.column_definitions().add(column(star(1.0), None));
    source.children().add(border(Some(50.0), Some(10.0)));

    let source_partner = shared_auto_and_star_grid();

    let target = Grid::new();
    target.column_definitions().add(column(star(1.0), None));

    let target_partner = shared_auto_and_star_grid();
    target_partner.children().add(border(Some(20.0), Some(10.0)));

    let source_scope = TestStackPanel::shared_size_scope(&[&source, &source_partner]);
    let target_scope = TestStackPanel::shared_size_scope(&[&target, &target_partner]);
    let panel = TestStackPanel::new();
    panel.children().add(&source_scope);
    panel.children().add(&target_scope);
    let root = TestRoot::with_child(&panel);

    root.execute_initial_layout_pass();
    execute_layout_pass(&root);
    assert_eq!(50.0, source_partner.column_definitions().get(0).actual_width());
    assert_eq!(20.0, target_partner.column_definitions().get(0).actual_width());

    source.column_definitions().remove(&shared);
    target.column_definitions().insert(0, shared.clone());
    execute_layout_pass(&root);
    execute_layout_pass(&root);

    assert_eq!(target, shared.parent().unwrap());
    assert_eq!(0.0, source_partner.column_definitions().get(0).actual_width());
    assert_eq!(20.0, target.column_definitions().get(0).actual_width());
}

#[test]
fn reassigning_the_same_definition_collection_is_inert() {
    let _scope = test_scope();
    let grid = shared_auto_and_star_grid();
    grid.children().add(border(Some(50.0), Some(10.0)));

    let other = shared_auto_and_star_grid();

    let scope = TestStackPanel::shared_size_scope(&[&grid, &other]);
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    execute_layout_pass(&root);
    assert_eq!(50.0, other.column_definitions().get(0).actual_width());

    let definitions = grid.column_definitions();
    grid.set_column_definitions(definitions.clone());
    execute_layout_pass(&root);
    execute_layout_pass(&root);

    assert!(definitions == grid.column_definitions());
    assert_eq!(50.0, other.column_definitions().get(0).actual_width());
}

#[test]
fn shared_size_group_is_registered_for_row_definitions_assigned_as_a_collection() {
    let _scope = test_scope();
    let grids = [Grid::new(), Grid::new()];
    for grid in &grids {
        grid.set_row_definitions(RowDefinitions::from_items([
            row(GridLength::AUTO, Some("A")),
            row(star(1.0), None),
        ]));
    }
    grids[0].children().add(border(Some(10.0), Some(50.0)));

    let scope = TestStackPanel::shared_size_scope(&[&grids[0], &grids[1]]);
    scope.orientation.set(Orientation::Horizontal);
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    execute_layout_pass(&root);

    assert_eq!(50.0, grids[0].row_definitions().get(0).actual_height());
    assert_eq!(50.0, grids[1].row_definitions().get(0).actual_height());
}

#[test]
fn collection_changes_are_tracked() {
    let grid = create_grid(&[
        (Some("A"), pixels(20.0)),
        (Some("A"), pixels(30.0)),
        (Some("A"), pixels(40.0)),
        (None, GridLength::default()),
    ]);
    let _root = root_with_scope(&grid, true);

    measure_and_arrange(&grid, 200.0);
    assert_group_width(&grid, Some("A"), 40.0);

    grid.column_definitions().remove_at(2);

    // NOTE: THIS IS BROKEN IN WPF
    // measure_and_arrange(&grid, 200.0);
    // assert_group_width(&grid, Some("A"), 30.0);

    grid.column_definitions().insert(1, column(pixels(30.0), Some("A")));

    measure_and_arrange(&grid, 200.0);
    assert_group_width(&grid, Some("A"), 30.0);

    grid.column_definitions().set(1, column(pixels(10.0), Some("A")));

    measure_and_arrange(&grid, 200.0);
    assert_group_width(&grid, Some("A"), 30.0);

    grid.column_definitions().set(1, column(pixels(50.0), Some("A")));

    // NOTE: THIS IS BROKEN IN WPF
    // measure_and_arrange(&grid, 200.0);
    // assert_group_width(&grid, Some("A"), 0.0);
}

#[test]
fn size_priorities_are_maintained() {
    let mut sizers = Vec::new();
    let grid = create_grid(&[
        (Some("A"), pixels(20.0)),
        (Some("A"), auto(20.0)),
        (Some("A"), star(1.0)),
        (Some("A"), star(1.0)),
        (None, GridLength::default()),
    ]);
    for i in 0..3 {
        sizers.push(add_sizer(&grid, i, 6.0 + i as f64 * 6.0));
    }
    let _root = root_with_scope(&grid, true);

    measure_and_arrange(&grid, 100.0);
    // all in group are equal to the first fixed column
    assert_group_width(&grid, Some("A"), 20.0);

    grid.column_definitions().get(0).set_shared_size_group(None);

    measure_and_arrange(&grid, 100.0);

    // NOTE: THIS IS BROKEN IN WPF
    // assert_group_width(&grid, Some("A"), 6.0 + 2.0 * 6.0);
    // grid.column_definitions().get(1).set_shared_size_group(None);

    // measure_and_arrange(&grid, 100.0);

    // NOTE: THIS IS BROKEN IN WPF
    // all in group are equal to width (MinWidth) of the sizer in the second column
    // assert_group_width(&grid, Some("A"), 6.0 + 1.0 * 6.0);

    // NOTE: THIS IS BROKEN IN WPF
    // grid.column_definitions().get(2).set_shared_size_group(None);

    // NOTE: THIS IS BROKEN IN WPF
    // grid.measure(Size::new(f64::INFINITY, 100.0));
    // grid.arrange(Rect::from_points(Point::default(), Point::new(100.0, 100.0)));
    // with no constraint star columns default to the MinWidth of the sizer in the column
    // assert_group_width(&grid, Some("A"), 0.0);
    assert_eq!(3, sizers.len());
}

#[test]
fn column_definitions_collection_is_read_only() {
    let grid = create_grid(&one_group(pixels(50.0)));
    let _root = root_with_scope(&grid, true);

    measure_and_arrange(&grid, 200.0);
    assert_group_width(&grid, Some("A"), 50.0);

    grid.column_definitions().set(0, column(pixels(25.0), Some("A")));
    grid.column_definitions().set(1, column(pixels(75.0), Some("B")));
    grid.column_definitions().set(2, column(pixels(75.0), Some("B")));
    grid.column_definitions().set(3, column(pixels(25.0), Some("A")));

    measure_and_arrange(&grid, 200.0);
    assert_group_width(&grid, Some("A"), 25.0);
    assert_group_width(&grid, Some("B"), 75.0);
}

#[test]
fn column_definitions_collection_reset_shared_size_group() {
    let grid = create_grid(&two_groups(pixels(25.0), pixels(75.0)));
    let _root = root_with_scope(&grid, true);

    measure_and_arrange(&grid, 200.0);
    assert_group_width(&grid, Some("A"), 25.0);
    assert_group_width(&grid, Some("B"), 75.0);

    for i in 0..4 {
        grid.column_definitions().get(i).set_shared_size_group(None);
        grid.column_definitions().get(i).set_width(pixels(50.0));
    }

    measure_and_arrange(&grid, 200.0);
    assert_group_width(&grid, None, 50.0);
}

#[test]
fn correct_grid_bounds_when_child_control_has_desired_size_larger_than_available_space() {
    // Issue #2746
    let grid = Grid::new();
    grid.set_row_definitions(RowDefinitions::parse("Auto").unwrap());
    grid.children().add(TestControl::new(Size::new(150.0, 150.0)));

    let parent = Decorator::new();
    parent.set_child(&grid);

    parent.measure(Size::new(100.0, 100.0));
    parent.arrange(Rect::from_size(grid.desired_size()));

    assert_eq!(Size::new(100.0, 100.0), grid.bounds().size());
}

fn change_property_and_verify_measure_requested(grid: &Grid, change: impl FnOnce()) {
    grid.measure(Size::new(100.0, 100.0));
    grid.arrange(Rect::from_size(grid.desired_size()));

    assert!(grid.is_measure_valid());
    assert!(grid.is_arrange_valid());

    change();

    assert!(!grid.is_measure_valid());
    assert!(!grid.is_arrange_valid());
}

fn grid_with_columns(s: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_column_definitions(ColumnDefinitions::parse(s).unwrap());
    grid
}

fn grid_with_rows(s: &str) -> Ref<Grid> {
    let grid = Grid::new();
    grid.set_row_definitions(RowDefinitions::parse(s).unwrap());
    grid
}

#[test]
fn changing_column_width_should_invalidate_grid() {
    for set_using_property in [true, false] {
        let grid = grid_with_columns("1*,1*");

        change_property_and_verify_measure_requested(&grid, || {
            let definition = grid.column_definitions().get(0);
            if set_using_property {
                definition.set_value(ColumnDefinition::width_property(), pixels(5.0));
            } else {
                definition.set_width(pixels(5.0));
            }
        });
    }
}

#[test]
fn changing_column_min_width_should_invalidate_grid() {
    for set_using_property in [true, false] {
        let grid = grid_with_columns("1*,1*");

        change_property_and_verify_measure_requested(&grid, || {
            let definition = grid.column_definitions().get(0);
            if set_using_property {
                definition.set_value(ColumnDefinition::min_width_property(), 5.0);
            } else {
                definition.set_min_width(5.0);
            }
        });
    }
}

#[test]
fn changing_column_max_width_should_invalidate_grid() {
    for set_using_property in [true, false] {
        let grid = grid_with_columns("1*,1*");

        change_property_and_verify_measure_requested(&grid, || {
            let definition = grid.column_definitions().get(0);
            if set_using_property {
                definition.set_value(ColumnDefinition::max_width_property(), 5.0);
            } else {
                definition.set_max_width(5.0);
            }
        });
    }
}

#[test]
fn changing_row_height_should_invalidate_grid() {
    for set_using_property in [true, false] {
        let grid = grid_with_rows("1*,1*");

        change_property_and_verify_measure_requested(&grid, || {
            let definition = grid.row_definitions().get(0);
            if set_using_property {
                definition.set_value(RowDefinition::height_property(), pixels(5.0));
            } else {
                definition.set_height(pixels(5.0));
            }
        });
    }
}

#[test]
fn changing_row_min_height_should_invalidate_grid() {
    for set_using_property in [true, false] {
        let grid = grid_with_rows("1*,1*");

        change_property_and_verify_measure_requested(&grid, || {
            let definition = grid.row_definitions().get(0);
            if set_using_property {
                definition.set_value(RowDefinition::min_height_property(), 5.0);
            } else {
                definition.set_min_height(5.0);
            }
        });
    }
}

#[test]
fn changing_row_max_height_should_invalidate_grid() {
    for set_using_property in [true, false] {
        let grid = grid_with_rows("1*,1*");

        change_property_and_verify_measure_requested(&grid, || {
            let definition = grid.row_definitions().get(0);
            if set_using_property {
                definition.set_value(RowDefinition::max_height_property(), 5.0);
            } else {
                definition.set_max_height(5.0);
            }
        });
    }
}

#[test]
fn adding_column_should_invalidate_grid() {
    let grid = grid_with_columns("1*,1*");

    change_property_and_verify_measure_requested(&grid, || {
        grid.column_definitions().add(ColumnDefinition::with_width(pixels(5.0)));
    });
}

#[test]
fn adding_row_should_invalidate_grid() {
    let grid = grid_with_rows("1*,1*");

    change_property_and_verify_measure_requested(&grid, || {
        grid.row_definitions().add(RowDefinition::with_height(pixels(5.0)));
    });
}

#[test]
fn replacing_columns_should_invalidate_grid() {
    let grid = grid_with_columns("1*,1*");

    change_property_and_verify_measure_requested(&grid, || {
        grid.set_column_definitions(ColumnDefinitions::parse("2*,1*").unwrap());
    });
}

#[test]
fn replacing_rows_should_invalidate_grid() {
    let grid = grid_with_rows("1*,1*");

    change_property_and_verify_measure_requested(&grid, || {
        grid.set_row_definitions(RowDefinitions::parse("2*,1*").unwrap());
    });
}

#[test]
fn removing_column_should_invalidate_grid() {
    let grid = grid_with_columns("1*,1*");

    change_property_and_verify_measure_requested(&grid, || {
        grid.column_definitions().remove_at(0);
    });
}

#[test]
fn removing_row_should_invalidate_grid() {
    let grid = grid_with_rows("1*,1*");

    change_property_and_verify_measure_requested(&grid, || {
        grid.row_definitions().remove_at(0);
    });
}

fn sized_decorator(width: f64, height: f64) -> Ref<Decorator> {
    let result = Decorator::new();
    result.set_width(width);
    result.set_height(height);
    result
}

#[test]
fn removing_child_should_invalidate_grid_and_be_operational() {
    let grid = grid_with_columns("*,Auto");

    grid.children().add(cell(Decorator::new(), 0, 0));
    grid.children().add(cell(sized_decorator(10.0, 10.0), 0, 1));

    let size = Size::new(100.0, 100.0);
    grid.measure(size);
    grid.arrange(Rect::from_size(size));

    assert!(grid.is_measure_valid());
    assert!(grid.is_arrange_valid());

    assert_eq!(90.0, child_bounds(&grid, 0).width);
    assert_eq!(10.0, child_bounds(&grid, 1).width);

    grid.children().remove_at(1);

    assert!(!grid.is_measure_valid());
    assert!(!grid.is_arrange_valid());

    grid.measure(size);
    grid.arrange(Rect::from_size(size));

    assert!(grid.is_measure_valid());
    assert!(grid.is_arrange_valid());

    assert_eq!(100.0, child_bounds(&grid, 0).width);
}

#[test]
fn adding_child_should_invalidate_grid_and_be_operational() {
    let grid = grid_with_columns("*,Auto");

    grid.children().add(cell(Decorator::new(), 0, 0));

    let size = Size::new(100.0, 100.0);
    grid.measure(size);
    grid.arrange(Rect::from_size(size));

    assert!(grid.is_measure_valid());
    assert!(grid.is_arrange_valid());

    assert_eq!(100.0, child_bounds(&grid, 0).width);

    grid.children().add(cell(sized_decorator(10.0, 10.0), 0, 1));

    assert!(!grid.is_measure_valid());
    assert!(!grid.is_arrange_valid());

    grid.measure(size);
    grid.arrange(Rect::from_size(size));

    assert!(grid.is_measure_valid());
    assert!(grid.is_arrange_valid());

    assert_eq!(90.0, child_bounds(&grid, 0).width);
    assert_eq!(10.0, child_bounds(&grid, 1).width);
}

#[test]
fn should_grid_controls_with_spacing() {
    let target = Grid::new();
    target.set_row_spacing(10.0);
    target.set_column_spacing(10.0);
    target.set_row_definitions(RowDefinitions::parse("100,100").unwrap());
    target.set_column_definitions(ColumnDefinitions::parse("100,100").unwrap());
    target.children().add(Border::new());
    target.children().add(cell(Border::new(), 0, 1));
    target.children().add(cell(Border::new(), 1, 0));
    target.children().add(cell(Border::new(), 1, 1));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Rect::new(0.0, 0.0, 210.0, 210.0), target.bounds());
    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), child_bounds(&target, 0));
    assert_eq!(Rect::new(110.0, 0.0, 100.0, 100.0), child_bounds(&target, 1));
    assert_eq!(Rect::new(0.0, 110.0, 100.0, 100.0), child_bounds(&target, 2));
    assert_eq!(Rect::new(110.0, 110.0, 100.0, 100.0), child_bounds(&target, 3));
}

#[test]
fn should_grid_controls_with_spacing_complicated() {
    let target = Grid::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_row_spacing(10.0);
    target.set_column_spacing(10.0);
    target.set_row_definitions(RowDefinitions::parse("50,*,2*,Auto").unwrap());
    target.set_column_definitions(ColumnDefinitions::parse("50,*,2*,Auto").unwrap());
    target.children().add(Border::new());
    target.children().add(cell(Border::new(), 1, 1));
    target.children().add(cell(Border::new(), 2, 2));
    target.children().add(cell(border(Some(30.0), Some(30.0)), 3, 3));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Rect::new(0.0, 0.0, 200.0, 200.0), target.bounds());
    assert_eq!(Rect::new(0.0, 0.0, 50.0, 50.0), child_bounds(&target, 0));
    assert_eq!(Rect::new(60.0, 60.0, 30.0, 30.0), child_bounds(&target, 1));
    assert_eq!(Rect::new(100.0, 100.0, 60.0, 60.0), child_bounds(&target, 2));
    assert_eq!(Rect::new(170.0, 170.0, 30.0, 30.0), child_bounds(&target, 3));
}

#[test]
fn should_grid_controls_with_spacing_overflow() {
    let target = Grid::new();
    target.set_width(100.0);
    target.set_height(100.0);
    target.set_column_spacing(20.0);
    target.set_row_spacing(20.0);
    target.set_column_definitions(ColumnDefinitions::parse("30,*,*,Auto").unwrap());
    target.set_row_definitions(RowDefinitions::parse("30,*,*,Auto").unwrap());
    target.children().add(Border::new());
    target.children().add(cell(Border::new(), 1, 1));
    target.children().add(cell(Border::new(), 2, 2));
    target.children().add(cell(border(Some(30.0), Some(30.0)), 3, 3));

    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), target.bounds());
    assert_eq!(Rect::new(0.0, 0.0, 30.0, 30.0), child_bounds(&target, 0));
    assert_eq!(Rect::new(50.0, 50.0, 0.0, 0.0), child_bounds(&target, 1));
    assert_eq!(Rect::new(70.0, 70.0, 0.0, 0.0), child_bounds(&target, 2));
    assert_eq!(Rect::new(90.0, 90.0, 30.0, 30.0), child_bounds(&target, 3));
}

#[test]
fn should_grid_controls_with_spacing_overflow2() {
    let target = Grid::new();
    target.set_height(100.0);
    target.set_column_spacing(20.0);
    target.set_column_definitions(ColumnDefinitions::parse("*,Auto").unwrap());
    target.children().add(border(Some(60.0), None));
    target.children().add(cell(border(Some(60.0), None), 0, 1));

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), target.bounds());
    assert_eq!(Rect::new(-20.0, 0.0, 60.0, 100.0), child_bounds(&target, 0));
    assert_eq!(Rect::new(40.0, 0.0, 60.0, 100.0), child_bounds(&target, 1));
}

#[test]
fn grid_controls_with_spacing_with_span() {
    let target = Grid::new();
    target.set_column_spacing(20.0);
    target.set_row_definitions(RowDefinitions::parse("Auto").unwrap());
    target.set_column_definitions(ColumnDefinitions::parse("20,20").unwrap());
    let child = border(None, Some(100.0));
    Grid::set_column_span(&child, 2);
    target.children().add(child);

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(Rect::new(0.0, 0.0, 60.0, 100.0), target.bounds());
    assert_eq!(Rect::new(0.0, 0.0, 60.0, 100.0), child_bounds(&target, 0));
}

#[test]
fn grid_controls_with_spacing_with_span_and_shared_size() {
    let _scope = test_scope();

    let sized_button = |row: i32, column: i32| {
        let button = Button::new();
        button.set_width(100.0);
        button.set_height(40.0);
        cell(button, row, column)
    };

    let grid1 = Grid::new();
    Grid::set_row(&grid1, 0);
    grid1.set_row_definitions(RowDefinitions::parse("Auto,*,Auto,Auto").unwrap());
    grid1.set_column_definitions(ColumnDefinitions::from_items([
        column(GridLength::AUTO, None),
        column(GridLength::STAR, None),
        column(GridLength::AUTO, None),
        column(GridLength::AUTO, Some("C3")),
    ]));
    grid1.set_row_spacing(10.0);
    grid1.set_column_spacing(10.0);
    let text = TextBlock::new();
    text.set_font_size(10.0);
    text.set_text(Some(
        "0: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
1: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
2: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
3: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
4: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
5: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
6: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
7: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
8: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890
9: 1234567890 1234567890 1234567890 1234567890 1234567890 1234567890",
    ));
    let scroll_viewer = ScrollViewer::new();
    scroll_viewer.set_content(Some(Control::boxed(text)));
    let scroll_viewer = cell(scroll_viewer, 0, 0);
    Grid::set_row_span(&scroll_viewer, 3);
    Grid::set_column_span(&scroll_viewer, 3);
    grid1.children().add(scroll_viewer);
    grid1.children().add(sized_button(3, 0));
    grid1.children().add(sized_button(3, 2));
    grid1.children().add(sized_button(0, 3));
    grid1.children().add(sized_button(2, 3));

    let grid2 = Grid::new();
    Grid::set_row(&grid2, 1);
    grid2.set_column_definitions(ColumnDefinitions::from_items([
        column(GridLength::STAR, None),
        column(GridLength::AUTO, Some("C3")),
    ]));
    let text = TextBlock::new();
    Grid::set_column(&text, 1);
    text.set_height(20.0);
    text.set_width(100.0);
    text.set_text(Some("1234567890"));
    grid2.children().add(text);

    let root = Grid::new();
    root.set_value(Grid::is_shared_size_scope_property(), true);
    root.set_row_definitions(RowDefinitions::parse("*,Auto").unwrap());
    root.set_row_spacing(10.0);
    root.set_margin(Thickness::uniform(10.0));
    root.children().add(&grid1);
    root.children().add(&grid2);
    root.measure(Size::new(550.0, 240.0));
    root.arrange(Rect::from_points(Point::default(), Point::new(550.0, 240.0)));

    assert_eq!(Rect::new(0.0, 0.0, 420.0, 140.0), child_bounds(&grid1, 0));
    assert_eq!(child_bounds(&grid1, 4).left(), child_bounds(&grid2, 0).left());
    assert_eq!(child_bounds(&grid1, 4).width, child_bounds(&grid2, 0).width);
}

#[test]
fn grid_with_column_spacing_and_column_definitions_unset() {
    let target = Grid::new();
    target.set_height(300.0);
    target.set_width(100.0);
    target.set_column_spacing(10.0);
    // Set the row definitions to leave the trivial layout path.
    target.set_row_definitions(RowDefinitions::parse("Auto,*").unwrap());
    let first = border(None, Some(80.0));
    first.set_margin(Thickness::uniform(10.0));
    target.children().add(cell(first, 0, 0));
    let second = Border::new();
    second.set_margin(Thickness::uniform(20.0));
    target.children().add(cell(second, 1, 0));

    target.measure(Size::new(100.0, 300.0));
    target.arrange(Rect::from_size(target.desired_size()));
    assert_eq!(Rect::new(10.0, 10.0, 80.0, 80.0), child_bounds(&target, 0));
    assert_eq!(Rect::new(20.0, 120.0, 60.0, 160.0), child_bounds(&target, 1));
}

// The tests below are not part of the reference suite.

#[test]
fn definitions_format_and_parse() {
    let columns = ColumnDefinitions::parse("Auto,*,2*,4").unwrap();
    assert_eq!(4, columns.count());
    assert_eq!("Auto,1*,2*,4", columns.to_string());
    assert!(columns.get(0).width().is_auto());

    let rows: RowDefinitions = "100 * Auto".parse().unwrap();
    assert_eq!(3, rows.count());
    assert_eq!("100,1*,Auto", rows.to_string());

    assert!(ColumnDefinitions::parse("Auto,x").is_err());
    assert!(RowDefinitions::parse("1,,2").is_err());
}

#[test]
fn definitions_track_their_index_and_parent() {
    let grid = Grid::new();
    let first = ColumnDefinition::new();
    let second = ColumnDefinition::new();
    assert_eq!(-1, first.index());

    grid.column_definitions().add(first.clone());
    grid.column_definitions().insert(0, second.clone());

    assert_eq!(1, first.index());
    assert_eq!(0, second.index());
    assert_eq!(grid, first.parent().unwrap());
    assert_eq!(grid, first.inheritance_parent().unwrap());

    grid.column_definitions().clear();

    assert!(first.parent().is_none());
    assert!(second.parent().is_none());
    assert!(first.inheritance_parent().is_none());
}

#[test]
fn shared_size_scope_set_before_grids_are_added_is_tracked() {
    let _scope = test_scope();
    let scope = TestStackPanel::new();
    Grid::set_is_shared_size_scope(&scope, true);
    assert!(Grid::get_is_shared_size_scope(&scope));

    let grids = [shared_auto_and_star_grid(), shared_auto_and_star_grid()];
    grids[0].children().add(border(Some(50.0), Some(10.0)));
    for grid in &grids {
        scope.children().add(grid);
    }
    let root = TestRoot::with_child(&scope);

    root.execute_initial_layout_pass();
    execute_layout_pass(&root);

    assert_eq!(50.0, grids[1].column_definitions().get(0).actual_width());

    Grid::set_is_shared_size_scope(&scope, false);
    execute_shared_size_layout_pass(&root);

    assert_eq!(0.0, grids[1].column_definitions().get(0).actual_width());
}

#[test]
fn shared_size_group_must_be_a_valid_identifier() {
    let definition = ColumnDefinition::new();
    definition.set_shared_size_group(Some("Group_1"));
    assert_eq!(Some("Group_1".to_owned()), definition.shared_size_group());

    for invalid in ["", "1Group", "Group 1", "a-b"] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ColumnDefinition::new().set_shared_size_group(Some(invalid));
        }));
        assert!(result.is_err(), "'{invalid}' should be rejected");
    }
}

#[test]
fn star_columns_are_rounded_to_layout_scaling() {
    let _scope = test_scope();
    for (scaling, expected) in [(1.0, [34.0, 33.0, 33.0]), (1.25, [33.6, 33.6, 32.8])] {
        let grid = grid_with_columns("*,*,*");
        for i in 0..3 {
            grid.children().add(cell(Border::new(), 0, i));
        }
        let root = TestRoot::with_child(&grid);
        root.set_layout_scaling(scaling);
        root.set_client_size(Size::new(100.0, 100.0));
        root.set_width(100.0);
        root.set_height(100.0);

        root.execute_initial_layout_pass();

        let widths: Vec<f64> = (0..3).map(|i| child_bounds(&grid, i).width).collect();
        assert!((100.0 - widths.iter().sum::<f64>()).abs() < 1e-9);
        for (width, expected) in widths.iter().zip(expected) {
            assert!((width - expected).abs() < 1e-9, "{widths:?} at scaling {scaling}");
        }
    }
}

#[test]
fn row_and_column_properties_reject_invalid_values() {
    for (property, value) in [
        (Grid::column_property(), -1),
        (Grid::row_property(), -1),
        (Grid::column_span_property(), 0),
        (Grid::row_span_property(), 0),
    ] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Border::new().set_value(property, value);
        }));
        assert!(result.is_err());
    }
}

/// The path the menu templates of the reference theme rely on: the scope is
/// set on an items presenter, which is not a logical ancestor of the menu
/// items; they take it from their visual parent.
#[test]
fn shared_size_scope_on_items_presenter_reaches_menu_item_templates() {
    use crate::presenters::ItemsPresenter;
    use crate::primitives::TemplatedControl;
    use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
    use crate::{ItemsControl, MenuItem};
    use ferroui_base::data::TemplateBinding;
    use std::rc::Rc;

    for is_shared_size_scope in [true, false] {
        let _scope = test_scope();

        let menu_item = |width: f64| {
            let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(move |_, ns| {
                let grid = Grid::new();
                grid.set_name(Some("grid".to_string()));
                grid.set_column_definitions(ColumnDefinitions::from_items([
                    column(GridLength::AUTO, Some("MenuItemIGT")),
                    column(GridLength::STAR, None),
                ]));
                grid.children().add(border(Some(width), Some(10.0)));
                grid.register_in_name_scope(&**ns).upcast()
            });
            let result = MenuItem::new();
            result.set_template(Some(template));
            result
        };
        let items = [menu_item(40.0), menu_item(100.0)];

        let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(move |_, ns| {
            let presenter = ItemsPresenter::new();
            presenter.set_name(Some("PART_ItemsPresenter".to_string()));
            Grid::set_is_shared_size_scope(&presenter, is_shared_size_scope);
            let property = ItemsControl::items_panel_property().as_property();
            presenter.bind_binding(property, &TemplateBinding::new(property));
            presenter.register_in_name_scope(&**ns).upcast()
        });
        let target = ItemsControl::new();
        target.set_value(TemplatedControl::template_property(), Some(template));
        for item in &items {
            target.items().add(Some(Control::boxed(item.clone())));
        }

        let root = TestRoot::with_child(&target);
        root.execute_initial_layout_pass();
        execute_shared_size_layout_pass(&root);

        let widths: Vec<f64> = items
            .iter()
            .map(|item| {
                let grid = item.visual_children().get(0).cast::<Grid>().unwrap();
                grid.column_definitions().get(0).actual_width()
            })
            .collect();
        let expected = if is_shared_size_scope { [100.0, 100.0] } else { [40.0, 100.0] };
        assert_eq!(&expected[..], &widths[..]);
    }
}
