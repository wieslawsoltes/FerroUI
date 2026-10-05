//! Port of the reference `MenuItemAutomationPeerTests`.

use super::{ControlAutomationPeer, MenuItemAutomationPeer};
use crate::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, IToggleProvider, ProviderAdapter, ToggleState,
};
use crate::automation::{
    AutomationPropertyChangedEventArgs, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
    TogglePatternIdentifiers,
};
use crate::platform::{IPopupImpl, ITopLevelImpl, IWindowImpl};
use crate::test_command::TestCommand;
use crate::test_support::{boxed_str, test_scope};
use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{Button, ContextMenu, Control, Menu, MenuFlyout, MenuItem, MenuItemToggleType, Window};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{AnyValue, BoxedValue, FerroObject, Ref};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn create_submenu_item() -> Ref<MenuItem> {
    let item = MenuItem::new();
    add_item(item.items(), &MenuItem::new());
    item
}

fn menu_item_with_header(header: &str) -> Ref<MenuItem> {
    let item = MenuItem::new();
    item.set_header(boxed_str(header));
    item
}

fn menu_item_with_toggle_type(toggle_type: MenuItemToggleType) -> Ref<MenuItem> {
    let item = MenuItem::new();
    item.set_toggle_type(toggle_type);
    item
}

fn add_item(items: impl std::borrow::Borrow<crate::ItemCollection>, item: &Ref<MenuItem>) {
    items.borrow().add(Some(Control::boxed(item)));
}

/// A menu with one top-level item, which has `children` as its items.
fn menu_with_top_level(children: &[&Ref<MenuItem>]) -> (Ref<Menu>, Ref<MenuItem>) {
    let top_level = menu_item_with_header("Top");
    for child in children {
        add_item(top_level.items(), child);
    }
    let menu = Menu::new();
    add_item(menu.items(), &top_level);
    (menu, top_level)
}

fn styled_window() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::styled_window())
}

fn create_nestable_popup(parent: Rc<dyn ITopLevelImpl>) -> Rc<dyn IPopupImpl> {
    let popup = MockWindowingPlatform::create_popup_mock(parent);
    let weak = Rc::downgrade(&popup);
    popup.setup_create_popup(move |_| {
        let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
        Some(create_nestable_popup(parent))
    });
    popup
}

fn create_window(menu: &Ref<Menu>) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(menu)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    window
}

fn get_expand_collapse_provider(menu_item: &Ref<MenuItem>) -> Rc<dyn IExpandCollapseProvider> {
    let provider =
        ControlAutomationPeer::create_peer_for_element(menu_item).get_provider::<dyn IExpandCollapseProvider>();
    provider.expect("the expand/collapse provider")
}

fn get_invoke_provider(menu_item: &Ref<MenuItem>) -> Rc<dyn IInvokeProvider> {
    let provider = ControlAutomationPeer::create_peer_for_element(menu_item).get_provider::<dyn IInvokeProvider>();
    provider.expect("the invoke provider")
}

fn get_provider(menu_item: &Ref<MenuItem>) -> Rc<dyn IToggleProvider> {
    let provider = ControlAutomationPeer::create_peer_for_element(menu_item).get_provider::<dyn IToggleProvider>();
    provider.expect("the toggle provider")
}

/// The reference casts the peer to the contract: the peer class implements
/// it whether or not the peer currently exposes it as a provider.
fn cast_to_expand_collapse_provider(menu_item: &Ref<MenuItem>) -> Rc<dyn IExpandCollapseProvider> {
    let peer = ControlAutomationPeer::create_peer_for_element(menu_item)
        .cast::<MenuItemAutomationPeer>()
        .expect("a menu item automation peer");
    ProviderAdapter::as_expand_collapse_provider(peer)
}

fn value_of<T: Clone + 'static>(value: Option<&BoxedValue>) -> Option<T> {
    let value: &dyn AnyValue = &**value?;
    value.downcast_ref::<T>().cloned()
}

#[test]
fn toggle_provider_is_not_exposed_when_toggle_type_none() {
    let _scope = test_scope();
    // The peer does not keep its control alive.
    let menu_item = MenuItem::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);

    assert!(peer.get_provider::<dyn IToggleProvider>().is_none());
}

#[test]
fn toggle_provider_is_exposed_for_checkable_items() {
    for toggle_type in [MenuItemToggleType::CheckBox, MenuItemToggleType::Radio] {
        let _scope = test_scope();
        let menu_item = menu_item_with_toggle_type(toggle_type);
        let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);

        assert!(peer.get_provider::<dyn IToggleProvider>().is_some());
    }
}

#[test]
fn toggle_state_reflects_is_checked() {
    let _scope = test_scope();
    let menu_item = menu_item_with_toggle_type(MenuItemToggleType::CheckBox);
    let provider = get_provider(&menu_item);

    assert_eq!(ToggleState::Off, provider.toggle_state());
    menu_item.set_is_checked(true);
    assert_eq!(ToggleState::On, provider.toggle_state());
}

#[test]
fn toggle_flips_check_box_item() {
    let _scope = test_scope();
    let menu_item = menu_item_with_toggle_type(MenuItemToggleType::CheckBox);
    let provider = get_provider(&menu_item);

    provider.toggle().unwrap();
    assert!(menu_item.is_checked());

    provider.toggle().unwrap();
    assert!(!menu_item.is_checked());
}

#[test]
fn toggle_checks_but_does_not_uncheck_radio_item() {
    let _scope = test_scope();
    let menu_item = menu_item_with_toggle_type(MenuItemToggleType::Radio);
    let provider = get_provider(&menu_item);

    provider.toggle().unwrap();
    assert!(menu_item.is_checked());

    provider.toggle().unwrap();
    assert!(menu_item.is_checked());
}

#[test]
fn toggle_raises_toggle_state_property_changed() {
    let _scope = test_scope();
    let menu_item = menu_item_with_toggle_type(MenuItemToggleType::CheckBox);
    let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);
    let provider = get_provider(&menu_item);

    let raised = Rc::new(Cell::new(0));
    let count = raised.clone();
    peer.property_changed(move |e| {
        if std::ptr::eq(e.property(), TogglePatternIdentifiers::toggle_state_property()) {
            assert_eq!(Some(ToggleState::Off), value_of::<ToggleState>(e.old_value()));
            assert_eq!(Some(ToggleState::On), value_of::<ToggleState>(e.new_value()));
            count.set(count.get() + 1);
        }
    });

    provider.toggle().unwrap();

    assert_eq!(1, raised.get());
}

#[test]
fn leaf_item_exposes_invoke_but_not_expand_collapse() {
    let _scope = test_scope();
    // The peer does not keep its control alive.
    let menu_item = MenuItem::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);

    assert!(peer.get_provider::<dyn IInvokeProvider>().is_some());
    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_none());
}

#[test]
fn submenu_item_exposes_expand_collapse_but_not_invoke() {
    let _scope = test_scope();
    let menu_item = create_submenu_item();
    let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);

    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
    assert!(peer.get_provider::<dyn IInvokeProvider>().is_none());
}

#[test]
fn providers_follow_items_being_added_and_removed() {
    let _scope = test_scope();
    let menu_item = MenuItem::new();
    let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);

    add_item(menu_item.items(), &MenuItem::new());

    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
    assert!(peer.get_provider::<dyn IInvokeProvider>().is_none());

    menu_item.items().clear();

    assert!(peer.get_provider::<dyn IInvokeProvider>().is_some());
    assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_none());
}

#[test]
fn expand_collapse_state_is_leaf_node_for_leaf_item() {
    let _scope = test_scope();
    let menu_item = MenuItem::new();
    let peer = cast_to_expand_collapse_provider(&menu_item);

    assert_eq!(ExpandCollapseState::LeafNode, peer.expand_collapse_state());
    assert!(!peer.shows_menu());
}

#[test]
fn expand_collapse_state_reflects_is_sub_menu_open() {
    let _scope = test_scope();
    let menu_item = create_submenu_item();
    let provider = get_expand_collapse_provider(&menu_item);

    assert_eq!(ExpandCollapseState::Collapsed, provider.expand_collapse_state());
    assert!(provider.shows_menu());

    menu_item.set_is_sub_menu_open(true);
    assert_eq!(ExpandCollapseState::Expanded, provider.expand_collapse_state());

    menu_item.set_is_sub_menu_open(false);
    assert_eq!(ExpandCollapseState::Collapsed, provider.expand_collapse_state());
}

#[test]
fn expand_and_collapse_set_is_sub_menu_open() {
    let _scope = test_scope();
    let menu_item = create_submenu_item();
    let provider = get_expand_collapse_provider(&menu_item);

    provider.expand().unwrap();
    assert!(menu_item.is_sub_menu_open());

    provider.collapse().unwrap();
    assert!(!menu_item.is_sub_menu_open());
}

#[test]
fn expand_collapse_raises_property_changed() {
    let _scope = test_scope();
    let menu_item = create_submenu_item();
    let peer = ControlAutomationPeer::create_peer_for_element(&menu_item);
    let provider = get_expand_collapse_provider(&menu_item);
    let raised: Rc<RefCell<Vec<AutomationPropertyChangedEventArgs>>> = Rc::new(RefCell::new(Vec::new()));

    let sink = raised.clone();
    peer.property_changed(move |e| {
        if std::ptr::eq(e.property(), ExpandCollapsePatternIdentifiers::expand_collapse_state_property()) {
            sink.borrow_mut().push(e.clone());
        }
    });

    provider.expand().unwrap();
    provider.collapse().unwrap();

    let raised = raised.borrow();
    assert_eq!(2, raised.len());
    assert_eq!(Some(ExpandCollapseState::Collapsed), value_of(raised[0].old_value()));
    assert_eq!(Some(ExpandCollapseState::Expanded), value_of(raised[0].new_value()));
    assert_eq!(Some(ExpandCollapseState::Expanded), value_of(raised[1].old_value()));
    assert_eq!(Some(ExpandCollapseState::Collapsed), value_of(raised[1].new_value()));
}

#[test]
fn expand_throws_for_leaf_item() {
    let _scope = test_scope();
    let menu_item = MenuItem::new();
    let peer = cast_to_expand_collapse_provider(&menu_item);

    assert!(catch_unwind(AssertUnwindSafe(|| peer.expand())).is_err());
}

#[test]
fn expand_throws_when_disabled() {
    let _scope = test_scope();
    let menu_item = create_submenu_item();
    menu_item.set_is_enabled(false);
    let provider = get_expand_collapse_provider(&menu_item);

    assert!(provider.expand().is_err());
    assert!(!menu_item.is_sub_menu_open());
}

#[test]
fn collapse_throws_for_leaf_item() {
    let _scope = test_scope();
    let menu_item = MenuItem::new();
    let peer = cast_to_expand_collapse_provider(&menu_item);

    assert!(catch_unwind(AssertUnwindSafe(|| peer.collapse())).is_err());
}

#[test]
fn collapse_throws_when_disabled() {
    let _scope = test_scope();
    let menu_item = create_submenu_item();
    menu_item.set_is_enabled(false);
    menu_item.set_is_sub_menu_open(true);
    let provider = get_expand_collapse_provider(&menu_item);

    assert!(provider.collapse().is_err());
    assert!(menu_item.is_sub_menu_open());
}

#[test]
fn invoke_raises_click() {
    let _scope = test_scope();
    let menu_item = MenuItem::new();
    let provider = get_invoke_provider(&menu_item);
    let clicked = Rc::new(Cell::new(0));

    let count = clicked.clone();
    menu_item.click(move |_, _| count.set(count.get() + 1));
    provider.invoke().unwrap();

    assert_eq!(1, clicked.get());
}

#[test]
fn invoke_executes_command() {
    let _scope = test_scope();
    let executed = Rc::new(Cell::new(0));
    let menu_item = MenuItem::new();
    let sum = executed.clone();
    let command = TestCommand::with_can_execute_and_execute(
        |_| true,
        move |p| sum.set(sum.get() + value_of::<i32>(p).expect("an integer parameter")),
    );
    menu_item.set_command(command.as_command());
    menu_item.set_command_parameter(Some(Rc::new(5_i32) as BoxedValue));
    let provider = get_invoke_provider(&menu_item);

    provider.invoke().unwrap();

    assert_eq!(5, executed.get());
}

#[test]
fn invoke_throws_when_disabled() {
    let _scope = test_scope();
    let menu_item = MenuItem::new();
    menu_item.set_is_enabled(false);
    let provider = get_invoke_provider(&menu_item);
    let clicked = Rc::new(Cell::new(0));

    let count = clicked.clone();
    menu_item.click(move |_, _| count.set(count.get() + 1));

    assert!(provider.invoke().is_err());
    assert_eq!(0, clicked.get());
}

#[test]
fn invoke_throws_when_command_cannot_execute() {
    let _scope = test_scope();
    let executed = Rc::new(Cell::new(0));
    let menu_item = MenuItem::new();
    let count = executed.clone();
    let command = TestCommand::with_can_execute_and_execute(|_| false, move |_| count.set(count.get() + 1));
    menu_item.set_command(command.as_command());
    let provider = get_invoke_provider(&menu_item);

    assert!(provider.invoke().is_err());
    assert_eq!(0, executed.get());
}

#[test]
fn expand_opens_top_level_menu_item() {
    let _app = styled_window();

    let child = menu_item_with_header("Child");
    let (menu, top_level) = menu_with_top_level(&[&child]);
    create_window(&menu);

    get_expand_collapse_provider(&top_level).expand().unwrap();

    assert!(menu.is_open());
    assert!(top_level.is_sub_menu_open());
    assert!(child.is_attached_to_visual_tree());
}

#[test]
fn collapse_closes_menu_for_top_level_menu_item() {
    let _app = styled_window();

    let (menu, top_level) = menu_with_top_level(&[&menu_item_with_header("Child")]);
    create_window(&menu);
    let provider = get_expand_collapse_provider(&top_level);

    provider.expand().unwrap();
    provider.collapse().unwrap();

    assert!(!top_level.is_sub_menu_open());
    assert!(!menu.is_open());
}

#[test]
fn collapse_does_nothing_for_collapsed_top_level_menu_item() {
    let _app = styled_window();

    let first = menu_item_with_header("First");
    add_item(first.items(), &menu_item_with_header("Child"));
    let second = menu_item_with_header("Second");
    add_item(second.items(), &menu_item_with_header("Child"));
    let menu = Menu::new();
    add_item(menu.items(), &first);
    add_item(menu.items(), &second);
    create_window(&menu);

    get_expand_collapse_provider(&second).expand().unwrap();
    get_expand_collapse_provider(&first).collapse().unwrap();

    assert!(second.is_sub_menu_open());
    assert!(menu.is_open());
}

#[test]
fn collapse_leaves_menu_open_for_nested_menu_item() {
    // The nested submenu opens a popup from within a popup.
    fn create_window_impl() -> Rc<dyn IWindowImpl> {
        let window_impl = MockWindowingPlatform::create_window_mock();
        let weak = Rc::downgrade(&window_impl);
        window_impl.setup_create_popup(move |_| {
            let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
            Some(create_nestable_popup(parent))
        });
        window_impl
    }
    let _app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_windowing_platform(MockWindowingPlatform::with_window_impl(create_window_impl)),
    );

    let nested = menu_item_with_header("Nested");
    add_item(nested.items(), &menu_item_with_header("Child"));
    let (menu, top_level) = menu_with_top_level(&[&nested]);
    create_window(&menu);
    let nested_provider = get_expand_collapse_provider(&nested);

    get_expand_collapse_provider(&top_level).expand().unwrap();
    nested_provider.expand().unwrap();
    nested_provider.collapse().unwrap();

    assert!(!nested.is_sub_menu_open());
    assert!(top_level.is_sub_menu_open());
    assert!(menu.is_open());
}

#[test]
fn invoke_in_menu_bubbles_click_to_menu() {
    let _app = styled_window();

    let child = menu_item_with_header("Child");
    let (menu, top_level) = menu_with_top_level(&[&child]);
    create_window(&menu);
    let clicked: Rc<RefCell<Vec<Option<Ref<FerroObject>>>>> = Rc::new(RefCell::new(Vec::new()));

    let sink = clicked.clone();
    menu.add_handler(MenuItem::click_event(), move |_, e| sink.borrow_mut().push(e.source()));
    get_expand_collapse_provider(&top_level).expand().unwrap();
    get_invoke_provider(&child).invoke().unwrap();

    assert_eq!(vec![Some(child.clone().upcast::<FerroObject>())], *clicked.borrow());
}

#[test]
fn invoke_in_menu_closes_menu() {
    let _app = styled_window();

    let child = menu_item_with_header("Child");
    let (menu, top_level) = menu_with_top_level(&[&child]);
    create_window(&menu);

    get_expand_collapse_provider(&top_level).expand().unwrap();
    assert!(menu.is_open());

    get_invoke_provider(&child).invoke().unwrap();

    assert!(!top_level.is_sub_menu_open());
    assert!(!menu.is_open());
}

#[test]
fn invoke_in_menu_respects_stays_open_on_click() {
    let _app = styled_window();

    let child = menu_item_with_header("Child");
    child.set_stays_open_on_click(true);
    let (menu, top_level) = menu_with_top_level(&[&child]);
    create_window(&menu);

    get_expand_collapse_provider(&top_level).expand().unwrap();
    get_invoke_provider(&child).invoke().unwrap();

    assert!(top_level.is_sub_menu_open());
    assert!(menu.is_open());
}

#[test]
fn invoke_in_menu_toggles_check_box_item() {
    let _app = styled_window();

    let child = menu_item_with_header("Child");
    child.set_toggle_type(MenuItemToggleType::CheckBox);
    let (menu, top_level) = menu_with_top_level(&[&child]);
    create_window(&menu);

    get_expand_collapse_provider(&top_level).expand().unwrap();
    get_invoke_provider(&child).invoke().unwrap();

    assert!(child.is_checked());
}

#[test]
fn invoke_in_context_menu_closes_context_menu() {
    let _app = styled_window();

    let child = menu_item_with_header("Child");
    let context_menu = ContextMenu::new();
    add_item(context_menu.items(), &child);
    let window = Window::new();
    window.set_context_menu(&context_menu);

    window.show();
    context_menu.open();
    assert!(context_menu.is_open());

    get_invoke_provider(&child).invoke().unwrap();

    assert!(!context_menu.is_open());
}

#[test]
fn invoke_in_menu_flyout_closes_flyout() {
    let _app = styled_window();

    let child = menu_item_with_header("Child");
    let flyout = MenuFlyout::new();
    add_item(flyout.items(), &child);
    let button = Button::new();
    button.set_context_flyout(&flyout);
    let window = Window::new();
    window.set_content(Some(Control::boxed(&button)));

    window.show();
    flyout.show_at(&button);
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
    assert!(flyout.is_open());

    get_invoke_provider(&child).invoke().unwrap();

    assert!(!flyout.is_open());
}
