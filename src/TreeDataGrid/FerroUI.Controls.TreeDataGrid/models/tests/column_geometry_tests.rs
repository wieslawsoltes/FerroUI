//! `Models/ColumnGeometryTests.cs`: the cached geometry of the column list (the column at a
//! position and the estimated column width) follows the widths and the changes of the columns.

use super::support::{name_column, pixels, Model, Random};
use crate::models::{ColumnList, ColumnRef, IColumnViewportEstimator, TextColumn};
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::{Rect, Size};
use ferroui_controls::GridLength;
use std::rc::{Rc, Weak};

fn column(width: f64) -> Rc<TextColumn<Model, Option<String>>> {
    name_column(pixels(width), None)
}

fn create_temporary_columns(column: ColumnRef) -> Weak<ColumnList> {
    let columns = ColumnList::from_columns([column]);
    columns.get_column_at(10.0);
    Rc::downgrade(&columns)
}

#[test]
fn observing_a_column_does_not_keep_its_old_view_collection_alive() {
    let column = column(20.0);
    let weak = create_temporary_columns(column.clone());
    assert!(weak.upgrade().is_none());
    assert_eq!(0, column.property_changed().handler_count(), "the dropped list no longer observes the column");
}

#[test]
fn geometry_tracks_widths_and_collection_changes() {
    let columns = ColumnList::new();
    let random = Random::new(815);
    for _ in 0..50 {
        columns.add(column(random.next_range(0, 100) as f64));
    }

    for step in 0..100 {
        assert_geometry(&columns);
        let index = random.next(columns.count());
        match step % 5 {
            0 => columns.set_column_width(index as i32, pixels(random.next_range(0, 100) as f64)),
            1 => columns.insert(index, column(random.next_range(0, 100) as f64)),
            2 => {
                let new_index = random.next(columns.count());
                columns.move_item(index, new_index);
            }
            3 => columns.set(index, column(random.next_range(0, 100) as f64)),
            _ => columns.remove_at(index),
        }
    }

    columns.clear();
    assert_geometry(&columns);
    columns.add(column(30.0));
    assert_geometry(&columns);
}

#[test]
fn geometry_tracks_auto_and_star_commits_and_direct_width_changes() {
    let auto_column: ColumnRef = name_column(GridLength::AUTO, None);
    let star_column: ColumnRef = name_column(GridLength::STAR, None);
    let columns = ColumnList::from_columns([column(20.0) as ColumnRef, auto_column, star_column, column(40.0) as ColumnRef]);

    assert_geometry(&columns); // NaN ends the known prefix but not the width estimate.
    columns.viewport_changed(Rect::new(0.0, 0.0, 300.0, 100.0));
    columns.cell_measured(1, 0, Size::new(50.0, 20.0));
    columns.commit_actual_widths();
    assert_geometry(&columns);
    columns.viewport_changed(Rect::new(0.0, 0.0, 500.0, 100.0));
    assert_geometry(&columns);
    columns.cell_measured(1, 1, Size::new(100.0, 20.0));
    columns.commit_actual_widths();
    assert_geometry(&columns);
    columns.get(0).as_update_column_layout().expect("a column that lays itself out").set_width(pixels(70.0));
    assert_geometry(&columns);

    // Synchronous layout observers must see the newly committed geometry.
    let observed = Rc::downgrade(&columns);
    columns.add_layout_invalidated(Rc::new(move || {
        if let Some(columns) = observed.upgrade() {
            assert_geometry(&columns);
        }
    }));
    columns.set_column_width(0, pixels(90.0));
}

fn assert_geometry(columns: &ColumnList) {
    let mut total = 0.0;
    let mut measured = 0;
    for column in columns.to_vec() {
        let width = column.actual_width();
        if !width.is_nan() && width > 0.0 {
            total += width;
            measured += 1;
        }
    }
    let expected = if measured == 0 { -1.0 } else { total / measured as f64 };
    assert_eq!(expected, IColumnViewportEstimator::estimate_element_size(columns));

    let mut x = -1.0;
    while x <= total + 1.0 {
        assert_eq!(linear_lookup(columns, x), columns.get_column_at(x));
        x += 1.0;
    }
    assert_eq!((-1, -1.0), columns.get_column_at(f64::NAN));
    assert_eq!((-1, -1.0), columns.get_column_at(f64::INFINITY));
}

fn linear_lookup(columns: &ColumnList, x: f64) -> (i32, f64) {
    let mut start = 0.0;
    for i in 0..columns.count() {
        let width = columns.get(i).actual_width();
        if x >= start && x < start + width {
            return (i as i32, start);
        }
        if width.is_nan() {
            break;
        }
        start += width;
    }
    (-1, -1.0)
}
