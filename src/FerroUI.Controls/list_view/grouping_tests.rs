//! Tests of the groups of a list view (#1258): the flat view of a grouped source, the headers of
//! the panel with virtualization, sticky headers, padding and placement.

use super::list_view_tests::list_view_template;
use super::*;
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{ItemsSource, ScrollViewer};
use ferroui_base::collections::FerroList;
use ferroui_base::layout::Layoutable;
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::{BoxedValue, Ref, StaticType as _, Thickness, Vector};
use std::cell::RefCell;
use std::rc::Rc;

const ITEM: f64 = 20.0;
const HEADER: f64 = 30.0;

fn group(name: &str, count: usize) -> Rc<CollectionViewGroup> {
    let items = (0..count).map(|index| boxed_str(&format!("{name}{index}")));
    CollectionViewGroup::new(boxed_str(name), ItemsSource::from_items(items))
}

fn groups(counts: &[usize]) -> Rc<GroupedItemsView> {
    let groups = counts.iter().enumerate().map(|(index, count)| group(&format!("G{index}-"), *count)).collect();
    GroupedItemsView::new(groups)
}

struct Grouped {
    list: Ref<ListView>,
    panel: Ref<ItemsStackPanel>,
    scroll: Ref<ScrollViewer>,
    root: Ref<TestRoot>,
}

impl Grouped {
    fn new(counts: &[usize], configure: impl Fn(&ItemsStackPanel, &GroupStyle)) -> Self {
        let list = ListView::new();
        list.set_template(list_view_template());
        list.set_height(200.0);
        let theme = ControlTheme::with_target_type(ListViewHeaderItem::TYPE);
        theme.setters().add(Setter::new(Layoutable::height_property(), HEADER));
        let style = GroupStyle::new();
        style.set_header_container_style(Some(theme));
        list.group_style().add(style.clone());
        list.container_prepared(|args| args.container().set_height(ITEM)).detach();
        list.set_items_source(Some(ItemsSource::new(groups(counts))));
        let root = TestRoot::with_child(list.clone());
        root.layout_manager().execute_initial_layout_pass();
        let panel = list.items_panel_root().unwrap().cast::<ItemsStackPanel>().unwrap();
        configure(&panel, &style);
        root.layout_manager().execute_layout_pass();
        let scroll = list.get_template_children().into_iter().find_map(|child| child.cast::<ScrollViewer>()).unwrap();
        Self { list, panel, scroll, root }
    }

    fn scroll_to(&self, offset: f64) {
        self.scroll.set_offset(Vector::new(0.0, offset));
        self.root.layout_manager().execute_layout_pass();
    }

    /// The visible headers, by their text, with their position in the panel.
    fn headers(&self) -> Vec<(String, f64)> {
        let mut headers: Vec<(String, f64)> = self
            .panel
            .children()
            .to_vec()
            .into_iter()
            .filter_map(|child| child.cast::<ListViewHeaderItem>())
            .filter(|header| header.is_visible())
            .map(|header| (text(&header.content()), header.bounds().y))
            .collect();
        headers.sort_by(|left, right| left.1.total_cmp(&right.1));
        headers
    }

    fn y_of(&self, index: i32) -> f64 {
        self.panel.container_from_index(index).expect("realized").bounds().y
    }
}

fn text(value: &Option<BoxedValue>) -> String {
    value.as_ref().and_then(string_of).unwrap_or_default()
}

fn text_of(view: &GroupedItemsView, index: usize) -> String {
    text(&view.get_at_for_test(index))
}

impl GroupedItemsView {
    fn get_at_for_test(&self, index: usize) -> Option<BoxedValue> {
        crate::items_source::IItemsList::get_at(self, index)
    }
}

#[test]
fn the_view_is_the_items_of_all_groups_in_order() {
    let _scope = test_scope();
    let view = groups(&[2, 0, 3]);
    assert_eq!(crate::items_source::IItemsList::count(&*view), 5);
    assert_eq!(text_of(&view, 1), "G0-1");
    assert_eq!(text_of(&view, 2), "G2-0");
    assert_eq!(view.group_of(1), Some((0, 1)));
    // The empty group takes no index.
    assert_eq!(view.group_of(2), Some((2, 0)));
    assert_eq!(view.group_of(5), None);
    assert_eq!((view.group_start(2), view.group_len(1)), (Some(2), 0));
    assert_eq!(crate::items_source::IItemsList::index_of(&*view, &boxed_str("G2-2")), 4);
}

#[test]
fn a_change_in_a_group_is_reported_at_its_flat_index() {
    let _scope = test_scope();
    let first: FerroList<Option<BoxedValue>> = FerroList::new();
    first.add(boxed_str("a"));
    let second: FerroList<Option<BoxedValue>> = FerroList::new();
    second.add(boxed_str("b"));
    let view = GroupedItemsView::new(vec![
        CollectionViewGroup::new(boxed_str("1"), ItemsSource::new(Rc::new(first))),
        CollectionViewGroup::new(boxed_str("2"), ItemsSource::new(Rc::new(second.clone()))),
    ]);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink = seen.clone();
    let token = crate::items_source::IItemsList::add_collection_changed(
        &*view,
        Rc::new(move |e: &crate::ItemsChangedEventArgs<'_>| sink.borrow_mut().push(e.new_starting_index)),
    );
    second.add(boxed_str("c"));
    assert_eq!(*seen.borrow(), vec![2]);
    assert_eq!(crate::items_source::IItemsList::count(&*view), 3);
    assert_eq!(text_of(&view, 2), "c");
    crate::items_source::IItemsList::remove_collection_changed(&*view, token.unwrap());
}

#[test]
fn a_collection_view_source_groups_its_source_on_request() {
    let _scope = test_scope();
    let source = CollectionViewSource::new();
    let groups = vec![group("A", 2), group("B", 1)];
    source.set_source(Some(ItemsSource::from_values(groups)));
    assert!(source.view().unwrap().downcast_ref::<GroupedItemsView>().is_none());
    source.set_is_source_grouped(true);
    assert_eq!(source.view().unwrap().count(), 3);
    assert_eq!(source.collection_groups().len(), 2);
}

#[test]
fn each_group_has_its_header_before_its_items() {
    let _scope = test_scope();
    let list = Grouped::new(&[2, 2], |_, _| {});
    assert_eq!(list.headers(), vec![("G0-".to_string(), 0.0), ("G1-".to_string(), HEADER + 2.0 * ITEM)]);
    assert_eq!(list.y_of(0), HEADER);
    assert_eq!(list.y_of(1), HEADER + ITEM);
    assert_eq!(list.y_of(2), 2.0 * HEADER + 2.0 * ITEM);
    // An index is the index of an item.
    list.list.set_selected_index(2);
    assert_eq!(text(&list.list.selected_item()), "G1-0");
}

#[test]
fn a_large_grouped_list_stays_virtualized_with_the_right_pinned_header() {
    let _scope = test_scope();
    let counts = vec![50; 200];
    let list = Grouped::new(&counts, |_, _| {});
    assert!(list.panel.children().count() < 40, "{}", list.panel.children().count());
    // The group of 50 items and its header take 1030.
    list.scroll_to(1030.0 * 100.0 + 100.0);
    assert!(list.panel.children().count() < 60, "{}", list.panel.children().count());
    let viewport = list.panel.view_port().y;
    let first = list.panel.first_visible_index();
    let source = list.list.items_source().unwrap();
    let (group, _) = source.downcast_ref::<GroupedItemsView>().unwrap().group_of(first as usize).unwrap();
    let pinned = list.headers().into_iter().find(|(text, _)| *text == format!("G{group}-"));
    assert_eq!(pinned.map(|(_, y)| y), Some(viewport));
}

#[test]
fn the_next_header_pushes_the_pinned_one_away() {
    let _scope = test_scope();
    let list = Grouped::new(&[20, 20], |_, _| {});
    // Step by step: a jump places what it realizes at estimated positions until it is measured.
    for offset in (20..=400).step_by(20) {
        list.scroll_to(f64::from(offset));
        // The items keep their exact positions while the list scrolls over them.
        let first = list.panel.first_realized_index();
        assert_eq!(list.y_of(first), HEADER + f64::from(first) * ITEM, "at {offset}");
    }
    // The header of the second group, at 430, has reached the end of the pinned one and pushes it.
    list.scroll_to(430.0 - HEADER + 10.0);
    let headers = list.headers();
    assert_eq!(headers[0], ("G0-".to_string(), 430.0 - HEADER));
    assert_eq!(headers[1], ("G1-".to_string(), 430.0));
}

#[test]
fn without_sticky_headers_a_header_scrolls_with_its_group() {
    let _scope = test_scope();
    let list = Grouped::new(&[20, 20], |panel, _| panel.set_are_sticky_group_headers_enabled(false));
    list.scroll_to(100.0);
    assert!(list.headers().iter().all(|(text, _)| text != "G0-"), "{:?}", list.headers());
}

#[test]
fn an_empty_group_shows_its_header_unless_it_hides() {
    let _scope = test_scope();
    let list = Grouped::new(&[1, 0, 1], |_, _| {});
    let texts: Vec<String> = list.headers().into_iter().map(|(text, _)| text).collect();
    assert_eq!(texts, ["G0-", "G1-", "G2-"]);
    assert_eq!(list.y_of(1), 3.0 * HEADER + ITEM);

    let list = Grouped::new(&[1, 0, 1], |panel, style| {
        style.set_hides_if_empty(true);
        panel.invalidate_measure();
    });
    let texts: Vec<String> = list.headers().into_iter().map(|(text, _)| text).collect();
    assert_eq!(texts, ["G0-", "G2-"]);
    assert_eq!(list.y_of(1), 2.0 * HEADER + ITEM);
}

#[test]
fn the_padding_of_a_group_surrounds_it() {
    let _scope = test_scope();
    let list = Grouped::new(&[1, 1], |panel, _| panel.set_group_padding(Thickness::new(4.0, 6.0, 4.0, 8.0)));
    assert_eq!(list.headers()[0].1, 6.0);
    assert_eq!(list.y_of(0), 6.0 + HEADER);
    assert_eq!(list.panel.container_from_index(0).unwrap().bounds().x, 4.0);
    assert_eq!(list.y_of(1), 6.0 + HEADER + ITEM + 8.0 + 6.0 + HEADER);
}

#[test]
fn a_list_without_groups_has_no_headers_and_no_insets() {
    let _scope = test_scope();
    let list = ListView::new();
    list.set_template(list_view_template());
    list.set_height(200.0);
    list.container_prepared(|args| args.container().set_height(ITEM)).detach();
    list.set_items_source(Some(ItemsSource::from_strs(["a", "b"])));
    let root = TestRoot::with_child(list.clone());
    root.layout_manager().execute_initial_layout_pass();
    let panel = list.items_panel_root().unwrap().cast::<ItemsStackPanel>().unwrap();
    assert_eq!(panel.children().count(), 2);
    assert_eq!(panel.container_from_index(1).unwrap().bounds().y, ITEM);
    assert!(!list.is_grouping());
}

/// `CollectionViewSource.ItemsPath` (#1424): the items of each group are the value of the path of
/// the group, which is the header; the view follows the lists at the path.
#[test]
fn the_items_path_reads_the_items_of_each_group() {
    let _scope = test_scope();
    let group = |name: &str, items: &[&str]| {
        let list = Rc::new(FerroList::from_items(items.iter().map(|item| item.to_string())));
        let control = crate::ItemsControl::new();
        control.set_name(Some(name.to_string()));
        control.set_items_source(Some(ItemsSource::new(list.clone())));
        (crate::Control::boxed(control), list)
    };
    let (first, first_items) = group("First", &["a", "b"]);
    let (second, _) = group("Second", &["c"]);
    let source = CollectionViewSource::new();
    source.set_is_source_grouped(true);
    source.set_items_path("ItemsSource");
    source.set_source(Some(ItemsSource::from_items([Some(first.clone()), Some(second)])));
    let view = source.view().expect("a view");
    let texts = |view: &ItemsSource| -> Vec<String> { view.iter().filter_map(|item| string_of(&item?)).collect() };
    assert_eq!(texts(&view), ["a", "b", "c"]);
    let groups = source.collection_groups();
    assert_eq!(groups.len(), 2);
    assert!(Rc::ptr_eq(groups[0].group().as_ref().expect("the header"), &first), "the group is its header");
    // The view follows the list at the path.
    let changes = Rc::new(RefCell::new(0));
    let counted = changes.clone();
    let token = view.list().add_collection_changed(Rc::new(move |_| *counted.borrow_mut() += 1));
    first_items.add("a2".to_string());
    assert_eq!(texts(&view), ["a", "b", "a2", "c"]);
    assert_eq!(*changes.borrow(), 1);
    if let Some(token) = token {
        view.list().remove_collection_changed(token);
    }
    // Without the path, each group is its own list: these groups are not lists.
    source.set_items_path("");
    assert!(texts(&source.view().expect("a view")).is_empty());
}
