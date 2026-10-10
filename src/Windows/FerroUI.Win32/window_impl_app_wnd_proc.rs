//! The window procedure of a window: the messages of the system are turned
//! into the callbacks of the window contract and into raw input.
//!
//! The first part of the file is what reads the parameters of messages and
//! computes with them; it is compiled, and tested, on every host. The
//! procedure itself is the second part.

use crate::interop::unmanaged_methods::{
    ModifierKeys, SizeCommand, WindowStyles, WindowsMessage, MINMAXINFO, RECT,
};
use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::RawInputModifiers;
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Size, Thickness};
use ferroui_controls::WindowState;

/// One notch of a mouse wheel.
pub(crate) const WHEEL_DELTA: f64 = 120.0;

/// The low 32 bits of a message parameter, as a signed value.
pub(crate) fn to_int32(value: isize) -> i32 {
    (value as i64 & 0xffff_ffff) as u32 as i32
}

pub(crate) fn high_word(param: i32) -> i32 {
    param >> 16
}

/// The system command of `WM_SYSCOMMAND`: the low four bits of the
/// parameter are used by the system.
pub(crate) fn get_sys_command(w_param: usize) -> i32 {
    to_int32(w_param as isize) & 0xfff0
}

/// The point of a message parameter that packs two signed 16-bit
/// coordinates.
pub(crate) fn point_from_l_param(l_param: isize) -> PixelPoint {
    PixelPoint::new(i32::from((to_int32(l_param) & 0xffff) as u16 as i16), i32::from((to_int32(l_param) >> 16) as i16))
}

/// The point of a message parameter in device independent pixels.
pub(crate) fn dip_from_l_param(l_param: isize, render_scaling: f64) -> Point {
    let point = point_from_l_param(l_param);
    Point::new(f64::from(point.x), f64::from(point.y)) / render_scaling
}

/// The client size of `WM_SIZE`, in device pixels.
pub(crate) fn client_size_from_l_param(l_param: isize) -> Size {
    Size::new(f64::from(to_int32(l_param) & 0xffff), f64::from(to_int32(l_param) >> 16))
}

/// The state `WM_SIZE` reports for a window.
pub(crate) fn window_state_from_size_command(
    size: i32,
    is_full_screen_active: bool,
    shown: bool,
    last_window_state: WindowState,
) -> WindowState {
    match size {
        SizeCommand::MAXIMIZED => WindowState::Maximized,
        SizeCommand::MINIMIZED => WindowState::Minimized,
        _ if is_full_screen_active => WindowState::FullScreen,
        // Ignore state changes for unshown windows. We always tell Windows that we are hidden
        // until shown, so the OS value should be ignored while we are in the unshown state.
        _ if !shown => last_window_state,
        _ => WindowState::Normal,
    }
}

/// The DPI of `WM_DPICHANGED`: the high word of the parameter.
pub(crate) fn dpi_from_w_param(w_param: usize) -> u32 {
    (w_param as u32) >> 16
}

/// The scrolled distance of a wheel message, in notches.
pub(crate) fn wheel_delta_from_w_param(w_param: usize) -> f64 {
    f64::from(to_int32(w_param as isize) >> 16) / WHEEL_DELTA
}

/// The modifiers of a mouse message: the keyboard modifiers and the mouse
/// buttons the message says are down.
pub(crate) fn get_mouse_modifiers(w_param: usize, keyboard_modifiers: RawInputModifiers) -> RawInputModifiers {
    let keys = ModifierKeys::from_bits_retain(to_int32(w_param as isize));
    let mut modifiers = keyboard_modifiers;

    if keys.contains(ModifierKeys::MK_LBUTTON) {
        modifiers |= RawInputModifiers::LEFT_MOUSE_BUTTON;
    }

    if keys.contains(ModifierKeys::MK_RBUTTON) {
        modifiers |= RawInputModifiers::RIGHT_MOUSE_BUTTON;
    }

    if keys.contains(ModifierKeys::MK_MBUTTON) {
        modifiers |= RawInputModifiers::MIDDLE_MOUSE_BUTTON;
    }

    if keys.contains(ModifierKeys::MK_XBUTTON1) {
        modifiers |= RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON;
    }

    if keys.contains(ModifierKeys::MK_XBUTTON2) {
        modifiers |= RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON;
    }

    modifiers
}

/// The raw event of a button message of the client area, or of the
/// non-client area; `None` for another message.
pub(crate) fn button_event_type(message: u32, w_param: usize) -> Option<RawPointerEventType> {
    let x_button = |first: RawPointerEventType, second: RawPointerEventType| {
        if high_word(to_int32(w_param as isize)) == 1 {
            first
        } else {
            second
        }
    };

    Some(match message {
        WindowsMessage::WM_LBUTTONDOWN => RawPointerEventType::LeftButtonDown,
        WindowsMessage::WM_RBUTTONDOWN => RawPointerEventType::RightButtonDown,
        WindowsMessage::WM_MBUTTONDOWN => RawPointerEventType::MiddleButtonDown,
        WindowsMessage::WM_XBUTTONDOWN => x_button(RawPointerEventType::XButton1Down, RawPointerEventType::XButton2Down),

        WindowsMessage::WM_LBUTTONUP => RawPointerEventType::LeftButtonUp,
        WindowsMessage::WM_RBUTTONUP => RawPointerEventType::RightButtonUp,
        WindowsMessage::WM_MBUTTONUP => RawPointerEventType::MiddleButtonUp,
        WindowsMessage::WM_XBUTTONUP => x_button(RawPointerEventType::XButton1Up, RawPointerEventType::XButton2Up),

        WindowsMessage::WM_NCLBUTTONDOWN => RawPointerEventType::NonClientLeftButtonDown,
        WindowsMessage::WM_NCRBUTTONDOWN => RawPointerEventType::RightButtonDown,
        WindowsMessage::WM_NCMBUTTONDOWN => RawPointerEventType::MiddleButtonDown,
        WindowsMessage::WM_NCXBUTTONDOWN => x_button(RawPointerEventType::XButton1Down, RawPointerEventType::XButton2Down),

        _ => return None,
    })
}

/// Whether the extra information of a mouse message says that the system
/// made the message up for touch or pen input.
pub(crate) fn should_ignore_touch_emulated_message(extra_info: i64) -> bool {
    // MI_WP_SIGNATURE
    // https://docs.microsoft.com/en-us/windows/win32/tablet/system-events-and-mouse-messages
    const MARKER: i64 = 0xFF51_5700;

    (extra_info & MARKER) == MARKER
}

/// The text of a `WM_CHAR` message, which carries one UTF-16 code unit.
///
/// A character outside the basic plane arrives as two messages, a high and
/// a low surrogate. The reference delivers each as a string of one unit; a
/// Rust string cannot hold half a pair, so the high surrogate is kept in
/// `pending_high_surrogate` and the pair is delivered as one text when the
/// low surrogate arrives. A high surrogate that is not followed by a low
/// one is dropped.
pub(crate) fn text_from_char_message(pending_high_surrogate: &mut Option<u16>, unit: u16) -> Option<String> {
    if (0xD800..0xDC00).contains(&unit) {
        *pending_high_surrogate = Some(unit);
        return None;
    }

    match pending_high_surrogate.take() {
        Some(high) if (0xDC00..0xE000).contains(&unit) => String::from_utf16(&[high, unit]).ok(),
        _ => String::from_utf16(&[unit]).ok(),
    }
}

/// The rectangle `WM_PAINT` asks for, in device independent pixels.
pub(crate) fn paint_rect(rc_paint: RECT, render_scaling: f64) -> Rect {
    let f = render_scaling;
    let r = rc_paint;
    Rect::new(
        f64::from(r.left) / f,
        f64::from(r.top) / f,
        f64::from(r.right - r.left) / f,
        f64::from(r.bottom - r.top) / f,
    )
}

/// Writes the minimum and maximum sizes of a window into the structure of
/// `WM_GETMINMAXINFO`: the sizes are client sizes in device independent
/// pixels, and the structure wants window sizes in device pixels.
pub(crate) fn apply_min_max_track_sizes(
    mmi: &mut MINMAXINFO,
    min_size: Size,
    max_size: Size,
    render_scaling: f64,
    border_thickness: Thickness,
) {
    if min_size.width > 0.0 {
        mmi.pt_min_track_size.x =
            ((min_size.width * render_scaling) + border_thickness.left + border_thickness.right) as i32;
    }

    if min_size.height > 0.0 {
        mmi.pt_min_track_size.y =
            ((min_size.height * render_scaling) + border_thickness.top + border_thickness.bottom) as i32;
    }

    if !max_size.width.is_infinite() && max_size.width > 0.0 {
        mmi.pt_max_track_size.x =
            ((max_size.width * render_scaling) + border_thickness.left + border_thickness.right) as i32;
    }

    if !max_size.height.is_infinite() && max_size.height > 0.0 {
        mmi.pt_max_track_size.y =
            ((max_size.height * render_scaling) + border_thickness.top + border_thickness.bottom) as i32;
    }
}

/// Gets the expected maximized rect for a window without a caption.
/// `adjust` grows a rectangle by the frame of the given styles
/// (`AdjustWindowRectEx` at the DPI of the window).
pub(crate) fn get_captionless_maximized_rect(
    style: WindowStyles,
    working_area: PixelRect,
    adjust: &dyn Fn(&mut RECT, WindowStyles, WindowStyles),
) -> PixelRect {
    let mut x = working_area.x;
    let mut y = working_area.y;
    let mut cx = working_area.width;
    let mut cy = working_area.height;

    let mut border_thickness = RECT::default();

    let mut adjusted_style = style & !WindowStyles::WS_CAPTION;

    if style.contains(WindowStyles::WS_BORDER) {
        adjusted_style |= WindowStyles::WS_BORDER;
    }

    if style.contains(WindowStyles::WS_CAPTION) {
        adjusted_style |= WindowStyles::WS_THICKFRAME;
    }

    adjust(&mut border_thickness, adjusted_style, WindowStyles::empty());

    x += border_thickness.left;
    y += border_thickness.top;
    cx += -border_thickness.left + border_thickness.right;
    cy += -border_thickness.top + border_thickness.bottom;

    PixelRect::new(x, y, cx, cy)
}

/// The frame `WM_NCCALCSIZE` has to take off the proposed window rectangle
/// of a window whose client area extends into its frame: the resize
/// borders stay outside the client area, the caption does not.
pub(crate) fn extended_client_area_border_thickness(
    style: WindowStyles,
    is_maximized: bool,
    adjust: &dyn Fn(&mut RECT, WindowStyles, WindowStyles),
) -> RECT {
    let mut border_thickness = RECT::default();

    // We told Windows we have a caption, but since we're actually extending into it, it should not be taken into account.
    if style.contains(WindowStyles::WS_CAPTION) {
        if is_maximized {
            adjust(
                &mut border_thickness,
                style & !WindowStyles::WS_CAPTION | WindowStyles::WS_BORDER | WindowStyles::WS_THICKFRAME,
                WindowStyles::empty(),
            );
        } else {
            // There's no extra border on top with WS_CAPTION: it's part of the caption.
            adjust(&mut border_thickness, style, WindowStyles::empty());
            border_thickness.top = 0;
        }
    } else if style.contains(WindowStyles::WS_BORDER) {
        if is_maximized {
            adjust(&mut border_thickness, style, WindowStyles::empty());
        } else {
            adjust(&mut border_thickness, style, WindowStyles::empty());

            let mut thin_border_thickness = RECT::default();
            adjust(&mut thin_border_thickness, style & !WindowStyles::WS_THICKFRAME, WindowStyles::empty());
            border_thickness.top = thin_border_thickness.top;
        }
    } else {
        adjust(&mut border_thickness, style, WindowStyles::empty());
    }

    border_thickness
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::input::{KeyInterop, WindowsKeyboardDevice};
    use crate::interop::unmanaged_methods::*;
    use crate::win32_platform::Win32Platform;
    use crate::win32_type_extensions::Win32TypeExtensions;
    use crate::window_impl::WindowImpl;
    use ferroui_base::input::raw::{
        IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
        RawTextInputEventArgs,
    };
    use ferroui_base::input::{IInputDevice, Key, KeyDeviceType, PhysicalKey};
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};
    use ferroui_base::Vector;
    use ferroui_controls::platform::{IScreenImpl, ScreensBaseImplExt};
    use ferroui_controls::{WindowCloseReason, WindowDecorations, WindowResizeReason};
    use std::rc::Rc;

    /// A raw event the procedure made from a message, with what the
    /// procedure reads back from it once it has been delivered.
    enum RawEvent {
        Key(Rc<RawKeyEventArgs>),
        Other(Rc<dyn IRawInputEventArgs>),
    }

    impl RawEvent {
        fn args(&self) -> Rc<dyn IRawInputEventArgs> {
            match self {
                RawEvent::Key(args) => args.clone(),
                RawEvent::Other(args) => args.clone(),
            }
        }
    }

    impl WindowImpl {
        /// The procedure of the application: handles the messages the
        /// toolkit is interested in and leaves the rest to the system.
        pub(crate) fn app_wnd_proc(&self, hwnd: isize, msg: u32, w_param: usize, l_param: isize) -> isize {
            let timestamp = u64::from(get_message_time() as u32);
            let mut e: Option<RawEvent> = None;
            let mut should_take_focus = false;
            let message = msg;

            match message {
                WindowsMessage::WM_ACTIVATE => {
                    let wa = to_int32(w_param as isize) & 0xffff;

                    match wa {
                        WindowActivate::WA_ACTIVE | WindowActivate::WA_CLICKACTIVE => {
                            self.invoke_activated();
                            // The input method of the keyboard layout
                            // follows the active window: stage 2 (Imm32).
                        }

                        WindowActivate::WA_INACTIVE => {
                            self.invoke_deactivated();
                        }

                        _ => {}
                    }

                    return 0;
                }

                WindowsMessage::WM_NCCALCSIZE if to_int32(w_param as isize) == 1 => {
                    if self.window_properties().decorations == WindowDecorations::None {
                        return 0;
                    }

                    // When the client area is extended into the frame, we are still requesting the standard styles matching
                    // the wanted decorations (such as WS_CAPTION or WS_BORDER) along with window bounds larger than the client size.
                    // This allows the window to have the standard resize borders *outside* of the client area.
                    // The logic for this lies in the Resize() method.
                    //
                    // After this happens, WM_NCCALCSIZE provides us with a new window area matching those requested bounds.
                    // We need to adjust that area back to our preferred client area, keeping the resize borders around it.
                    //
                    // The same logic applies when the window gets maximized, the only difference being that Windows chose
                    // the final bounds instead of us.
                    if self.is_client_area_extended() {
                        let placement = get_window_placement(hwnd);
                        if placement.show_cmd != ShowWindowCommand::SHOW_MINIMIZED {
                            // SAFETY: the parameter of the WM_NCCALCSIZE
                            // message this procedure is processing, with a
                            // true wParam.
                            let mut rect = unsafe { read_nc_calc_size_rect(l_param) };

                            let style = WindowStyles::from_bits_retain(get_window_long(self.hwnd(), WindowLongParam::GWL_STYLE));
                            let border_thickness = extended_client_area_border_thickness(
                                style,
                                placement.show_cmd == ShowWindowCommand::SHOW_MAXIMIZED,
                                &|rect, style, ex_style| self.adjust_window_rect(rect, style, ex_style),
                            );

                            rect.left -= border_thickness.left;
                            rect.top -= border_thickness.top;
                            rect.right -= border_thickness.right;
                            rect.bottom -= border_thickness.bottom;

                            // SAFETY: as above.
                            unsafe { write_nc_calc_size_rect(l_param, rect) };

                            return 0;
                        }
                    }
                }

                WindowsMessage::WM_CLOSE => {
                    let prevent_closing = self.invoke_closing(WindowCloseReason::WindowClosing);
                    if prevent_closing == Some(true) {
                        return 0;
                    }

                    self.before_close_cleanup(false);

                    // Used to distinguish between programmatic and regular close requests.
                    self.set_is_close_requested(true);
                }

                WindowsMessage::WM_DESTROY => {
                    // The first and foremost thing to do - notify the TopLevel
                    self.invoke_closed();

                    // The automation provider of the window, the input
                    // method and the drop target are released here by the
                    // reference: they arrive with their stages.

                    self.framebuffer().dispose();

                    //Window doesn't exist anymore
                    self.on_destroyed();

                    //Free other resources
                    self.dispose_impl();

                    // Schedule cleanup of anything that requires window to be destroyed
                    if let Some(this) = self.this() {
                        Dispatcher::ui_thread().post_local(move || this.after_close_cleanup(), DispatcherPriority::default());
                    }
                    return 0;
                }

                WindowsMessage::WM_DPICHANGED => {
                    let dpi = dpi_from_w_param(w_param);
                    // SAFETY: the parameter of the WM_DPICHANGED message
                    // this procedure is processing.
                    let new_display_rect = unsafe { read_rect(l_param) };
                    self.set_dpi(dpi);
                    self.invoke_scaling_changed(self.scaling());

                    let old = self.set_resize_reason(WindowResizeReason::DpiChange);
                    set_window_pos(
                        hwnd,
                        0,
                        new_display_rect.left,
                        new_display_rect.top,
                        new_display_rect.right - new_display_rect.left,
                        new_display_rect.bottom - new_display_rect.top,
                        SetWindowPosFlags::SWP_NOZORDER | SetWindowPosFlags::SWP_NOACTIVATE,
                    );
                    self.set_resize_reason(old);

                    return 0;
                }

                // WM_GETICON: the window has no icon before stage 2 builds
                // the icons, so the message is left to the system, as the
                // reference does for a window without an icon.
                WindowsMessage::WM_KEYDOWN => {
                    e = self.try_create_raw_key_event_args(RawKeyEventType::KeyDown, timestamp, w_param, l_param, true);
                }

                WindowsMessage::WM_SYSKEYDOWN => {
                    e = self.try_create_raw_key_event_args(RawKeyEventType::KeyDown, timestamp, w_param, l_param, false);
                }

                WindowsMessage::WM_SYSCOMMAND => {
                    // Disable system handling of Alt/F10 menu keys.
                    if get_sys_command(w_param) == SysCommands::SC_KEYMENU && high_word(to_int32(l_param)) <= 0 {
                        return 0;
                    }
                }

                WindowsMessage::WM_MENUCHAR => {
                    // mute the system beep
                    return (MenuCharParam::MNC_CLOSE << 16) as isize;
                }

                WindowsMessage::WM_KEYUP => {
                    e = self.try_create_raw_key_event_args(RawKeyEventType::KeyUp, timestamp, w_param, l_param, true);
                    self.set_ignore_wm_char(false);
                }

                WindowsMessage::WM_SYSKEYUP => {
                    e = self.try_create_raw_key_event_args(RawKeyEventType::KeyUp, timestamp, w_param, l_param, false);
                    self.set_ignore_wm_char(false);
                }

                WindowsMessage::WM_CHAR => {
                    // (While an input method composes, the reference
                    // ignores the message: stage 2.)

                    // Ignore control chars and chars that were handled in WM_KEYDOWN.
                    if to_int32(w_param as isize) >= 32 && !self.ignore_wm_char() {
                        // Deviation (DEVIATIONS.md, Windows platform
                        // backend): the two halves of a surrogate pair
                        // are delivered as one text.
                        if let Some(text) = self.text_from_char_message(to_int32(w_param as isize) as u16) {
                            let device: Rc<dyn IInputDevice> = WindowsKeyboardDevice::instance();
                            e = Some(RawEvent::Other(Rc::new(RawTextInputEventArgs::new(
                                device,
                                timestamp,
                                self.owner(),
                                text,
                            ))));
                        }
                    }
                }

                WindowsMessage::WM_LBUTTONDOWN
                | WindowsMessage::WM_RBUTTONDOWN
                | WindowsMessage::WM_MBUTTONDOWN
                | WindowsMessage::WM_XBUTTONDOWN => {
                    if !self.is_mouse_in_pointer_enabled() {
                        should_take_focus = self.should_take_focus_on_click();
                        if !should_ignore_touch_emulated_message(get_message_extra_info() as i64) {
                            let type_ = button_event_type(message, w_param).expect("a button message");
                            e = Some(self.pointer_event(timestamp, type_, self.dip_from_l_param(l_param), w_param));
                        }
                    }
                }

                WindowsMessage::WM_LBUTTONUP
                | WindowsMessage::WM_RBUTTONUP
                | WindowsMessage::WM_MBUTTONUP
                | WindowsMessage::WM_XBUTTONUP => {
                    if !self.is_mouse_in_pointer_enabled()
                        && !should_ignore_touch_emulated_message(get_message_extra_info() as i64)
                    {
                        let type_ = button_event_type(message, w_param).expect("a button message");
                        e = Some(self.pointer_event(timestamp, type_, self.dip_from_l_param(l_param), w_param));
                    }
                }

                WindowsMessage::WM_MOUSEMOVE => {
                    if !self.is_mouse_in_pointer_enabled()
                        && !should_ignore_touch_emulated_message(get_message_extra_info() as i64)
                    {
                        if !self.tracking_mouse() {
                            track_mouse_leave(self.hwnd());
                        }

                        let point = self.dip_from_l_param(l_param);

                        // The points between this move and the previous one
                        // (the history of the mouse of the system) are
                        // attached lazily by the reference: stage 2.
                        e = Some(self.pointer_event(timestamp, RawPointerEventType::Move, point, w_param));
                    }
                }

                WindowsMessage::WM_MOUSEWHEEL => {
                    if !self.is_mouse_in_pointer_enabled() {
                        e = Some(self.wheel_event(
                            timestamp,
                            self.point_to_client_impl(point_from_l_param(l_param)),
                            Vector::new(0.0, wheel_delta_from_w_param(w_param)),
                            w_param,
                        ));
                    }
                }

                WindowsMessage::WM_MOUSEHWHEEL => {
                    if !self.is_mouse_in_pointer_enabled() {
                        e = Some(self.wheel_event(
                            timestamp,
                            self.point_to_client_impl(point_from_l_param(l_param)),
                            Vector::new(-wheel_delta_from_w_param(w_param), 0.0),
                            w_param,
                        ));
                    }
                }

                WindowsMessage::WM_MOUSELEAVE => {
                    if !self.is_mouse_in_pointer_enabled() {
                        self.set_tracking_mouse(false);
                        e = Some(self.pointer_event_with_modifiers(
                            timestamp,
                            RawPointerEventType::LeaveWindow,
                            Point::new(-1.0, -1.0),
                            WindowsKeyboardDevice::modifiers(),
                        ));
                    }
                }

                // covers WM_CANCELMODE which sends WM_CAPTURECHANGED in DefWindowProc
                WindowsMessage::WM_CAPTURECHANGED => {
                    if !self.is_mouse_in_pointer_enabled() && !self.is_our_window(l_param) {
                        self.set_tracking_mouse(false);
                        e = Some(self.pointer_event_with_modifiers(
                            timestamp,
                            RawPointerEventType::CancelCapture,
                            Point::new(-1.0, -1.0),
                            WindowsKeyboardDevice::modifiers(),
                        ));
                    }
                }

                WindowsMessage::WM_NCLBUTTONDOWN
                | WindowsMessage::WM_NCRBUTTONDOWN
                | WindowsMessage::WM_NCMBUTTONDOWN
                | WindowsMessage::WM_NCXBUTTONDOWN => {
                    if !self.is_mouse_in_pointer_enabled() {
                        let type_ = button_event_type(message, w_param).expect("a button message");
                        e = Some(self.pointer_event(
                            timestamp,
                            type_,
                            self.point_to_client_impl(point_from_l_param(l_param)),
                            w_param,
                        ));
                    }
                }

                // WM_TOUCH and the pointer messages (WM_POINTERDOWN, ...,
                // WM_POINTERWHEEL): touch and pen input are stage 2. The
                // messages are left to the system, which turns them into
                // the mouse messages this procedure ignores as emulated.
                WindowsMessage::WM_NCPAINT => {
                    if !self.has_full_decorations() {
                        return 0;
                    }
                }

                WindowsMessage::WM_NCACTIVATE => {
                    if !self.has_full_decorations() {
                        return 1;
                    }
                }

                WindowsMessage::WM_PAINT => {
                    if let Some(paint) = begin_paint(self.hwnd()) {
                        let rect = paint_rect(paint.rc_paint(), self.scaling());
                        self.invoke_paint(rect);
                        drop(paint);
                    }

                    return 0;
                }

                WindowsMessage::WM_ENTERSIZEMOVE => {
                    self.set_resize_reason(WindowResizeReason::User);
                }

                WindowsMessage::WM_SHOWWINDOW => {
                    self.on_show_hide_message(w_param != 0);
                }

                WindowsMessage::WM_SIZE => {
                    let size = to_int32(w_param as isize);

                    let window_state = window_state_from_size_command(
                        size,
                        self.is_full_screen_active(),
                        self.shown(),
                        self.last_window_state(),
                    );

                    let state_changed = window_state != self.last_window_state();
                    self.set_last_window_state(window_state);

                    if size == SizeCommand::RESTORED || size == SizeCommand::MAXIMIZED {
                        let client_size = client_size_from_l_param(l_param);
                        self.invoke_resized(client_size / self.scaling(), self.resize_reason());
                    }

                    if is_window_visible(self.hwnd()) && !self.shown() {
                        self.set_shown(true);
                    }

                    if state_changed {
                        let mut new_window_properties = self.window_properties();

                        new_window_properties.window_state = window_state;

                        self.update_window_properties(new_window_properties, false);

                        self.invoke_window_state_changed(window_state);

                        if self.is_client_area_extended() {
                            self.extend_client_area();

                            self.invoke_extend_client_area_to_decorations_changed(true);
                        }
                    } else if window_state == WindowState::Maximized && self.is_client_area_extended() {
                        self.extend_client_area();

                        self.invoke_extend_client_area_to_decorations_changed(true);
                    }

                    return 0;
                }

                WindowsMessage::WM_EXITSIZEMOVE => {
                    self.set_resize_reason(WindowResizeReason::Unspecified);
                }

                WindowsMessage::WM_MOVE => {
                    self.invoke_position_changed(self.position_impl());
                    return 0;
                }

                WindowsMessage::WM_GETMINMAXINFO => {
                    // SAFETY: the parameter of the WM_GETMINMAXINFO message
                    // this procedure is processing.
                    let mut mmi = unsafe { read_min_max_info(l_param) };

                    self.set_max_track_size(mmi.pt_max_track_size);

                    // A window without a caption (i.e. None and BorderOnly decorations) maximizes to the whole screen
                    // by default. Adjust that to the screen's working area instead.
                    let style = self.get_style();
                    if !style.contains(WindowStyles::WS_CAPTION | WindowStyles::WS_THICKFRAME) {
                        if let Some(screen) = self.screen().screen_from_hwnd(self.hwnd(), MONITOR::MONITOR_DEFAULTTONEAREST) {
                            let working_area = (*screen).as_ref().working_area();
                            let maximized_rect = get_captionless_maximized_rect(style, working_area, &|rect, style, ex_style| {
                                self.adjust_window_rect(rect, style, ex_style)
                            });
                            // We aren't changing ptMaxPosition because its coordinates must always target the primary screen.
                            // We can't do that, since the work area might not be the same for all screens.
                            // Instead, only set the desired max size here.
                            // WM_WINDOWPOSCHANGING moves the window to the correct position.
                            mmi.pt_max_size.x = maximized_rect.width;
                            mmi.pt_max_size.y = maximized_rect.height;
                        }
                    }

                    apply_min_max_track_sizes(&mut mmi, self.min_size(), self.max_size(), self.scaling(), self.border_thickness());

                    // SAFETY: as above.
                    unsafe { write_min_max_info(l_param, &mmi) };
                    return 0;
                }

                WindowsMessage::WM_WINDOWPOSCHANGING => {
                    // SAFETY: the parameter of the WM_WINDOWPOSCHANGING
                    // message this procedure is processing.
                    let pos = unsafe { read_window_pos(l_param) };
                    let style = self.get_style();
                    let flags = SetWindowPosFlags::from_bits_retain(pos.flags);

                    // A window without a caption (i.e. None and BorderOnly decorations) maximizes to the whole screen
                    // by default. Adjust that to the screen's working area instead.
                    if !style.contains(WindowStyles::WS_CAPTION | WindowStyles::WS_THICKFRAME)
                        && style.contains(WindowStyles::WS_MAXIMIZE)
                        && !self.is_full_screen_active()
                        && !flags.contains(SetWindowPosFlags::SWP_NOMOVE | SetWindowPosFlags::SWP_NOSIZE)
                    {
                        let placement = get_window_placement(self.hwnd());

                        // Prefer ScreenFromRect with the window's restored bounds.
                        // If the window was minimized, ScreenFromHwnd won't return the correct monitor at this point.
                        let working_area = self
                            .screen()
                            .screen_from_rect(placement.normal_position.to_pixel_rect())
                            .map(|screen| screen.working_area())
                            .or_else(|| {
                                self.screen()
                                    .screen_from_hwnd(self.hwnd(), MONITOR::MONITOR_DEFAULTTONEAREST)
                                    .map(|screen| (*screen).as_ref().working_area())
                            });

                        if let Some(working_area) = working_area {
                            let maximized_rect = get_captionless_maximized_rect(style, working_area, &|rect, style, ex_style| {
                                self.adjust_window_rect(rect, style, ex_style)
                            });
                            // SAFETY: as above.
                            unsafe {
                                write_window_pos_bounds(
                                    l_param,
                                    maximized_rect.x,
                                    maximized_rect.y,
                                    maximized_rect.width,
                                    maximized_rect.height,
                                )
                            };
                            return 0;
                        }
                    }
                }

                WindowsMessage::WM_DISPLAYCHANGE => {
                    self.on_display_change();
                    self.screen().on_changed();

                    Win32Platform::update_timer_fps();

                    return 0;
                }

                WindowsMessage::WM_KILLFOCUS => {
                    // (While an input method composes, the reference
                    // delays the notification until the composition ends:
                    // stage 2.)
                    self.invoke_lost_focus();
                }

                // WM_INPUTLANGCHANGE and the WM_IME_* messages: the input
                // method of the backend (Imm32) is stage 2; the messages
                // get the default processing of the system, which shows
                // its own composition window.
                //
                // WM_GETOBJECT: the automation provider of the window
                // belongs to the automation project of the backend.
                WindowsMessage::WM_WINDOWPOSCHANGED => {
                    // SAFETY: the parameter of the WM_WINDOWPOSCHANGED
                    // message this procedure is processing.
                    let win_pos = unsafe { read_window_pos(l_param) };
                    if (win_pos.flags & SetWindowPosFlags::SWP_SHOWWINDOW.bits()) != 0 {
                        self.on_show_hide_message(true);
                    } else if (win_pos.flags & SetWindowPosFlags::SWP_HIDEWINDOW.bits()) != 0 {
                        self.on_show_hide_message(false);
                    }
                }

                _ => {}
            }

            if should_take_focus {
                set_focus(self.hwnd());
            }

            if let Some(e) = e {
                if let Some(input) = self.input_callback() {
                    let args = e.args();
                    input(args.clone());

                    if message == WindowsMessage::WM_KEYDOWN {
                        if matches!(&e, RawEvent::Key(key_args) if key_args.key() == Key::ImeProcessed) {
                            self.set_ignore_wm_char(true);
                        } else {
                            // Handling a WM_KEYDOWN message should cause the subsequent WM_CHAR message to
                            // be ignored. This should be safe to do as WM_CHAR should only be produced in
                            // response to the call to TranslateMessage/DispatchMessage after a WM_KEYDOWN
                            // is handled.
                            self.set_ignore_wm_char(args.handled());
                        }
                    }

                    if args.handled() {
                        return 0;
                    }
                }
            }

            def_window_proc(hwnd, msg, w_param, l_param)
        }

        pub(crate) fn is_our_window(&self, hwnd: isize) -> bool {
            if hwnd == 0 {
                return false;
            }

            if hwnd == self.hwnd() {
                return true;
            }

            WindowImpl::is_our_window_global(hwnd)
        }

        pub(crate) fn on_show_hide_message(&self, shown: bool) {
            self.set_shown(shown);

            if self.is_client_area_extended() {
                self.extend_client_area();
            }
        }

        fn dip_from_l_param(&self, l_param: isize) -> Point {
            dip_from_l_param(l_param, self.scaling())
        }

        fn mouse_input_device(&self) -> Rc<dyn IInputDevice> {
            self.mouse_device().device().clone()
        }

        fn pointer_event(&self, timestamp: u64, type_: RawPointerEventType, position: Point, w_param: usize) -> RawEvent {
            self.pointer_event_with_modifiers(
                timestamp,
                type_,
                position,
                get_mouse_modifiers(w_param, WindowsKeyboardDevice::modifiers()),
            )
        }

        fn pointer_event_with_modifiers(
            &self,
            timestamp: u64,
            type_: RawPointerEventType,
            position: Point,
            modifiers: RawInputModifiers,
        ) -> RawEvent {
            RawEvent::Other(Rc::new(RawPointerEventArgs::new(
                self.mouse_input_device(),
                timestamp,
                self.owner(),
                type_,
                position,
                modifiers,
            )))
        }

        fn wheel_event(&self, timestamp: u64, position: Point, delta: Vector, w_param: usize) -> RawEvent {
            RawEvent::Other(Rc::new(RawMouseWheelEventArgs::new(
                self.mouse_input_device(),
                timestamp,
                self.owner(),
                position,
                delta,
                get_mouse_modifiers(w_param, WindowsKeyboardDevice::modifiers()),
            )))
        }

        fn try_create_raw_key_event_args(
            &self,
            event_type: RawKeyEventType,
            timestamp: u64,
            w_param: usize,
            l_param: isize,
            use_key_symbol: bool,
        ) -> Option<RawEvent> {
            let virtual_key = to_int32(w_param as isize);
            let key_data = to_int32(l_param);
            let key = KeyInterop::key_from_virtual_key(virtual_key, key_data);
            let physical_key = KeyInterop::physical_key_from_virtual_key(virtual_key, key_data);

            // Avoid calling GetKeySymbol() for WM_SYSKEYDOWN/UP:
            // it ultimately calls ToUnicodeEx, which corrupts keyboard state for system key events.
            // Use MapVirtualKey-based fallback instead: it's layout-aware without touching keyboard state.
            let key_symbol = if use_key_symbol {
                KeyInterop::get_key_symbol(virtual_key, key_data)
            } else {
                KeyInterop::get_key_symbol_from_virtual_key(virtual_key)
            };

            if key == Key::None
                && physical_key == PhysicalKey::None
                && key_symbol.as_deref().is_none_or(|symbol| symbol.trim().is_empty())
            {
                return None;
            }

            let device: Rc<dyn IInputDevice> = WindowsKeyboardDevice::instance();
            Some(RawEvent::Key(Rc::new(RawKeyEventArgs::new(
                device,
                timestamp,
                self.owner(),
                event_type,
                key,
                WindowsKeyboardDevice::modifiers(),
                physical_key,
                key_symbol,
                KeyDeviceType::Keyboard,
            ))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A message parameter that packs two 16-bit values.
    fn make_l_param(low: i16, high: i16) -> isize {
        ((i32::from(high as u16) << 16) | i32::from(low as u16)) as u32 as isize
    }

    #[test]
    fn the_low_32_bits_of_a_parameter_are_read_as_a_signed_value() {
        assert_eq!(to_int32(5), 5);
        assert_eq!(to_int32(0xffff_ffff_u32 as isize), -1);
        assert_eq!(to_int32(-1), -1);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(to_int32(0x1_0000_0005), 5);
        assert_eq!(high_word(0x0002_0001), 2);
        assert_eq!(high_word(to_int32(make_l_param(0, -1))), -1);
    }

    #[test]
    fn the_coordinates_of_a_message_are_signed() {
        assert_eq!(point_from_l_param(make_l_param(10, 20)), PixelPoint::new(10, 20));
        // A point left of or above the primary monitor.
        assert_eq!(point_from_l_param(make_l_param(-300, -5)), PixelPoint::new(-300, -5));
        assert_eq!(dip_from_l_param(make_l_param(150, -30), 1.5), Point::new(100.0, -20.0));
    }

    #[test]
    fn the_client_size_of_a_size_message() {
        assert_eq!(client_size_from_l_param(make_l_param(800, 600)), Size::new(800.0, 600.0));
        assert_eq!(client_size_from_l_param(make_l_param(0, 0)), Size::new(0.0, 0.0));
    }

    #[test]
    fn the_system_command_ignores_the_low_four_bits() {
        use crate::interop::unmanaged_methods::SysCommands;
        assert_eq!(get_sys_command(0xF100), SysCommands::SC_KEYMENU);
        assert_eq!(get_sys_command(0xF012), SysCommands::SC_MOVE);
        assert_eq!(get_sys_command(0xF063), SysCommands::SC_CLOSE);
    }

    #[test]
    fn the_state_of_a_size_message() {
        let state = window_state_from_size_command;
        assert_eq!(state(SizeCommand::MAXIMIZED, false, true, WindowState::Normal), WindowState::Maximized);
        assert_eq!(state(SizeCommand::MINIMIZED, true, true, WindowState::FullScreen), WindowState::Minimized);
        // A restored window that covers its monitor without a frame.
        assert_eq!(state(SizeCommand::RESTORED, true, true, WindowState::Normal), WindowState::FullScreen);
        // A window that was never shown keeps the state it was given.
        assert_eq!(state(SizeCommand::RESTORED, false, false, WindowState::Maximized), WindowState::Maximized);
        assert_eq!(state(SizeCommand::RESTORED, false, true, WindowState::Maximized), WindowState::Normal);
    }

    #[test]
    fn the_dpi_of_a_dpi_change_is_the_high_word() {
        assert_eq!(dpi_from_w_param((144 << 16) | 144), 144);
        assert_eq!(dpi_from_w_param((96 << 16) | 96), 96);
    }

    #[test]
    fn a_wheel_notch_is_one() {
        let w_param = |delta: i16, keys: u16| ((u32::from(delta as u16) << 16) | u32::from(keys)) as usize;
        assert_eq!(wheel_delta_from_w_param(w_param(120, 0)), 1.0);
        assert_eq!(wheel_delta_from_w_param(w_param(-120, 0x0008)), -1.0);
        assert_eq!(wheel_delta_from_w_param(w_param(30, 0)), 0.25);
    }

    #[test]
    fn the_buttons_of_a_mouse_message_join_the_keyboard_modifiers() {
        assert_eq!(get_mouse_modifiers(0, RawInputModifiers::NONE), RawInputModifiers::NONE);
        assert_eq!(
            get_mouse_modifiers(ModifierKeys::MK_LBUTTON.bits() as usize, RawInputModifiers::SHIFT),
            RawInputModifiers::SHIFT | RawInputModifiers::LEFT_MOUSE_BUTTON
        );
        let all = ModifierKeys::MK_LBUTTON
            | ModifierKeys::MK_RBUTTON
            | ModifierKeys::MK_MBUTTON
            | ModifierKeys::MK_XBUTTON1
            | ModifierKeys::MK_XBUTTON2;
        assert_eq!(
            get_mouse_modifiers(all.bits() as usize, RawInputModifiers::NONE),
            RawInputModifiers::LEFT_MOUSE_BUTTON
                | RawInputModifiers::RIGHT_MOUSE_BUTTON
                | RawInputModifiers::MIDDLE_MOUSE_BUTTON
                | RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON
                | RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON
        );
        // The control and shift flags of the message are not read: the
        // keyboard modifiers come from the keyboard state.
        assert_eq!(
            get_mouse_modifiers((ModifierKeys::MK_CONTROL | ModifierKeys::MK_SHIFT).bits() as usize, RawInputModifiers::NONE),
            RawInputModifiers::NONE
        );
    }

    #[test]
    fn the_raw_events_of_the_button_messages() {
        let x1 = 1usize << 16;
        let x2 = 2usize << 16;
        let cases = [
            (WindowsMessage::WM_LBUTTONDOWN, 0, RawPointerEventType::LeftButtonDown),
            (WindowsMessage::WM_RBUTTONDOWN, 0, RawPointerEventType::RightButtonDown),
            (WindowsMessage::WM_MBUTTONDOWN, 0, RawPointerEventType::MiddleButtonDown),
            (WindowsMessage::WM_XBUTTONDOWN, x1, RawPointerEventType::XButton1Down),
            (WindowsMessage::WM_XBUTTONDOWN, x2, RawPointerEventType::XButton2Down),
            (WindowsMessage::WM_LBUTTONUP, 0, RawPointerEventType::LeftButtonUp),
            (WindowsMessage::WM_RBUTTONUP, 0, RawPointerEventType::RightButtonUp),
            (WindowsMessage::WM_MBUTTONUP, 0, RawPointerEventType::MiddleButtonUp),
            (WindowsMessage::WM_XBUTTONUP, x1, RawPointerEventType::XButton1Up),
            (WindowsMessage::WM_XBUTTONUP, x2, RawPointerEventType::XButton2Up),
            (WindowsMessage::WM_NCLBUTTONDOWN, 0, RawPointerEventType::NonClientLeftButtonDown),
            (WindowsMessage::WM_NCRBUTTONDOWN, 0, RawPointerEventType::RightButtonDown),
            (WindowsMessage::WM_NCMBUTTONDOWN, 0, RawPointerEventType::MiddleButtonDown),
            (WindowsMessage::WM_NCXBUTTONDOWN, x1, RawPointerEventType::XButton1Down),
            (WindowsMessage::WM_NCXBUTTONDOWN, x2, RawPointerEventType::XButton2Down),
        ];
        for (message, w_param, expected) in cases {
            assert_eq!(button_event_type(message, w_param), Some(expected), "message {message:#x}");
        }
        assert_eq!(button_event_type(WindowsMessage::WM_MOUSEMOVE, 0), None);
    }

    #[test]
    fn mouse_messages_made_from_touch_are_recognised() {
        assert!(should_ignore_touch_emulated_message(0xFF51_5700));
        assert!(should_ignore_touch_emulated_message(0xFF51_5780));
        assert!(!should_ignore_touch_emulated_message(0));
        assert!(!should_ignore_touch_emulated_message(0x0051_5700));
    }

    #[test]
    fn a_character_message_is_one_code_unit_and_a_pair_is_one_text() {
        let mut pending = None;
        assert_eq!(text_from_char_message(&mut pending, u16::from(b'a')).as_deref(), Some("a"));
        assert_eq!(text_from_char_message(&mut pending, 0x00E9).as_deref(), Some("\u{e9}"));

        // U+1F600 arrives as D83D, DE00.
        assert_eq!(text_from_char_message(&mut pending, 0xD83D), None);
        assert_eq!(pending, Some(0xD83D));
        assert_eq!(text_from_char_message(&mut pending, 0xDE00).as_deref(), Some("\u{1F600}"));
        assert_eq!(pending, None);

        // A high surrogate that is not followed by a low one is dropped.
        assert_eq!(text_from_char_message(&mut pending, 0xD83D), None);
        assert_eq!(text_from_char_message(&mut pending, u16::from(b'b')).as_deref(), Some("b"));
        // A low surrogate on its own is no text.
        assert_eq!(text_from_char_message(&mut pending, 0xDE00), None);
    }

    #[test]
    fn the_paint_rectangle_is_scaled_to_device_independent_pixels() {
        let rect = paint_rect(RECT { left: 30, top: 60, right: 330, bottom: 210 }, 1.5);
        assert_eq!(rect, Rect::new(20.0, 40.0, 200.0, 100.0));
    }

    #[test]
    fn the_track_sizes_are_client_sizes_plus_the_frame() {
        let mut mmi = MINMAXINFO::default();
        mmi.pt_min_track_size = crate::interop::unmanaged_methods::POINT { x: 136, y: 39 };
        mmi.pt_max_track_size = crate::interop::unmanaged_methods::POINT { x: 3000, y: 2000 };
        let border = Thickness::new(8.0, 31.0, 8.0, 8.0);

        // No limits: the structure keeps what the system proposed.
        let mut unchanged = mmi;
        apply_min_max_track_sizes(&mut unchanged, Size::new(0.0, 0.0), Size::new(f64::INFINITY, f64::INFINITY), 1.0, border);
        assert_eq!(unchanged, mmi);

        apply_min_max_track_sizes(&mut mmi, Size::new(200.0, 100.0), Size::new(800.0, f64::INFINITY), 1.5, border);
        assert_eq!((mmi.pt_min_track_size.x, mmi.pt_min_track_size.y), (316, 189));
        assert_eq!((mmi.pt_max_track_size.x, mmi.pt_max_track_size.y), (1216, 2000));
    }

    /// A frame of 8 pixels for a sizing border, 1 for a thin border, and a
    /// caption of 23 pixels above it.
    fn adjust(rect: &mut RECT, style: WindowStyles, _ex_style: WindowStyles) {
        let border = if style.contains(WindowStyles::WS_THICKFRAME) {
            8
        } else if style.contains(WindowStyles::WS_BORDER) {
            1
        } else {
            0
        };
        let caption = if style.contains(WindowStyles::WS_CAPTION) { 23 } else { 0 };
        rect.left -= border;
        rect.right += border;
        rect.bottom += border;
        rect.top -= border + caption;
    }

    #[test]
    fn a_window_without_a_caption_maximizes_to_the_working_area() {
        let working_area = PixelRect::new(0, 0, 1920, 1040);

        // No decorations: exactly the working area.
        let style = WindowStyles::WS_CLIPCHILDREN;
        assert_eq!(get_captionless_maximized_rect(style, working_area, &adjust), working_area);

        // A border only: the border is outside of the working area, as the
        // frame of a maximized window is.
        let style = WindowStyles::WS_BORDER;
        assert_eq!(get_captionless_maximized_rect(style, working_area, &adjust), PixelRect::new(-1, -1, 1922, 1042));
    }

    #[test]
    fn the_frame_taken_off_an_extended_client_area() {
        let full = WindowStyles::WS_CAPTION | WindowStyles::WS_THICKFRAME | WindowStyles::WS_SYSMENU;

        // Restored with a caption: the sizing borders stay, the top (the
        // caption and its border) is client area.
        let border = extended_client_area_border_thickness(full, false, &adjust);
        assert_eq!(border, RECT { left: -8, top: 0, right: 8, bottom: 8 });

        // Maximized with a caption: the frame without the caption on every
        // side, which is off screen.
        let border = extended_client_area_border_thickness(full, true, &adjust);
        assert_eq!(border, RECT { left: -8, top: -8, right: 8, bottom: 8 });

        // A border without a caption, restored: the top is the thin border.
        let border_only = WindowStyles::WS_BORDER | WindowStyles::WS_THICKFRAME;
        let border = extended_client_area_border_thickness(border_only, false, &adjust);
        assert_eq!(border, RECT { left: -8, top: -1, right: 8, bottom: 8 });
    }
}
