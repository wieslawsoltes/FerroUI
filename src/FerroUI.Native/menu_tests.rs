//! Tests of the menu export logic against recording fakes of the native
//! menu objects: nothing here touches AppKit.

use crate::ferro_native_menu_exporter::FerroNativeMenuExporter;
use crate::ferro_native_platform_extensions::MacOSPlatformOptions;
use crate::frn_menu::{native_title, IMenuFactory};
use crate::frn_menu_item::{can_activate, to_native_gesture, to_native_toggle_type};
use crate::interop::*;
use crate::mac_os_native_menu_commands::MacOSNativeMenuCommands;
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, Ref};
use ferroui_controls::platform::{INativeMenuExporter, ITopLevelNativeMenuExporter};
use ferroui_controls::{MenuItemToggleType, NativeMenu, NativeMenuItem, NativeMenuItemBase, NativeMenuItemSeparator};
use ferroui_microcom::{ComPtr, HResult};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_void, CStr};
use std::rc::Rc;

fn text(s: Option<&CStr>) -> String {
    s.map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
}

#[derive(Default)]
struct MenuState {
    title: RefCell<String>,
    /// The addresses of the native items, in order.
    items: RefCell<Vec<usize>>,
}

struct FakeMenu(Rc<MenuState>);

impl IFrnMenuImpl for FakeMenu {
    fn insert_item(&self, index: i32, item: Option<&IFrnMenuItem>) -> Result<(), HResult> {
        let item = item.expect("item") as *const IFrnMenuItem as usize;
        self.0.items.borrow_mut().insert(index as usize, item);
        Ok(())
    }

    fn remove_item(&self, item: Option<&IFrnMenuItem>) -> Result<(), HResult> {
        let item = item.expect("item") as *const IFrnMenuItem as usize;
        self.0.items.borrow_mut().retain(|candidate| *candidate != item);
        Ok(())
    }

    fn set_title(&self, utf8string: Option<&CStr>) -> Result<(), HResult> {
        *self.0.title.borrow_mut() = text(utf8string);
        Ok(())
    }

    fn clear(&self) -> Result<(), HResult> {
        self.0.items.borrow_mut().clear();
        Ok(())
    }
}

#[derive(Default)]
struct ItemState {
    separator: bool,
    title: RefCell<String>,
    tool_tip: RefCell<String>,
    gesture: Cell<(i32, i32)>,
    checked: Cell<bool>,
    visible: Cell<bool>,
    toggle_type: Cell<i32>,
    icon_length: Cell<usize>,
    /// The address of the native submenu; 0 for none.
    sub_menu: Cell<usize>,
    action: RefCell<Option<(ComPtr<IFrnPredicateCallback>, ComPtr<IFrnActionCallback>)>>,
}

struct FakeMenuItem(Rc<ItemState>);

impl IFrnMenuItemImpl for FakeMenuItem {
    fn set_sub_menu(&self, menu: Option<&IFrnMenu>) -> Result<(), HResult> {
        self.0.sub_menu.set(menu.map_or(0, |menu| menu as *const IFrnMenu as usize));
        Ok(())
    }

    fn set_title(&self, utf8string: Option<&CStr>) -> Result<(), HResult> {
        *self.0.title.borrow_mut() = text(utf8string);
        Ok(())
    }

    fn set_tool_tip(&self, utf8string: Option<&CStr>) -> Result<(), HResult> {
        *self.0.tool_tip.borrow_mut() = text(utf8string);
        Ok(())
    }

    fn set_gesture(&self, key: FrnKey, modifiers: FrnInputModifiers) -> Result<(), HResult> {
        self.0.gesture.set((key.0, modifiers.0));
        Ok(())
    }

    fn set_action(
        &self,
        predicate: Option<&IFrnPredicateCallback>,
        callback: Option<&IFrnActionCallback>,
    ) -> Result<(), HResult> {
        *self.0.action.borrow_mut() = match (predicate, callback) {
            (Some(predicate), Some(callback)) => Some((ComPtr::from_ref(predicate), ComPtr::from_ref(callback))),
            _ => None,
        };
        Ok(())
    }

    fn set_is_checked(&self, is_checked: bool) -> Result<(), HResult> {
        self.0.checked.set(is_checked);
        Ok(())
    }

    fn set_is_visible(&self, is_visible: bool) -> Result<(), HResult> {
        self.0.visible.set(is_visible);
        Ok(())
    }

    fn set_toggle_type(&self, toggle_type: FrnMenuItemToggleType) -> Result<(), HResult> {
        self.0.toggle_type.set(toggle_type.0);
        Ok(())
    }

    fn set_icon(&self, _data: *mut c_void, length: usize) -> Result<(), HResult> {
        self.0.icon_length.set(length);
        Ok(())
    }
}

/// The fake native side: every menu and item it created, by address.
#[derive(Default)]
struct FakeFactory {
    menus: RefCell<HashMap<usize, Rc<MenuState>>>,
    events: RefCell<HashMap<usize, ComPtr<IFrnMenuEvents>>>,
    items: RefCell<HashMap<usize, Rc<ItemState>>>,
    /// Kept so that addresses are never reused during a test.
    keep_alive: RefCell<Vec<(Option<ComPtr<IFrnMenu>>, Option<ComPtr<IFrnMenuItem>>)>>,
    app_menu: Cell<usize>,
    dock_menu: Cell<usize>,
    services_menu: Cell<usize>,
    set_dock_menu_calls: Cell<u32>,
}

impl FakeFactory {
    fn item(&self, native: ComPtr<IFrnMenuItem>, state: Rc<ItemState>) -> ComPtr<IFrnMenuItem> {
        self.items.borrow_mut().insert(native.as_ptr() as usize, state);
        self.keep_alive.borrow_mut().push((None, Some(native.clone())));
        native
    }

    fn menu(&self, address: usize) -> Rc<MenuState> {
        self.menus.borrow().get(&address).cloned().expect("a menu created by the factory")
    }

    /// The items of a native menu, in order.
    fn items_of(&self, menu: usize) -> Vec<Rc<ItemState>> {
        let addresses = self.menu(menu).items.borrow().clone();
        addresses.iter().map(|address| self.items.borrow().get(address).cloned().expect("item")).collect()
    }

    fn titles_of(&self, menu: usize) -> Vec<String> {
        self.items_of(menu)
            .iter()
            .map(|item| if item.separator { "-".to_string() } else { item.title.borrow().clone() })
            .collect()
    }
}

impl IMenuFactory for FakeFactory {
    fn create_menu(&self, events: &IFrnMenuEvents) -> ComPtr<IFrnMenu> {
        let state = Rc::new(MenuState::default());
        let native = IFrnMenu::from_impl(FakeMenu(state.clone()));
        let address = native.as_ptr() as usize;
        self.menus.borrow_mut().insert(address, state);
        self.events.borrow_mut().insert(address, ComPtr::from_ref(events));
        self.keep_alive.borrow_mut().push((Some(native.clone()), None));
        native
    }

    fn create_menu_item(&self) -> ComPtr<IFrnMenuItem> {
        let state = Rc::new(ItemState::default());
        self.item(IFrnMenuItem::from_impl(FakeMenuItem(state.clone())), state)
    }

    fn create_menu_item_separator(&self) -> ComPtr<IFrnMenuItem> {
        let state = Rc::new(ItemState { separator: true, ..ItemState::default() });
        self.item(IFrnMenuItem::from_impl(FakeMenuItem(state.clone())), state)
    }

    fn set_services_menu(&self, menu: &IFrnMenu) {
        self.services_menu.set(menu as *const IFrnMenu as usize);
    }

    fn set_app_menu(&self, menu: &IFrnMenu) {
        self.app_menu.set(menu as *const IFrnMenu as usize);
    }

    fn set_dock_menu(&self, menu: &IFrnMenu) {
        self.dock_menu.set(menu as *const IFrnMenu as usize);
        self.set_dock_menu_calls.set(self.set_dock_menu_calls.get() + 1);
    }
}

fn item(header: &str) -> Ref<NativeMenuItem> {
    NativeMenuItem::with_header(header)
}

fn base(item: &Ref<NativeMenuItem>) -> Ref<NativeMenuItemBase> {
    item.clone().upcast()
}

/// A dock exporter over the fake factory with `menu` exported.
fn export(menu: &Ref<NativeMenu>) -> (Rc<FakeFactory>, Rc<FerroNativeMenuExporter>) {
    let factory = Rc::new(FakeFactory::default());
    let exporter = FerroNativeMenuExporter::for_dock_with(factory.clone());
    exporter.set_native_menu(Some(menu.clone()));
    (factory, exporter)
}

fn run_queued_resets() {
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn titles_lose_access_key_markers_and_blank_titles_are_empty() {
    assert_eq!(native_title(Some("_File")), "File");
    assert_eq!(native_title(Some("Save _As")), "Save As");
    assert_eq!(native_title(Some("   ")), "");
    assert_eq!(native_title(None), "");
}

#[test]
fn gestures_and_toggle_types_map_to_the_native_values() {
    assert_eq!(to_native_gesture(None), (FrnKey::FrnKeyNone, FrnInputModifiers::FrnInputModifiersNone));
    let gesture = KeyGesture::new(Key::Q, KeyModifiers::META | KeyModifiers::ALT);
    assert_eq!(
        to_native_gesture(Some(&gesture)),
        (FrnKey::FrnKeyQ, FrnInputModifiers::Windows | FrnInputModifiers::Alt)
    );
    assert_eq!(to_native_toggle_type(MenuItemToggleType::None), FrnMenuItemToggleType::None);
    assert_eq!(to_native_toggle_type(MenuItemToggleType::CheckBox), FrnMenuItemToggleType::CheckMark);
    assert_eq!(to_native_toggle_type(MenuItemToggleType::Radio), FrnMenuItemToggleType::Radio);
}

#[test]
fn an_item_can_be_activated_when_it_has_something_to_run_and_is_enabled() {
    let _dispatcher = Dispatcher::unit_test_scope();
    assert!(!can_activate(None));

    let plain = item("Plain");
    assert!(!can_activate(Some(&plain)));

    let _subscription = plain.click(|_| {});
    assert!(can_activate(Some(&plain)));

    plain.set_is_enabled(false);
    assert!(!can_activate(Some(&plain)));
}

#[test]
fn exporting_a_menu_creates_the_native_items_in_order() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let menu = NativeMenu::new();
    let open = item("_Open");
    open.set_tool_tip(Some("Opens a file".to_string()));
    open.set_gesture(Some(KeyGesture::new(Key::O, KeyModifiers::META)));
    open.set_toggle_type(MenuItemToggleType::CheckBox);
    open.set_is_checked(true);
    menu.add(open);
    menu.add(NativeMenuItemSeparator::new());
    menu.add(item("Close"));

    let (factory, exporter) = export(&menu);

    assert!(exporter.is_native_menu_exported());
    let dock_menu = factory.dock_menu.get();
    assert_ne!(dock_menu, 0);
    assert_eq!(*factory.menu(dock_menu).title.borrow(), "");
    assert_eq!(factory.titles_of(dock_menu), ["Open", "-", "Close"]);

    let items = factory.items_of(dock_menu);
    assert_eq!(*items[0].tool_tip.borrow(), "Opens a file");
    assert_eq!(items[0].gesture.get(), (Key::O.value(), KeyModifiers::META.bits()));
    assert_eq!(items[0].toggle_type.get(), FrnMenuItemToggleType::CheckMark.0);
    assert!(items[0].checked.get());
    assert!(items[0].visible.get());
    assert!(items[0].action.borrow().is_some());
    assert!(items[1].separator);
    assert_eq!(items[2].gesture.get(), (0, 0));
}

#[test]
fn item_changes_reach_the_native_item() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let menu = NativeMenu::new();
    let managed = item("Before");
    menu.add(managed.clone());
    let (factory, _exporter) = export(&menu);
    let native = factory.items_of(factory.dock_menu.get()).remove(0);

    managed.set_header(Some("_After".to_string()));
    managed.set_tool_tip(Some("tip".to_string()));
    managed.set_gesture(Some(KeyGesture::new(Key::A, KeyModifiers::SHIFT)));
    managed.set_toggle_type(MenuItemToggleType::Radio);
    managed.set_is_checked(true);
    managed.set_is_visible(false);

    assert_eq!(*native.title.borrow(), "After");
    assert_eq!(*native.tool_tip.borrow(), "tip");
    assert_eq!(native.gesture.get(), (Key::A.value(), KeyModifiers::SHIFT.bits()));
    assert_eq!(native.toggle_type.get(), FrnMenuItemToggleType::Radio.0);
    assert!(native.checked.get());
    assert!(!native.visible.get());

    managed.set_header(None);
    managed.set_tool_tip(None);
    managed.set_gesture(None);
    assert_eq!(*native.title.borrow(), "");
    assert_eq!(*native.tool_tip.borrow(), "");
    assert_eq!(native.gesture.get(), (0, 0));
}

#[test]
fn the_native_action_asks_the_item_and_raises_its_click() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let menu = NativeMenu::new();
    let managed = item("Run");
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let subscription = managed.click(move |_| counter.set(counter.get() + 1));
    menu.add(managed.clone());
    let (factory, _exporter) = export(&menu);
    let native = factory.items_of(factory.dock_menu.get()).remove(0);
    let (predicate, callback) = native.action.borrow().clone().expect("action");

    assert!(predicate.evaluate());
    callback.run();
    assert_eq!(clicks.get(), 1);

    managed.set_is_enabled(false);
    assert!(!predicate.evaluate());

    managed.set_is_enabled(true);
    subscription.dispose();
    assert!(!predicate.evaluate());
}

#[test]
fn item_list_changes_are_applied_by_the_queued_reset() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let menu = NativeMenu::new();
    let (a, b, c) = (item("A"), item("B"), item("C"));
    menu.add(a.clone());
    menu.add(b.clone());
    menu.add(c.clone());
    let (factory, exporter) = export(&menu);
    let dock_menu = factory.dock_menu.get();
    let native_before = factory.menu(dock_menu).items.borrow().clone();

    // Move C to the front: nothing happens until the queued reset runs.
    menu.items().move_item(2, 0);
    assert_eq!(factory.titles_of(dock_menu), ["A", "B", "C"]);
    run_queued_resets();
    assert_eq!(factory.titles_of(dock_menu), ["C", "A", "B"]);
    // The native items were moved, not recreated.
    let native_after = factory.menu(dock_menu).items.borrow().clone();
    assert_eq!(native_after, [native_before[2], native_before[0], native_before[1]]);

    // Remove the middle item and append a new one.
    menu.items().remove_at(1);
    menu.add(item("D"));
    run_queued_resets();
    assert_eq!(factory.titles_of(dock_menu), ["C", "B", "D"]);
    let native_menu = exporter.native_menu().expect("native menu");
    let managed = native_menu.managed_items();
    assert_eq!(managed.len(), 3);
    assert!(managed[0].ptr_eq(&base(&c)));
    assert!(managed[1].ptr_eq(&base(&b)));

    // The removed item no longer updates a native item.
    let removed_native = factory.items.borrow().get(&native_before[0]).cloned().unwrap();
    a.set_header(Some("changed".to_string()));
    assert_eq!(*removed_native.title.borrow(), "A");
    assert!(removed_native.action.borrow().is_some(), "the fake keeps what it was last given");

    // The native menu is handed to the dock once.
    assert_eq!(factory.set_dock_menu_calls.get(), 1);

    menu.items().clear();
    run_queued_resets();
    assert!(factory.titles_of(dock_menu).is_empty());
}

#[test]
fn submenus_are_created_updated_and_removed() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let menu = NativeMenu::new();
    let parent = item("_Recent");
    let sub = NativeMenu::new();
    sub.add(item("One"));
    parent.set_menu(Some(sub.clone()));
    menu.add(parent.clone());
    let (factory, exporter) = export(&menu);
    let dock_menu = factory.dock_menu.get();
    let native_parent = factory.items_of(dock_menu).remove(0);

    let native_sub = native_parent.sub_menu.get();
    assert_ne!(native_sub, 0);
    assert_eq!(*factory.menu(native_sub).title.borrow(), "Recent");
    assert_eq!(factory.titles_of(native_sub), ["One"]);
    assert_eq!(factory.services_menu.get(), 0);

    // A change of the submenu items queues a reset of the whole tree.
    sub.add(item("Two"));
    run_queued_resets();
    assert_eq!(factory.titles_of(native_sub), ["One", "Two"]);

    // Removing the submenu is picked up by the next update.
    parent.set_menu(None);
    exporter.set_native_menu(Some(menu.clone()));
    assert_eq!(native_parent.sub_menu.get(), 0);
    let native_item = exporter.native_menu().unwrap();
    assert_eq!(native_item.managed_items().len(), 1);
}

#[test]
fn the_services_submenu_is_announced_to_the_system() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let menu = NativeMenu::new();
    let services = item("Services");
    let sub = NativeMenu::new();
    sub.set_value(MacOSNativeMenuCommands::is_services_submenu_property(), true);
    services.set_menu(Some(sub));
    menu.add(services);
    let (factory, _exporter) = export(&menu);

    let native_services = factory.items_of(factory.dock_menu.get()).remove(0);
    assert_ne!(native_services.sub_menu.get(), 0);
    assert_eq!(factory.services_menu.get(), native_services.sub_menu.get());
}

#[test]
fn native_menu_events_reach_the_menu() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let menu = NativeMenu::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let (l1, l2, l3) = (log.clone(), log.clone(), log.clone());
    let _s1 = menu.needs_update(move |_| l1.borrow_mut().push("needs_update"));
    let _s2 = menu.opening(move |_| l2.borrow_mut().push("opening"));
    let _s3 = menu.closed(move |_| l3.borrow_mut().push("closed"));
    let (factory, _exporter) = export(&menu);
    let events = factory.events.borrow().get(&factory.dock_menu.get()).cloned().expect("events");

    events.needs_update();
    events.opening();
    events.closed();

    assert_eq!(*log.borrow(), ["needs_update", "opening", "closed"]);
}

#[test]
fn the_dock_exporter_ignores_a_cleared_menu_and_other_targets_export_an_empty_one() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let factory = Rc::new(FakeFactory::default());
    let exporter = FerroNativeMenuExporter::for_dock_with(factory.clone());
    assert!(!exporter.is_native_menu_exported());

    exporter.set_native_menu(None);

    assert!(!exporter.is_native_menu_exported());
    assert_eq!(factory.dock_menu.get(), 0);
    assert!(factory.menus.borrow().is_empty());
}

#[test]
fn the_application_menu_gets_the_default_and_the_standard_items() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    let factory = Rc::new(FakeFactory::default());

    let exporter = FerroNativeMenuExporter::for_application_with(factory.clone());

    assert!(exporter.is_native_menu_exported());
    // The menu bar holds one item whose submenu is the application menu.
    let menu_bar = factory.app_menu.get();
    assert_ne!(menu_bar, 0);
    let holder_items = factory.items_of(menu_bar);
    assert_eq!(holder_items.len(), 1);
    let app_menu = holder_items[0].sub_menu.get();
    assert_eq!(
        factory.titles_of(app_menu),
        ["About FerroUI", "-", "Services", "-", "Hide Application", "Hide Others", "Show All", "-", "Quit"]
    );

    let items = factory.items_of(app_menu);
    // Services is the services submenu of the system.
    assert_eq!(factory.services_menu.get(), items[2].sub_menu.get());
    assert_ne!(factory.services_menu.get(), 0);
    // Hide: Cmd+H; Hide Others: Cmd+Alt+Q; Quit: Cmd+Q.
    assert_eq!(items[4].gesture.get(), (Key::H.value(), KeyModifiers::META.bits()));
    assert_eq!(items[5].gesture.get(), (Key::Q.value(), (KeyModifiers::META | KeyModifiers::ALT).bits()));
    assert_eq!(items[8].gesture.get(), (Key::Q.value(), KeyModifiers::META.bits()));
    // Hide and Quit have handlers, so the system may activate them; the
    // about item has none yet.
    let enabled = |index: usize| items[index].action.borrow().as_ref().unwrap().0.evaluate();
    assert!(enabled(4));
    assert!(enabled(8));
    assert!(!enabled(0));
}

#[test]
fn the_standard_application_menu_items_can_be_disabled() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind_to_self(Rc::new(MacOSPlatformOptions {
        disable_default_application_menu_items: true,
        ..MacOSPlatformOptions::default()
    }));
    let factory = Rc::new(FakeFactory::default());

    let _exporter = FerroNativeMenuExporter::for_application_with(factory.clone());

    let app_menu = factory.items_of(factory.app_menu.get())[0].sub_menu.get();
    assert_eq!(factory.titles_of(app_menu), ["About FerroUI"]);
}

#[test]
fn nothing_is_exported_when_native_menus_are_disabled() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind_to_self(Rc::new(MacOSPlatformOptions { disable_native_menus: true, ..MacOSPlatformOptions::default() }));
    let factory = Rc::new(FakeFactory::default());

    let application = FerroNativeMenuExporter::for_application_with(factory.clone());
    let dock = FerroNativeMenuExporter::for_dock_with(factory.clone());
    let menu = NativeMenu::new();
    menu.add(item("A"));
    dock.set_native_menu(Some(menu));

    assert!(!application.is_native_menu_exported());
    assert!(!dock.is_native_menu_exported());
    assert!(factory.menus.borrow().is_empty());
    assert_eq!(factory.app_menu.get(), 0);
    assert_eq!(factory.dock_menu.get(), 0);
}
