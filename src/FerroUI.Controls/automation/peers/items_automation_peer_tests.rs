//! Tests of the peers of the items controls, their items and the scroll
//! viewer. NOT PORTS: the reference has no unit tests for these peers;
//! these cover the wiring of the port.

use super::{
    AutomationControlType, AutomationPeer, ComboBoxAutomationPeer, ControlAutomationPeer, ItemsControlAutomationPeer,
    ListBoxAutomationPeer, ListItemAutomationPeer, ScrollViewerAutomationPeer, TreeViewAutomationPeer,
    TreeViewItemAutomationPeer,
};
use crate::automation::provider::{
    IExpandCollapseProvider, IScrollProvider, ISelectionItemProvider, ISelectionProvider, IValueProvider,
};
use crate::automation::{
    AccessibilityView, AutomationProperties, ElementNotEnabledException, IsOffscreenBehavior,
    ScrollPatternIdentifiers, SelectionPatternIdentifiers,
};
use crate::presenters::{ItemsPresenter, ScrollContentPresenter};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::{
    ComboBox, ComboBoxItem, ContextMenu, Control, ItemsControl, ItemsSource, ListBox, ListBoxItem, Menu, MenuItem,
    ScrollViewer, SelectionMode, TabControl, TabItem, TreeView, TreeViewItem,
};
use ferroui_base::data::{BindingMode, IndexerBinding};
use ferroui_base::{Ref, TypeInfo};
use std::cell::Cell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn create_peer(control: &Control) -> Ref<AutomationPeer> {
    ControlAutomationPeer::create_peer_for_element(control)
}

fn is_type(peer: &Ref<AutomationPeer>, type_: &'static TypeInfo) -> bool {
    std::ptr::eq(peer.get_type(), type_)
}

fn scroll_viewer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ScrollViewer>(|_, scope| {
        let presenter = ScrollContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        presenter.register_in_name_scope(&**scope).upcast()
    })
}

fn list_box_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<ListBox>(|parent, scope| {
        let presenter = ItemsPresenter::new();
        presenter.set_name(Some("PART_ItemsPresenter".to_string()));
        let property = ItemsControl::items_panel_property().as_property();
        presenter.bind_binding(property, &IndexerBinding::new(parent.clone().upcast(), property, BindingMode::OneWay));

        let scroll_viewer = ScrollViewer::new();
        scroll_viewer.set_name(Some("PART_ScrollViewer".to_string()));
        scroll_viewer.set_template(Some(scroll_viewer_template()));
        scroll_viewer.set_content(Some(Control::boxed(presenter.register_in_name_scope(&**scope))));
        scroll_viewer.register_in_name_scope(&**scope).upcast()
    })
}

/// A list box with three items, laid out in a root.
fn list_box() -> (Ref<TestRoot>, Ref<ListBox>) {
    let target = ListBox::new();
    target.set_template(Some(list_box_template()));
    target.set_items_source(Some(ItemsSource::from_strs(["a", "b", "c"])));
    target.set_width(100.0);
    target.set_height(100.0);
    let root = TestRoot::new();
    root.set_child(target.clone());
    root.execute_initial_layout_pass();
    (root, target)
}

fn item_peer(list_box: &Ref<ListBox>, index: i32) -> Ref<AutomationPeer> {
    create_peer(&list_box.container_from_index(index).expect("a realized container"))
}

fn selection_item(peer: &Ref<AutomationPeer>) -> Rc<dyn ISelectionItemProvider> {
    peer.get_provider::<dyn ISelectionItemProvider>().expect("the selection item provider")
}

#[test]
fn controls_create_their_peers() {
    let _scope = test_scope();

    let items_control = ItemsControl::new();
    let peer = create_peer(&items_control);
    assert!(is_type(&peer, ItemsControlAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::List, peer.get_automation_control_type());

    let list_box = ListBox::new();
    let peer = create_peer(&list_box);
    assert!(is_type(&peer, ListBoxAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::List, peer.get_automation_control_type());

    let list_box_item = ListBoxItem::new();
    let peer = create_peer(&list_box_item);
    assert!(is_type(&peer, ListItemAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::ListItem, peer.get_automation_control_type());
    assert!(peer.is_content_element());
    assert!(peer.is_control_element());

    let combo_box = ComboBox::new();
    let peer = create_peer(&combo_box);
    assert!(is_type(&peer, ComboBoxAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::ComboBox, peer.get_automation_control_type());

    let tree_view = TreeView::new();
    let peer = create_peer(&tree_view);
    assert!(is_type(&peer, TreeViewAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::Tree, peer.get_automation_control_type());

    let tree_view_item = TreeViewItem::new();
    let peer = create_peer(&tree_view_item);
    assert!(is_type(&peer, TreeViewItemAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::TreeItem, peer.get_automation_control_type());

    let scroll_viewer = ScrollViewer::new();
    let peer = create_peer(&scroll_viewer);
    assert!(is_type(&peer, ScrollViewerAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::ScrollViewer, peer.get_automation_control_type());
    assert!(!peer.is_content_element());
    assert!(peer.is_control_element());
}

#[test]
fn classes_override_the_defaults_of_the_automation_properties() {
    let _scope = test_scope();

    let tab_item = TabItem::new();
    let peer = create_peer(&tab_item);
    assert!(is_type(&peer, ListItemAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::TabItem, peer.get_automation_control_type());
    assert_eq!(IsOffscreenBehavior::FromClip, AutomationProperties::get_is_offscreen_behavior(&tab_item));

    let tab_control = TabControl::new();
    let peer = create_peer(&tab_control);
    assert!(is_type(&peer, ItemsControlAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::Tab, peer.get_automation_control_type());

    let combo_box_item = ComboBoxItem::new();
    let peer = create_peer(&combo_box_item);
    assert!(is_type(&peer, ListItemAutomationPeer::TYPE));
    assert_eq!(AutomationControlType::ComboBoxItem, peer.get_automation_control_type());
    assert_eq!(IsOffscreenBehavior::FromClip, AutomationProperties::get_is_offscreen_behavior(&combo_box_item));

    let menu = Menu::new();
    assert_eq!(AutomationControlType::Menu, create_peer(&menu).get_automation_control_type());
    assert_eq!(AccessibilityView::Control, AutomationProperties::get_accessibility_view(&menu));

    let context_menu = ContextMenu::new();
    assert_eq!(AutomationControlType::Menu, create_peer(&context_menu).get_automation_control_type());
    assert_eq!(AccessibilityView::Control, AutomationProperties::get_accessibility_view(&context_menu));

    let list_box_item = ListBoxItem::new();
    assert_eq!(IsOffscreenBehavior::FromClip, AutomationProperties::get_is_offscreen_behavior(&list_box_item));
    let tree_view_item = TreeViewItem::new();
    assert_eq!(IsOffscreenBehavior::FromClip, AutomationProperties::get_is_offscreen_behavior(&tree_view_item));
    let menu_item = MenuItem::new();
    assert_eq!(IsOffscreenBehavior::FromClip, AutomationProperties::get_is_offscreen_behavior(&menu_item));
}

#[test]
fn list_box_peer_provides_the_selection() {
    let _scope = test_scope();
    let (_root, target) = list_box();
    let peer = create_peer(&target);
    let provider = peer.get_provider::<dyn ISelectionProvider>().expect("the selection provider");

    assert!(!provider.can_select_multiple());
    assert!(!provider.is_selection_required());
    assert!(provider.get_selection().is_empty());

    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    peer.property_changed(move |e| {
        if std::ptr::eq(e.property(), SelectionPatternIdentifiers::selection_property()) {
            assert!(e.old_value().is_none());
            assert!(e.new_value().is_none());
            count.set(count.get() + 1);
        }
    });

    target.set_selected_index(1);

    assert_eq!(1, raised.get());
    assert_eq!(vec![item_peer(&target, 1)], provider.get_selection());

    target.set_selection_mode(SelectionMode::MULTIPLE | SelectionMode::ALWAYS_SELECTED);

    assert!(provider.can_select_multiple());
    assert!(provider.is_selection_required());
}

#[test]
fn list_item_peer_selects_its_item() {
    let _scope = test_scope();
    let (_root, target) = list_box();
    target.set_selection_mode(SelectionMode::MULTIPLE);
    let peer = create_peer(&target);
    let first = selection_item(&item_peer(&target, 0));
    let second = selection_item(&item_peer(&target, 1));

    assert!(!first.is_selected());
    assert!(first.selection_container().is_some_and(|container| container.peer() == peer));

    first.select().unwrap();
    assert!(first.is_selected());
    assert_eq!(0, target.selected_index());

    second.add_to_selection().unwrap();
    assert!(first.is_selected());
    assert!(second.is_selected());

    first.remove_from_selection().unwrap();
    assert!(!first.is_selected());
    assert!(second.is_selected());

    second.select().unwrap();
    assert_eq!(1, target.selected_index());

    target.set_is_enabled(false);
    assert_eq!(Err(ElementNotEnabledException::new()), first.select());
    assert!(first.add_to_selection().is_err());
    assert!(first.remove_from_selection().is_err());
}

#[test]
fn items_control_peer_reports_no_scrolling_without_a_scroll_provider() {
    let _scope = test_scope();
    let (_root, target) = list_box();
    let provider = create_peer(&target).get_provider::<dyn IScrollProvider>().expect("the scroll provider");

    assert!(!provider.horizontally_scrollable());
    assert!(!provider.vertically_scrollable());
    assert_eq!(-1.0, provider.horizontal_scroll_percent());
    assert_eq!(-1.0, provider.vertical_scroll_percent());
    assert_eq!(0.0, provider.horizontal_view_size());
    assert_eq!(0.0, provider.vertical_view_size());
    assert!(provider.set_scroll_percent(50.0, 50.0).is_ok());
}

#[test]
fn scroll_viewer_peer_scrolls_its_owner() {
    let _scope = test_scope();
    let content = crate::Border::new();
    content.set_width(100.0);
    content.set_height(400.0);
    let target = ScrollViewer::new();
    target.set_template(Some(scroll_viewer_template()));
    target.set_content(Some(Control::boxed(content)));
    target.set_width(100.0);
    target.set_height(100.0);
    let root = TestRoot::new();
    root.set_child(target.clone());
    root.execute_initial_layout_pass();

    let peer = create_peer(&target).cast::<ScrollViewerAutomationPeer>().expect("a scroll viewer peer");
    let provider = peer.get_provider::<dyn IScrollProvider>().expect("the scroll provider");

    assert!(peer.get_horizontal_scroll_bar_peer().is_none());
    assert!(peer.get_vertical_scroll_bar_peer().is_none());
    assert!(!provider.horizontally_scrollable());
    assert!(provider.vertically_scrollable());
    assert_eq!(ScrollPatternIdentifiers::NO_SCROLL, provider.horizontal_scroll_percent());
    assert_eq!(0.0, provider.vertical_scroll_percent());
    assert_eq!(100.0, provider.horizontal_view_size());
    assert_eq!(25.0, provider.vertical_view_size());

    provider.set_scroll_percent(ScrollPatternIdentifiers::NO_SCROLL, 50.0).unwrap();

    assert_eq!(150.0, target.offset().y);
    assert_eq!(50.0, provider.vertical_scroll_percent());

    // A direction that cannot scroll, and a percentage out of range.
    assert!(catch_unwind(AssertUnwindSafe(|| provider.set_scroll_percent(50.0, 50.0))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| provider.set_scroll_percent(ScrollPatternIdentifiers::NO_SCROLL, 101.0)))
        .is_err());

    target.set_is_enabled(false);
    assert!(provider.set_scroll_percent(ScrollPatternIdentifiers::NO_SCROLL, 0.0).is_err());
}

#[test]
fn scroll_viewer_of_a_template_is_not_a_control_element() {
    let _scope = test_scope();
    let (_root, target) = list_box();
    let scroll_viewer = target.get_direct_value(ListBox::scroll_property()).expect("the scroll viewer of the template");
    let scroll_viewer = scroll_viewer.cast::<ScrollViewer>().expect("a scroll viewer");

    assert!(!create_peer(&scroll_viewer).is_control_element());
}

#[test]
fn combo_box_peer_represents_the_selection_of_a_closed_drop_down() {
    let _scope = test_scope();
    let target = ComboBox::new();
    target.set_items_source(Some(ItemsSource::from_strs(["a", "b"])));
    let peer = create_peer(&target);
    let selection = peer.get_provider::<dyn ISelectionProvider>().expect("the selection provider");
    let value = peer.get_provider::<dyn IValueProvider>().expect("the value provider");
    let expand_collapse = peer.get_provider::<dyn IExpandCollapseProvider>().expect("the expand/collapse provider");

    assert!(expand_collapse.shows_menu());
    assert!(selection.get_selection().is_empty());
    assert_eq!(None, value.value());

    target.set_selected_index(1);

    let selected = selection.get_selection();
    assert_eq!(1, selected.len());
    assert_eq!("b", selected[0].get_name());
    assert_eq!("ComboBoxItem", selected[0].get_class_name());
    assert_eq!(AutomationControlType::ListItem, selected[0].get_automation_control_type());
    assert!(selected[0].get_parent() == Some(peer.clone()));
    assert!(selected[0].get_children().is_empty());
    assert!(selected[0].is_enabled());
    assert!(!selected[0].is_control_element());
    assert_eq!(Some("b".to_string()), value.value());

    // The same peer represents the next selected item.
    target.set_selected_index(0);

    let reselected = selection.get_selection();
    assert!(reselected[0] == selected[0]);
    assert_eq!("a", reselected[0].get_name());
}

#[test]
fn tree_view_item_peer_selects_its_item() {
    let _scope = test_scope();
    let target = TreeViewItem::new();
    let provider = selection_item(&create_peer(&target));

    assert!(!provider.is_selected());
    assert!(provider.selection_container().is_none());

    provider.add_to_selection().unwrap();
    assert!(target.is_selected());

    provider.remove_from_selection().unwrap();
    assert!(!target.is_selected());

    provider.select().unwrap();
    assert!(target.is_selected());

    target.set_is_enabled(false);
    assert!(provider.select().is_err());
}
