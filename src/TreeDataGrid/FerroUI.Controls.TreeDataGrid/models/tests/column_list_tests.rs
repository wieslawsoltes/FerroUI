//! `Models/ColumnListTests.cs`: the widths the column list commits for pixel, auto and star
//! columns, their batches and the invalidation of the layout.

use super::support::{country_column, name_column, pixels, stars, Model};
use crate::models::{ColumnList, ColumnRef, IColumnLayoutBatch, TextColumn, TextColumnOptions};
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::{Rect, Size};
use ferroui_controls::GridLength;
use std::cell::Cell;
use std::rc::Rc;

fn columns(columns: Vec<ColumnRef>) -> Rc<ColumnList> {
    ColumnList::from_columns(columns)
}

fn width_of(list: &ColumnList, index: usize) -> f64 {
    list.get(index).actual_width()
}

#[test]
fn columns_are_sized_at_end_of_measure() {
    let target = columns(vec![
        name_column(pixels(100.0), None),
        name_column(GridLength::AUTO, None),
        name_column(stars(1.0), None),
        name_column(stars(3.0), None),
    ]);

    target.viewport_changed(Rect::new(0.0, 0.0, 500.0, 500.0));

    for row in 0..10 {
        for col in 0..target.count() {
            target.cell_measured(col as i32, row, Size::new(51.0 + row as f64, 10.0));
        }
    }

    target.commit_actual_widths();

    assert_eq!(100.0, width_of(&target, 0));
    assert_eq!(60.0, width_of(&target, 1));
    assert_eq!(85.0, width_of(&target, 2));
    assert_eq!(255.0, width_of(&target, 3));
}

#[test]
fn star_column_leaves_room_for_unmeasured_auto_columns() {
    const VIEWPORT_WIDTH: f64 = 300.0;

    let target = columns(vec![
        name_column(GridLength::AUTO, None),
        name_column(stars(1.0), None),
        country_column(GridLength::AUTO, None),
    ]);

    target.viewport_changed(Rect::new(0.0, 0.0, VIEWPORT_WIDTH, VIEWPORT_WIDTH));

    let measured = [50.0, 500.0, 60.0];
    let mut width_so_far = 0.0;

    let mut col = 0;
    while col < target.count() && width_so_far < VIEWPORT_WIDTH {
        let size = target.cell_measured(col as i32, 0, Size::new(measured[col], 10.0));
        width_so_far += size.width;
        col += 1;
    }

    target.commit_actual_widths();

    assert_eq!(50.0, width_of(&target, 0));
    assert_eq!(VIEWPORT_WIDTH - 50.0 - 60.0, width_of(&target, 1));
    assert_eq!(60.0, width_of(&target, 2));
}

#[test]
fn star_column_respects_min_width_of_unmeasured_auto_columns() {
    const VIEWPORT_WIDTH: f64 = 300.0;

    let target = columns(vec![
        name_column(GridLength::AUTO, None),
        name_column(stars(1.0), None),
        country_column(GridLength::AUTO, None),
    ]);

    target.viewport_changed(Rect::new(0.0, 0.0, VIEWPORT_WIDTH, VIEWPORT_WIDTH));

    target.cell_measured(0, 0, Size::new(50.0, 10.0));
    target.cell_measured(1, 0, Size::new(500.0, 10.0));

    target.commit_actual_widths();

    let column = target.get(2);
    let unmeasured_auto_column = column
        .as_any()
        .downcast_ref::<TextColumn<Model, Option<String>>>()
        .expect("a text column of the model");
    let min_width = unmeasured_auto_column.options().min_width().value();

    assert_eq!(50.0, width_of(&target, 0));
    assert_eq!(VIEWPORT_WIDTH - 50.0 - min_width, width_of(&target, 1));
    assert_eq!(min_width, width_of(&target, 2));
}

#[test]
fn layout_is_invalidated_at_end_of_measure_if_auto_sized_column_changes_width() {
    let target = columns(vec![name_column(GridLength::AUTO, None), country_column(GridLength::AUTO, None)]);

    target.viewport_changed(Rect::new(0.0, 0.0, 500.0, 500.0));

    for row in 0..10 {
        for col in 0..target.count() {
            target.cell_measured(col as i32, row, Size::new(40.0, 10.0));
        }
    }

    target.commit_actual_widths();

    target.cell_measured(0, 1, Size::new(50.0, 10.0));

    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    target.add_layout_invalidated(Rc::new(move || counter.set(counter.get() + 1)));

    target.commit_actual_widths();

    assert_eq!(1, raised.get());
}

#[test]
fn actual_width_batch_commits_all_auto_measurements_once() {
    let target = columns(vec![name_column(GridLength::AUTO, None)]);
    let batch: &dyn IColumnLayoutBatch = &*target;
    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    target.add_layout_invalidated(Rc::new(move || counter.set(counter.get() + 1)));

    batch.begin_actual_width_batch();

    for row in 0..100 {
        target.cell_measured(0, row, Size::new(20.0 + row as f64, 10.0));
        target.commit_actual_widths();
    }

    assert!(width_of(&target, 0).is_nan());
    assert_eq!(0, raised.get());

    let needs_final_measure = batch.end_actual_width_batch();

    assert!(!needs_final_measure);
    assert_eq!(119.0, width_of(&target, 0));
    assert_eq!(1, raised.get());
}

#[test]
fn actual_width_batch_propagates_deferred_final_measure_request() {
    let target = ColumnList::new();
    let batch: &dyn IColumnLayoutBatch = &*target;

    batch.begin_actual_width_batch();
    batch.request_final_measure();

    assert!(batch.end_actual_width_batch());

    batch.begin_actual_width_batch();

    assert!(!batch.end_actual_width_batch());
}

#[test]
fn nested_actual_width_batch_commits_only_at_outer_scope() {
    let target = columns(vec![name_column(GridLength::AUTO, None)]);
    let batch: &dyn IColumnLayoutBatch = &*target;
    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    target.add_layout_invalidated(Rc::new(move || counter.set(counter.get() + 1)));

    batch.begin_actual_width_batch();
    target.cell_measured(0, 0, Size::new(42.0, 10.0));
    batch.begin_actual_width_batch();
    batch.request_final_measure();

    assert!(!batch.end_actual_width_batch());
    assert!(width_of(&target, 0).is_nan());
    assert_eq!(0, raised.get());

    assert!(batch.end_actual_width_batch());
    assert_eq!(42.0, width_of(&target, 0));
    assert_eq!(1, raised.get());
}

#[test]
fn clean_commit_does_not_suppress_later_measurement_and_viewport_changes() {
    let star_column = country_column(stars(1.0), None);
    let target = columns(vec![
        name_column(pixels(50.0), None),
        country_column(GridLength::AUTO, None),
        star_column.clone(),
    ]);

    target.viewport_changed(Rect::new(0.0, 0.0, 300.0, 500.0));
    target.cell_measured(0, 0, Size::new(50.0, 10.0));
    target.cell_measured(1, 0, Size::new(80.0, 10.0));
    target.cell_measured(2, 0, Size::new(80.0, 10.0));
    target.commit_actual_widths();
    target.commit_actual_widths();

    assert_eq!(50.0, width_of(&target, 0));
    assert_eq!(80.0, width_of(&target, 1));
    assert_eq!(170.0, width_of(&target, 2));

    target.cell_measured(1, 1, Size::new(100.0, 10.0));
    target.commit_actual_widths();

    assert_eq!(100.0, width_of(&target, 1));
    assert_eq!(150.0, width_of(&target, 2));

    target.commit_actual_widths();
    target.viewport_changed(Rect::new(0.0, 0.0, 400.0, 500.0));

    assert_eq!(250.0, width_of(&target, 2));

    star_column.options().set_min_width(pixels(300.0));
    target.cell_measured(2, 1, Size::new(100.0, 10.0));
    target.commit_actual_widths();

    assert_eq!(300.0, width_of(&target, 2));
}

#[test]
fn clean_commit_does_not_suppress_later_column_collection_changes() {
    let target = columns(vec![name_column(pixels(50.0), None), country_column(stars(1.0), None)]);

    target.viewport_changed(Rect::new(0.0, 0.0, 200.0, 500.0));
    target.cell_measured(0, 0, Size::new(50.0, 10.0));
    target.cell_measured(1, 0, Size::new(100.0, 10.0));
    target.commit_actual_widths();
    target.commit_actual_widths();

    target.remove_at(0);
    target.commit_actual_widths();
    assert_eq!(200.0, width_of(&target, 0));

    target.insert(0, name_column(pixels(100.0), None));
    target.commit_actual_widths();

    assert_eq!(100.0, width_of(&target, 0));
    assert_eq!(100.0, width_of(&target, 1));
}

#[test]
fn star_column_with_auto_min_width_initializes_from_first_measurement() {
    let options = Rc::new(TextColumnOptions::<Model>::default());
    options.set_min_width(GridLength::AUTO);
    let target = create_fixed_and_star_columns(options);

    target.viewport_changed(Rect::new(0.0, 0.0, 200.0, 500.0));
    let measured = target.cell_measured(1, 0, Size::new(80.0, 10.0));
    target.commit_actual_widths();

    assert_eq!(80.0, measured.width);
    assert_eq!(150.0, width_of(&target, 1));
}

#[test]
fn star_column_with_auto_max_width_initializes_from_first_measurement() {
    let options = Rc::new(TextColumnOptions::<Model>::default());
    options.set_max_width(Some(GridLength::AUTO));
    let target = create_fixed_and_star_columns(options);

    target.viewport_changed(Rect::new(0.0, 0.0, 200.0, 500.0));
    target.cell_measured(1, 0, Size::new(80.0, 10.0));
    target.commit_actual_widths();

    assert_eq!(80.0, width_of(&target, 1));
}

#[test]
fn star_columns_redistribute_around_min_width_on_first_layout() {
    let options = Rc::new(TextColumnOptions::<Model>::default());
    options.set_min_width(pixels(200.0));
    let target = create_constrained_star_columns(options);

    target.viewport_changed(Rect::new(0.0, 0.0, 300.0, 500.0));
    target.commit_actual_widths();

    assert_eq!(200.0, width_of(&target, 0));
    assert_eq!(100.0, width_of(&target, 1));
}

#[test]
fn star_columns_redistribute_around_max_width_on_first_layout() {
    let options = Rc::new(TextColumnOptions::<Model>::default());
    options.set_max_width(Some(pixels(100.0)));
    let target = create_constrained_star_columns(options);

    target.viewport_changed(Rect::new(0.0, 0.0, 300.0, 500.0));
    target.commit_actual_widths();

    assert_eq!(100.0, width_of(&target, 0));
    assert_eq!(200.0, width_of(&target, 1));
}

#[test]
fn moving_columns_does_not_hide_a_later_constraint_change() {
    let moved_options = Rc::new(TextColumnOptions::<Model>::default());
    moved_options.set_min_width(pixels(30.0));
    let other_options = Rc::new(TextColumnOptions::<Model>::default());
    other_options.set_min_width(pixels(100.0));
    let moved = name_column(pixels(50.0), Some(moved_options.clone()));
    let target = columns(vec![moved.clone(), country_column(pixels(50.0), Some(other_options))]);

    target.commit_actual_widths();
    moved_options.set_min_width(pixels(100.0));
    let observed_move = Rc::new(Cell::new(false));
    {
        let observed_move = observed_move.clone();
        let weak_target = Rc::downgrade(&target);
        let moved_column: ColumnRef = moved.clone();
        target.add_collection_changed(Rc::new(move |e: &crate::models::ColumnsChangedArgs<'_>| {
            if e.action == NotifyCollectionChangedAction::Move {
                let target = weak_target.upgrade().expect("the column list");
                observed_move.set(true);
                assert!(std::ptr::addr_eq(Rc::as_ptr(&moved_column), Rc::as_ptr(&target.get(1))));
                target.cell_measured(1, 0, Size::new(50.0, 10.0));
                target.commit_actual_widths();
            }
        }));
    }

    target.move_item(0, 1);

    assert!(observed_move.get());
    assert_eq!(100.0, crate::models::IColumn::actual_width(&*moved));
}

fn create_fixed_and_star_columns(options: Rc<TextColumnOptions<Model>>) -> Rc<ColumnList> {
    columns(vec![name_column(pixels(50.0), None), country_column(stars(1.0), Some(options))])
}

fn create_constrained_star_columns(options: Rc<TextColumnOptions<Model>>) -> Rc<ColumnList> {
    columns(vec![name_column(stars(1.0), Some(options)), country_column(stars(1.0), None)])
}
