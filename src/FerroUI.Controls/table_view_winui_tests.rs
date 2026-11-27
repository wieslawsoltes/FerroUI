//! Tests of the WinUI additions of the table view (`docs/porting/winui-controls.md`, section
//! 10.11): each is opt-in, so the first test of each group checks that its default keeps the
//! upstream table view, the others its WinUI behaviour.

use super::table_view_tests::*;
use crate::primitives::TemplatedControl;
use crate::{TableView, TableViewDensity, TableViewGridLinesVisibility};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use std::rc::Rc;

fn brush(r: u8, g: u8, b: u8) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(r, g, b)))
}

fn same_brush(a: &Option<Rc<dyn IBrush>>, b: &Rc<dyn IBrush>) -> bool {
    a.as_ref().is_some_and(|a| Rc::ptr_eq(a, b))
}

fn table(items: &[&str]) -> ferroui_base::Ref<TableView> {
    let target = create_target(strs(items));
    target.columns().add(column());
    target
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

#[test]
fn winui_presentation_additions_default_to_upstream() {
    let _app = start();
    let target = table(&["a", "b", "c"]);
    let _root = prepare(&target);

    assert_eq!(target.grid_lines_visibility(), TableViewGridLinesVisibility::All);
    assert_eq!(target.density(), TableViewDensity::Standard);
    assert!(target.row_background().is_none());
    assert!(target.alternating_row_background().is_none());
    assert!(target.empty_template().is_none());
    for class in [":nohorizontalgridlines", ":noverticalgridlines", ":compact", ":comfortable"] {
        assert!(!target.classes().contains(class), "{class}");
    }
    // The table sets nothing on its rows.
    for row in rows(&target) {
        assert!(!row.is_set(TemplatedControl::background_property().as_property()));
    }
}

// ---------------------------------------------------------------------------
// GridLinesVisibility and Density
// ---------------------------------------------------------------------------

#[test]
fn grid_lines_visibility_sets_the_pseudo_classes_of_the_lines_left_out() {
    let _app = start();
    let target = table(&["a"]);

    let cases = [
        (TableViewGridLinesVisibility::Horizontal, false, true),
        (TableViewGridLinesVisibility::Vertical, true, false),
        (TableViewGridLinesVisibility::None, true, true),
        (TableViewGridLinesVisibility::All, false, false),
    ];
    for (visibility, no_horizontal, no_vertical) in cases {
        target.set_grid_lines_visibility(visibility);
        assert_eq!(target.classes().contains(":nohorizontalgridlines"), no_horizontal, "{visibility:?}");
        assert_eq!(target.classes().contains(":noverticalgridlines"), no_vertical, "{visibility:?}");
    }
}

#[test]
fn density_sets_the_pseudo_class_of_the_density() {
    let _app = start();
    let target = table(&["a"]);

    target.set_density(TableViewDensity::Compact);
    assert!(target.classes().contains(":compact"));
    assert!(!target.classes().contains(":comfortable"));

    target.set_density(TableViewDensity::Comfortable);
    assert!(!target.classes().contains(":compact"));
    assert!(target.classes().contains(":comfortable"));

    target.set_density(TableViewDensity::Standard);
    assert!(!target.classes().contains(":compact"));
    assert!(!target.classes().contains(":comfortable"));
}

// ---------------------------------------------------------------------------
// RowBackground and AlternatingRowBackground
// ---------------------------------------------------------------------------

#[test]
fn row_background_is_set_on_every_row() {
    let _app = start();
    let target = table(&["a", "b", "c"]);
    let row_brush = brush(1, 2, 3);
    target.set_row_background(Some(row_brush.clone()));
    let _root = prepare(&target);

    let rows = rows(&target);
    assert_eq!(rows.len(), 3);
    for row in &rows {
        assert!(same_brush(&row.background(), &row_brush));
    }
}

#[test]
fn alternating_row_background_is_set_on_the_rows_at_odd_indexes() {
    let _app = start();
    let target = table(&["a", "b", "c", "d"]);
    let row_brush = brush(1, 2, 3);
    let alternating_brush = brush(4, 5, 6);
    target.set_row_background(Some(row_brush.clone()));
    target.set_alternating_row_background(Some(alternating_brush.clone()));
    let _root = prepare(&target);

    for row in rows(&target) {
        let index = target.index_from_container(&row.clone().upcast());
        let expected = if index % 2 == 1 { &alternating_brush } else { &row_brush };
        assert!(same_brush(&row.background(), expected), "row {index}");
    }
}

#[test]
fn row_backgrounds_follow_changes_of_the_properties() {
    let _app = start();
    let target = table(&["a", "b"]);
    let _root = prepare(&target);
    let alternating_brush = brush(4, 5, 6);

    target.set_alternating_row_background(Some(alternating_brush.clone()));
    let rows = rows(&target);
    assert!(rows[0].background().is_none());
    assert!(same_brush(&rows[1].background(), &alternating_brush));

    // Cleared again: the table removes the background it set.
    target.set_alternating_row_background(None);
    for row in &rows {
        assert!(!row.is_set(TemplatedControl::background_property().as_property()));
    }
}

#[test]
fn row_backgrounds_follow_the_index_of_a_row_when_items_are_inserted() {
    use ferroui_base::collections::FerroList;

    let _app = start();
    let items = Rc::new(FerroList::<String>::new());
    for value in ["a", "b", "c"] {
        items.add(value.to_string());
    }
    let target = create_target(crate::ItemsSource::from(items.clone()));
    target.columns().add(column());
    let alternating_brush = brush(4, 5, 6);
    target.set_alternating_row_background(Some(alternating_brush.clone()));
    let root = prepare(&target);

    items.insert(0, "z".to_string());
    layout(&root);

    for row in rows(&target) {
        let index = target.index_from_container(&row.clone().upcast());
        assert_eq!(same_brush(&row.background(), &alternating_brush), index % 2 == 1, "row {index}");
    }
}

#[test]
fn a_row_background_the_table_did_not_set_is_kept() {
    let _app = start();
    let target = table(&["a"]);
    let _root = prepare(&target);
    let own_brush = brush(7, 8, 9);
    let row = single_row(&target);
    row.set_background(Some(own_brush.clone()));

    target.set_row_background(None);
    target.set_alternating_row_background(None);

    assert!(same_brush(&row.background(), &own_brush));
}
