//! Exports `NativeMenu` trees to the system: the application menu, the
//! menu of a window, of a tray icon and of the dock.

use crate::ferro_native_platform_extensions::MacOSPlatformOptions;
use crate::frn_menu::{FrnMenu, IMenuFactory};
use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::mac_os_native_menu_commands::MacOSNativeMenuCommands;
use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::platform::{INativeMenuExporter, ITopLevelNativeMenuExporter};
use ferroui_controls::{Application, NativeDock, NativeMenu, NativeMenuItem, NativeMenuItemSeparator};
use ferroui_microcom::ComPtr;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// What a menu is exported to.
pub(crate) enum MenuTarget {
    Application,
    Window(ComPtr<IFrnWindow>),
    TrayIcon(ComPtr<IFrnTrayIcon>),
    Dock,
}

/// Exports a `NativeMenu` to one target and keeps the native menu in sync.
pub struct FerroNativeMenuExporter {
    weak_self: Weak<FerroNativeMenuExporter>,
    factory: Rc<dyn IMenuFactory>,
    target: MenuTarget,
    reset_queued: Cell<bool>,
    exported: Cell<bool>,
    menu: RefCell<Option<Ref<NativeMenu>>>,
    native_menu: RefCell<Option<Rc<FrnMenu>>>,
    application_commands: Option<ComPtr<IFrnApplicationCommands>>,
    /// The subscription to the dock menu of the application.
    dock_menu_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

fn mac_options() -> MacOSPlatformOptions {
    FerroLocator::current().get_service::<MacOSPlatformOptions>().as_deref().cloned().unwrap_or_default()
}

impl FerroNativeMenuExporter {
    fn new(
        factory: Rc<dyn IMenuFactory>,
        target: MenuTarget,
        reset_queued: bool,
        application_commands: Option<ComPtr<IFrnApplicationCommands>>,
    ) -> Rc<FerroNativeMenuExporter> {
        Rc::new_cyclic(|weak_self| FerroNativeMenuExporter {
            weak_self: weak_self.clone(),
            factory,
            target,
            reset_queued: Cell::new(reset_queued),
            exported: Cell::new(false),
            menu: RefCell::new(None),
            native_menu: RefCell::new(None),
            application_commands,
            dock_menu_subscription: RefCell::new(None),
        })
    }

    fn with_commands(factory: &ComPtr<IFerroNativeFactory>, target: MenuTarget) -> Rc<FerroNativeMenuExporter> {
        let application_commands = factory.create_application_commands().check();
        let this = Self::new(Rc::new(factory.clone()), target, true, application_commands);

        this.do_layout_reset(false);
        this
    }

    /// The exporter of the menu of a window.
    pub(crate) fn for_window(
        native_window: ComPtr<IFrnWindow>,
        factory: &ComPtr<IFerroNativeFactory>,
    ) -> Rc<FerroNativeMenuExporter> {
        Self::with_commands(factory, MenuTarget::Window(native_window))
    }

    /// The exporter of the application menu.
    pub(crate) fn for_application(factory: &ComPtr<IFerroNativeFactory>) -> Rc<FerroNativeMenuExporter> {
        Self::with_commands(factory, MenuTarget::Application)
    }

    /// The exporter of the menu of a tray icon.
    pub(crate) fn for_tray_icon(
        tray_icon: ComPtr<IFrnTrayIcon>,
        factory: &ComPtr<IFerroNativeFactory>,
    ) -> Rc<FerroNativeMenuExporter> {
        Self::with_commands(factory, MenuTarget::TrayIcon(tray_icon))
    }

    /// The exporter of the dock menu: follows the dock menu of the
    /// application.
    pub(crate) fn for_dock(factory: &ComPtr<IFerroNativeFactory>) -> Rc<FerroNativeMenuExporter> {
        Self::for_dock_with(Rc::new(factory.clone()))
    }

    pub(crate) fn for_dock_with(factory: Rc<dyn IMenuFactory>) -> Rc<FerroNativeMenuExporter> {
        let this = Self::new(factory, MenuTarget::Dock, false, None);

        if mac_options().disable_native_menus {
            return this;
        }

        let weak = Rc::downgrade(&this);
        let subscription = NativeDock::menu_property().changed().subscribe(move |args| {
            if args.sender().is::<Application>() {
                if let Some(this) = weak.upgrade() {
                    this.set_native_menu(args.get_new_value::<Option<Ref<NativeMenu>>>());
                }
            }
        });
        *this.dock_menu_subscription.borrow_mut() = Some(subscription);

        if let Some(app) = Application::current() {
            if let Some(dock_menu) = NativeDock::get_menu(&app) {
                this.set_native_menu(Some(dock_menu));
            }
        }

        this
    }

    pub(crate) fn update_if_needed(&self) {
        if self.reset_queued.get() {
            self.do_layout_reset(false);
        }
    }

    fn create_default_app_menu() -> Ref<NativeMenu> {
        let result = NativeMenu::new();

        // The about dialog is not ported yet: the item has no click handler
        // until it is, so the system shows it disabled.
        let about_item = NativeMenuItem::with_header("About FerroUI");

        result.add(about_item);

        result
    }

    fn populate_standard_osx_menu_items(&self, app_menu: &Ref<NativeMenu>) {
        app_menu.add(NativeMenuItemSeparator::new());

        let services_menu = NativeMenuItem::with_header("Services");
        let services_sub_menu = NativeMenu::new();
        services_sub_menu.set_value(MacOSNativeMenuCommands::is_services_submenu_property(), true);
        services_menu.set_menu(Some(services_sub_menu));

        app_menu.add(services_menu);

        app_menu.add(NativeMenuItemSeparator::new());

        let application_name =
            Application::current().and_then(|app| app.name()).unwrap_or_else(|| "Application".to_string());
        let hide_item = NativeMenuItem::with_header(&format!("Hide {application_name}"));
        hide_item.set_gesture(Some(KeyGesture::new(Key::H, KeyModifiers::META)));

        let commands = self.application_commands.clone();
        let _ = hide_item.click(move |_| {
            if let Some(commands) = &commands {
                commands.hide_app().check();
            }
        });

        app_menu.add(hide_item);

        let hide_others_item = NativeMenuItem::with_header("Hide Others");
        hide_others_item.set_gesture(Some(KeyGesture::new(Key::Q, KeyModifiers::META | KeyModifiers::ALT)));
        let commands = self.application_commands.clone();
        let _ = hide_others_item.click(move |_| {
            if let Some(commands) = &commands {
                commands.hide_others().check();
            }
        });
        app_menu.add(hide_others_item);

        let show_all_item = NativeMenuItem::with_header("Show All");
        let commands = self.application_commands.clone();
        let _ = show_all_item.click(move |_| {
            if let Some(commands) = &commands {
                commands.show_all().check();
            }
        });

        app_menu.add(show_all_item);

        app_menu.add(NativeMenuItemSeparator::new());

        let quit_item = NativeMenuItem::with_header("Quit");
        quit_item.set_gesture(Some(KeyGesture::new(Key::Q, KeyModifiers::META)));
        let _ = quit_item.click(|_| {
            let lifetime = Application::current().and_then(|app| app.application_lifetime());
            let Some(lifetime) = lifetime else {
                return;
            };
            if let Some(lifetime) = lifetime.as_classic_desktop_style_application_lifetime() {
                lifetime.try_shutdown(0);
            } else if let Some(controlled_lifetime) = lifetime.as_controlled_application_lifetime() {
                controlled_lifetime.shutdown(0);
            }
        });

        app_menu.add(quit_item);
    }

    fn do_layout_reset(&self, force_update: bool) {
        if mac_options().disable_native_menus {
            return;
        }

        if self.reset_queued.get() || force_update {
            self.reset_queued.set(false);

            let menu = self.menu.borrow().clone();
            match &self.target {
                MenuTarget::Application => {
                    let app = Application::current();
                    let app_menu = app.as_ref().and_then(|app| NativeMenu::get_menu(app));

                    let app_menu = match app_menu {
                        Some(app_menu) => app_menu,
                        None => {
                            let app_menu = Self::create_default_app_menu();

                            if let Some(app) = &app {
                                NativeMenu::set_menu(app, Some(app_menu.clone()));
                            }
                            app_menu
                        }
                    };

                    self.set_app_menu(&app_menu);
                }

                MenuTarget::Window(native_window) => {
                    if let Some(menu) = menu {
                        if self.ensure_native_menu(&menu) {
                            let native_menu = self.native_menu.borrow().clone().expect("just created");
                            native_window.set_main_menu(Some(native_menu.native())).check();
                        }
                    }
                }

                MenuTarget::TrayIcon(tray_icon) => {
                    if let Some(menu) = menu {
                        if self.ensure_native_menu(&menu) {
                            let native_menu = self.native_menu.borrow().clone().expect("just created");
                            tray_icon.set_menu(Some(native_menu.native())).check();
                        }
                    }
                }

                MenuTarget::Dock => {
                    if let Some(menu) = menu {
                        if self.ensure_native_menu(&menu) {
                            let native_menu = self.native_menu.borrow().clone().expect("just created");
                            self.factory.set_dock_menu(native_menu.native());
                        }
                    }
                }
            }

            self.exported.set(true);
        }
    }

    pub(crate) fn queue_reset(&self) {
        if self.reset_queued.get() {
            return;
        }
        self.reset_queued.set(true);
        let this = self.weak_self.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = this.upgrade() {
                    this.do_layout_reset(false);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    fn this(&self) -> Rc<FerroNativeMenuExporter> {
        self.weak_self.upgrade().expect("the exporter is alive while it is borrowed")
    }

    /// Creates the native menu for `menu` on first use and updates it;
    /// returns whether it was just created (and still has to be handed to
    /// its target).
    fn ensure_native_menu(&self, menu: &Ref<NativeMenu>) -> bool {
        let mut set_menu = false;

        let native_menu = self.native_menu.borrow().clone();
        let native_menu = match native_menu {
            Some(native_menu) => native_menu,
            None => {
                let native_menu = FrnMenu::create(&*self.factory);

                native_menu.initialize(&self.this(), menu, Some(""));
                *self.native_menu.borrow_mut() = Some(native_menu.clone());

                set_menu = true;
                native_menu
            }
        };

        native_menu.update(&*self.factory, menu);

        set_menu
    }

    /// Exports the application menu: `menu` becomes the submenu of the
    /// first item of the menu bar.
    fn set_app_menu(&self, menu: &Ref<NativeMenu>) {
        let menu_item = menu.parent();

        let app_menu_holder = menu_item.as_ref().and_then(|menu_item| menu_item.parent());

        let menu_item = menu_item.unwrap_or_else(NativeMenuItem::new);

        let app_menu_holder = app_menu_holder.unwrap_or_else(|| {
            let app_menu_holder = NativeMenu::new();

            app_menu_holder.add(menu_item.clone());
            app_menu_holder
        });

        menu_item.set_menu(Some(menu.clone()));

        let mut set_menu = false;

        let native_menu = self.native_menu.borrow().clone();
        let native_menu = match native_menu {
            Some(native_menu) => native_menu,
            None => {
                let native_menu = FrnMenu::create(&*self.factory);

                native_menu.initialize(&self.this(), &app_menu_holder, Some(""));
                *self.native_menu.borrow_mut() = Some(native_menu.clone());

                if !mac_options().disable_default_application_menu_items {
                    self.populate_standard_osx_menu_items(menu);
                }

                set_menu = true;
                native_menu
            }
        };

        native_menu.update(&*self.factory, &app_menu_holder);

        if set_menu {
            self.factory.set_app_menu(native_menu.native());
        }
    }

    #[cfg(test)]
    pub(crate) fn native_menu(&self) -> Option<Rc<FrnMenu>> {
        self.native_menu.borrow().clone()
    }

    #[cfg(test)]
    pub(crate) fn for_application_with(factory: Rc<dyn IMenuFactory>) -> Rc<FerroNativeMenuExporter> {
        let this = Self::new(factory, MenuTarget::Application, true, None);
        this.do_layout_reset(false);
        this
    }
}

impl INativeMenuExporter for FerroNativeMenuExporter {
    fn set_native_menu(&self, menu: Option<Ref<NativeMenu>>) {
        if matches!(self.target, MenuTarget::Dock) {
            let has_menu = menu.is_some();
            *self.menu.borrow_mut() = menu;

            if has_menu {
                self.do_layout_reset(true);
            }
        } else {
            *self.menu.borrow_mut() = Some(menu.unwrap_or_else(NativeMenu::new));
            self.do_layout_reset(true);
        }
    }
}

impl ITopLevelNativeMenuExporter for FerroNativeMenuExporter {
    fn is_native_menu_exported(&self) -> bool {
        self.exported.get()
    }

    /// Never raised, as in the reference implementation.
    fn on_is_native_menu_exported_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

impl Drop for FerroNativeMenuExporter {
    fn drop(&mut self) {
        if let Some(subscription) = self.dock_menu_subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }
}
