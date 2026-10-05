//! Tests of the native menu classes.
//!
//! NOT PORTS: the reference has no unit tests for these classes (its only
//! related tests are the ones of the automation peer of the native menu
//! bar, which are ported next to that peer). These tests were written for
//! this code base and cover the behaviours that are easy to get wrong.

use crate::platform::{INativeMenuExporter, ITopLevelNativeMenuExporter, IWindowImpl};
use crate::test_command::TestCommand;
use crate::test_support::string_of;
use crate::testing::{MockWindowImpl, MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{
    Control, Menu, MenuItem, NativeDock, NativeMenu, NativeMenuBar, NativeMenuItem, NativeMenuItemBase,
    NativeMenuItemSeparator, Separator, Window,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroObjectExtensions, Ref};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn panic_message(f: impl FnOnce()) -> String {
    let error = catch_unwind(AssertUnwindSafe(f)).expect_err("the call panics");
    error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|message| message.to_string()))
        .expect("a panic message")
}

/// An exporter that records the menus it is given.
struct FakeExporter {
    is_exported: Cell<bool>,
    menus: RefCell<Vec<Option<Ref<NativeMenu>>>>,
    handlers: RefCell<Vec<Rc<dyn Fn()>>>,
}

impl FakeExporter {
    fn new(is_exported: bool) -> Rc<Self> {
        Rc::new(Self { is_exported: Cell::new(is_exported), menus: RefCell::new(Vec::new()), handlers: RefCell::new(Vec::new()) })
    }

    fn set_is_exported(&self, value: bool) {
        self.is_exported.set(value);
        let handlers = self.handlers.borrow().clone();
        for handler in handlers {
            handler();
        }
    }
}

impl INativeMenuExporter for FakeExporter {
    fn set_native_menu(&self, menu: Option<Ref<NativeMenu>>) {
        self.menus.borrow_mut().push(menu);
    }
}

impl ITopLevelNativeMenuExporter for FakeExporter {
    fn is_native_menu_exported(&self) -> bool {
        self.is_exported.get()
    }

    fn on_is_native_menu_exported_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.handlers.borrow_mut().push(handler);
        Disposable::create(|| {})
    }
}

struct TestApplication {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

/// The styled window services; the windows have `exporter` as their native
/// menu exporter feature when one is given.
fn application(exporter: Option<Rc<FakeExporter>>) -> TestApplication {
    let create_window_impl = move || -> Rc<dyn IWindowImpl> {
        let window_impl: Rc<MockWindowImpl> = MockWindowingPlatform::create_window_mock();
        if let Some(exporter) = &exporter {
            let feature: Rc<dyn ITopLevelNativeMenuExporter> = exporter.clone();
            window_impl.setup_feature::<dyn ITopLevelNativeMenuExporter>(feature);
        }
        window_impl
    };
    let services = TestServices::styled_window()
        .with_windowing_platform(MockWindowingPlatform::with_window_impl(create_window_impl));
    let app = UnitTestApplication::start(services);
    TestApplication { _text: TextTestScope::new(), _app: app }
}

fn item(header: &str) -> Ref<NativeMenuItem> {
    NativeMenuItem::with_header(header)
}

#[test]
fn items_get_and_lose_their_parent_on_add_remove_and_clear() {
    let _scope = Dispatcher::unit_test_scope();
    let menu = NativeMenu::new();
    let (a, b, c) = (item("a"), item("b"), NativeMenuItemSeparator::new());

    menu.add(a.clone());
    menu.items().add(b.clone().upcast());
    menu.add(c.clone());

    assert_eq!(a.parent(), Some(menu.clone()));
    assert_eq!(b.parent(), Some(menu.clone()));
    assert_eq!(c.parent(), Some(menu.clone()));
    assert_eq!(c.header().as_deref(), Some("-"));
    assert_eq!(menu.iter().count(), 3);
    assert_eq!((&*menu).into_iter().next(), Some(a.clone().upcast::<NativeMenuItemBase>()));

    let a_base: Ref<NativeMenuItemBase> = a.clone().upcast();
    assert!(menu.items().remove(&a_base));
    assert_eq!(a.parent(), None);
    assert_eq!(b.parent(), Some(menu.clone()));

    // A clear is notified as a removal, so the remaining items lose their
    // parent too.
    menu.items().clear();
    assert_eq!(b.parent(), None);
    assert_eq!(c.parent(), None);

    // An item that left a menu can be added to another one.
    let other = NativeMenu::new();
    other.add(a.clone());
    assert_eq!(a.parent(), Some(other));
}

#[test]
fn parent_changes_are_notified() {
    let _scope = Dispatcher::unit_test_scope();
    let menu = NativeMenu::new();
    let a = item("a");
    let changes = Rc::new(Cell::new(0));
    let _subscription = a.get_property_changed_observable(NativeMenuItemBase::parent_property().as_property()).subscribe({
        let changes = changes.clone();
        move |_| changes.set(changes.get() + 1)
    });

    menu.add(a.clone());
    menu.items().clear();

    assert_eq!(changes.get(), 2);
}

#[test]
fn adding_an_item_that_already_has_a_parent_panics() {
    let _scope = Dispatcher::unit_test_scope();
    let (first, second) = (NativeMenu::new(), NativeMenu::new());
    let a = item("Open");
    first.add(a.clone());

    let message = panic_message(|| second.add(a.clone()));

    assert_eq!(
        message,
        "The menu item NativeMenuItem (Header = Open) already has a parent NativeMenu \
         while trying to add it as a child of NativeMenu."
    );
    assert_eq!(second.items().count(), 0);
    assert_eq!(a.parent(), Some(first.clone()));

    // The same menu rejects it as well.
    let message = panic_message(|| first.add(a.clone()));
    assert!(message.contains("already has a parent"));
    assert_eq!(first.items().count(), 1);
}

#[test]
fn a_menu_becomes_the_submenu_of_one_item_only() {
    let _scope = Dispatcher::unit_test_scope();
    let (first, second) = (item("first"), item("second"));
    let menu = NativeMenu::new();

    first.set_menu(Some(menu.clone()));
    assert_eq!(menu.parent(), Some(first.clone()));

    // Setting it again on its parent is fine.
    first.set_menu(Some(menu.clone()));

    let message = panic_message(|| second.set_menu(Some(menu.clone())));
    assert_eq!(message, "NativeMenu already has a parent");
    assert_eq!(second.menu(), None);
    assert_eq!(menu.parent(), Some(first));
}

#[test]
fn raise_clicked_raises_click_and_executes_the_command_that_can_execute() {
    let _scope = Dispatcher::unit_test_scope();
    let target = item("target");
    assert!(!target.has_click_handlers());

    let clicks = Rc::new(Cell::new(0));
    let subscription = target.click({
        let clicks = clicks.clone();
        move |_| clicks.set(clicks.get() + 1)
    });
    assert!(target.has_click_handlers());

    let can_execute = Rc::new(Cell::new(true));
    let executed = Rc::new(RefCell::new(Vec::new()));
    let command = TestCommand::with_can_execute_and_execute(
        {
            let can_execute = can_execute.clone();
            move |_| can_execute.get()
        },
        {
            let executed = executed.clone();
            move |parameter| executed.borrow_mut().push(parameter.and_then(string_of))
        },
    );
    let parameter: BoxedValue = Rc::new("parameter".to_string());
    target.set_command_parameter(Some(parameter));
    target.set_command(command.as_command());
    assert!(target.is_enabled());

    let bridge = target.to_exporter_events_bridge();
    bridge.raise_clicked();
    assert_eq!(clicks.get(), 1);
    assert_eq!(*executed.borrow(), vec![Some("parameter".to_string())]);

    // A command that cannot execute disables the item and is not executed;
    // the click event is still raised.
    can_execute.set(false);
    command.raise_can_execute_changed();
    assert!(!target.is_enabled());
    bridge.raise_clicked();
    assert_eq!(clicks.get(), 2);
    assert_eq!(executed.borrow().len(), 1);

    subscription.dispose();
    assert!(!target.has_click_handlers());

    // Removing the command unsubscribes from it and enables the item.
    assert_eq!(command.subscription_count(), 1);
    target.set_command(None);
    assert_eq!(command.subscription_count(), 0);
    assert!(target.is_enabled());
}

#[test]
fn a_command_does_not_keep_the_item_alive() {
    let _scope = Dispatcher::unit_test_scope();
    let command = TestCommand::new(true);
    let target = item("target");
    target.set_command(command.as_command());
    let weak = target.downgrade();

    drop(target);

    assert!(weak.upgrade().is_none());
    // The subscription went with the item: the command keeps no handler.
    assert_eq!(command.subscription_count(), 0);
    command.raise_can_execute_changed();
    assert_eq!(command.subscription_count(), 0);
}

#[test]
fn raise_clicked_reads_the_command_again_after_can_execute() {
    let _scope = Dispatcher::unit_test_scope();
    let target = item("target");
    let executed_second = Rc::new(Cell::new(false));
    let second = TestCommand::with_can_execute_and_execute(|_| true, {
        let executed_second = executed_second.clone();
        move |_| executed_second.set(true)
    });
    // "Can execute" of the first command replaces the command of the item.
    let first = TestCommand::with_can_execute({
        let (target, second) = (target.downgrade(), second.clone());
        move |_| {
            if let Some(target) = target.upgrade() {
                target.set_command(second.as_command());
            }
            true
        }
    });
    target.set_command(first.as_command());

    target.to_exporter_events_bridge().raise_clicked();

    assert!(executed_second.get());
}

#[test]
fn menu_events_are_raised_through_the_bridge() {
    let _scope = Dispatcher::unit_test_scope();
    let menu = NativeMenu::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let record = |name: &'static str| {
        let log = log.clone();
        move |_: &NativeMenu| log.borrow_mut().push(name)
    };
    let _needs_update = menu.needs_update(record("needs update"));
    let _opening = menu.opening(record("opening"));
    let closed = menu.closed(record("closed"));

    let bridge = menu.to_exporter_events_bridge();
    bridge.raise_needs_update();
    bridge.raise_opening();
    bridge.raise_closed();
    closed.dispose();
    bridge.raise_closed();

    assert_eq!(*log.borrow(), vec!["needs update", "opening", "closed"]);
}

#[test]
fn is_native_menu_exported_is_read_only() {
    let _app = application(None);
    let window = Window::new();
    assert!(!NativeMenu::get_is_native_menu_exported(&window));

    let message = panic_message(|| window.set_value(NativeMenu::is_native_menu_exported_property(), true));

    assert_eq!(message, "IsNativeMenuExported property is read-only");
}

#[test]
fn the_menu_of_a_top_level_reaches_its_exporter() {
    let exporter = FakeExporter::new(true);
    let _app = application(Some(exporter.clone()));
    let window = Window::new();
    let menu = NativeMenu::new();

    NativeMenu::set_menu(&window, Some(menu.clone()));

    assert_eq!(NativeMenu::get_menu(&window), Some(menu.clone()));
    assert_eq!(*exporter.menus.borrow(), vec![Some(menu.clone())]);
    assert!(NativeMenu::get_is_native_menu_exported(&window));

    exporter.set_is_exported(false);
    assert!(!NativeMenu::get_is_native_menu_exported(&window));

    NativeMenu::set_menu(&window, None);
    assert_eq!(*exporter.menus.borrow(), vec![Some(menu), None]);
}

#[test]
fn a_top_level_without_an_exporter_keeps_the_menu_only() {
    let _app = application(None);
    let window = Window::new();
    let menu = NativeMenu::new();

    NativeMenu::set_menu(&window, Some(menu.clone()));
    NativeDock::set_menu(&window, Some(menu.clone()));

    assert_eq!(NativeMenu::get_menu(&window), Some(menu.clone()));
    assert_eq!(NativeDock::get_menu(&window), Some(menu));
    assert!(!NativeMenu::get_is_native_menu_exported(&window));
}

#[test]
fn the_menu_bar_shows_the_items_of_the_native_menu_of_the_window() {
    let exporter = FakeExporter::new(false);
    let _app = application(Some(exporter.clone()));

    let clicks = Rc::new(Cell::new(0));
    let file = item("File");
    let open = item("Open");
    let _click = open.click({
        let clicks = clicks.clone();
        move |_| clicks.set(clicks.get() + 1)
    });
    let file_menu = NativeMenu::new();
    file_menu.add(open.clone());
    file.set_menu(Some(file_menu));
    let edit = item("Edit");
    edit.set_tool_tip(Some("Editing".to_string()));
    let menu = NativeMenu::new();
    menu.add(file.clone());
    menu.add(NativeMenuItemSeparator::new());
    menu.add(edit.clone());

    let window = Window::new();
    let bar = NativeMenuBar::new();
    window.set_content(Some(Control::boxed(&bar)));
    NativeMenu::set_menu(&window, Some(menu.clone()));
    window.show();
    window.layout_manager().execute_initial_layout_pass();

    let presenter = bar.find_descendant_of_type::<Menu>(false).expect("the presenter");
    assert_eq!(presenter.name().as_deref(), Some("PART_NativeMenuPresenter"));
    assert!(presenter.is_visible());
    assert_eq!(presenter.item_count(), 3);

    let container = |index: i32| presenter.container_from_index(index).expect("a container");
    let file_container = container(0).cast::<MenuItem>().expect("a menu item");
    assert!(container(1).is::<Separator>());
    let edit_container = container(2).cast::<MenuItem>().expect("a menu item");

    let header = |item: &Ref<MenuItem>| item.header().as_ref().and_then(string_of);
    assert_eq!(header(&file_container).as_deref(), Some("File"));
    assert_eq!(header(&edit_container).as_deref(), Some("Edit"));
    assert_eq!(crate::ToolTip::get_tip(&edit_container).as_ref().and_then(string_of).as_deref(), Some("Editing"));
    assert_eq!(file_container.item_count(), 1);

    // The containers follow the native items.
    file.set_header(Some("Files".to_string()));
    assert_eq!(header(&file_container).as_deref(), Some("Files"));
    edit.set_is_enabled(false);
    assert!(!edit_container.is_enabled());
    edit.set_is_visible(false);
    assert!(!edit_container.is_visible());

    // The checked state is bound both ways.
    edit.set_is_checked(true);
    assert!(edit_container.is_checked());
    edit_container.set_is_checked(false);
    assert!(!edit.is_checked());

    // The items of a submenu get containers of the same kind, and a click
    // on one reaches the click handlers of its native item.
    file_container.set_is_sub_menu_open(true);
    window.layout_manager().execute_layout_pass();
    let open_container =
        file_container.container_from_index(0).and_then(|container| container.cast::<MenuItem>()).expect("a menu item");
    assert_eq!(header(&open_container).as_deref(), Some("Open"));
    open_container.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(MenuItem::click_event()));
    assert_eq!(clicks.get(), 1);
    file_container.set_is_sub_menu_open(false);

    // Items added later appear, and the bar hides when the platform
    // exports the menu.
    menu.add(item("Help"));
    window.layout_manager().execute_layout_pass();
    assert_eq!(presenter.item_count(), 4);

    exporter.set_is_exported(true);
    assert!(!presenter.is_visible());

    NativeMenu::set_menu(&window, None);
    assert_eq!(presenter.item_count(), 0);

    window.close();
}
