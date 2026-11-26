//! Tests of `ItemsStackPanel.ItemsUpdatingScrollMode` (#1424): what stays in view when items are
//! added before the viewport or at the end of the list.

use super::list_view_tests::list_view_template;
use super::*;
use crate::templates::FuncTemplate;
use crate::test_support::{test_scope, TestRoot};
use crate::{ItemsSource, Panel, ScrollViewer};
use ferroui_base::collections::FerroList;
use ferroui_base::{Ref, Vector};
use std::rc::Rc;

const ITEM: f64 = 20.0;
const HEIGHT: f64 = 200.0;

struct Scrolled {
    items: Rc<FerroList<String>>,
    list: Ref<ListView>,
    panel: Ref<ItemsStackPanel>,
    scroll: Ref<ScrollViewer>,
    root: Ref<TestRoot>,
}

impl Scrolled {
    /// A list of `count` items of 20 px in a viewport of 200 px, its panel in `mode`.
    fn new(count: usize, mode: ItemsUpdatingScrollMode) -> Self {
        let items = Rc::new(FerroList::from_items((0..count).map(|index| format!("Item {index}"))));
        let list = ListView::new();
        list.set_template(list_view_template());
        list.set_height(HEIGHT);
        list.set_items_panel(FuncTemplate::new(move || {
            let panel = ItemsStackPanel::new();
            panel.set_items_updating_scroll_mode(mode);
            Some(panel.upcast::<Panel>())
        }));
        list.container_prepared(|args| args.container().set_height(ITEM))
            .detach();
        list.set_items_source(Some(ItemsSource::new(items.clone())));
        let root = TestRoot::with_child(list.clone());
        root.layout_manager().execute_initial_layout_pass();
        root.layout_manager().execute_layout_pass();
        let panel = list.items_panel_root().unwrap().cast::<ItemsStackPanel>().unwrap();
        let scroll = list
            .get_template_children()
            .into_iter()
            .find_map(|child| child.cast::<ScrollViewer>())
            .unwrap();
        Self {
            items,
            list,
            panel,
            scroll,
            root,
        }
    }

    fn layout(&self) {
        for _ in 0..3 {
            self.root.layout_manager().execute_layout_pass();
        }
    }

    fn scroll_to(&self, offset: f64) {
        self.scroll.set_offset(Vector::new(0.0, offset));
        self.layout();
    }

    /// The largest offset.
    fn end(&self) -> f64 {
        self.scroll.extent().height - self.scroll.viewport().height
    }

    fn offset(&self) -> f64 {
        self.scroll.offset().y
    }

    /// The top of the item at `index` in the viewport.
    fn top_in_viewport(&self, index: i32) -> f64 {
        self.panel.container_from_index(index).expect("realized").bounds().y - self.offset()
    }

    fn insert_at_start(&self, count: usize) {
        for index in 0..count {
            self.items.insert(index, format!("New {index}"));
        }
        self.layout();
    }
}

#[test]
fn keep_items_in_view_keeps_the_first_visible_item_in_place() {
    let _scope = test_scope();
    let list = Scrolled::new(30, ItemsUpdatingScrollMode::KeepItemsInView);
    assert_eq!(
        list.panel.items_updating_scroll_mode(),
        ItemsUpdatingScrollMode::KeepItemsInView
    );
    list.scroll_to(100.0);
    assert_eq!(list.top_in_viewport(5), 0.0);
    list.insert_at_start(2);
    // Item 5 is now at index 7, still at the top of the viewport.
    assert_eq!(list.offset(), 140.0);
    assert_eq!(list.top_in_viewport(7), 0.0);
}

#[test]
fn keep_items_in_view_at_the_start_shows_the_inserted_items() {
    let _scope = test_scope();
    let list = Scrolled::new(30, ItemsUpdatingScrollMode::KeepItemsInView);
    list.insert_at_start(2);
    assert_eq!(list.offset(), 0.0);
    assert_eq!(list.top_in_viewport(0), 0.0);
}

#[test]
fn keep_scroll_offset_keeps_the_offset_and_moves_the_items() {
    let _scope = test_scope();
    let list = Scrolled::new(30, ItemsUpdatingScrollMode::KeepScrollOffset);
    list.scroll_to(100.0);
    list.insert_at_start(2);
    assert_eq!(list.offset(), 100.0);
    // Item 5 moved down by the two new items; index 5 is at the top.
    assert_eq!(list.top_in_viewport(7), 40.0);
    assert_eq!(list.top_in_viewport(5), 0.0);
}

#[test]
fn switching_to_keep_scroll_offset_stops_the_anchoring_of_realized_items() {
    let _scope = test_scope();
    let list = Scrolled::new(30, ItemsUpdatingScrollMode::KeepItemsInView);
    list.scroll_to(100.0);
    list.panel
        .set_items_updating_scroll_mode(ItemsUpdatingScrollMode::KeepScrollOffset);
    list.insert_at_start(2);
    assert_eq!(list.offset(), 100.0);
    list.panel
        .set_items_updating_scroll_mode(ItemsUpdatingScrollMode::KeepItemsInView);
    list.layout();
    list.insert_at_start(1);
    assert_eq!(list.offset(), 120.0);
}

#[test]
fn keep_last_item_in_view_starts_at_the_end_and_follows_added_items() {
    let _scope = test_scope();
    let list = Scrolled::new(30, ItemsUpdatingScrollMode::KeepLastItemInView);
    // The extent estimates the items before the realized ones; the last item ends the viewport.
    assert_eq!(list.offset(), list.end());
    assert_eq!(list.top_in_viewport(29), HEIGHT - ITEM);
    list.items.add("Last".to_string());
    list.layout();
    assert_eq!(list.offset(), list.end());
    assert_eq!(list.top_in_viewport(30), HEIGHT - ITEM);
    assert_eq!(list.list.items_source().map(|source| source.count()), Some(31));
}

#[test]
fn keep_last_item_in_view_scrolled_up_keeps_the_items_in_view() {
    let _scope = test_scope();
    let list = Scrolled::new(30, ItemsUpdatingScrollMode::KeepLastItemInView);
    list.scroll_to(100.0);
    list.items.add("Last".to_string());
    list.layout();
    assert_eq!(list.offset(), 100.0);
    list.insert_at_start(1);
    assert_eq!(list.offset(), 120.0);
}

#[test]
fn keep_last_item_in_view_of_a_short_list_needs_no_scrolling() {
    let _scope = test_scope();
    let list = Scrolled::new(3, ItemsUpdatingScrollMode::KeepLastItemInView);
    assert_eq!(list.offset(), 0.0);
    for index in 0..9 {
        list.items.add(format!("Message {index}"));
        list.layout();
    }
    // 12 items of 20 px in 200 px: scrolled to the end after each one.
    assert_eq!(list.offset(), 40.0);
    assert_eq!(list.top_in_viewport(11), HEIGHT - ITEM);
}
