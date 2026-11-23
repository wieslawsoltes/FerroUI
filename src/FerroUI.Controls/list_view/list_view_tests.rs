//! Tests of the list view (#443, LV-1): WinUI's selection modes on the flags of the base, the
//! containers and the panel, item clicks, selected ranges, the container events and the peers.

use super::*;
use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::automation::provider::IInvokeProvider;
use crate::presenters::{ItemsPresenter, ScrollContentPresenter};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{Control, ItemsControl, ItemsSource, Panel, ScrollViewer, SelectionMode};
use ferroui_base::data::TemplateBinding;
use ferroui_base::{Ref, StaticType as _};
use std::cell::RefCell;
use std::rc::Rc;

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let panel = Panel::new();
        panel.children().add(presenter.register_in_name_scope(&**scope));
        panel.upcast()
    })
}

/// WinUI's template of ListView: a scroll viewer around the items presenter.
fn list_view_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &TemplateBinding::new(property));
        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));
        scroll_viewer.register_in_name_scope(&**scope).upcast()
    }))
}

fn list_view(count: usize) -> (Ref<ListView>, Ref<TestRoot>) {
    let list = ListView::new();
    list.set_template(list_view_template());
    list.set_height(400.0);
    list.set_items_source(Some(ItemsSource::from_items((0..count).map(|index| boxed_str(&format!("Item {index}"))))));
    let root = TestRoot::with_child(list.clone());
    root.layout_manager().execute_initial_layout_pass();
    (list, root)
}

#[test]
fn the_selection_modes_set_the_flags_of_the_base() {
    let _scope = test_scope();
    let list = ListView::new();
    assert_eq!(list.selection_mode(), ListViewSelectionMode::Single);
    let base_mode = |list: &ListView| crate::primitives::SelectingItemsControl::selection_mode(list);
    assert_eq!(base_mode(&list), SelectionMode::SINGLE);
    list.set_selection_mode(ListViewSelectionMode::Multiple);
    assert_eq!(base_mode(&list), SelectionMode::MULTIPLE | SelectionMode::TOGGLE);
    list.set_selection_mode(ListViewSelectionMode::Extended);
    assert_eq!(base_mode(&list), SelectionMode::MULTIPLE);
}

#[test]
fn selection_mode_by_name_is_winui_s() {
    let _scope = test_scope();
    let property =
        ferroui_base::FerroPropertyRegistry::instance().find_registered(ListView::TYPE, "SelectionMode").unwrap();
    assert!(std::ptr::eq(property, ListViewBase::list_view_selection_mode_property().as_property()));
}

#[test]
fn the_containers_are_list_view_items_in_an_items_stack_panel() {
    let _scope = test_scope();
    let (list, _root) = list_view(5);
    assert!(list.items_panel_root().is_some_and(|panel| panel.downcast_ref::<ItemsStackPanel>().is_some()));
    let container = list.container_from_index(0).unwrap();
    assert!(container.downcast_ref::<ListViewItem>().is_some());
    let panel = list.items_panel_root().unwrap().cast::<ItemsStackPanel>().unwrap();
    assert_eq!(panel.first_cache_index(), 0);
    assert_eq!(panel.first_visible_index(), 0);
    assert!(panel.last_visible_index() >= 0);
}

#[test]
fn none_clears_the_selection() {
    let _scope = test_scope();
    let (list, _root) = list_view(3);
    list.set_selected_index(1);
    list.set_selection_mode(ListViewSelectionMode::None);
    assert_eq!(list.selected_index(), -1);
}

#[test]
fn item_clicks_carry_the_data_item() {
    let _scope = test_scope();
    let (list, _root) = list_view(3);
    let clicked = Rc::new(RefCell::new(Vec::new()));
    let log = clicked.clone();
    let _subscription = list.item_click(move |_, e| {
        log.borrow_mut().push(e.clicked_item().as_ref().and_then(string_of));
    });
    list.invoke_item(1);
    assert!(clicked.borrow().is_empty(), "item clicks are off by default");
    list.set_is_item_click_enabled(true);
    list.invoke_item(1);
    assert_eq!(*clicked.borrow(), vec![Some("Item 1".to_string())]);
}

#[test]
fn ranges_of_selected_items() {
    let _scope = test_scope();
    let (list, _root) = list_view(10);
    list.set_selection_mode(ListViewSelectionMode::Multiple);
    list.select_range(ItemIndexRange::new(2, 3));
    list.select_range(ItemIndexRange::new(7, 1));
    assert_eq!(list.selected_ranges(), vec![ItemIndexRange::new(2, 3), ItemIndexRange::new(7, 1)]);
    list.deselect_range(ItemIndexRange::new(3, 1));
    assert_eq!(
        list.selected_ranges(),
        vec![ItemIndexRange::new(2, 1), ItemIndexRange::new(4, 1), ItemIndexRange::new(7, 1)]
    );
    assert_eq!(ItemIndexRange::new(2, 3).last_index(), 4);
    list.select_all();
    assert_eq!(list.selected_ranges(), vec![ItemIndexRange::new(0, 10)]);
}

#[test]
fn the_container_events_are_raised() {
    let _scope = test_scope();
    let list = ListView::new();
    list.set_template(list_view_template());
    list.set_height(400.0);
    let changing = Rc::new(RefCell::new(Vec::new()));
    let log = changing.clone();
    let _changing = list.container_content_changing(move |_, e| {
        log.borrow_mut().push((e.item_index(), e.in_recycle_queue(), e.phase()));
    });
    let chosen = Rc::new(RefCell::new(0));
    let count = chosen.clone();
    let _choosing = list.choosing_item_container(move |_, e| {
        *count.borrow_mut() += 1;
        if e.item_index() == 0 {
            e.set_item_container(Some(ListViewItem::new().upcast()));
        }
    });
    list.set_items_source(Some(ItemsSource::from_items((0..2).map(|index| boxed_str(&format!("Item {index}"))))));
    let root = TestRoot::with_child(list.clone());
    root.layout_manager().execute_initial_layout_pass();
    assert_eq!(*chosen.borrow(), 2);
    assert!(changing.borrow().contains(&(0, false, 0)));
    assert!(changing.borrow().contains(&(1, false, 0)));
}

#[test]
fn the_peers_are_a_list_and_list_items() {
    let _scope = test_scope();
    let (list, _root) = list_view(2);
    let peer = ControlAutomationPeer::create_peer_for_element(&list.clone().upcast());
    assert_eq!(peer.get_automation_control_type(), AutomationControlType::List);
    let container = list.container_from_index(0).unwrap();
    let item_peer = ControlAutomationPeer::create_peer_for_element(&container);
    assert_eq!(item_peer.get_automation_control_type(), AutomationControlType::ListItem);
    let clicked = Rc::new(RefCell::new(0));
    let count = clicked.clone();
    let _subscription = list.item_click(move |_, _| *count.borrow_mut() += 1);
    list.set_is_item_click_enabled(true);
    let invoke = item_peer.get_provider::<dyn IInvokeProvider>().expect("the Invoke pattern");
    invoke.invoke().unwrap();
    assert_eq!(*clicked.borrow(), 1);
}

#[test]
fn a_grid_view_has_grid_view_items_in_an_items_wrap_grid() {
    let _scope = test_scope();
    let grid = GridView::new();
    grid.set_template(list_view_template());
    grid.set_width(300.0);
    grid.set_height(300.0);
    grid.set_items_source(Some(ItemsSource::from_items((0..5).map(|index| boxed_str(&format!("Item {index}"))))));
    let root = TestRoot::with_child(grid.clone());
    root.layout_manager().execute_initial_layout_pass();
    let panel = grid.items_panel_root().and_then(|panel| panel.cast::<ItemsWrapGrid>()).expect("an ItemsWrapGrid");
    assert_eq!(panel.orientation(), ferroui_base::layout::Orientation::Horizontal);
    assert!(grid.container_from_index(4).is_some_and(|container| container.downcast_ref::<GridViewItem>().is_some()));
    let peer = ControlAutomationPeer::create_peer_for_element(&grid.clone().upcast());
    assert_eq!(peer.get_automation_control_type(), AutomationControlType::List);
    let item_peer = ControlAutomationPeer::create_peer_for_element(&grid.container_from_index(0).unwrap());
    assert_eq!(item_peer.get_automation_control_type(), AutomationControlType::ListItem);
}

#[test]
fn the_wrap_grid_lays_out_cells_of_one_size() {
    let _scope = test_scope();
    let panel = ItemsWrapGrid::new();
    panel.set_orientation(ferroui_base::layout::Orientation::Horizontal);
    panel.set_item_width(100.0);
    panel.set_item_height(50.0);
    let children: Vec<Ref<Control>> = (0..5).map(|_| crate::Border::new().upcast()).collect();
    for child in &children {
        panel.children().add(child.clone());
    }
    panel.measure(ferroui_base::Size::new(350.0, f64::INFINITY));
    // Three cells fit in a row: two rows.
    assert_eq!(panel.desired_size(), ferroui_base::Size::new(300.0, 100.0));
    panel.arrange(ferroui_base::Rect::new(0.0, 0.0, 350.0, 100.0));
    assert_eq!(children[3].bounds(), ferroui_base::Rect::new(0.0, 50.0, 100.0, 50.0));
    // At most two in a row: three rows.
    panel.set_maximum_rows_or_columns(2);
    panel.measure(ferroui_base::Size::new(350.0, f64::INFINITY));
    assert_eq!(panel.desired_size(), ferroui_base::Size::new(200.0, 150.0));
}
