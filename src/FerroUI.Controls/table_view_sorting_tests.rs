//! Tests of the sorting of the table view and of the WinUI column types (WinUI additions,
//! `docs/porting/winui-controls.md`, section 10.11): the first test checks that the defaults keep
//! the upstream table view.

use super::table_view_tests::*;
use crate::{
    AssignedBinding, ItemsSource, TableView, TableViewColumn, TableViewSortCycle, TableViewSortDirection,
    TableViewSortedEventArgs, TableViewSource, TableViewTemplateColumn, TableViewTextColumn, ToolTip,
};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::Value;
use ferroui_base::data::model::Model;
use ferroui_base::data::{BindingBase, ReflectionBinding};
use ferroui_base::{ferro_model, BoxedValue, Ref};
use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

struct Item {
    id: i32,
    name: String,
    category: String,
}

ferro_model!(Item, |b| b
    .read_only::<Value<i32>>("Id", |item| item.id)
    .read_only::<Value<String>>("Name", |item| item.name.clone())
    .read_only::<Value<String>>("Category", |item| item.category.clone()));

fn item(id: i32, name: &str, category: &str) -> Rc<Item> {
    Model::new_model(Item { id, name: name.to_string(), category: category.to_string() })
}

fn items() -> Vec<Rc<Item>> {
    vec![item(2, "Item 2", "Beta"), item(3, "Item 3", "Alpha"), item(1, "Item 1", "Gamma"), item(4, "Item 4", "Alpha")]
}

fn source_of(items: &[Rc<Item>]) -> ItemsSource {
    ItemsSource::from_items(items.iter().map(|item| Some(Rc::clone(item) as BoxedValue)))
}

fn bound(path: &str) -> Option<AssignedBinding> {
    let binding: Rc<dyn BindingBase> = ReflectionBinding::new(path);
    Some(AssignedBinding::new(binding))
}

fn text_column(path: &str) -> Ref<TableViewColumn> {
    let column = TableViewTextColumn::new();
    column.set_binding(bound(path));
    column.upcast()
}

/// The ids of the items in the order the table presents them.
fn presented_ids(target: &TableView) -> Vec<i32> {
    let view = target.items_view();
    (0..view.count())
        .map(|index| {
            let value = view.get_at(index).expect("an item");
            value.downcast_ref::<Item>().expect("an item of the test").id
        })
        .collect()
}

fn table_with(items: &[Rc<Item>]) -> (Ref<TableView>, Ref<TableViewColumn>, Ref<TableViewColumn>) {
    let target = create_target(source_of(items));
    let id = text_column("Id");
    let name = text_column("Category");
    target.columns().add(id.clone());
    target.columns().add(name.clone());
    (target, id, name)
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

#[test]
fn winui_sorting_additions_default_to_upstream() {
    let _app = start();
    let items = items();
    let (target, id, _) = table_with(&items);
    let _root = prepare(&target);
    let source = target.items_source().expect("a source");

    assert!(!target.can_user_sort_columns());
    assert_eq!(id.can_sort(), None);
    assert_eq!(id.sort_cycle(), TableViewSortCycle::AscendingDescendingNone);
    assert_eq!(id.sort_direction(), TableViewSortDirection::None);
    assert_eq!(id.sort_member_path(), None);
    assert!(!id.can_user_effectively_sort());
    assert!(id.header_tool_tip().is_none());
    assert!(id.cell_tool_tip_binding().is_none());

    let presenter = get_column_headers_presenter(&target);
    assert!(!header(&presenter, 0).classes().contains(":sortable"));

    // A click on a header does not sort: the table presents its items source as upstream.
    target.on_column_header_clicked(&id);
    assert_eq!(id.sort_direction(), TableViewSortDirection::None);
    assert!(target.items_view().source().ptr_eq(&source));
    assert_eq!(presented_ids(&target), vec![2, 3, 1, 4]);
}

// ---------------------------------------------------------------------------
// The sort cycle
// ---------------------------------------------------------------------------

#[test]
fn the_sort_cycles_go_through_their_directions() {
    use TableViewSortDirection::{Ascending, Descending, None};
    let cases = [
        (TableViewSortCycle::AscendingDescendingNone, [Ascending, Descending, None]),
        (TableViewSortCycle::DescendingAscendingNone, [Descending, Ascending, None]),
        (TableViewSortCycle::AscendingDescending, [Ascending, Descending, Ascending]),
        (TableViewSortCycle::DescendingAscending, [Descending, Ascending, Descending]),
    ];
    for (cycle, expected) in cases {
        let mut direction = None;
        for step in expected {
            direction = cycle.next(direction);
            assert_eq!(direction, step, "{cycle:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Sorting
// ---------------------------------------------------------------------------

#[test]
fn a_click_on_a_sortable_header_sorts_through_the_cycle() {
    let _app = start();
    let items = items();
    let (target, id, _) = table_with(&items);
    target.set_can_user_sort_columns(true);
    let _root = prepare(&target);
    let source = target.items_source().expect("a source");
    assert!(id.can_user_effectively_sort());

    target.on_column_header_clicked(&id);
    assert_eq!(id.sort_direction(), TableViewSortDirection::Ascending);
    assert_eq!(presented_ids(&target), vec![1, 2, 3, 4]);
    // The items source is not changed.
    assert!(target.items_source().expect("a source").ptr_eq(&source));

    target.on_column_header_clicked(&id);
    assert_eq!(id.sort_direction(), TableViewSortDirection::Descending);
    assert_eq!(presented_ids(&target), vec![4, 3, 2, 1]);

    target.on_column_header_clicked(&id);
    assert_eq!(id.sort_direction(), TableViewSortDirection::None);
    assert_eq!(presented_ids(&target), vec![2, 3, 1, 4]);
    assert!(target.items_view().source().ptr_eq(&source));
}

#[test]
fn a_column_can_sort_decides_over_the_table() {
    let _app = start();
    let items = items();
    let (target, id, category) = table_with(&items);
    target.set_can_user_sort_columns(true);
    id.set_can_sort(Some(false));
    let _root = prepare(&target);

    target.on_column_header_clicked(&id);
    assert_eq!(id.sort_direction(), TableViewSortDirection::None);

    target.set_can_user_sort_columns(false);
    category.set_can_sort(Some(true));
    assert!(category.can_user_effectively_sort());
    target.on_column_header_clicked(&category);
    assert_eq!(category.sort_direction(), TableViewSortDirection::Ascending);
}

#[test]
fn equal_values_keep_the_order_of_the_source() {
    let _app = start();
    let items = items();
    let (target, _, category) = table_with(&items);
    let _root = prepare(&target);

    target.sort(&category, TableViewSortDirection::Ascending);
    // Alpha (3, 4 in source order), Beta (2), Gamma (1).
    assert_eq!(presented_ids(&target), vec![3, 4, 2, 1]);
    target.sort(&category, TableViewSortDirection::Descending);
    assert_eq!(presented_ids(&target), vec![1, 2, 3, 4]);
}

#[test]
fn sorting_by_another_column_unsorts_the_first() {
    let _app = start();
    let items = items();
    let (target, id, category) = table_with(&items);
    let _root = prepare(&target);

    target.sort(&id, TableViewSortDirection::Descending);
    target.sort(&category, TableViewSortDirection::Ascending);

    assert_eq!(id.sort_direction(), TableViewSortDirection::None);
    assert_eq!(category.sort_direction(), TableViewSortDirection::Ascending);
}

#[test]
fn sorted_is_raised_with_the_column_and_the_direction() {
    let _app = start();
    let items = items();
    let (target, id, _) = table_with(&items);
    let _root = prepare(&target);
    let raised: Rc<RefCell<Vec<(Ref<TableViewColumn>, TableViewSortDirection)>>> = Rc::default();
    let sink = raised.clone();
    target.sorted(move |_, e: &TableViewSortedEventArgs| sink.borrow_mut().push((e.column().clone(), e.direction())));

    target.sort(&id, TableViewSortDirection::Ascending);
    target.sort(&id, TableViewSortDirection::None);

    let raised = raised.borrow();
    assert_eq!(raised.len(), 2);
    assert!(raised[0].0 == id && raised[0].1 == TableViewSortDirection::Ascending);
    assert!(raised[1].0 == id && raised[1].1 == TableViewSortDirection::None);
}

#[test]
fn sorting_keeps_the_selected_item_selected() {
    let _app = start();
    let items = items();
    let (target, id, _) = table_with(&items);
    let _root = prepare(&target);
    target.set_selected_index(0);
    let selected = target.selected_item().expect("a selected item");

    target.sort(&id, TableViewSortDirection::Ascending);

    assert_eq!(target.selected_index(), 1);
    assert!(Rc::ptr_eq(&target.selected_item().expect("a selected item"), &selected));
}

#[test]
fn a_sorted_table_sorts_its_items_again_when_they_change() {
    let _app = start();
    let list = Rc::new(FerroList::<Option<BoxedValue>>::new());
    for item in items() {
        list.add(Some(item as BoxedValue));
    }
    let target = create_target(ItemsSource::from(list.clone()));
    let id = text_column("Id");
    target.columns().add(id.clone());
    let _root = prepare(&target);
    target.sort(&id, TableViewSortDirection::Ascending);

    list.add(Some(item(0, "Item 0", "Alpha") as BoxedValue));

    assert_eq!(presented_ids(&target), vec![0, 1, 2, 3, 4]);
}

#[test]
fn a_new_items_source_of_a_sorted_table_is_sorted() {
    let _app = start();
    let items = items();
    let (target, id, _) = table_with(&items);
    let _root = prepare(&target);
    target.sort(&id, TableViewSortDirection::Descending);

    target.set_items_source(Some(source_of(&[item(7, "a", "x"), item(9, "b", "y"), item(8, "c", "z")])));

    assert_eq!(presented_ids(&target), vec![9, 8, 7]);
}

#[test]
fn a_table_view_source_is_sorted_in_place() {
    let _app = start();
    let items = items();
    let view = TableViewSource::from(source_of(&items));
    let target = create_target(ItemsSource::from(view.clone()));
    let id = text_column("Id");
    target.columns().add(id.clone());
    let _root = prepare(&target);

    target.sort(&id, TableViewSortDirection::Ascending);

    // The table presents the view it was given, now in the order of the sort.
    assert!(target.items_view().source().ptr_eq(&ItemsSource::from(view.clone())));
    assert_eq!(presented_ids(&target), vec![1, 2, 3, 4]);
    assert_eq!(view.source_index(0), Some(2));
}

#[test]
fn a_template_column_sorts_by_its_sort_member_path() {
    let _app = start();
    let items = items();
    let target = create_target(source_of(&items));
    let column = TableViewTemplateColumn::new();
    column.set_cell_template(Some(text_block_template()));
    column.set_sort_member_path(Some("Name".to_string()));
    let column: Ref<TableViewColumn> = column.upcast();
    target.columns().add(column.clone());
    let _root = prepare(&target);

    target.sort(&column, TableViewSortDirection::Descending);

    assert_eq!(presented_ids(&target), vec![4, 3, 2, 1]);
}

#[test]
fn the_header_has_the_pseudo_classes_of_the_sort() {
    let _app = start();
    let items = items();
    let (target, id, _) = table_with(&items);
    target.set_can_user_sort_columns(true);
    let _root = prepare(&target);
    let presenter = get_column_headers_presenter(&target);

    assert!(header(&presenter, 0).classes().contains(":sortable"));
    target.sort(&id, TableViewSortDirection::Ascending);
    assert!(header(&presenter, 0).classes().contains(":sortascending"));
    target.sort(&id, TableViewSortDirection::Descending);
    assert!(!header(&presenter, 0).classes().contains(":sortascending"));
    assert!(header(&presenter, 0).classes().contains(":sortdescending"));
    target.set_can_user_sort_columns(false);
    assert!(!header(&presenter, 0).classes().contains(":sortable"));
}

// ---------------------------------------------------------------------------
// Tool tips
// ---------------------------------------------------------------------------

#[test]
fn the_tool_tips_of_the_header_and_the_cells_come_from_the_column() {
    let _app = start();
    let items = items();
    let (target, id, _) = table_with(&items[..1]);
    id.set_header_tool_tip(Some(Rc::new("The id".to_string()) as BoxedValue));
    id.set_cell_tool_tip_binding(bound("Name"));
    let _root = prepare(&target);

    let presenter = get_column_headers_presenter(&target);
    let tip = ToolTip::get_tip(&header(&presenter, 0));
    assert_eq!(tip.as_ref().and_then(crate::test_support::string_of).as_deref(), Some("The id"));

    let row = single_row(&target);
    let cell = cell(&get_cells_presenter(&row), 0);
    let tip = ToolTip::get_tip(&cell);
    assert_eq!(tip.as_ref().and_then(crate::test_support::string_of).as_deref(), Some("Item 2"));

    id.set_header_tool_tip(None);
    id.set_cell_tool_tip_binding(None);
    assert!(ToolTip::get_tip(&header(&presenter, 0)).is_none());
    assert!(ToolTip::get_tip(&cell).is_none());
}
