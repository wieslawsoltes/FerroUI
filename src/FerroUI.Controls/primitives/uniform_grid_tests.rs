use crate::primitives::UniformGrid;
use crate::Border;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{Rect, Ref, Size};

fn border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

fn grid(children: &[(f64, f64)]) -> Ref<UniformGrid> {
    let target = UniformGrid::new();
    for &(width, height) in children {
        target.children().add(border(width, height));
    }
    target
}

fn measure_and_arrange(target: &UniformGrid) {
    target.measure(Size::INFINITY);
    target.arrange(Rect::from_size(target.desired_size()));
}

const FIVE: [(f64, f64); 5] = [(50.0, 70.0), (30.0, 50.0), (80.0, 90.0), (20.0, 30.0), (40.0, 60.0)];

#[test]
fn grid_columns_equals_rows_for_auto_columns_and_rows() {
    let target = grid(&FIVE[..3]);

    measure_and_arrange(&target);

    // 2 * 2 grid => each cell: 80 x 90
    // Final size => (2 * 80) x (2 * 90) = 160 x 180
    assert_eq!(target.bounds().size(), Size::new(160.0, 180.0));
}

#[test]
fn grid_expands_vertically_for_columns_with_auto_rows() {
    let target = grid(&FIVE);
    target.set_columns(2);

    measure_and_arrange(&target);

    // 2 * 3 grid => each cell: 80 x 90
    // Final size => (2 * 80) x (3 * 90) = 160 x 270
    assert_eq!(target.bounds().size(), Size::new(160.0, 270.0));
}

#[test]
fn grid_extends_for_columns_and_first_column_with_auto_rows() {
    let target = grid(&FIVE);
    target.set_columns(3);
    target.set_first_column(2);

    measure_and_arrange(&target);

    // 3 * 3 grid => each cell: 80 x 90
    // Final size => (3 * 80) x (3 * 90) = 240 x 270
    assert_eq!(target.bounds().size(), Size::new(240.0, 270.0));
}

#[test]
fn grid_expands_horizontally_for_rows_with_auto_columns() {
    let target = grid(&FIVE);
    target.set_rows(2);

    measure_and_arrange(&target);

    // 3 * 2 grid => each cell: 80 x 90
    // Final size => (3 * 80) x (2 * 90) = 240 x 180
    assert_eq!(target.bounds().size(), Size::new(240.0, 180.0));
}

#[test]
fn grid_size_is_limited_by_rows_and_columns() {
    let target = grid(&FIVE);
    target.set_columns(2);
    target.set_rows(2);

    measure_and_arrange(&target);

    // 2 * 2 grid => each cell: 80 x 90
    // Final size => (2 * 80) x (2 * 90) = 160 x 180
    assert_eq!(target.bounds().size(), Size::new(160.0, 180.0));
}

#[test]
fn not_visible_children_are_ignored() {
    let target = grid(&FIVE);
    target.children().get(2).set_is_visible(false);

    measure_and_arrange(&target);

    // Visible children: 4
    // Auto => 2 x 2 grid => each cell: 50 x 70
    // Final size => (2 * 50) x (2 * 70) = 100 x 140
    assert_eq!(target.bounds().size(), Size::new(100.0, 140.0));
}

#[test]
fn grid_respects_column_spacing_for_auto_columns_and_rows() {
    // We have 3 visible children and no fixed rows/columns => 2x2 grid
    // Largest child is 80 x 90. Column spacing = 10, row spacing = 0
    let target = grid(&FIVE[..3]);
    target.set_column_spacing(10.0);

    measure_and_arrange(&target);

    // Without spacing => width = 2*80 = 160, height = 2*90 = 180
    // With column spacing 10 => total width = 2*80 + 1*10 = 170
    // Row spacing 0 => total height = 180
    assert_eq!(target.bounds().size(), Size::new(170.0, 180.0));
}

#[test]
fn grid_respects_row_spacing_for_auto_columns_and_rows() {
    // 3 visible children => 2x2 grid again
    // Largest child is 80 x 90. Row spacing = 15, column spacing = 0
    let target = grid(&FIVE[..3]);
    target.set_row_spacing(15.0);

    measure_and_arrange(&target);

    // Without spacing => width = 160, height = 180
    // With row spacing 15 => total height = 2*90 + 1*15 = 195
    // Column spacing 0 => total width = 160
    assert_eq!(target.bounds().size(), Size::new(160.0, 195.0));
}

#[test]
fn grid_respects_both_row_and_column_spacing_for_fixed_grid() {
    // 4 visible children => 2 rows x 2 columns; the largest child dictates
    // the cell size: 80x90. Row spacing 10, column spacing 5.
    let target = grid(&FIVE[..4]);
    target.set_rows(2);
    target.set_columns(2);
    target.set_row_spacing(10.0);
    target.set_column_spacing(5.0);

    measure_and_arrange(&target);

    // Each cell = 80 x 90
    // Final width = (2 * 80) + (1 * 5)  = 160 + 5  = 165
    // Final height = (2 * 90) + (1 * 10) = 180 + 10 = 190
    assert_eq!(target.bounds().size(), Size::new(165.0, 190.0));
}

#[test]
fn grid_respects_spacing_when_invisible_child_exists() {
    // 3 *visible* children => auto => 2x2 grid.
    // Spacing is added to confirm no extra columns/rows are added for the
    // invisible child.
    let target = grid(&[(50.0, 70.0), (80.0, 90.0), (30.0, 50.0), (40.0, 60.0)]);
    target.set_row_spacing(5.0);
    target.set_column_spacing(5.0);
    target.children().get(1).set_is_visible(false);

    measure_and_arrange(&target);

    // The largest visible child is 50x70. So each cell is 50x70.
    // For a 2x2 grid with 3 visible children:
    //  - total width  = (2 * 50) + (1 * 5) = 100 + 5  = 105
    //  - total height = (2 * 70) + (1 * 5) = 140 + 5 = 145
    assert_eq!(target.bounds().size(), Size::new(105.0, 145.0));
}

#[test]
fn grid_ensures_consistent_cell_width_when_use_layout_rounding() {
    // Test scenario: 800x600 resolution, 21 children, 3 rows, 7 columns, 1
    // pixel spacing. Verifies that all cells have consistent width when
    // layout rounding is enabled.
    let target = UniformGrid::new();
    target.set_rows(3);
    target.set_columns(7);
    target.set_row_spacing(1.0);
    target.set_column_spacing(1.0);
    target.set_use_layout_rounding(true);

    // Add 21 children
    for _ in 0..21 {
        target.children().add(Border::new());
    }

    // Arrange at 800x600 resolution
    target.measure(Size::new(800.0, 600.0));
    target.arrange(Rect::new(0.0, 0.0, 800.0, 600.0));

    // Available width = 800, column spacing takes up 6 pixels (7 columns - 1
    // = 6 gaps). Available width for cells = 800 - 6 = 794. Width per cell =
    // 794 / 7 = 113.428... With layout rounding, this should be rounded to
    // ensure consistent cell sizes.
    let expected_cell_width = ((800.0 - 6.0) / 7.0_f64).round_ties_even();

    // Verify all children have the same width
    let children = target.children().to_vec();
    let first_child_width = children[0].bounds().width;
    for (i, child) in children.iter().enumerate() {
        assert!(child.is::<Border>());
        assert!(
            (child.bounds().width - first_child_width).abs() < 0.01,
            "Child {i} has width {}, expected {first_child_width}",
            child.bounds().width
        );
    }

    // Verify the calculated width matches expected
    assert!(
        (first_child_width - expected_cell_width).abs() < 0.01,
        "Child width {first_child_width} does not match expected {expected_cell_width}"
    );
}

#[test]
fn measure_with_rows_and_columns_zero_and_non_zero_spacing_produces_zero_desired_size() {
    // The measure override is called by the measure core, which ensures that
    // the desired size is never negative; the override itself is called here
    // because derived classes see its raw result.
    let target = UniformGrid::new();
    target.set_rows(0);
    target.set_columns(0);
    target.set_row_spacing(10.0);
    target.set_column_spacing(20.0);

    let available_size = Size::new(100.0, 100.0);

    let desired_size = <UniformGrid as LayoutableImpl>::measure_override(&target, available_size);

    // Fail case:
    // Because rows and columns are 0, the calculation becomes:
    //   total width = max width * 0 + column spacing * (0 - 1) = -column spacing
    //   total height = max height * 0 + row spacing * (0 - 1) = -row spacing
    // Expected: (0, 0)
    assert_eq!(desired_size.width, 0.0);
    assert_eq!(desired_size.height, 0.0);
}

#[test]
fn arrange_does_not_panic_when_row_spacing_takes_all_available_height() {
    // Minimum required height = 20 (2 row gaps size 10).
    // Provide height of 19 so that row gaps take all available space; thus,
    // available height for children may be negative. In that case, the grid
    // should arrange its children with rects of height 0.
    let target = UniformGrid::new();
    target.set_columns(1);
    target.set_row_spacing(10.0);
    for _ in 0..3 {
        target.children().add(Border::new());
    }

    let available_size = Size::new(100.0, 19.0);

    target.measure(Size::INFINITY);

    // Fail case: arranging panics if any child rect contains a negative
    // dimension.
    target.arrange(Rect::from_size(available_size));
}

#[test]
fn arrange_does_not_panic_when_column_spacing_takes_all_available_width() {
    // Minimum required width = 20 (2 column gaps size 10).
    // Provide width of 19 so that column gaps take all available space; thus,
    // available width for children may be negative. In that case, the grid
    // should arrange its children with rects of width 0.
    let target = UniformGrid::new();
    target.set_rows(1);
    target.set_column_spacing(10.0);
    for _ in 0..3 {
        target.children().add(Border::new());
    }

    let available_size = Size::new(19.0, 100.0);

    target.measure(Size::INFINITY);

    // Fail case: arranging panics if any child rect contains a negative
    // dimension.
    target.arrange(Rect::from_size(available_size));
}
