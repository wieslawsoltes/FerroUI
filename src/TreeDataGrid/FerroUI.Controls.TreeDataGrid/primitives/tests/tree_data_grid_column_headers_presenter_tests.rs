//! `TreeDataGridColumnHeadersPresenterTests.cs`.
//!
//! The reference's `TestColumn : ColumnBase<string>` (a column with no minimum width whose cells
//! are never created) is the layout test column of the support module.

use super::support::{
    is_red, layout, red_brush, run_jobs, scroll_viewer, start_app, test_window, visual_children_of, LayoutTestColumn,
};
use crate::models::{ColumnListBase, ColumnRef, IColumns};
use crate::primitives::{TreeDataGridColumnHeader, TreeDataGridColumnHeadersPresenter, TreeDataGridElementFactory};
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{Rect, Ref, Vector};
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl};
use ferroui_controls::{GridLength, GridUnitType, ScrollViewer, Window};
use std::rc::Rc;

struct Target {
    target: Ref<TreeDataGridColumnHeadersPresenter>,
    scroll: Ref<ScrollViewer>,
    columns: Rc<ColumnListBase>,
    _root: Ref<Window>,
}

fn create_target(
    root: Option<Ref<Window>>,
    width: Option<GridLength>,
    column_count: usize,
    additional_styles: Vec<Ref<Style>>,
) -> Target {
    let columns = ColumnListBase::new();
    let width = width.unwrap_or(GridLength::new(10.0, GridUnitType::Pixel));

    for i in 0..column_count {
        columns.add(LayoutTestColumn::<String>::new(&format!("Column {i}"), Some(width), None) as ColumnRef);
    }

    let target = TreeDataGridColumnHeadersPresenter::new();
    target.set_element_factory(Some(TreeDataGridElementFactory::new()));
    target.set_items(Some(columns.clone() as Rc<dyn IColumns>));

    let scroll = scroll_viewer(target.clone().upcast());
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto);

    let root = root.unwrap_or_else(|| test_window(None, None));
    root.set_content(Some(ferroui_controls::Control::boxed(&scroll)));

    for style in additional_styles {
        root.styles().add(style);
    }

    root.update_layout();
    run_jobs();

    Target { target, scroll, columns, _root: root }
}

fn default_target() -> Target {
    create_target(None, None, 100, Vec::new())
}

fn assert_column_indexes(target: &TreeDataGridColumnHeadersPresenter, first_column_index: i32, column_count: i32) {
    let mut indexes: Vec<i32> = visual_children_of::<TreeDataGridColumnHeader>(target)
        .iter()
        .filter(|x| x.is_visible())
        .map(|x| x.column_index())
        .collect();
    indexes.sort();

    assert_eq!((first_column_index..first_column_index + column_count).collect::<Vec<i32>>(), indexes);
}

fn assert_recyclable(target: &TreeDataGridColumnHeadersPresenter, count: usize) {
    let recyclable = visual_children_of::<TreeDataGridColumnHeader>(target).into_iter().filter(|x| !x.is_visible()).count();
    assert_eq!(count, recyclable);
}

#[test]
fn creates_initial_columns() {
    let _app = start_app();
    let t = default_target();

    assert_column_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn updates_auto_column_actual_width() {
    let _app = start_app();

    // We're testing Auto columns so make cells have a width of 10.
    let root = test_window(None, None);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<TreeDataGridColumnHeader>(),
        [Setter::new(Layoutable::width_property(), 10.0)],
    ));

    let t = create_target(Some(root), Some(GridLength::AUTO), 100, Vec::new());

    for i in 0..t.columns.count() {
        let column = t.columns.get(i);
        assert_eq!(if i < 10 { 10.0 } else { 0.0 }, column.actual_width());
    }
}

#[test]
fn scrolls_right_one_row() {
    let _app = start_app();
    let t = default_target();

    t.scroll.set_offset(Vector::new(10.0, 0.0));
    layout(&t.target);

    assert_column_indexes(&t.target, 1, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn scrolls_right_more_than_a_page() {
    let _app = start_app();
    let t = default_target();

    t.scroll.set_offset(Vector::new(200.0, 0.0));
    layout(&t.target);

    assert_column_indexes(&t.target, 20, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn scrolls_left_more_than_a_page() {
    let _app = start_app();
    let t = default_target();

    t.scroll.set_offset(Vector::new(200.0, 0.0));
    layout(&t.target);

    t.scroll.set_offset(Vector::new(0.0, 0.0));
    layout(&t.target);

    assert_column_indexes(&t.target, 0, 10);
    assert_recyclable(&t.target, 0);
}

#[test]
fn bring_into_view_realizes_and_scrolls_to_distant_column() {
    let _app = start_app();
    let t = default_target();

    let brought = t
        .target
        .bring_into_view(50, None)
        .and_then(|element| element.cast::<TreeDataGridColumnHeader>())
        .expect("a column header");
    layout(&t.target);

    assert_eq!(50, brought.column_index());
    assert!(t.target.realized_elements().iter().flatten().any(|element| element.ptr_eq(&brought)));
    assert!(t.scroll.offset().x > 0.0);

    let viewport =
        Rect::new(t.scroll.offset().x, t.scroll.offset().y, t.scroll.viewport().width, t.scroll.viewport().height);
    assert!(brought.bounds().intersects(viewport));
}

#[test]
fn moving_columns_preserves_realized_header_identity() {
    let _app = start_app();
    let t = default_target();
    let headers = t.target.realized_elements();

    t.columns.move_item(2, 7);
    layout(&t.target);

    assert_eq!(headers[0], t.target.try_get_element(0));
    assert_eq!(headers[1], t.target.try_get_element(1));
    for old_index in 3..=7 {
        assert_eq!(headers[old_index as usize], t.target.try_get_element(old_index - 1));
    }
    assert_eq!(headers[2], t.target.try_get_element(7));
    assert_eq!(headers[8], t.target.try_get_element(8));
    assert_eq!(headers[9], t.target.try_get_element(9));
    assert_column_indexes(&t.target, 0, 10);
}

#[test]
fn updates_column_width_when_width_changes() {
    let _app = start_app();
    let t = default_target();

    for child in t.target.get_visual_children().iter() {
        assert_eq!(10.0, child.bounds().width);
    }

    t.columns.set_column_width(1, GridLength::new(12.0, GridUnitType::Pixel));
    layout(&t.target);

    for column in visual_children_of::<TreeDataGridColumnHeader>(&t.target) {
        assert_eq!(if column.column_index() == 1 { 12.0 } else { 10.0 }, column.bounds().width);
    }
}

#[test]
fn nth_child_handles_deletion_and_addition_correctly() {
    let _app = start_app();
    let style = Style::with_setters(
        Selectors::of_type::<TreeDataGridColumnHeadersPresenter>()
            .descendant()
            .of_type::<TreeDataGridColumnHeader>()
            .nth_child(2, 0),
        [Setter::new(TemplatedControl::background_property(), red_brush())],
    );
    let t = create_target(None, None, 100, vec![style]);

    layout(&t.target);

    let red = visual_children_of::<TreeDataGridColumnHeader>(&t.target)
        .into_iter()
        .filter(|header| is_red(header.background()))
        .count();
    assert_eq!(5, red);
}
