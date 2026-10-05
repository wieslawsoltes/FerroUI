//! Tests of the layout helper of the table view.

use super::table_view_layout_helper::TableViewLayoutHelper;
use crate::test_support::test_scope;
use crate::{GridLength, GridUnitType, TableViewColumn};
use ferroui_base::collections::FerroList;
use ferroui_base::Ref;

fn column(width: GridLength) -> Ref<TableViewColumn> {
    let column = TableViewColumn::new();
    column.set_width(width);
    column
}

fn hidden_column(width: GridLength) -> Ref<TableViewColumn> {
    let column = column(width);
    column.set_is_visible(false);
    column
}

fn star(value: f64) -> GridLength {
    GridLength::new(value, GridUnitType::Star)
}

fn pixels(value: f64) -> GridLength {
    GridLength::from_pixels(value)
}

fn list<const N: usize>(columns: [Ref<TableViewColumn>; N]) -> FerroList<Ref<TableViewColumn>> {
    FerroList::from_items(columns)
}

#[test]
fn update_actual_widths_with_no_columns_returns_false() {
    let _scope = test_scope();
    let columns = FerroList::<Ref<TableViewColumn>>::new();

    assert!(!TableViewLayoutHelper::update_actual_widths(&columns, 100.0, false, 1.0));
}

#[test]
fn update_actual_widths_single_star_column_gets_full_width() {
    let _scope = test_scope();
    let columns = list([column(star(1.0))]);

    assert!(TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0));
    assert_eq!(columns.get(0).actual_width(), 200.0);
}

#[test]
fn update_actual_widths_distributes_star_columns_proportionally() {
    let _scope = test_scope();
    let columns = list([column(star(1.0)), column(star(3.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 400.0, false, 1.0);

    assert_eq!(columns.get(0).actual_width(), 100.0);
    assert_eq!(columns.get(1).actual_width(), 300.0);
}

#[test]
fn update_actual_widths_pixel_column_uses_its_fixed_width() {
    let _scope = test_scope();
    let columns = list([column(pixels(80.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 500.0, false, 1.0);

    assert_eq!(columns.get(0).actual_width(), 80.0);
}

#[test]
fn update_actual_widths_pixel_plus_star_subtracts_fixed_from_star_budget() {
    let _scope = test_scope();
    let columns = list([column(pixels(50.0)), column(star(1.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);

    assert_eq!(columns.get(0).actual_width(), 50.0);
    assert_eq!(columns.get(1).actual_width(), 150.0);
}

#[test]
fn update_actual_widths_star_clamped_to_zero_when_pixel_exceeds_available() {
    let _scope = test_scope();
    let columns = list([column(pixels(300.0)), column(star(1.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);

    assert_eq!(columns.get(0).actual_width(), 300.0);
    assert_eq!(columns.get(1).actual_width(), 0.0);
}

#[test]
fn update_actual_widths_treats_auto_as_one_star() {
    let _scope = test_scope();
    let columns = list([column(GridLength::AUTO), column(star(1.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);

    assert_eq!(columns.get(0).actual_width(), 100.0);
    assert_eq!(columns.get(1).actual_width(), 100.0);
}

#[test]
fn update_actual_widths_zeros_hidden_columns_and_preserves_configured_widths() {
    for unit in [GridUnitType::Pixel, GridUnitType::Star, GridUnitType::Auto] {
        let _scope = test_scope();
        let hidden = column(GridLength::new(2.0, unit));
        let columns = list([hidden.clone(), column(star(1.0))]);
        TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);
        let actual_width = hidden.actual_width();

        hidden.set_is_visible(false);
        TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);

        assert_eq!(hidden.width(), GridLength::new(2.0, unit));
        assert_eq!(hidden.actual_width(), 0.0);
        assert_eq!(columns.get(1).actual_width(), 200.0);

        hidden.set_is_visible(true);
        TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);

        assert_eq!(hidden.actual_width(), actual_width);
    }
}

#[test]
fn update_actual_widths_falls_back_to_1000_for_infinite_width() {
    let _scope = test_scope();
    let columns = list([column(star(1.0)), column(star(1.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, f64::INFINITY, false, 1.0);

    assert_eq!(columns.get(0).actual_width(), 500.0);
    assert_eq!(columns.get(1).actual_width(), 500.0);
}

#[test]
fn update_actual_widths_returns_false_when_no_change() {
    let _scope = test_scope();
    let columns = list([column(star(1.0)), column(pixels(50.0))]);

    assert!(TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0));
    assert!(!TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0));
}

#[test]
fn update_actual_widths_returns_true_when_width_changes() {
    let _scope = test_scope();
    let columns = list([column(star(1.0))]);

    assert!(TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0));
    assert!(TableViewLayoutHelper::update_actual_widths(&columns, 300.0, false, 1.0));
    assert_eq!(columns.get(0).actual_width(), 300.0);
}

#[test]
fn update_actual_widths_spreads_rounding_across_star_columns() {
    let _scope = test_scope();
    let columns = list([column(star(2.0)), column(star(3.0)), column(star(2.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 100.0, true, 1.0);

    // Weights 2:3:2 over 100 give 28.57 / 42.86 / 28.57. Rounding each
    // column independently would produce 29 / 43 / 29 = 101; spreading the
    // remainder keeps the total at 100.
    assert_eq!(columns.get(0).actual_width(), 29.0);
    assert_eq!(columns.get(1).actual_width(), 42.0);
    assert_eq!(columns.get(2).actual_width(), 29.0);
    assert_eq!(
        columns.get(0).actual_width() + columns.get(1).actual_width() + columns.get(2).actual_width(),
        100.0
    );
}

#[test]
fn update_actual_widths_spreads_rounding_with_intertwined_fixed_columns() {
    let _scope = test_scope();
    let columns = list([
        column(pixels(50.0)),
        column(star(1.0)),
        column(pixels(30.0)),
        column(star(3.0)),
        column(star(2.0)),
    ]);

    TableViewLayoutHelper::update_actual_widths(&columns, 201.0, true, 1.0);

    // Fixed columns keep their exact size (80 total). The remaining 121px
    // star budget is split 1:3:2 (20.17 / 60.5 / 40.33). Rounding each
    // independently would drop to 200; spreading the remainder gives the
    // extra pixel to the 3* column so everything fills exactly 201.
    assert_eq!(columns.get(0).actual_width(), 50.0);
    assert_eq!(columns.get(1).actual_width(), 20.0);
    assert_eq!(columns.get(2).actual_width(), 30.0);
    assert_eq!(columns.get(3).actual_width(), 61.0);
    assert_eq!(columns.get(4).actual_width(), 40.0);
    assert_eq!(
        columns.get(0).actual_width()
            + columns.get(1).actual_width()
            + columns.get(2).actual_width()
            + columns.get(3).actual_width()
            + columns.get(4).actual_width(),
        201.0
    );
}

#[test]
fn update_actual_widths_spreads_rounding_at_fractional_layout_scale() {
    let _scope = test_scope();
    let columns = list([column(star(3.0)), column(star(1.0)), column(star(3.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 100.0, true, 2.0);

    // At 2x scale widths snap to half-pixels (whole device pixels). Weights
    // 3:1:3 give 42.86 / 14.29 / 42.86; rounding each independently would
    // drift the total to 100.5, while spreading the remainder keeps it at
    // 100.
    assert_eq!(columns.get(0).actual_width(), 43.0);
    assert_eq!(columns.get(1).actual_width(), 14.0);
    assert_eq!(columns.get(2).actual_width(), 43.0);
    assert_eq!(
        columns.get(0).actual_width() + columns.get(1).actual_width() + columns.get(2).actual_width(),
        100.0
    );
}

#[test]
fn update_actual_widths_ignores_hidden_columns() {
    let _scope = test_scope();
    let columns = list([
        column(star(3.0)),
        hidden_column(pixels(100.0)),
        column(star(1.0)),
        hidden_column(star(5.0)),
        column(star(3.0)),
    ]);

    TableViewLayoutHelper::update_actual_widths(&columns, 100.0, true, 2.0);

    assert_eq!(columns.get(0).actual_width(), 43.0);
    assert_eq!(columns.get(1).actual_width(), 0.0);
    assert_eq!(columns.get(2).actual_width(), 14.0);
    assert_eq!(columns.get(3).actual_width(), 0.0);
    assert_eq!(columns.get(4).actual_width(), 43.0);
    assert_eq!(
        columns.get(0).actual_width() + columns.get(2).actual_width() + columns.get(4).actual_width(),
        100.0
    );
}

#[test]
fn update_actual_widths_rounds_fixed_column_width_and_star_absorbs_remainder() {
    let _scope = test_scope();
    let columns = list([column(pixels(50.4)), column(star(1.0)), column(star(2.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, true, 1.0);

    // The fixed column rounds to a whole pixel (50); the remaining 150px
    // star budget is split 1:2 between the star columns.
    assert_eq!(columns.get(0).actual_width(), 50.0);
    assert_eq!(columns.get(1).actual_width(), 50.0);
    assert_eq!(columns.get(2).actual_width(), 100.0);
}

#[test]
fn update_actual_widths_sets_all_hidden_widths_to_zero() {
    let _scope = test_scope();
    let first = TableViewColumn::new();
    first.set_is_visible(false);
    let second = TableViewColumn::new();
    second.set_is_visible(false);
    let columns = list([first, second]);

    assert!(TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0));

    for column in columns.snapshot().iter() {
        assert_eq!(column.actual_width(), 0.0);
    }
    assert!(!TableViewLayoutHelper::needs_actual_widths(&columns));
    assert!(!TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0));
}

#[test]
fn needs_actual_widths_returns_false_for_empty_columns() {
    let _scope = test_scope();
    let columns = FerroList::<Ref<TableViewColumn>>::new();

    assert!(!TableViewLayoutHelper::needs_actual_widths(&columns));
}

#[test]
fn needs_actual_widths_returns_true_when_first_column_has_nan_width() {
    let _scope = test_scope();
    let columns = list([TableViewColumn::new(), TableViewColumn::new()]);

    assert!(TableViewLayoutHelper::needs_actual_widths(&columns));
}

#[test]
fn needs_actual_widths_returns_false_after_widths_are_computed() {
    let _scope = test_scope();
    let columns = list([column(star(1.0)), column(star(1.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);

    assert!(!TableViewLayoutHelper::needs_actual_widths(&columns));
}

#[test]
fn needs_actual_widths_detects_reset_when_first_column_is_hidden() {
    let _scope = test_scope();
    let columns = list([TableViewColumn::new(), TableViewColumn::new()]);
    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);
    columns.get(0).set_is_visible(false);

    TableViewLayoutHelper::reset_actual_widths(&columns);

    assert!(columns.get(0).actual_width().is_nan());
    assert!(TableViewLayoutHelper::needs_actual_widths(&columns));

    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);

    assert_eq!(columns.get(0).actual_width(), 0.0);
    assert_eq!(columns.get(1).actual_width(), 200.0);
    assert!(!TableViewLayoutHelper::needs_actual_widths(&columns));
}

#[test]
fn reset_actual_widths_sets_all_widths_to_nan() {
    let _scope = test_scope();
    let columns = list([column(pixels(50.0)), column(star(1.0))]);

    TableViewLayoutHelper::update_actual_widths(&columns, 200.0, false, 1.0);
    TableViewLayoutHelper::reset_actual_widths(&columns);

    assert!(columns.get(0).actual_width().is_nan());
    assert!(columns.get(1).actual_width().is_nan());
}
