//! The native menu item that mirrors a `NativeMenuItemBase`.

use crate::ferro_native_menu_exporter::FerroNativeMenuExporter;
use crate::frn_menu::{native_title, FrnMenu, IMenuFactory};
use crate::frn_string::to_c_string;
use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::mac_os_native_menu_commands::MacOSNativeMenuCommands;
use crate::menu_action_callback::MenuActionCallback;
use crate::predicate_callback::PredicateCallback;
use ferroui_base::input::KeyGesture;
use ferroui_base::media::imaging::{BitmapEncoderOptions, IBitmap, PngBitmapEncoderOptions};
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{FerroObjectExtensions, Ref};
use ferroui_controls::{MenuItemToggleType, NativeMenuItem, NativeMenuItemBase};
use ferroui_microcom::ComPtr;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::{Rc, Weak};

/// The native key and modifiers of a gesture; no gesture is "no key".
pub(crate) fn to_native_gesture(gesture: Option<&KeyGesture>) -> (FrnKey, FrnInputModifiers) {
    match gesture {
        None => (FrnKey::FrnKeyNone, FrnInputModifiers::FrnInputModifiersNone),
        Some(gesture) => (FrnKey(gesture.key().value()), FrnInputModifiers(gesture.key_modifiers().bits())),
    }
}

pub(crate) fn to_native_toggle_type(toggle_type: MenuItemToggleType) -> FrnMenuItemToggleType {
    FrnMenuItemToggleType(toggle_type as i32)
}

/// Whether native code may activate `item`: it has something to run (a
/// command or click handlers) and is enabled.
pub(crate) fn can_activate(item: Option<&NativeMenuItem>) -> bool {
    match item {
        Some(item) if item.command().is_some() || item.has_click_handlers() => item.is_enabled(),
        _ => false,
    }
}

/// A native menu item and the menu item it mirrors.
pub(crate) struct FrnMenuItem {
    weak_self: Weak<FrnMenuItem>,
    native: ComPtr<IFrnMenuItem>,
    sub_menu: RefCell<Option<Rc<FrnMenu>>>,
    property_disposables: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// The callbacks of the current action; kept alive until replaced.
    current_action: RefCell<Option<(ComPtr<IFrnPredicateCallback>, ComPtr<IFrnActionCallback>)>>,
    managed_menu_item: Ref<NativeMenuItemBase>,
}

impl FrnMenuItem {
    pub(crate) fn new(native: ComPtr<IFrnMenuItem>, managed_menu_item: Ref<NativeMenuItemBase>) -> Rc<FrnMenuItem> {
        Rc::new_cyclic(|weak_self| FrnMenuItem {
            weak_self: weak_self.clone(),
            native,
            sub_menu: RefCell::new(None),
            property_disposables: RefCell::new(Vec::new()),
            current_action: RefCell::new(None),
            managed_menu_item,
        })
    }

    pub(crate) fn native(&self) -> &ComPtr<IFrnMenuItem> {
        &self.native
    }

    pub(crate) fn managed_menu_item(&self) -> &Ref<NativeMenuItemBase> {
        &self.managed_menu_item
    }

    fn update_title(&self, title: Option<&str>) {
        self.native.set_title(Some(&to_c_string(&native_title(title)))).check();
    }

    fn update_tool_tip(&self, tool_tip: Option<&str>) {
        self.native.set_tool_tip(Some(&to_c_string(tool_tip.unwrap_or("")))).check();
    }

    fn update_is_visible(&self, is_visible: bool) {
        self.native.set_is_visible(is_visible).check();
    }

    fn update_is_checked(&self, is_checked: bool) {
        self.native.set_is_checked(is_checked).check();
    }

    fn update_toggle_type(&self, toggle_type: MenuItemToggleType) {
        self.native.set_toggle_type(to_native_toggle_type(toggle_type)).check();
    }

    fn update_icon(&self, icon: Option<&Rc<dyn IBitmap>>) {
        match icon {
            // SAFETY: a null pointer with a zero length clears the icon.
            None => unsafe { self.native.set_icon(std::ptr::null_mut(), 0) }.check(),
            Some(icon) => {
                let mut image_data = Vec::new();
                if let Err(error) = icon.save(&mut image_data, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
                {
                    panic!("Unable to save the menu item icon: {error}");
                }

                // SAFETY: the native side copies the image data during the call.
                unsafe { self.native.set_icon(image_data.as_mut_ptr() as *mut c_void, image_data.len()) }.check();
            }
        }
    }

    fn update_gesture(&self, gesture: Option<&KeyGesture>) {
        let (key, modifiers) = to_native_gesture(gesture);
        self.native.set_gesture(key, modifiers).check();
    }

    fn update_action(&self, item: Option<Ref<NativeMenuItem>>) {
        let predicate_item = item.clone();
        let action = IFrnPredicateCallback::from_impl(PredicateCallback::new(move || {
            can_activate(predicate_item.as_deref())
        }));

        let callback = IFrnActionCallback::from_impl(MenuActionCallback::new(move || {
            if let Some(item) = &item {
                item.to_exporter_events_bridge().raise_clicked();
            }
        }));

        self.native.set_action(Some(&action), Some(&callback)).check();

        // The previous callbacks are released once native code has the new ones.
        let previous = self.current_action.replace(Some((action, callback)));
        drop(previous);
    }

    fn observe<T: 'static>(
        &self,
        observable: Rc<dyn ferroui_base::reactive::IObservable<T>>,
        update: impl Fn(&FrnMenuItem, T) + 'static,
    ) {
        let this = self.weak_self.clone();
        let subscription = observable.subscribe_fn(move |value| {
            if let Some(this) = this.upgrade() {
                update(&this, value);
            }
        });
        self.property_disposables.borrow_mut().push(subscription);
    }

    pub(crate) fn initialize(&self) {
        let Some(item) = self.managed_menu_item.cast::<NativeMenuItem>() else {
            return;
        };

        self.update_title(item.header().as_deref());

        self.update_tool_tip(item.tool_tip().as_deref());

        self.update_gesture(item.gesture().as_ref());

        self.update_action(Some(item.clone()));

        self.update_toggle_type(item.toggle_type());

        self.update_icon(item.icon().as_ref());

        self.update_is_checked(item.is_checked());

        self.update_is_visible(item.is_visible());

        self.observe(item.get_observable(NativeMenuItem::header_property()), |this, x| {
            this.update_title(x.as_deref())
        });

        self.observe(item.get_observable(NativeMenuItem::tool_tip_property()), |this, x| {
            this.update_tool_tip(x.as_deref())
        });

        self.observe(item.get_observable(NativeMenuItem::gesture_property()), |this, x| {
            this.update_gesture(x.as_ref())
        });

        self.observe(item.get_observable(NativeMenuItem::command_property()), |this, _| {
            this.update_action(this.managed_menu_item.cast::<NativeMenuItem>())
        });

        self.observe(item.get_observable(NativeMenuItem::toggle_type_property()), |this, x| {
            this.update_toggle_type(x)
        });

        self.observe(item.get_observable(NativeMenuItem::is_checked_property()), |this, x| {
            this.update_is_checked(x)
        });

        self.observe(item.get_observable(NativeMenuItem::is_visible_property()), |this, x| {
            this.update_is_visible(x)
        });

        self.observe(item.get_observable(NativeMenuItem::icon_property()), |this, x| this.update_icon(x.as_ref()));
    }

    pub(crate) fn deinitialize(&self) {
        let sub_menu = self.sub_menu.borrow_mut().take();
        if let Some(sub_menu) = sub_menu {
            self.native.set_sub_menu(None).check();
            sub_menu.deinitialise();
        }

        let property_disposables = std::mem::take(&mut *self.property_disposables.borrow_mut());
        for disposable in property_disposables {
            disposable.dispose();
        }
        let current_action = self.current_action.borrow_mut().take();
        drop(current_action);
    }

    /// Creates, updates or removes the native submenu to match the menu of
    /// `item`.
    ///
    /// # Panics
    /// Panics when `item` is not the item this native item mirrors.
    pub(crate) fn update(
        &self,
        exporter: &Rc<FerroNativeMenuExporter>,
        factory: &dyn IMenuFactory,
        item: &Ref<NativeMenuItem>,
    ) {
        if !self.managed_menu_item.ptr_eq(item) {
            panic!("The item does not match the menuitem being updated.");
        }

        match item.menu() {
            Some(menu) => {
                let sub_menu = self.sub_menu.borrow().clone();
                let sub_menu = match sub_menu {
                    Some(sub_menu) => sub_menu,
                    None => {
                        let sub_menu = FrnMenu::create(factory);

                        if menu.get_value(MacOSNativeMenuCommands::is_services_submenu_property()) {
                            factory.set_services_menu(sub_menu.native());
                        }

                        sub_menu.initialize(exporter, &menu, item.header().as_deref());

                        self.native.set_sub_menu(Some(sub_menu.native())).check();
                        *self.sub_menu.borrow_mut() = Some(sub_menu.clone());
                        sub_menu
                    }
                };

                sub_menu.update(factory, &menu);
            }
            None => {
                let sub_menu = self.sub_menu.borrow_mut().take();
                if let Some(sub_menu) = sub_menu {
                    sub_menu.deinitialise();

                    self.native.set_sub_menu(None).check();
                }
            }
        }
    }
}
