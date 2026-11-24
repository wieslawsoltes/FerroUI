//! Tests of the tab view (#418): the tabs of `TabItems` and `TabItemsSource`, the selection and the
//! content, closing, adding, the keyboard, the close button states, the tab widths and the peers.

use super::*;
use crate::automation::peers::{AutomationControlType, ControlAutomationPeer};
use crate::presenters::{ContentPresenter, ItemsPresenter, ScrollContentPresenter};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{Button, ColumnDefinitions, Control, Grid, ItemsControl, ItemsSource, Panel, ScrollViewer, StackPanel};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{BoxedValue, Ref};
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

/// The strip of WinUI's template: a scroll viewer around the items presenter, in a row.
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

/// A tab: the header presenter and the close button.
fn tab_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, scope| {
        let header = ContentPresenter::new();
        header.set_name(Some("ContentPresenter".to_string()));
        let close = Button::new();
        close.set_name(Some("CloseButton".to_string()));
        let row = StackPanel::new();
        row.children().add(header.register_in_name_scope(&**scope).upcast::<Control>());
        row.children().add(close.register_in_name_scope(&**scope).upcast::<Control>());
        row.upcast()
    }))
}

/// WinUI's template of TabView, reduced to its parts: the columns of the strip, the list of tabs,
/// the add button and the content of the selected tab.
fn tab_view_template() -> Option<Rc<dyn IControlTemplate>> {
    Some(FuncControlTemplate::new(|_, scope| {
        let strip = Grid::new();
        strip.set_name(Some("TabContainerGrid".to_string()));
        strip.set_column_definitions(ColumnDefinitions::parse("Auto,Auto,Auto,*").unwrap());
        let columns = strip.column_definitions();
        for (index, name) in ["LeftContentColumn", "TabColumn", "AddButtonColumn", "RightContentColumn"].iter().enumerate() {
            scope.register(name, columns.get(index).upcast());
        }
        let list = TabViewListView::new();
        list.set_name(Some("TabListView".to_string()));
        list.set_template(list_view_template());
        Grid::set_column(&list, 1);
        let add = Button::new();
        add.set_name(Some("AddButton".to_string()));
        Grid::set_column(&add, 2);
        strip.children().add(list.register_in_name_scope(&**scope).upcast::<Control>());
        strip.children().add(add.register_in_name_scope(&**scope).upcast::<Control>());
        let content = ContentPresenter::new();
        content.set_name(Some("TabContentPresenter".to_string()));
        let root = StackPanel::new();
        root.children().add(strip.register_in_name_scope(&**scope).upcast::<Control>());
        root.children().add(content.register_in_name_scope(&**scope).upcast::<Control>());
        root.upcast()
    }))
}

fn tab(header: &str, content: &str) -> Ref<TabViewItem> {
    let tab = TabViewItem::new();
    tab.set_template(tab_template());
    tab.set_header(boxed_str(header));
    tab.set_content(boxed_str(content));
    tab
}

fn tab_view(headers: &[&str]) -> (Ref<TabView>, Ref<TestRoot>) {
    let view = TabView::new();
    view.set_template(tab_view_template());
    view.set_width(400.0);
    for header in headers {
        view.tab_items().add(Some(Control::boxed(tab(header, &format!("{header} content")))));
    }
    let root = TestRoot::with_child(view.clone());
    root.layout_manager().execute_initial_layout_pass();
    (view, root)
}

fn shown_content(view: &TabView) -> Option<String> {
    let presenter = view.get_template_children().into_iter().find_map(|child| {
        child.get_self_and_visual_descendants().find_map(|visual| {
            visual.cast::<ContentPresenter>().filter(|presenter| presenter.name().as_deref() == Some("TabContentPresenter"))
        })
    })?;
    presenter.content().as_ref().and_then(string_of)
}

fn tab_at(view: &TabView, index: i32) -> Ref<TabViewItem> {
    view.container_from_index(index).and_then(|container| container.cast::<TabViewItem>()).expect("a tab")
}

fn press(view: &TabView, key: Key, modifiers: KeyModifiers) -> bool {
    let mut args = KeyEventArgs::new();
    args.key = key;
    args.key_modifiers = modifiers;
    args.set_routed_event(Some(InputElement::key_down_event()));
    view.raise_event(&args);
    args.handled()
}

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

#[test]
fn the_tabs_of_tab_items_are_the_tabs_themselves_and_the_first_is_shown() {
    let _scope = test_scope();
    let (view, _root) = tab_view(&["One", "Two", "Three"]);
    assert_eq!(view.selected_index(), 0);
    assert_eq!(string_of(tab_at(&view, 2).header().as_ref().unwrap()).as_deref(), Some("Three"));
    assert_eq!(shown_content(&view).as_deref(), Some("One content"));
}

#[test]
fn selecting_a_tab_shows_its_content_and_raises_selection_changed() {
    let _scope = test_scope();
    let (view, root) = tab_view(&["One", "Two"]);
    let changes = Rc::new(RefCell::new(0));
    let counter = changes.clone();
    view.selection_changed(move |_, _| *counter.borrow_mut() += 1).detach();
    view.set_selected_index(1);
    root.layout_manager().execute_layout_pass();
    assert_eq!(shown_content(&view).as_deref(), Some("Two content"));
    assert!(tab_at(&view, 1).is_selected());
    assert_eq!(*changes.borrow(), 1);
}

#[test]
fn the_close_button_asks_the_tab_view_and_the_tab_to_close_it() {
    let _scope = test_scope();
    let (view, _root) = tab_view(&["One", "Two"]);
    let requested = Rc::new(RefCell::new(Vec::new()));
    let sink = requested.clone();
    view.tab_close_requested(move |_, args| sink.borrow_mut().push(("view", args.tab()))).detach();
    let second = tab_at(&view, 1);
    let sink = requested.clone();
    second.close_requested(move |_, args| sink.borrow_mut().push(("tab", args.tab()))).detach();
    click(&second.close_button().expect("the close button"));
    let requested = requested.borrow();
    assert_eq!(requested.len(), 2);
    assert_eq!(requested[0], ("view", second.clone()));
    assert_eq!(requested[1], ("tab", second.clone()));
    // The application closes the tab: the tab view does not remove it itself.
    assert_eq!(view.tab_items().count(), 2);
}

#[test]
fn removing_the_selected_tab_selects_the_next_one() {
    let _scope = test_scope();
    let (view, root) = tab_view(&["One", "Two", "Three"]);
    view.set_selected_index(1);
    root.layout_manager().execute_layout_pass();
    view.tab_items().remove_at(1);
    root.layout_manager().execute_layout_pass();
    assert_eq!(view.selected_index(), 1);
    assert_eq!(string_of(tab_at(&view, 1).header().as_ref().unwrap()).as_deref(), Some("Three"));
}

#[test]
fn ctrl_tab_cycles_the_selection_and_ctrl_f4_asks_to_close_the_selected_tab() {
    let _scope = test_scope();
    let (view, root) = tab_view(&["One", "Two", "Three"]);
    assert!(press(&view, Key::Tab, KeyModifiers::CONTROL));
    assert_eq!(view.selected_index(), 1);
    assert!(press(&view, Key::Tab, KeyModifiers::CONTROL | KeyModifiers::SHIFT));
    assert!(press(&view, Key::Tab, KeyModifiers::CONTROL | KeyModifiers::SHIFT));
    assert_eq!(view.selected_index(), 2, "around the start");
    root.layout_manager().execute_layout_pass();
    let closed = Rc::new(RefCell::new(None));
    let sink = closed.clone();
    view.tab_close_requested(move |_, args| *sink.borrow_mut() = Some(args.tab())).detach();
    assert!(press(&view, Key::F4, KeyModifiers::CONTROL));
    assert_eq!(closed.borrow().clone(), Some(tab_at(&view, 2)));
    tab_at(&view, 2).set_is_closable(false);
    assert!(!press(&view, Key::F4, KeyModifiers::CONTROL), "a tab that cannot close");
}

#[test]
fn the_close_button_state_follows_is_closable_and_the_overlay_mode() {
    let _scope = test_scope();
    let (view, _root) = tab_view(&["One", "Two"]);
    let (first, second) = (tab_at(&view, 0), tab_at(&view, 1));
    let collapsed = |tab: &TabViewItem| tab.pseudo_classes().contains(":close-button-collapsed");
    assert!(!collapsed(&first) && !collapsed(&second));
    view.set_close_button_overlay_mode(TabViewCloseButtonOverlayMode::OnPointerOver);
    assert!(!collapsed(&first), "the selected tab shows its close button");
    assert!(collapsed(&second));
    view.set_close_button_overlay_mode(TabViewCloseButtonOverlayMode::Always);
    second.set_is_closable(false);
    assert!(collapsed(&second));
}

#[test]
fn compact_tabs_that_are_not_selected_are_compact() {
    let _scope = test_scope();
    let (view, _root) = tab_view(&["One", "Two"]);
    view.set_tab_width_mode(TabViewWidthMode::Compact);
    assert!(!tab_at(&view, 0).pseudo_classes().contains(":compact"));
    assert!(tab_at(&view, 1).pseudo_classes().contains(":compact"));
}

#[test]
fn the_tabs_of_a_source_are_made_for_its_items() {
    let _scope = test_scope();
    let view = TabView::new();
    view.set_template(tab_view_template());
    view.set_width(400.0);
    view.set_tab_items_source(Some(ItemsSource::from_strs(["a", "b"])));
    let root = TestRoot::with_child(view.clone());
    root.layout_manager().execute_initial_layout_pass();
    let closed = Rc::new(RefCell::new(None::<Option<BoxedValue>>));
    let sink = closed.clone();
    view.tab_close_requested(move |_, args| *sink.borrow_mut() = Some(args.item())).detach();
    let second = tab_at(&view, 1);
    second.set_template(tab_template());
    root.layout_manager().execute_layout_pass();
    click(&second.close_button().expect("the close button"));
    let item = closed.borrow().clone().flatten();
    assert_eq!(item.as_ref().and_then(string_of).as_deref(), Some("b"), "the item of the source");
}

#[test]
fn the_add_button_raises_add_tab_button_click() {
    let _scope = test_scope();
    let (view, _root) = tab_view(&["One"]);
    let clicks = Rc::new(RefCell::new(0));
    let counter = clicks.clone();
    view.add_tab_button_click(move |_| *counter.borrow_mut() += 1).detach();
    let add = view
        .get_template_children()
        .into_iter()
        .find_map(|child| {
            child
                .get_self_and_visual_descendants()
                .find_map(|visual| visual.cast::<Button>().filter(|button| button.name().as_deref() == Some("AddButton")))
        })
        .expect("the add button");
    click(&add);
    assert_eq!(*clicks.borrow(), 1);
}

#[test]
fn equal_tabs_share_the_width_of_the_strip_within_their_limits() {
    let _scope = test_scope();
    let (view, _root) = tab_view(&["One", "Two", "Three"]);
    // The strip is 400 wide and the other columns are empty: 400 / 3 is inside 48 and 200.
    let width = tab_at(&view, 0).width();
    assert!((width - 400.0 / 3.0).abs() < 1.0, "{width}");
    let (wide, _root) = tab_view(&["One"]);
    assert_eq!(tab_at(&wide, 0).width(), 200.0, "the maximum width");
}

#[test]
fn the_peers_are_a_tab_and_its_tab_items() {
    let _scope = test_scope();
    let (view, _root) = tab_view(&["One"]);
    let peer = ControlAutomationPeer::create_peer_for_element(&view);
    assert_eq!(peer.get_automation_control_type(), AutomationControlType::Tab);
    let tab_peer = ControlAutomationPeer::create_peer_for_element(&tab_at(&view, 0));
    assert_eq!(tab_peer.get_automation_control_type(), AutomationControlType::TabItem);
    assert_eq!(tab_peer.get_name().as_str(), "One");
}
