//! `TreeDataGridRowsPresenterTests_VariableHeight.cs`.
//!
//! The random heights of the reference come from `new Random(0)`; the port's fixed sequence is
//! another, and the tests depend only on the heights being varied, between 10 and 99.

use super::support::{
    has_logical_child, has_visual_child, layout, model_of, rows_of, run_jobs, same_model, scroll_viewer, start_app,
    test_window, visual_children_of, TestRandom,
};
use crate::models::{IColumns, ITreeDataGridRows};
use crate::primitives::{RealizedStackElements, TreeDataGridElementFactory, TreeDataGridRow, TreeDataGridRowsPresenter};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::{Event, INotifyPropertyChanged, Model};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{ferro_model, Rect, Ref, Size, Vector};
use ferroui_controls::{Border, Control, ScrollViewer, Window};
use std::cell::Cell;
use std::rc::Rc;

struct RowModel {
    id: Cell<i32>,
    height: Cell<f64>,
    property_changed: Event<str>,
}

ferro_model!(RowModel, |b| b
    .notify_property_changed()
    .read_only::<Value<i32>>("Id", |m| m.id.get())
    .property::<Value<f64>>("Height", |m| m.height.get(), |m, v| m.set_height(v)));

impl INotifyPropertyChanged for RowModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl RowModel {
    fn new(id: i32, height: f64) -> Rc<Self> {
        Model::new_model(Self { id: Cell::new(id), height: Cell::new(height), property_changed: Event::new() })
    }

    fn set_height(&self, value: f64) {
        if self.height.get() != value {
            self.height.set(value);
            self.property_changed.raise("Height");
        }
    }
}

struct Target {
    target: Ref<TreeDataGridRowsPresenter>,
    scroll: Ref<ScrollViewer>,
    items: Rc<FerroList<Rc<RowModel>>>,
    _root: Ref<Window>,
}

fn create_random_heights(count: usize) -> Vec<f64> {
    let mut random = TestRandom::new(0);
    (0..count).map(|_| f64::from(random.next(90) + 10)).collect()
}

fn create_target(
    columns: Option<Rc<dyn IColumns>>,
    item_count: usize,
    root_size: Option<Size>,
    row_heights: Option<Vec<f64>>,
) -> Target {
    let heights = row_heights.unwrap_or_else(|| create_random_heights(item_count));
    let items = Rc::new(FerroList::from_items(
        heights.iter().enumerate().map(|(index, height)| RowModel::new(index as i32, *height)),
    ));

    let rows = rows_of(items.clone());

    let target = TreeDataGridRowsPresenter::new();
    target.set_element_factory(Some(TreeDataGridElementFactory::new()));
    target.set_items(Some(rows as Rc<dyn ITreeDataGridRows>));
    target.set_columns(columns);

    let scroll = scroll_viewer(target.clone().upcast());

    let root = test_window(Some(scroll.clone().upcast()), Some(root_size.unwrap_or(Size::new(100.0, 1000.0))));
    root.styles().add(Style::with_setters(
        Selectors::of_type::<TreeDataGridRow>(),
        [Setter::new_binding_base(Layoutable::height_property().as_property(), ReflectionBinding::new("Height"))],
    ));

    root.update_layout();
    run_jobs();

    Target { target, scroll, items, _root: root }
}

fn default_target() -> Target {
    create_target(None, 100, None, None)
}

fn realized_rows(target: &TreeDataGridRowsPresenter) -> Vec<Ref<TreeDataGridRow>> {
    target.realized_elements().into_iter().flatten().map(|element| element.cast::<TreeDataGridRow>().expect("a row")).collect()
}

fn realized_row_indexes(target: &TreeDataGridRowsPresenter) -> Vec<i32> {
    realized_rows(target).iter().map(|row| row.row_index()).collect()
}

fn visible_rows(target: &TreeDataGridRowsPresenter) -> Vec<Ref<TreeDataGridRow>> {
    visual_children_of::<TreeDataGridRow>(target).into_iter().filter(|x| x.is_visible()).collect()
}

fn get_first_row_index(target: &TreeDataGridRowsPresenter) -> i32 {
    visible_rows(target).iter().map(|x| x.row_index()).min().expect("a visible row")
}

fn get_last_row_index(target: &TreeDataGridRowsPresenter) -> i32 {
    visible_rows(target).iter().map(|x| x.row_index()).max().expect("a visible row")
}

fn row(element: Option<Ref<Control>>) -> Ref<TreeDataGridRow> {
    element.and_then(|element| element.cast::<TreeDataGridRow>()).expect("a row")
}

fn viewport_of(scroll: &ScrollViewer) -> Rect {
    Rect::new(scroll.offset().x, scroll.offset().y, scroll.viewport().width, scroll.viewport().height)
}

fn assert_contiguous_rows(target: &TreeDataGridRowsPresenter, items: &FerroList<Rc<RowModel>>) {
    let rows = realized_rows(target);

    for (i, row) in rows.iter().enumerate() {
        assert_eq!(i as i32, row.row_index());
        assert!(same_model(&items.get(i), row.data_context().as_ref()));

        if i > 0 {
            assert!((rows[i - 1].bounds().bottom() - row.bounds().top()).abs() < 0.001);
        }
    }
}

#[test]
fn estimates_unrealized_row_positions_from_realized_range() {
    let _app = start_app();
    let target = RealizedStackElements::new(false);
    let estimated_size = Cell::new(25.0);

    target.add(10, Border::new().upcast(), 1000.0, 20.0);
    target.add(11, Border::new().upcast(), 1020.0, 40.0);

    assert_eq!(970.0, target.get_or_estimate_element_u(9, &estimated_size));
    assert_eq!(1060.0, target.get_or_estimate_element_u(12, &estimated_size));
    assert_eq!(1120.0, target.get_or_estimate_element_u(14, &estimated_size));
}

#[test]
fn does_not_use_stale_row_position_after_insert_before_range() {
    let _app = start_app();
    let target = RealizedStackElements::new(false);
    let estimated_size = Cell::new(25.0);

    target.add(10, Border::new().upcast(), 1000.0, 20.0);
    target.items_inserted(0, 1, |_, _, _| {}, |_| {});

    assert!(target.get_element_u(11).is_nan());
    assert_eq!(220.0, target.get_or_estimate_element_u(11, &estimated_size));
}

fn scroll_down_to_bottom(step: f64) {
    let _app = start_app();
    let t = default_target();

    layout(&t.target);

    let mut index = get_first_row_index(&t.target);
    assert_eq!(0, index);

    while t.scroll.offset().y < t.scroll.extent().height - t.scroll.viewport().height {
        t.scroll.set_offset(Vector::new(0.0, t.scroll.offset().y + step));
        layout(&t.target);

        let new_index = get_first_row_index(&t.target);
        assert!(new_index >= index, "{new_index} > {index} failed");
        index = new_index;
    }
}

#[test]
fn scroll_down_to_bottom_10() {
    scroll_down_to_bottom(10.0);
}

#[test]
fn scroll_down_to_bottom_20() {
    scroll_down_to_bottom(20.0);
}

#[test]
fn scroll_down_to_bottom_50() {
    scroll_down_to_bottom(50.0);
}

#[test]
fn scroll_to_bottom() {
    let _app = start_app();
    let t = default_target();

    let _subscription = t.scroll.property_changed(|_| {});

    layout(&t.target);

    let index = get_first_row_index(&t.target);
    assert_eq!(0, index);

    t.scroll.set_offset(Vector::new(0.0, t.scroll.extent().height - t.scroll.viewport().height));
    layout(&t.target);

    let last_index = get_last_row_index(&t.target);
    assert_eq!(t.items.count() as i32 - 1, last_index);
}

#[test]
fn bring_into_view_does_not_leave_unrealized_row_visible() {
    let _app = start_app();
    let t = create_target(None, 227, Some(Size::new(100.0, 600.0)), None);
    let target_indexes = [206, 5, 185, 25, 220, 100, 200, 0];

    for target_index in target_indexes {
        let brought = row(t.target.bring_into_view(target_index, None));
        layout(&t.target);

        assert_eq!(target_index, brought.row_index());
        assert!(same_model(&t.items.get(target_index as usize), brought.data_context().as_ref()));

        let mut visible = visible_rows(&t.target);
        visible.sort_by(|a, b| a.bounds().top().total_cmp(&b.bounds().top()));
        let mut realized = realized_rows(&t.target);
        realized.sort_by(|a, b| a.bounds().top().total_cmp(&b.bounds().top()));

        assert!(realized.contains(&brought));
        assert_eq!(realized, visible);

        for i in 1..visible.len() {
            assert_eq!(visible[i - 1].row_index() + 1, visible[i].row_index());
            assert!(
                (visible[i - 1].bounds().bottom() - visible[i].bounds().top()).abs() < 0.001,
                "Rows {} and {} overlap or have a gap.",
                visible[i - 1].row_index(),
                visible[i].row_index()
            );
        }
    }
}

#[test]
fn bring_into_view_correctly_scrolls_down_to_a_page_of_smaller_rows() {
    let _app = start_app();
    let heights: Vec<f64> = (0..20).map(|x| f64::from(((29 - x) / 10) * 10)).collect();
    let t = create_target(None, heights.len(), Some(Size::new(100.0, 100.0)), Some(heights));

    let brought = row(t.target.bring_into_view(19, None));

    assert_eq!(19, brought.row_index());
    assert_eq!(Rect::new(0.0, 380.0, 100.0, 10.0), brought.bounds());
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(100.0, 390.0), t.scroll.extent());
    assert_eq!(Vector::new(0.0, 290.0), t.scroll.offset());
    assert_eq!((10..20).collect::<Vec<i32>>(), realized_row_indexes(&t.target));
}

#[test]
fn bring_into_view_correctly_scrolls_down_to_a_page_of_larger_rows() {
    let _app = start_app();
    let heights: Vec<f64> = (0..20).map(|x| f64::from(((x / 10) + 1) * 10)).collect();
    let t = create_target(None, heights.len(), Some(Size::new(100.0, 100.0)), Some(heights));

    let brought = row(t.target.bring_into_view(19, None));

    assert_eq!(19, brought.row_index());
    assert_eq!(Rect::new(0.0, 190.0, 100.0, 20.0), brought.bounds());
    assert_eq!(Size::new(100.0, 100.0), t.scroll.viewport());
    assert_eq!(Size::new(100.0, 210.0), t.scroll.extent());
    assert_eq!(Vector::new(0.0, 110.0), t.scroll.offset());
    assert_eq!((15..20).collect::<Vec<i32>>(), realized_row_indexes(&t.target));
}

#[test]
fn realized_row_height_changes_preserve_anchor_while_updating_extent() {
    let _app = start_app();
    let t = create_target(None, 20, Some(Size::new(100.0, 100.0)), Some(vec![50.0; 20]));

    t.target.bring_into_view(5, None);

    let anchor = row(t.target.try_get_element(4));
    assert_eq!(vec![4, 5], realized_row_indexes(&t.target));
    assert_eq!(Size::new(100.0, 1000.0), t.scroll.extent());
    assert_eq!(Vector::new(0.0, 200.0), t.scroll.offset());

    for item in t.items.to_vec() {
        item.set_height(25.0);
    }
    layout(&t.target);

    assert_eq!(anchor, realized_rows(&t.target)[0]);
    assert_eq!(Rect::new(0.0, 200.0, 100.0, 25.0), anchor.bounds());
    assert_eq!(Size::new(100.0, 600.0), t.scroll.extent());
    assert_eq!(Vector::new(0.0, 200.0), t.scroll.offset());
    assert_eq!(vec![4, 5, 6, 7], realized_row_indexes(&t.target));
}

#[test]
fn realized_row_height_change_preserves_scroll_anchor() {
    let _app = start_app();
    let t = create_target(None, 100, Some(Size::new(100.0, 100.0)), Some(vec![25.0; 100]));

    t.scroll.set_offset(Vector::new(0.0, 1000.0));
    layout(&t.target);

    let anchor = row(t.target.try_get_element(40));
    assert_eq!(1000.0, anchor.bounds().top());

    t.items.get(40).set_height(200.0);
    layout(&t.target);

    let realized = realized_rows(&t.target);
    assert_eq!(anchor, realized[0]);
    assert_eq!(40, realized[0].row_index());
    assert_eq!(1000.0, realized[0].bounds().top());
    assert!(realized[realized.len() - 1].bounds().bottom() >= t.scroll.offset().y + t.scroll.viewport().height);
}

#[test]
fn focused_row_is_kept_outside_viewport_when_row_heights_change() {
    let _app = start_app();
    let t = create_target(None, 20, Some(Size::new(100.0, 100.0)), Some(vec![50.0; 20]));

    t.target.bring_into_view(5, None);
    let focused = row(t.target.try_get_element(5));
    focused.set_focusable(true);
    focused.focus();

    t.scroll.set_offset(Vector::new(0.0, 400.0));
    layout(&t.target);
    assert!(!realized_rows(&t.target).contains(&focused));

    for item in t.items.to_vec() {
        item.set_height(25.0);
    }
    layout(&t.target);

    let viewport = viewport_of(&t.scroll);
    assert!(focused.is_keyboard_focus_within());
    assert!(!focused.bounds().intersects(viewport));
    assert!(!realized_rows(&t.target).contains(&focused));
    assert_eq!(8, realized_rows(&t.target)[0].row_index());
    assert_eq!(400.0, realized_rows(&t.target)[0].bounds().top());
}

#[test]
fn pixel_scrolls_in_both_directions_preserve_variable_height_virtualization() {
    let _app = start_app();
    let t = create_target(None, 40, Some(Size::new(100.0, 200.0)), None);
    let encountered_rows: std::cell::RefCell<Vec<Ref<TreeDataGridRow>>> = std::cell::RefCell::new(Vec::new());
    let maximum_realized_count = Cell::new(0usize);

    let assert_state = || {
        let rows = realized_rows(&t.target);
        let viewport_start = t.scroll.offset().y;
        let viewport_end = t.scroll.extent().height.min(viewport_start + t.scroll.viewport().height);

        assert!(!rows.is_empty());
        assert!(rows.iter().enumerate().all(|(i, row)| !rows[..i].contains(row)));
        assert!(rows[0].bounds().top() <= viewport_start + 0.001);
        assert!(rows[rows.len() - 1].bounds().bottom() >= viewport_end - 0.001);

        for (i, row) in rows.iter().enumerate() {
            assert!(same_model(&t.items.get(row.row_index() as usize), row.data_context().as_ref()));
            assert!(has_visual_child(&t.target, row));
            assert!(has_logical_child(&t.target, row));

            if i > 0 {
                assert_eq!(rows[i - 1].row_index() + 1, row.row_index());
                assert!((rows[i - 1].bounds().bottom() - row.bounds().top()).abs() < 0.001);
            }

            let mut encountered = encountered_rows.borrow_mut();
            if !encountered.contains(row) {
                encountered.push(row.clone());
            }
        }

        maximum_realized_count.set(maximum_realized_count.get().max(rows.len()));
    };

    let maximum_offset = (t.scroll.extent().height - t.scroll.viewport().height) as i32;

    for offset in 0..=maximum_offset {
        t.scroll.set_offset(Vector::new(0.0, f64::from(offset)));
        layout(&t.target);
        assert_state();
    }

    let mutation_index = ((realized_rows(&t.target)[0].row_index() + 1) as usize).min(t.items.count());
    let inserted = RowModel::new(-1, 37.0);
    t.items.insert(mutation_index, inserted);
    layout(&t.target);
    assert_state();
    t.items.remove_at(mutation_index);
    layout(&t.target);
    assert_state();

    let mut offset = t.scroll.offset().y as i32;
    while offset >= 0 {
        t.scroll.set_offset(Vector::new(0.0, f64::from(offset)));
        layout(&t.target);
        assert_state();
        offset -= 1;
    }

    let encountered = encountered_rows.borrow().len();
    assert!(
        encountered <= maximum_realized_count.get() + 2,
        "Encountered {encountered} controls for a maximum realized count of {}.",
        maximum_realized_count.get()
    );
}

#[test]
fn insert_and_remove_preserve_variable_height_row_identity_and_positions() {
    let _app = start_app();
    let t = create_target(None, 100, Some(Size::new(100.0, 400.0)), None);
    let original_rows = realized_rows(&t.target);
    let inserted = RowModel::new(-1, 73.0);

    t.items.insert(2, inserted.clone());
    layout(&t.target);

    assert!(same_model(&inserted, row(t.target.try_get_element(2)).data_context().as_ref()));
    let mut preserved_count = 0;
    for old_index in 2..original_rows.len() - 1 {
        let Some(shifted_row) = t.target.try_get_element(old_index as i32 + 1) else {
            break;
        };

        assert!(shifted_row.ptr_eq(&original_rows[old_index]));
        preserved_count += 1;
    }
    assert!(preserved_count > 0);
    assert_contiguous_rows(&t.target, &t.items);

    t.items.remove_at(2);
    layout(&t.target);

    for index in 0..2 + preserved_count {
        assert!(t.target.try_get_element(index as i32).is_some_and(|element| element.ptr_eq(&original_rows[index])));
    }
    assert_contiguous_rows(&t.target, &t.items);
}

#[test]
fn move_preserves_variable_height_row_identity_and_positions() {
    let _app = start_app();
    let t = create_target(None, 100, Some(Size::new(100.0, 400.0)), None);
    let rows_by_item: Vec<(Rc<RowModel>, Ref<TreeDataGridRow>)> =
        realized_rows(&t.target).into_iter().map(|row| (model_of::<RowModel>(row.data_context()), row)).collect();

    t.items.move_item(2, 7);
    layout(&t.target);

    for (item, row) in &rows_by_item {
        let new_index = t.items.index_of(item).expect("the index of the item") as i32;
        if let Some(element) = t.target.try_get_element(new_index) {
            assert!(element.ptr_eq(row));
        }
    }

    assert_contiguous_rows(&t.target, &t.items);
}
