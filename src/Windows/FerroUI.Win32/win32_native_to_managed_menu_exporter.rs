//! Port of `Win32NativeToManagedMenuExporter.cs`: Windows has no native
//! menu to export a menu to, so the exporter keeps the menu it is given,
//! and what shows it (the tray icon) builds a managed menu from it.

use ferroui_base::Ref;
use ferroui_controls::platform::INativeMenuExporter;
use ferroui_controls::NativeMenu;
use std::cell::RefCell;

#[derive(Default)]
pub(crate) struct Win32NativeToManagedMenuExporter {
    native_menu: RefCell<Option<Ref<NativeMenu>>>,
}

impl Win32NativeToManagedMenuExporter {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_native_menu(&self) -> Option<Ref<NativeMenu>> {
        self.native_menu.borrow().clone()
    }
}

impl INativeMenuExporter for Win32NativeToManagedMenuExporter {
    fn set_native_menu(&self, native_menu: Option<Ref<NativeMenu>>) {
        *self.native_menu.borrow_mut() = native_menu;
    }
}
