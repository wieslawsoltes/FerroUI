//! Port of `TrayIconImpl.cs`: an icon in the notification area of the task
//! bar. The shell sends its mouse messages to the message window of the
//! platform; a right click shows the menu of the icon as a managed menu in
//! a window of its own.
//!
//! The icons of the thread are kept by their identifier, as the reference
//! keeps them in a static; the table holds them weakly, and an icon that
//! is dropped without being disposed removes itself from the notification
//! area (the reference does that in a finalizer).

use crate::interop::unmanaged_methods::WindowsMessage;

/// Custom Win32 window messages for the NotifyIcon
pub(crate) struct CustomWindowsMessage;

impl CustomWindowsMessage {
    #[allow(dead_code)]
    pub(crate) const WM_TRAYICON: u32 = WindowsMessage::WM_APP + 1024;
    pub(crate) const WM_TRAYMOUSE: u32 = WindowsMessage::WM_USER + 1024;
}

/// What a message of the mouse over a tray icon asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrayMouseAction {
    Clicked,
    RightClicked,
    None,
}

/// The action of a `WM_TRAYMOUSE` message from its second parameter, which
/// is the mouse message the shell saw over the icon.
pub(crate) fn tray_mouse_action(l_param: isize) -> TrayMouseAction {
    match l_param as i32 as u32 {
        WindowsMessage::WM_LBUTTONUP => TrayMouseAction::Clicked,
        WindowsMessage::WM_RBUTTONUP => TrayMouseAction::RightClicked,
        _ => TrayMouseAction::None,
    }
}

#[cfg(windows)]
pub(crate) use imp::TrayIconImpl;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::icon_impl::IconImpl;
    use crate::interop::unmanaged_methods::{
        change_window_message_filter_ex, get_cursor_pos, get_dpi_for_monitor, has_get_dpi_for_monitor, monitor_from_point,
        register_window_message, sh_app_bar_message, shell_notify_icon, AppBarMessage, MessageFilterFlag, APPBARDATA, MONITOR,
        MONITOR_DPI_TYPE, NIF, NIM, NOTIFYICONDATA, POINT,
    };
    use crate::interop::win32_icon::Win32Icon;
    use crate::platform_constants::PlatformConstants;
    use crate::win32_native_to_managed_menu_exporter::Win32NativeToManagedMenuExporter;
    use crate::win32_platform::Win32Platform;
    use ferroui_base::input::InputElementImpl;
    use ferroui_base::interactivity::InteractiveImpl;
    use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::media::imaging::WriteableBitmap;
    use ferroui_base::platform::{AlphaFormat, PixelFormat};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::{
        ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, PixelPoint, PixelSize, Point, Rect, Ref,
        Size, StaticType, StyledElementImpl, TypeInfo, Vector, VisualImpl, WeakRef,
    };
    use ferroui_controls::generators::RecycleKey;
    use ferroui_controls::platform::{INativeMenuExporter, ITrayIconImpl, IWindowIconImpl};
    use ferroui_controls::primitives::popup_positioning::{
        IManagedPopupPositionerPopup, IPopupPositioner, ManagedPopupPositioner, ManagedPopupPositionerScreenInfo, PopupAnchor,
        PopupGravity, PopupPositionerConstraintAdjustment, PopupPositionerParameters,
    };
    use ferroui_controls::primitives::{SelectingItemsControlImpl, TemplatedControlImpl};
    use ferroui_controls::{
        create_container_for_native_item, ContentControlImpl, Control, ControlImpl, ItemsControlImpl, ItemsControlImplExt,
        ItemsSource, MenuBaseImpl,
        MenuFlyoutPresenter, SizeToContent, TopLevelImpl, TopLevelImplExt, Window, WindowBaseImpl, WindowDecorations, WindowImpl,
        WindowResizeReason, WindowTransparencyLevel, WindowTransparencyLevelCollection,
    };
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::rc::{Rc, Weak};

    thread_local! {
        static EMPTY_ICON: RefCell<Option<Rc<Win32Icon>>> = const { RefCell::new(None) };
        static NEXT_UNIQUE_ID: Cell<i32> = const { Cell::new(0) };
        static TASK_BAR_MONITOR: Cell<isize> = const { Cell::new(0) };
        static TRAY_ICONS: RefCell<HashMap<i32, Weak<TrayIconImpl>>> = RefCell::new(HashMap::new());
        static WM_TASKBARCREATED: u32 = register_window_message("TaskbarCreated");
    }

    pub(crate) struct TrayIconImpl {
        unique_id: i32,
        icon_added: Cell<bool>,
        icon_impl: RefCell<Option<Rc<IconImpl>>>,
        icon_stale: Cell<bool>,
        icon: RefCell<Option<Win32Icon>>,
        tooltip_text: RefCell<Option<String>>,
        exporter: Rc<Win32NativeToManagedMenuExporter>,
        disposed_value: Cell<bool>,
        on_clicked: RefCell<Option<Rc<dyn Fn()>>>,
    }

    impl TrayIconImpl {
        pub(crate) fn change_window_message_filter(hwnd: isize) {
            change_window_message_filter_ex(hwnd, WM_TASKBARCREATED.with(|message| *message), MessageFilterFlag::MSGFLT_ALLOW);
        }

        pub(crate) fn new() -> Rc<Self> {
            Self::find_task_bar_monitor();

            let unique_id = NEXT_UNIQUE_ID.with(|next| {
                next.set(next.get() + 1);
                next.get()
            });

            let this = Rc::new(Self {
                unique_id,
                icon_added: Cell::new(false),
                icon_impl: RefCell::new(None),
                icon_stale: Cell::new(false),
                icon: RefCell::new(None),
                tooltip_text: RefCell::new(None),
                exporter: Rc::new(Win32NativeToManagedMenuExporter::new()),
                disposed_value: Cell::new(false),
                on_clicked: RefCell::new(None),
            });

            TRAY_ICONS.with(|icons| icons.borrow_mut().insert(unique_id, Rc::downgrade(&this)));

            this
        }

        /// The tray icons of the thread that are alive.
        fn tray_icons() -> Vec<Rc<TrayIconImpl>> {
            TRAY_ICONS.with(|icons| icons.borrow().values().filter_map(Weak::upgrade).collect())
        }

        pub(crate) fn proc_wnd(hwnd: isize, msg: u32, w_param: usize, l_param: isize) {
            match msg {
                CustomWindowsMessage::WM_TRAYMOUSE => {
                    let value = TRAY_ICONS.with(|icons| icons.borrow().get(&(w_param as i32)).and_then(Weak::upgrade));
                    if let Some(value) = value {
                        value.wnd_proc(hwnd, msg, w_param, l_param);
                    }
                }
                WindowsMessage::WM_DISPLAYCHANGE => {
                    Self::find_task_bar_monitor();
                    for tray in Self::tray_icons() {
                        if tray.icon_added.get() {
                            tray.icon_stale.set(true);
                            tray.update_icon(false);
                        }
                    }
                }
                _ => {
                    if msg == WM_TASKBARCREATED.with(|message| *message) {
                        Self::find_task_bar_monitor();
                        for tray in Self::tray_icons() {
                            if tray.icon_added.get() {
                                tray.update_icon(true);
                                tray.update_icon(false);
                            }
                        }
                    }
                }
            }
        }

        fn find_task_bar_monitor() {
            let mut task_bar_data = APPBARDATA::default();
            if sh_app_bar_message(AppBarMessage::ABM_GETTASKBARPOS, &mut task_bar_data) != 0 {
                TASK_BAR_MONITOR.with(|monitor| {
                    monitor.set(monitor_from_point(
                        POINT { x: task_bar_data.rc.left, y: task_bar_data.rc.top },
                        MONITOR::MONITOR_DEFAULTTOPRIMARY,
                    ))
                });
            }
        }

        fn update_icon(&self, remove: bool) {
            let mut new_icon: Option<Win32Icon> = None;
            if self.icon_stale.get() {
                if let Some(icon_impl) = self.icon_impl.borrow().as_ref() {
                    let scaling = Self::get_task_bar_mon_scaling_or_default();
                    match icon_impl.load_small_icon(scaling) {
                        Ok(icon) => new_icon = Some(icon),
                        Err(error) => {
                            if let Some(log) = Logger::try_get(LogEventLevel::Warning, LogArea::WIN32_PLATFORM) {
                                log.log(None, &format!("The icon of a tray icon could not be made: {error}"));
                            }
                        }
                    }
                }
            }

            let mut icon_data = NOTIFYICONDATA { h_wnd: Win32Platform::instance().handle(), u_id: self.unique_id, ..Default::default() };

            if !remove {
                icon_data.u_flags = (NIF::TIP | NIF::MESSAGE | NIF::ICON).bits();
                icon_data.u_callback_message = CustomWindowsMessage::WM_TRAYMOUSE as i32;
                let handle = if self.icon_stale.get() {
                    new_icon.as_ref().map(Win32Icon::handle)
                } else {
                    self.icon.borrow().as_ref().map(Win32Icon::handle)
                };
                icon_data.h_icon = handle.unwrap_or_else(|| Self::get_or_create_empty_icon().handle());
                icon_data.set_tip(self.tooltip_text.borrow().as_deref().unwrap_or(""));

                if !self.icon_added.get() {
                    shell_notify_icon(NIM::ADD, &icon_data);
                    self.icon_added.set(true);
                } else {
                    shell_notify_icon(NIM::MODIFY, &icon_data);
                }
            } else {
                icon_data.u_flags = 0;
                shell_notify_icon(NIM::DELETE, &icon_data);
                self.icon_added.set(false);
            }

            if self.icon_stale.get() {
                if let Some(icon) = self.icon.borrow_mut().take() {
                    icon.dispose();
                }
                *self.icon.borrow_mut() = new_icon;
                self.icon_stale.set(false);
            }
        }

        fn get_or_create_empty_icon() -> Rc<Win32Icon> {
            EMPTY_ICON.with(|empty_icon| {
                empty_icon
                    .borrow_mut()
                    .get_or_insert_with(|| {
                        let bitmap = WriteableBitmap::new(
                            PixelSize::new(32, 32),
                            Vector::new(96.0, 96.0),
                            Some(PixelFormat::BGRA8888),
                            Some(AlphaFormat::Unpremul),
                        );
                        let icon = Win32Icon::from_bitmap(&bitmap, PixelPoint::default());
                        bitmap.dispose();
                        Rc::new(icon)
                    })
                    .clone()
            })
        }

        fn get_task_bar_mon_scaling_or_default() -> f64 {
            if has_get_dpi_for_monitor() && Win32Platform::windows_version() > PlatformConstants::WINDOWS8_1 {
                if let Some((dpi_x, dpi_y)) =
                    get_dpi_for_monitor(TASK_BAR_MONITOR.with(Cell::get), MONITOR_DPI_TYPE::MDT_EFFECTIVE_DPI)
                {
                    debug_assert!(dpi_x == dpi_y);
                    return f64::from(dpi_x) / 96.0;
                }
            }

            1.0
        }

        fn wnd_proc(&self, _hwnd: isize, _msg: u32, _w_param: usize, l_param: isize) {
            // Determine the type of message and call the matching event handlers
            match tray_mouse_action(l_param) {
                TrayMouseAction::Clicked => {
                    let on_clicked = self.on_clicked.borrow().clone();
                    if let Some(on_clicked) = on_clicked {
                        on_clicked();
                    }
                }
                TrayMouseAction::RightClicked => self.on_right_clicked(),
                TrayMouseAction::None => {}
            }
        }

        fn on_right_clicked(&self) {
            let Some(menu) = self.exporter.get_native_menu() else {
                return;
            };
            let items = menu.items();
            if items.count() == 0 {
                return;
            }

            let tray_menu = TrayPopupRoot::new();
            tray_menu.set_name(Some(format!("FerroTrayPopupRoot_{}", self.tooltip_text.borrow().as_deref().unwrap_or(""))));
            tray_menu.set_window_decorations(WindowDecorations::None);
            tray_menu.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
            tray_menu.set_background(None);
            tray_menu.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![
                WindowTransparencyLevel::transparent(),
            ]));
            let presenter = TrayIconMenuFlyoutPresenter::new();
            presenter.set_items_source(Some(ItemsSource::from(Rc::new(items))));
            tray_menu.set_content(Some(Control::boxed(presenter)));

            let pt = get_cursor_pos();

            tray_menu.set_position(PixelPoint::new(pt.x, pt.y));

            tray_menu.show();
        }

        fn dispose_core(&self) {
            if !self.disposed_value.get() {
                self.update_icon(true);

                // A table that is gone belongs to a thread that is ending.
                let _ = TRAY_ICONS.try_with(|icons| icons.borrow_mut().remove(&self.unique_id));
                if let Some(icon) = self.icon.borrow_mut().take() {
                    icon.dispose();
                }

                self.disposed_value.set(true);
            }
        }
    }

    impl ITrayIconImpl for TrayIconImpl {
        fn set_icon(&self, icon: Option<Rc<dyn IWindowIconImpl>>) {
            let icon_impl = icon.and_then(|icon| match IconImpl::from_window_icon(&icon) {
                Ok(icon) => Some(icon),
                Err(error) => panic!("The icon of the tray icon could not be read: {error}"),
            });
            *self.icon_impl.borrow_mut() = icon_impl;
            self.icon_stale.set(true);
            self.update_icon(!self.icon_added.get());
        }

        fn set_is_visible(&self, visible: bool) {
            self.update_icon(!visible);
        }

        fn set_tool_tip_text(&self, text: Option<&str>) {
            *self.tooltip_text.borrow_mut() = text.map(str::to_string);
            self.update_icon(!self.icon_added.get());
        }

        fn menu_exporter(&self) -> Option<Rc<dyn INativeMenuExporter>> {
            Some(self.exporter.clone())
        }

        fn on_clicked(&self) -> Option<Rc<dyn Fn()>> {
            self.on_clicked.borrow().clone()
        }

        fn set_on_clicked(&self, value: Option<Rc<dyn Fn()>>) {
            *self.on_clicked.borrow_mut() = value;
        }
    }

    impl IDisposable for TrayIconImpl {
        fn dispose(&self) {
            self.dispose_core();
        }
    }

    impl Drop for TrayIconImpl {
        fn drop(&mut self) {
            // The platform may be gone when the last tray icon is dropped
            // at the end of the thread; the shell removes the icon of a
            // window that no longer exists by itself.
            if !self.disposed_value.get() && Win32Platform::try_instance().is_some() {
                self.dispose_core();
            }
        }
    }

    // ---- the presenter of the menu -------------------------------------

    #[repr(C)]
    struct TrayIconMenuFlyoutPresenter {
        base: MenuFlyoutPresenter,
    }

    ferro_class!(TrayIconMenuFlyoutPresenter: MenuFlyoutPresenter);

    ferro_impl_classes!(
        TrayIconMenuFlyoutPresenter: FerroObjectImpl,
        VisualImpl,
        LayoutableImpl,
        InteractiveImpl,
        InputElementImpl,
        ControlImpl,
        TemplatedControlImpl,
        SelectingItemsControlImpl
    );

    impl TrayIconMenuFlyoutPresenter {
        fn new() -> Ref<Self> {
            instantiate(Self { base: MenuFlyoutPresenter::construct() })
        }
    }

    impl StyledElementImpl for TrayIconMenuFlyoutPresenter {
        fn style_key_override(_this: &Self) -> &'static TypeInfo {
            <MenuFlyoutPresenter as StaticType>::TYPE
        }
    }

    impl MenuBaseImpl for TrayIconMenuFlyoutPresenter {
        fn close(this: &Self) {
            // DefaultMenuInteractionHandler calls this
            let host = this.find_logical_ancestor_of_type::<TrayPopupRoot>(false);
            if let Some(host) = host {
                this.set_selected_index(-1);
                host.close();
            }
        }
    }

    impl ItemsControlImpl for TrayIconMenuFlyoutPresenter {
        fn create_container_for_item_override(
            this: &Self,
            item: &Option<BoxedValue>,
            index: i32,
            recycle_key: Option<RecycleKey>,
        ) -> Ref<Control> {
            create_container_for_native_item(item, index, recycle_key.clone())
                .unwrap_or_else(|| Self::parent_create_container_for_item_override(this, item, index, recycle_key))
        }
    }

    // ---- the window of the menu ----------------------------------------

    #[repr(C)]
    struct TrayPopupRoot {
        base: Window,
        positioner: ManagedPopupPositioner,
        positioner_helper: Rc<TrayIconManagedPopupPositionerPopupImplHelper>,
        deactivated: RefCell<Option<Rc<dyn IDisposable>>>,
    }

    ferro_class!(TrayPopupRoot: Window);

    ferro_impl_classes!(
        TrayPopupRoot: FerroObjectImpl,
        StyledElementImpl,
        VisualImpl,
        InteractiveImpl,
        InputElementImpl,
        ControlImpl,
        TemplatedControlImpl,
        ContentControlImpl,
        WindowBaseImpl,
        WindowImpl
    );

    impl TrayPopupRoot {
        fn new() -> Ref<Self> {
            let positioner_helper = Rc::new(TrayIconManagedPopupPositionerPopupImplHelper::new());
            let this = instantiate(Self {
                base: Window::construct(ferroui_controls::platform::PlatformManager::create_window()),
                positioner: ManagedPopupPositioner::new(positioner_helper.clone()),
                positioner_helper,
                deactivated: RefCell::new(None),
            });
            *this.positioner_helper.target.borrow_mut() = Some(this.downgrade());
            this.set_topmost(true);

            let weak = this.downgrade();
            *this.deactivated.borrow_mut() = Some(this.deactivated(move || {
                if let Some(this) = weak.upgrade() {
                    this.tray_popup_root_deactivated();
                }
            }));

            this.set_show_in_taskbar(false);

            this.set_show_activated(true);

            this
        }

        fn tray_popup_root_deactivated(&self) {
            self.close();
        }

        fn move_resize(&self, position: PixelPoint, size: Size, _scaling: f64) {
            if let Some(platform_impl) = Window::platform_impl(self) {
                platform_impl.move_(position);
                platform_impl.resize(size, WindowResizeReason::Layout);
            }
        }
    }

    impl TopLevelImpl for TrayPopupRoot {
        fn on_closed(this: &Self) {
            Self::parent_on_closed(this);
            if let Some(deactivated) = this.deactivated.borrow_mut().take() {
                deactivated.dispose();
            }
            this.positioner_helper.dispose();
        }
    }

    impl LayoutableImpl for TrayPopupRoot {
        fn arrange_core(this: &Self, final_rect: Rect) {
            Self::parent_arrange_core(this, final_rect);

            let scaling = this.screens().primary().map_or(1.0, |screen| screen.scaling());
            let mut parameters = PopupPositionerParameters::default();
            parameters.set_anchor(PopupAnchor::TOP_LEFT);
            parameters.set_gravity(PopupGravity::BOTTOM_RIGHT);
            parameters.anchor_rectangle = Rect::from_position_size(this.position().to_point(scaling), Size::new(1.0, 1.0));
            parameters.size = final_rect.size();
            parameters.constraint_adjustment =
                PopupPositionerConstraintAdjustment::FLIP_X | PopupPositionerConstraintAdjustment::FLIP_Y;
            this.positioner.update(parameters);
        }
    }

    struct TrayIconManagedPopupPositionerPopupImplHelper {
        /// The window the helper moves; the reference gives the helper a
        /// delegate of the window, which here is found through a weak
        /// reference set once the window exists.
        target: RefCell<Option<WeakRef<TrayPopupRoot>>>,
        hidden_window: Ref<Window>,
    }

    impl TrayIconManagedPopupPositionerPopupImplHelper {
        fn new() -> Self {
            Self { target: RefCell::new(None), hidden_window: Window::new() }
        }

        fn dispose(&self) {
            self.hidden_window.close();
        }
    }

    impl IManagedPopupPositionerPopup for TrayIconManagedPopupPositionerPopupImplHelper {
        fn screens(&self) -> Vec<ManagedPopupPositionerScreenInfo> {
            self.hidden_window
                .screens()
                .all()
                .iter()
                .map(|s| ManagedPopupPositionerScreenInfo::new(s.bounds().to_rect(1.0), s.bounds().to_rect(1.0)))
                .collect()
        }

        fn parent_client_area_screen_geometry(&self) -> Rect {
            if let Some(screen) = self.hidden_window.screens().primary() {
                let bounds = screen.bounds();
                let point = bounds.top_left();
                let size = bounds.size();
                return Rect::new(
                    f64::from(point.x),
                    f64::from(point.y),
                    f64::from(size.width) * screen.scaling(),
                    f64::from(size.height) * screen.scaling(),
                );
            }
            Rect::default()
        }

        fn move_and_resize(&self, device_point: Point, virtual_size: Size) {
            let target = self.target.borrow().as_ref().and_then(WeakRef::upgrade);
            if let Some(target) = target {
                target.move_resize(PixelPoint::new(device_point.x as i32, device_point.y as i32), virtual_size, self.scaling());
            }
        }

        fn scaling(&self) -> f64 {
            self.hidden_window.screens().primary().map_or(1.0, |screen| screen.scaling())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_custom_messages_are_above_the_ranges_of_the_application_and_the_user() {
        assert_eq!(CustomWindowsMessage::WM_TRAYICON, 0x8000 + 1024);
        assert_eq!(CustomWindowsMessage::WM_TRAYMOUSE, 0x0400 + 1024);
    }

    /// The structure is the version of `NOTIFYICONDATAW` that ends with
    /// the flags of the balloon: 952 bytes on a 64-bit system, with the
    /// members where the system headers have them.
    #[cfg(target_pointer_width = "64")]
    #[test]
    fn the_notification_data_has_the_layout_of_the_system() {
        use crate::interop::unmanaged_methods::{APPBARDATA, NOTIFYICONDATA};
        use std::mem::{offset_of, size_of};

        assert_eq!(size_of::<NOTIFYICONDATA>(), 952);
        assert_eq!(NOTIFYICONDATA::default().cb_size, 952);
        assert_eq!(offset_of!(NOTIFYICONDATA, h_wnd), 8);
        assert_eq!(offset_of!(NOTIFYICONDATA, u_id), 16);
        assert_eq!(offset_of!(NOTIFYICONDATA, u_flags), 20);
        assert_eq!(offset_of!(NOTIFYICONDATA, u_callback_message), 24);
        assert_eq!(offset_of!(NOTIFYICONDATA, h_icon), 32);
        assert_eq!(offset_of!(NOTIFYICONDATA, sz_tip), 40);
        assert_eq!(offset_of!(NOTIFYICONDATA, dw_state), 296);
        assert_eq!(offset_of!(NOTIFYICONDATA, sz_info), 304);
        assert_eq!(offset_of!(NOTIFYICONDATA, u_timeout_or_version), 816);
        assert_eq!(offset_of!(NOTIFYICONDATA, sz_info_title), 820);
        assert_eq!(offset_of!(NOTIFYICONDATA, dw_info_flags), 948);

        assert_eq!(size_of::<APPBARDATA>(), 48);
        assert_eq!(APPBARDATA::default().cb_size, 48);
        assert_eq!(offset_of!(APPBARDATA, rc), 24);
        assert_eq!(offset_of!(APPBARDATA, l_param), 40);
    }

    #[test]
    fn the_tip_is_cut_between_characters_and_keeps_its_terminator() {
        use crate::interop::unmanaged_methods::NOTIFYICONDATA;

        let mut data = NOTIFYICONDATA::default();
        data.set_tip("tip");
        assert_eq!(&data.sz_tip[..4], &[0x74, 0x69, 0x70, 0]);

        data.set_tip(&"a".repeat(300));
        assert!(data.sz_tip[..127].iter().all(|&unit| unit == 0x61));
        assert_eq!(data.sz_tip[127], 0);

        // 126 units, then a character of two units, which does not fit
        // before the terminator.
        data.set_tip(&format!("{}\u{1F600}", "a".repeat(126)));
        assert_eq!(data.sz_tip[125], 0x61);
        assert_eq!(data.sz_tip[126], 0);

        data.set_tip("");
        assert!(data.sz_tip.iter().all(|&unit| unit == 0));
    }

    #[test]
    fn a_button_released_over_the_icon_is_a_click_or_a_right_click() {
        assert_eq!(tray_mouse_action(WindowsMessage::WM_LBUTTONUP as isize), TrayMouseAction::Clicked);
        assert_eq!(tray_mouse_action(WindowsMessage::WM_RBUTTONUP as isize), TrayMouseAction::RightClicked);
        assert_eq!(tray_mouse_action(WindowsMessage::WM_MOUSEMOVE as isize), TrayMouseAction::None);
        assert_eq!(tray_mouse_action(WindowsMessage::WM_LBUTTONDOWN as isize), TrayMouseAction::None);
    }
}
