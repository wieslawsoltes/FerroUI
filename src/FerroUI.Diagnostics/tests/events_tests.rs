//! Not from upstream: the upstream project has no tests. The tree of the routed events of the
//! events page and the rows of its TreeDataGrid (a tree view upstream), the recording of a fired
//! event with its route, the hot keys page and the selection of the tabs of the main view model
//! with the extension pages (an addition of the port).

use crate::diagnostics::models::EventChainLink;
use crate::diagnostics::view_models::{
    DevToolsPage, EventTreeNodeBase, EventsPageViewModel, HotKeyPageViewModel, MainViewModel, TreePageViewModel,
};
use crate::diagnostics::{DevToolsOptions, DevToolsViewKind, HotKeyConfiguration};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{InputElement, Key, KeyGesture, KeyModifiers};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs, RoutedEventRegistry, RoutingStrategies};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::templates::FuncControlTemplate;
use ferroui_controls::testing::{MockImplKind, MockWindowImpl, TestServices, UnitTestApplication};
use ferroui_controls::{Button, ContentControl, Control, StackPanel, TextBlock, TopLevel};
use ferroui_treedatagrid_core::models::IRows;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The names of the properties a view model raises its notification for.
fn record_property_changes(source: &dyn INotifyPropertyChanged) -> Rc<RefCell<Vec<String>>> {
    let raised = Rc::new(RefCell::new(Vec::new()));
    let sink = raised.clone();
    source.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));
    raised
}

/// The node of the owner named `name`.
fn owner_node(vm: &EventsPageViewModel, name: &str) -> Rc<EventTreeNodeBase> {
    vm.nodes().to_vec().into_iter().find(|node| node.text() == name).unwrap_or_else(|| panic!("the node of {name}"))
}

/// The node of `event` under its owner.
fn event_node(vm: &EventsPageViewModel, event: RoutedEvent) -> Rc<EventTreeNodeBase> {
    owner_node(vm, event.owner_type().name())
        .children()
        .expect("the children of an owner")
        .to_vec()
        .into_iter()
        .find(|node| node.as_event_tree_node().is_some_and(|node| node.event() == event))
        .unwrap_or_else(|| panic!("the node of {event}"))
}

/// Whether the handler of a link is `object`.
fn is_handler(link: &EventChainLink, object: &Ref<impl ferroui_base::ObjectType>) -> bool {
    ValueTypes::as_object(&*link.handler()).is_some_and(|handler| handler.ptr_eq(object))
}

/// A tree in a top level that is not a window: an embeddable root whose
/// content (presented by a content presenter) is a panel with a button.
fn top_level_with_button() -> (Ref<EmbeddableControlRoot>, Ref<StackPanel>, Ref<Button>) {
    let panel = StackPanel::new();
    let button = Button::new();
    panel.children().add(button.clone());

    let root = EmbeddableControlRoot::with_impl(MockWindowImpl::bare(MockImplKind::TopLevel));
    root.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let presenter = ContentPresenter::new();
        presenter.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        scope.register("PART_ContentPresenter", presenter.clone().upcast::<FerroObject>());
        presenter.upcast()
    })));
    root.set_content(Some(Control::boxed(panel.clone())));
    root.prepare();
    assert!(TopLevel::get_top_level(Some(&button)).is_some(), "the button is in the top level");

    (root, panel, button)
}

// --- the tree of the routed events ---------------------------------------------------------

#[test]
fn the_event_tree_groups_the_registered_routed_events_by_owner() {
    let _scope = Dispatcher::unit_test_scope();
    crate::register_types();
    // Routed events are registered by their accessors.
    let click = Button::click_event().as_routed_event();
    let key_down = InputElement::key_down_event().as_routed_event();
    let got_focus = InputElement::got_focus_event().as_routed_event();

    let vm = EventsPageViewModel::new(Weak::new());
    let nodes = vm.nodes().to_vec();

    // One node per owner type, ordered by the name of the type.
    let names: Vec<String> = nodes.iter().map(|node| node.text().to_string()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(sorted, names);
    assert!(nodes.iter().all(|node| node.as_event_owner_tree_node().is_some() && node.is_expanded()));

    // Every registered event once, under its owner, ordered by name.
    let registered = RoutedEventRegistry::instance().get_all_registered();
    let in_tree: usize = nodes.iter().map(|node| node.children().expect("children").count()).sum();
    assert_eq!(registered.len(), in_tree);
    for node in &nodes {
        let children = node.children().expect("children").to_vec();
        let child_names: Vec<String> = children.iter().map(|child| child.text().to_string()).collect();
        let mut sorted = child_names.clone();
        sorted.sort();
        assert_eq!(sorted, child_names);
        for child in children {
            let event = child.as_event_tree_node().expect("a node of an event").event();
            assert_eq!(node.text(), event.owner_type().name());
            assert_eq!(event.name(), child.text());
            assert!(child.parent().is_some_and(|parent| Rc::ptr_eq(&parent, node)));
            assert!(child.children().is_none());
        }
    }

    // The default events are enabled, the others are not; an owner with both is undetermined.
    assert_eq!(Some(true), event_node(&vm, click).is_enabled());
    assert_eq!(Some(true), event_node(&vm, key_down).is_enabled());
    assert_eq!(Some(false), event_node(&vm, got_focus).is_enabled());
    assert_eq!(None, owner_node(&vm, "InputElement").is_enabled());
}

#[test]
fn enabling_an_owner_enables_its_events_and_an_event_updates_its_owner() {
    let _scope = Dispatcher::unit_test_scope();
    crate::register_types();
    let key_down = InputElement::key_down_event().as_routed_event();
    let _ = InputElement::got_focus_event();

    let vm = EventsPageViewModel::new(Weak::new());
    let owner = owner_node(&vm, "InputElement");

    vm.disable_all();
    assert_eq!(Some(false), owner.is_enabled());
    assert!(owner.children().expect("children").to_vec().iter().all(|child| child.is_enabled() == Some(false)));

    owner.set_is_enabled(Some(true));
    assert!(owner.children().expect("children").to_vec().iter().all(|child| child.is_enabled() == Some(true)));

    event_node(&vm, key_down).set_is_enabled(Some(false));
    assert_eq!(None, owner.is_enabled());

    vm.enable_default();
    assert_eq!(Some(true), event_node(&vm, key_down).is_enabled());
}

#[test]
fn the_filter_shows_the_matching_events_and_their_owners() {
    let _scope = Dispatcher::unit_test_scope();
    crate::register_types();
    let click = Button::click_event().as_routed_event();
    let key_down = InputElement::key_down_event().as_routed_event();
    let got_focus = InputElement::got_focus_event().as_routed_event();

    let vm = EventsPageViewModel::new(Weak::new());
    vm.events_filter().set_filter_string("KeyDown".to_string());

    assert!(owner_node(&vm, "InputElement").is_visible());
    assert!(event_node(&vm, key_down).is_visible());
    assert!(!event_node(&vm, got_focus).is_visible());
    assert!(!owner_node(&vm, "Button").is_visible());
    assert!(!event_node(&vm, click).is_visible());

    // A visible event is selected by its type, a hidden one is not.
    let raised = record_property_changes(&*vm);
    vm.select_event_by_type(click);
    assert!(vm.selected_node().is_none());
    vm.select_event_by_type(key_down);
    assert!(vm.selected_node().is_some_and(|node| Rc::ptr_eq(&node, &event_node(&vm, key_down))));
    assert_eq!(vec!["SelectedNode".to_string()], *raised.borrow());

    vm.events_filter().set_filter_string(String::new());
    assert!(owner_node(&vm, "Button").is_visible());
    assert!(event_node(&vm, got_focus).is_visible());
}

/// The nodes of the rows of the TreeDataGrid of the events page, in order.
fn row_models(vm: &EventsPageViewModel) -> Vec<Rc<EventTreeNodeBase>> {
    vm.nodes_source().typed_rows().to_vec_typed().iter().map(|row| row.model().clone()).collect()
}

fn has_row(models: &[Rc<EventTreeNodeBase>], node: &Rc<EventTreeNodeBase>) -> bool {
    models.iter().any(|model| Rc::ptr_eq(model, node))
}

/// The number of nodes of the tree: the owners and their events.
fn node_count(vm: &EventsPageViewModel) -> usize {
    vm.nodes().to_vec().iter().map(|node| 1 + node.children().expect("children").count()).sum()
}

#[test]
fn the_check_boxes_of_the_rows_of_the_events_tree_enable_events_and_owners_in_three_states() {
    let _scope = Dispatcher::unit_test_scope();
    crate::register_types();
    let key_down = InputElement::key_down_event().as_routed_event();
    let _ = InputElement::got_focus_event();

    let vm = EventsPageViewModel::new(Weak::new());
    let rows = vm.nodes_source().typed_rows();

    // The owners are expanded: a row for every owner and every event, an owner before its events.
    let models = row_models(&vm);
    assert_eq!(node_count(&vm), models.len());
    let owner = owner_node(&vm, "InputElement");
    let event = event_node(&vm, key_down);
    let owner_row = models.iter().position(|model| Rc::ptr_eq(model, &owner)).expect("the row of the owner");
    let event_row = models.iter().position(|model| Rc::ptr_eq(model, &event)).expect("the row of the event");
    assert!(owner_row < event_row);

    // The check box of a row is two-way with `IsEnabled` of its node: unchecking the owner
    // unchecks its events, checking it checks them.
    let owner_model = rows.to_vec_typed()[owner_row].model().clone();
    owner_model.set_is_enabled(Some(false));
    assert!(owner.children().expect("children").to_vec().iter().all(|child| child.is_enabled() == Some(false)));
    owner_model.set_is_enabled(Some(true));
    assert!(owner.children().expect("children").to_vec().iter().all(|child| child.is_enabled() == Some(true)));

    // Unchecking one event leaves the owner undetermined; checking it again checks the owner.
    let event_model = rows.to_vec_typed()[event_row].model().clone();
    event_model.set_is_enabled(Some(false));
    assert_eq!(Some(false), event.is_enabled());
    assert_eq!(None, owner.is_enabled());
    event_model.set_is_enabled(Some(true));
    assert_eq!(Some(true), owner.is_enabled());

    // The rows are the same rows: the changes of the check boxes do not make them again.
    assert_eq!(models.len(), IRows::count(&*rows));
    assert!(Rc::ptr_eq(rows.to_vec_typed()[event_row].model(), &event));

    // Collapsing an owner removes the rows of its events.
    owner.set_is_expanded(false);
    let count = owner.children().expect("children").count();
    assert_eq!(models.len() - count, IRows::count(&*rows));
    assert!(!has_row(&row_models(&vm), &event));
}

#[test]
fn the_filter_removes_the_rows_of_the_hidden_events_and_keeps_the_selection() {
    let _scope = Dispatcher::unit_test_scope();
    crate::register_types();
    let click = Button::click_event().as_routed_event();
    let key_down = InputElement::key_down_event().as_routed_event();
    let got_focus = InputElement::got_focus_event().as_routed_event();

    let vm = EventsPageViewModel::new(Weak::new());
    let rows = vm.nodes_source().typed_rows();
    assert_eq!(node_count(&vm), IRows::count(&*rows));

    vm.events_filter().set_filter_string("KeyDown".to_string());

    // The rows are the visible nodes.
    let models = row_models(&vm);
    assert!(models.iter().all(|model| model.is_visible()));
    assert!(has_row(&models, &owner_node(&vm, "InputElement")));
    assert!(has_row(&models, &event_node(&vm, key_down)));
    assert!(!has_row(&models, &event_node(&vm, got_focus)));
    assert!(!has_row(&models, &owner_node(&vm, "Button")));
    assert!(!has_row(&models, &event_node(&vm, click)));

    // Selecting an event by its type selects its row.
    vm.select_event_by_type(key_down);
    let selection = vm.nodes_source().row_selection().expect("the selection of the rows");
    let key_down_node = event_node(&vm, key_down);
    assert!(selection.base().selected_item().is_some_and(|selected| Rc::ptr_eq(&selected, &key_down_node)));
    let row_index = vm.selected_row_index();
    assert!(row_index >= 0);
    assert!(Rc::ptr_eq(rows.to_vec_typed()[row_index as usize].model(), &key_down_node));

    // Clearing the filter shows every row again; the event stays selected.
    vm.events_filter().set_filter_string(String::new());
    assert_eq!(node_count(&vm), IRows::count(&*rows));
    assert!(vm.selected_node().is_some_and(|selected| Rc::ptr_eq(&selected, &key_down_node)));
    assert!(selection.base().selected_item().is_some_and(|selected| Rc::ptr_eq(&selected, &key_down_node)));

    // Selecting a row selects its node.
    let owner = owner_node(&vm, "Button");
    let owner_index = vm.index_path_of(&owner).expect("the row of the owner");
    selection.base().set_selected_index(owner_index);
    assert!(vm.selected_node().is_some_and(|selected| Rc::ptr_eq(&selected, &owner)));
}

// --- the recording of fired events ---------------------------------------------------------

#[test]
fn an_enabled_event_is_recorded_with_the_chain_of_its_route() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (root, panel, button) = top_level_with_button();

    // Click is one of the default events.
    let vm = EventsPageViewModel::new(Weak::new());
    let _ = button.add_handler(Button::click_event(), |_, e: &RoutedEventArgs| e.set_handled(true));

    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));

    let recorded = vm.recorded_events().to_vec();
    assert_eq!(1, recorded.len());
    let fired = &recorded[0];
    assert_eq!(Button::click_event().as_routed_event(), fired.event());

    // The chain of the bubble route: the button first, then its ancestors up to the root.
    let chain = fired.event_chain().to_vec();
    assert!(chain.len() >= 3, "{} links", chain.len());
    assert!(is_handler(&chain[0], &button));
    assert!(is_handler(&chain[1], &panel));
    assert!(chain.iter().any(|link| is_handler(link, &root)));
    assert!(chain.iter().all(|link| link.route() == RoutingStrategies::BUBBLE && !link.begins_new_route()));
    assert!(Rc::ptr_eq(&fired.originator(), &chain[0]));

    // The handler of the button handled the event: the next link is the first handled one.
    assert!(!chain[0].handled());
    assert!(chain[1..].iter().all(|link| link.handled()));
    assert!(fired.is_handled());
    assert!(fired.handled_by().is_some_and(|handled_by| Rc::ptr_eq(&handled_by, &chain[1])));
    assert_eq!(
        format!("Click on Button;{}strategies: Bubble; handled by: StackPanel", if cfg!(windows) { "\r\n" } else { "\n" }),
        fired.display_text()
    );

    // A second raise is another event; a disabled event is not recorded.
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(2, vm.recorded_events().count());
    event_node(&vm, Button::click_event().as_routed_event()).set_is_enabled(Some(false));
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(2, vm.recorded_events().count());

    vm.clear();
    assert_eq!(0, vm.recorded_events().count());
}

#[test]
fn a_tunnelling_and_bubbling_event_begins_a_new_route_in_its_chain() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (_root, _panel, button) = top_level_with_button();
    let event = RoutedEvent::register_with_owner::<RoutedEventArgs>(
        "DiagnosticsTestTunnelBubble",
        RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        Button::TYPE,
    )
    .as_routed_event();

    let vm = EventsPageViewModel::new(Weak::new());
    let node = event_node(&vm, event);
    assert_eq!(Some(false), node.is_enabled());
    node.set_is_enabled(Some(true));

    button.raise_event(&RoutedEventArgs::with_event(&event));

    let recorded = vm.recorded_events().to_vec();
    assert_eq!(1, recorded.len());
    let chain = recorded[0].event_chain().to_vec();
    let tunnel = chain.iter().take_while(|link| link.route() == RoutingStrategies::TUNNEL).count();
    assert!(tunnel >= 3);
    assert_eq!(2 * tunnel, chain.len());
    // The tunnel route ends at the button, the bubble route starts there.
    assert!(is_handler(&chain[tunnel - 1], &button));
    assert!(is_handler(&chain[tunnel], &button));
    assert!(chain[tunnel].begins_new_route());
    assert_eq!(1, chain.iter().filter(|link| link.begins_new_route()).count());
    assert!(!recorded[0].is_handled());
    assert!(recorded[0].handled_by().is_none());
}

#[test]
fn an_event_of_an_element_in_no_top_level_is_not_recorded() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let panel = StackPanel::new();
    let button = Button::new();
    panel.children().add(button.clone());

    let vm = EventsPageViewModel::new(Weak::new());
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));

    // As upstream, an element that is in no top level counts as an element of DevTools.
    assert_eq!(0, vm.recorded_events().count());
}

// --- the hot keys page ---------------------------------------------------------------------

#[test]
fn the_hot_keys_page_lists_the_hot_keys_of_the_options() {
    let _scope = Dispatcher::unit_test_scope();
    crate::register_types();
    let vm = HotKeyPageViewModel::new();
    assert!(vm.hot_key_descriptions().is_none());
    let raised = record_property_changes(&*vm);

    vm.set_options(&DevToolsOptions::new());

    assert_eq!(vec!["HotKeyDescriptions".to_string()], *raised.borrow());
    let descriptions = vm.hot_key_descriptions().expect("the hot keys").to_vec();
    let rows: Vec<(String, String)> =
        descriptions.iter().map(|description| (description.gesture(), description.brief_description())).collect();
    assert_eq!(
        vec![
            ("F12".to_string(), "Launch DevTools".to_string()),
            ("Alt+S".to_string(), "Freeze Value Frames".to_string()),
            ("Alt+D".to_string(), "Unfreeze Value Frames".to_string()),
            ("Ctrl+Shift".to_string(), "Inspect Control Under Pointer".to_string()),
            ("Ctrl+Alt+F".to_string(), "Toggle Popup Freeze".to_string()),
            ("F8".to_string(), "Screenshot Selected Control".to_string()),
        ],
        rows
    );
    assert_eq!(
        Some("Prevents visible Popups from closing so they can be inspected".to_string()),
        descriptions[4].detailed_description()
    );

    // Other gestures.
    let options = DevToolsOptions::new()
        .with_gesture(KeyGesture::new(Key::F11, KeyModifiers::SHIFT))
        .with_hot_keys(
            HotKeyConfiguration::new().with_inspect_hovered_control(KeyGesture::new(Key::None, KeyModifiers::ALT)),
        );
    vm.set_options(&options);
    let descriptions = vm.hot_key_descriptions().expect("the hot keys").to_vec();
    assert_eq!("Shift+F11", descriptions[0].gesture());
    assert_eq!("Alt", descriptions[3].gesture());
}

// --- the main view model -------------------------------------------------------------------

/// Whether the content of the main view model is `page`.
fn content_is<T: PartialEq + 'static>(vm: &MainViewModel, page: &Rc<T>) -> bool {
    from_markup_value::<Rc<T>>(&vm.content()).is_some_and(|content| Rc::ptr_eq(&content, page))
}

#[test]
fn the_selected_tab_selects_the_content_of_the_main_view_model() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let root = StackPanel::new();

    let vm = MainViewModel::new(root.clone().upcast());
    // The combined tree is the first tab and the default one (an addition of the port).
    assert_eq!(MainViewModel::COMBINED_TREE_TAB, vm.selected_tab());
    assert!(content_is::<TreePageViewModel>(&vm, &vm.combined_tree()));
    assert!(vm.root().ptr_eq(&root));

    let raised = record_property_changes(&*vm);
    vm.set_selected_tab(MainViewModel::LOGICAL_TREE_TAB);
    assert!(content_is::<TreePageViewModel>(&vm, &vm.logical_tree()));
    assert_eq!(vec!["Content".to_string(), "SelectedTab".to_string()], *raised.borrow());
    vm.set_selected_tab(MainViewModel::VISUAL_TREE_TAB);
    assert!(content_is::<TreePageViewModel>(&vm, &vm.visual_tree()));

    vm.set_selected_tab(MainViewModel::EVENTS_TAB);
    assert!(content_is::<EventsPageViewModel>(&vm, &vm.events()));
    vm.set_selected_tab(MainViewModel::HOT_KEYS_TAB);
    assert!(content_is::<HotKeyPageViewModel>(&vm, &vm.hot_keys()));
    vm.set_selected_tab(0);
    vm.show_hot_keys();
    assert_eq!(MainViewModel::HOT_KEYS_TAB, vm.selected_tab());
    assert!(content_is::<HotKeyPageViewModel>(&vm, &vm.hot_keys()));

    // Any other tab is the combined tree; without registered pages there are no extension tabs.
    vm.set_selected_tab(MainViewModel::FIRST_EXTENSION_TAB);
    assert!(content_is::<TreePageViewModel>(&vm, &vm.combined_tree()));
    vm.set_selected_tab(-1);
    assert!(content_is::<TreePageViewModel>(&vm, &vm.combined_tree()));
    assert!(vm.extension_pages().is_empty());

    // The options select the launch view and set the hot keys page.
    vm.set_options(&DevToolsOptions::new().with_launch_view(DevToolsViewKind::Events));
    assert_eq!(MainViewModel::EVENTS_TAB, vm.selected_tab());
    assert!(content_is::<EventsPageViewModel>(&vm, &vm.events()));
    assert!(vm.show_implemented_interfaces());
    assert_eq!(6, vm.hot_keys().hot_key_descriptions().expect("the hot keys").count());

    // The toggles of the menu.
    assert!(!vm.show_details_property_type());
    vm.toggle_show_details_property_type(None);
    assert!(vm.show_details_property_type());
    assert!(vm.should_visualize_margin_padding());
    vm.toggle_visualize_margin_padding();
    assert!(!vm.should_visualize_margin_padding());
    // No tree page is the content: no screenshot.
    assert!(!vm.can_shot(None));

    vm.dispose();
}

#[test]
fn registered_pages_are_tabs_after_the_tabs_of_the_main_view_model() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let created = Rc::new(Cell::new(0));
    let registration = MainViewModel::register_page(DevToolsPage::new("Components", {
        let created = created.clone();
        move |main: &Rc<MainViewModel>| {
            created.set(created.get() + 1);
            let text_block = TextBlock::new();
            text_block.set_text(Some(main.root().get_type().name()));
            text_block.upcast()
        }
    }));
    assert_eq!(vec!["Components".to_string()], titles(&MainViewModel::registered_pages()));

    let root = StackPanel::new();
    let vm = MainViewModel::new(root.clone().upcast());
    assert_eq!(vec!["Components".to_string()], titles(&vm.extension_pages()));
    // The view is created when the tab is first selected.
    assert_eq!(0, created.get());

    vm.set_selected_tab(MainViewModel::FIRST_EXTENSION_TAB);
    assert_eq!(MainViewModel::FIRST_EXTENSION_TAB, vm.selected_tab());
    let view = vm.content().as_ref().and_then(Control::from_boxed).expect("the view of the page");
    let text_block = view.cast::<TextBlock>().expect("the view the factory created");
    assert_eq!(Some("StackPanel".to_string()), text_block.text());
    assert_eq!(1, created.get());

    // The view is kept for the window.
    vm.set_selected_tab(0);
    vm.set_selected_tab(MainViewModel::FIRST_EXTENSION_TAB);
    let again = vm.content().as_ref().and_then(Control::from_boxed).expect("the view of the page");
    assert!(again.ptr_eq(&view));
    assert_eq!(1, created.get());

    // A tab after the last page is the combined tree.
    vm.set_selected_tab(MainViewModel::FIRST_EXTENSION_TAB + 1);
    assert!(content_is::<TreePageViewModel>(&vm, &vm.combined_tree()));

    // Removing the registration leaves the windows that are open; the next ones have no page.
    registration.dispose();
    assert!(MainViewModel::registered_pages().is_empty());
    assert_eq!(1, vm.extension_pages().len());
    let next = MainViewModel::new(root.upcast());
    assert!(next.extension_pages().is_empty());

    vm.dispose();
    next.dispose();
}

fn titles(pages: &[DevToolsPage]) -> Vec<String> {
    pages.iter().map(|page| page.title().to_string()).collect()
}

// --- the top level under the pointer (Ctrl+Shift) -----------------------------------------

#[test]
fn the_top_level_of_an_input_root_is_found_through_its_host() {
    crate::register_types();
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (root, _panel, _button) = top_level_with_button();

    // The root element of the input root of a top level is the host of its decorations, with the
    // top level as its child: the hot keys that need the hovered top level (Ctrl+Shift to inspect
    // the control under the pointer) find the top level through it.
    let input_root = root.input_root();
    assert!(
        input_root.try_root_element().and_then(|element| element.cast::<TopLevel>()).is_none(),
        "the root element is the host of the top level, not the top level"
    );
    let top_level = crate::diagnostics::views::top_level_of_root(&*input_root).expect("the top level of the root");
    assert!(top_level.ptr_eq(&root.clone().upcast::<TopLevel>()));
}
