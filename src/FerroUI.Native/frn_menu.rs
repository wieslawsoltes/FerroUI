//! The native menu that mirrors a `NativeMenu`, and the receiver of its
//! native events.

use crate::ferro_native_menu_exporter::FerroNativeMenuExporter;
use crate::frn_menu_item::FrnMenuItem;
use crate::frn_string::to_c_string;
use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::Ref;
use ferroui_controls::primitives::AccessText;
use ferroui_controls::{NativeMenu, NativeMenuItem, NativeMenuItemBase, NativeMenuItemSeparator};
use ferroui_microcom::ComPtr;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// What the menu code needs from the native factory. The native factory
/// implements it; tests use a recording fake.
pub(crate) trait IMenuFactory {
    fn create_menu(&self, events: &IFrnMenuEvents) -> ComPtr<IFrnMenu>;
    fn create_menu_item(&self) -> ComPtr<IFrnMenuItem>;
    fn create_menu_item_separator(&self) -> ComPtr<IFrnMenuItem>;
    fn set_services_menu(&self, menu: &IFrnMenu);
    fn set_app_menu(&self, menu: &IFrnMenu);
    fn set_dock_menu(&self, menu: &IFrnMenu);
}

impl IMenuFactory for ComPtr<IFerroNativeFactory> {
    fn create_menu(&self, events: &IFrnMenuEvents) -> ComPtr<IFrnMenu> {
        IFerroNativeFactory::create_menu(self, Some(events)).check().expect("the native menu")
    }

    fn create_menu_item(&self) -> ComPtr<IFrnMenuItem> {
        IFerroNativeFactory::create_menu_item(self).check().expect("the native menu item")
    }

    fn create_menu_item_separator(&self) -> ComPtr<IFrnMenuItem> {
        IFerroNativeFactory::create_menu_item_separator(self).check().expect("the native menu item separator")
    }

    fn set_services_menu(&self, menu: &IFrnMenu) {
        IFerroNativeFactory::set_services_menu(self, Some(menu)).check();
    }

    fn set_app_menu(&self, menu: &IFrnMenu) {
        IFerroNativeFactory::set_app_menu(self, Some(menu)).check();
    }

    fn set_dock_menu(&self, menu: &IFrnMenu) {
        IFerroNativeFactory::set_dock_menu(self, Some(menu)).check();
    }
}

/// The title native code is given for a header: macOS does not process
/// access key markers, so they are removed; a blank title is empty.
pub(crate) fn native_title(title: Option<&str>) -> String {
    let title = AccessText::remove_access_key_marker(title);
    match title {
        Some(title) if !title.trim().is_empty() => title,
        _ => String::new(),
    }
}

/// Receives the events of a native menu.
struct MenuEvents {
    parent: Rc<RefCell<Weak<FrnMenu>>>,
}

impl MenuEvents {
    fn parent(&self) -> Option<Rc<FrnMenu>> {
        self.parent.borrow().upgrade()
    }
}

impl IFrnMenuEventsImpl for MenuEvents {
    fn needs_update(&self) {
        crate::callback_base::guard((), || {
            if let Some(parent) = self.parent() {
                parent.raise_needs_update();
            }
        })
    }

    fn opening(&self) {
        crate::callback_base::guard((), || {
            if let Some(parent) = self.parent() {
                parent.raise_opening();
            }
        })
    }

    fn closed(&self) {
        crate::callback_base::guard((), || {
            if let Some(parent) = self.parent() {
                parent.raise_closed();
            }
        })
    }
}

/// A native menu and the `NativeMenu` it mirrors.
pub(crate) struct FrnMenu {
    native: ComPtr<IFrnMenu>,
    exporter: RefCell<Weak<FerroNativeMenuExporter>>,
    managed_menu: RefCell<Option<Ref<NativeMenu>>>,
    menu_items: RefCell<Vec<Rc<FrnMenuItem>>>,
    items_changed_token: Cell<Option<u64>>,
}

impl FrnMenu {
    pub(crate) fn create(factory: &dyn IMenuFactory) -> Rc<FrnMenu> {
        let parent = Rc::new(RefCell::new(Weak::new()));
        let events = IFrnMenuEvents::from_impl(MenuEvents { parent: parent.clone() });

        let menu = Rc::new(FrnMenu {
            native: factory.create_menu(&events),
            exporter: RefCell::new(Weak::new()),
            managed_menu: RefCell::new(None),
            menu_items: RefCell::new(Vec::new()),
            items_changed_token: Cell::new(None),
        });

        *parent.borrow_mut() = Rc::downgrade(&menu);

        menu
    }

    pub(crate) fn native(&self) -> &ComPtr<IFrnMenu> {
        &self.native
    }

    fn exporter(&self) -> Option<Rc<FerroNativeMenuExporter>> {
        self.exporter.borrow().upgrade()
    }

    #[track_caller]
    pub(crate) fn managed_menu(&self) -> Ref<NativeMenu> {
        self.managed_menu.borrow().clone().expect("the menu is initialized")
    }

    fn update_title(&self, title: Option<&str>) {
        self.native.set_title(Some(&to_c_string(&native_title(title)))).check();
    }

    pub(crate) fn raise_needs_update(&self) {
        self.managed_menu().to_exporter_events_bridge().raise_needs_update();

        if let Some(exporter) = self.exporter() {
            exporter.update_if_needed();
        }
    }

    pub(crate) fn raise_opening(&self) {
        self.managed_menu().to_exporter_events_bridge().raise_opening();
    }

    pub(crate) fn raise_closed(&self) {
        self.managed_menu().to_exporter_events_bridge().raise_closed();
    }

    fn remove_and_dispose(&self, item: &Rc<FrnMenuItem>) {
        self.menu_items.borrow_mut().retain(|candidate| !Rc::ptr_eq(candidate, item));
        self.native.remove_item(Some(item.native())).check();

        item.deinitialize();
    }

    fn move_existing_to(&self, index: usize, item: &Rc<FrnMenuItem>) {
        {
            let mut menu_items = self.menu_items.borrow_mut();
            menu_items.retain(|candidate| !Rc::ptr_eq(candidate, item));
            menu_items.insert(index, item.clone());
        }

        self.native.remove_item(Some(item.native())).check();
        self.native.insert_item(index as i32, Some(item.native())).check();
    }

    fn create_new_at(&self, factory: &dyn IMenuFactory, index: usize, item: &Ref<NativeMenuItemBase>) -> Rc<FrnMenuItem> {
        let result = Self::create_new(factory, item);

        result.initialize();

        self.menu_items.borrow_mut().insert(index, result.clone());

        self.native.insert_item(index as i32, Some(result.native())).check();

        result
    }

    fn create_new(factory: &dyn IMenuFactory, item: &Ref<NativeMenuItemBase>) -> Rc<FrnMenuItem> {
        let native_item = if item.is::<NativeMenuItemSeparator>() {
            factory.create_menu_item_separator()
        } else {
            factory.create_menu_item()
        };

        FrnMenuItem::new(native_item, item.clone())
    }

    pub(crate) fn initialize(
        self: &Rc<Self>,
        exporter: &Rc<FerroNativeMenuExporter>,
        managed_menu: &Ref<NativeMenu>,
        title: Option<&str>,
    ) {
        *self.exporter.borrow_mut() = Rc::downgrade(exporter);
        *self.managed_menu.borrow_mut() = Some(managed_menu.clone());

        let this = Rc::downgrade(self);
        let token = managed_menu.items().add_collection_changed(Rc::new(move |_| {
            if let Some(this) = this.upgrade() {
                this.on_menu_items_changed();
            }
        }));
        self.items_changed_token.set(Some(token));

        self.update_title(title);
    }

    pub(crate) fn deinitialise(&self) {
        if let Some(token) = self.items_changed_token.take() {
            self.managed_menu().items().remove_collection_changed(token);
        }

        let menu_items = self.menu_items.borrow().clone();
        for item in menu_items {
            item.deinitialize();
        }
    }

    /// Brings the native items in line with the items of `menu`: items are
    /// created, moved and removed so that position `i` holds the native
    /// item of `menu.items()[i]`, and every item updates its submenu.
    ///
    /// # Panics
    /// Panics when `menu` is not the menu this native menu mirrors.
    pub(crate) fn update(&self, factory: &dyn IMenuFactory, menu: &Ref<NativeMenu>) {
        if !menu.ptr_eq(&self.managed_menu()) {
            panic!("The menu being updated does not match.");
        }

        let items = menu.items().to_vec();
        for (i, managed_item) in items.iter().enumerate() {
            let at_index = self.menu_items.borrow().get(i).cloned();
            let existing = self
                .menu_items
                .borrow()
                .iter()
                .find(|candidate| candidate.managed_menu_item().ptr_eq(managed_item))
                .cloned();

            let native_item = match (at_index, existing) {
                (None, _) => self.create_new_at(factory, i, managed_item),
                (Some(at_index), _) if at_index.managed_menu_item().ptr_eq(managed_item) => at_index,
                (Some(_), Some(existing)) => {
                    self.move_existing_to(i, &existing);
                    existing
                }
                (Some(_), None) => self.create_new_at(factory, i, managed_item),
            };

            if let Some(nmi) = managed_item.cast::<NativeMenuItem>() {
                if let Some(exporter) = self.exporter() {
                    native_item.update(&exporter, factory, &nmi);
                }
            }
        }

        loop {
            let last = {
                let menu_items = self.menu_items.borrow();
                if menu_items.len() > items.len() {
                    menu_items.last().cloned()
                } else {
                    None
                }
            };
            let Some(last) = last else { break };
            self.remove_and_dispose(&last);
        }
    }

    fn on_menu_items_changed(&self) {
        if let Some(exporter) = self.exporter() {
            exporter.queue_reset();
        }
    }

    /// The managed items of the native items, in native order (for tests).
    #[cfg(test)]
    pub(crate) fn managed_items(&self) -> Vec<Ref<NativeMenuItemBase>> {
        self.menu_items.borrow().iter().map(|item| item.managed_menu_item().clone()).collect()
    }
}
