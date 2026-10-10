//! The window procedure of a window: the messages of the system are turned
//! into the callbacks of the window contract and into raw input.
//!
//! The first part of the file is what reads the parameters of messages and
//! computes with them; it is compiled, and tested, on every host. The
//! procedure itself is the second part.

use crate::interop::unmanaged_methods::{
    ModifierKeys, PenFlags, PointerButtonChangeType, PointerFlags, PointerInputType, SizeCommand, TouchInputFlags,
    WindowStyles, WindowsMessage, MINMAXINFO, MOUSEMOVEPOINT, POINTER_INFO, RECT,
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

/// The most entries of the history of a pointer that are read for one
/// message.
pub(crate) const MAX_POINTER_HISTORY_SIZE: u32 = 512;

/// The most points of the history of the mouse that are read for one move.
pub(crate) const MOUSE_HISTORY_SIZE: usize = 64;

/// The modifiers of a pointer message: the keyboard modifiers and the
/// buttons the flags of the pointer say are down.
pub(crate) fn get_pointer_input_modifiers(flags: u32, keyboard_modifiers: RawInputModifiers) -> RawInputModifiers {
    let flags = PointerFlags::from_bits_retain(flags);
    let mut modifiers = keyboard_modifiers;

    if flags.contains(PointerFlags::POINTER_FLAG_FIRSTBUTTON) {
        modifiers |= RawInputModifiers::LEFT_MOUSE_BUTTON;
    }

    if flags.contains(PointerFlags::POINTER_FLAG_SECONDBUTTON) {
        modifiers |= RawInputModifiers::RIGHT_MOUSE_BUTTON;
    }

    if flags.contains(PointerFlags::POINTER_FLAG_THIRDBUTTON) {
        modifiers |= RawInputModifiers::MIDDLE_MOUSE_BUTTON;
    }

    if flags.contains(PointerFlags::POINTER_FLAG_FOURTHBUTTON) {
        modifiers |= RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON;
    }

    if flags.contains(PointerFlags::POINTER_FLAG_FIFTHBUTTON) {
        modifiers |= RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON;
    }

    modifiers
}

/// The modifiers the flags of a pen stand for.
pub(crate) fn get_pen_modifiers(pen_flags: u32) -> RawInputModifiers {
    let flags = PenFlags::from_bits_retain(pen_flags);
    let mut modifiers = RawInputModifiers::NONE;

    if flags.contains(PenFlags::PEN_FLAGS_BARREL) {
        modifiers |= RawInputModifiers::PEN_BARREL_BUTTON;
    }
    if flags.contains(PenFlags::PEN_FLAGS_ERASER) {
        modifiers |= RawInputModifiers::PEN_ERASER;
    }
    if flags.contains(PenFlags::PEN_FLAGS_INVERTED) {
        modifiers |= RawInputModifiers::PEN_INVERTED;
    }

    modifiers
}

/// The raw event of a change of the buttons of a pointer.
pub(crate) fn to_event_type(button_change_type: u32, is_touch: bool) -> RawPointerEventType {
    match button_change_type {
        PointerButtonChangeType::POINTER_CHANGE_FIRSTBUTTON_DOWN if is_touch => RawPointerEventType::TouchBegin,
        PointerButtonChangeType::POINTER_CHANGE_FIRSTBUTTON_DOWN => RawPointerEventType::LeftButtonDown,
        PointerButtonChangeType::POINTER_CHANGE_SECONDBUTTON_DOWN => RawPointerEventType::RightButtonDown,
        PointerButtonChangeType::POINTER_CHANGE_THIRDBUTTON_DOWN => RawPointerEventType::MiddleButtonDown,
        PointerButtonChangeType::POINTER_CHANGE_FOURTHBUTTON_DOWN => RawPointerEventType::XButton1Down,
        PointerButtonChangeType::POINTER_CHANGE_FIFTHBUTTON_DOWN => RawPointerEventType::XButton2Down,

        PointerButtonChangeType::POINTER_CHANGE_FIRSTBUTTON_UP if is_touch => RawPointerEventType::TouchEnd,
        PointerButtonChangeType::POINTER_CHANGE_FIRSTBUTTON_UP => RawPointerEventType::LeftButtonUp,
        PointerButtonChangeType::POINTER_CHANGE_SECONDBUTTON_UP => RawPointerEventType::RightButtonUp,
        PointerButtonChangeType::POINTER_CHANGE_THIRDBUTTON_UP => RawPointerEventType::MiddleButtonUp,
        PointerButtonChangeType::POINTER_CHANGE_FOURTHBUTTON_UP => RawPointerEventType::XButton1Up,
        PointerButtonChangeType::POINTER_CHANGE_FIFTHBUTTON_UP => RawPointerEventType::XButton2Up,
        _ if is_touch => RawPointerEventType::TouchUpdate,
        _ => RawPointerEventType::Move,
    }
}

/// The raw event of a pointer message.
pub(crate) fn get_event_type(message: u32, info: &POINTER_INFO) -> RawPointerEventType {
    let is_touch = info.pointer_type == PointerInputType::PT_TOUCH;
    if PointerFlags::from_bits_retain(info.pointer_flags).contains(PointerFlags::POINTER_FLAG_CANCELED) {
        return if is_touch { RawPointerEventType::TouchCancel } else { RawPointerEventType::CancelCapture };
    }

    let event_type = to_event_type(info.button_change_type, is_touch);
    if event_type == RawPointerEventType::LeftButtonDown && message == WindowsMessage::WM_NCPOINTERDOWN {
        return RawPointerEventType::NonClientLeftButtonDown;
    }

    event_type
}

/// The raw event of one contact of a `WM_TOUCH` message.
pub(crate) fn touch_input_event_type(flags: u32) -> RawPointerEventType {
    let flags = TouchInputFlags::from_bits_retain(flags);
    if flags.contains(TouchInputFlags::TOUCHEVENTF_UP) {
        RawPointerEventType::TouchEnd
    } else if flags.contains(TouchInputFlags::TOUCHEVENTF_DOWN) {
        RawPointerEventType::TouchBegin
    } else {
        RawPointerEventType::TouchUpdate
    }
}

/// The location of a pointer on the screen with the precision of its
/// device: the raw location in the units of the device, mapped from the
/// rectangle of the device to the rectangle of its display.
pub(crate) fn himetric_location(info: &POINTER_INFO, pointer_device_rect: RECT, display_rect: RECT) -> Point {
    let display_width = f64::from(display_rect.right - display_rect.left);
    let display_height = f64::from(display_rect.bottom - display_rect.top);
    let device_width = f64::from(pointer_device_rect.right - pointer_device_rect.left);
    let device_height = f64::from(pointer_device_rect.bottom - pointer_device_rect.top);

    Point::new(
        f64::from(info.pt_himetric_location_raw_x) * display_width / device_width + f64::from(display_rect.left),
        f64::from(info.pt_himetric_location_raw_y) * display_height / device_height + f64::from(display_rect.top),
    )
}

/// The point of a mouse move as the history of the mouse of the system
/// names it: screen coordinates cut to 16 bits, and the time of the message.
pub(crate) fn mouse_move_point(screen_x: i32, screen_y: i32, timestamp: u64) -> MOUSEMOVEPOINT {
    MOUSEMOVEPOINT { x: screen_x & 0xFFFF, y: screen_y & 0xFFFF, time: timestamp as i32, dw_extra_info: 0 }
}

/// The points of the history of the mouse that lie between two moves, in
/// screen pixels, oldest first.
pub(crate) fn intermediate_mouse_points(
    history: &[MOUSEMOVEPOINT],
    move_point: MOUSEMOVEPOINT,
    prev_move_point: MOUSEMOVEPOINT,
) -> Vec<PixelPoint> {
    // The history can be missing if the point wasn't found or there is such a delay that
    // the original points were erased from the buffer.
    if history.len() <= 1 {
        return Vec::new();
    }

    let mut sorted_points: Vec<(i32, PixelPoint)> = Vec::with_capacity(history.len());

    for mp in history {
        let x = if mp.x > 32767 { mp.x - 65536 } else { mp.x };
        let y = if mp.y > 32767 { mp.y - 65536 } else { mp.y };

        if mp.time <= prev_move_point.time || mp.time >= move_point.time {
            continue;
        }

        sorted_points.push((mp.time, PixelPoint::new(x, y)));
    }

    // sorting is required to ensure points are in order from oldest to newest
    sorted_points.sort_by_key(|(time, _)| *time);

    sorted_points.into_iter().map(|(_, point)| point).collect()
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
        RawPointerPoint, RawTextInputEventArgs, RawTouchEventArgs,
    };
    use ferroui_base::input::{IInputDevice, IntermediatePoints, Key, KeyDeviceType, PhysicalKey};
    use std::cell::LazyCell;
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

    /// The device of a pointer message.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum PointerDeviceKind {
        Mouse,
        Touch,
        Pen,
    }

    /// What the system says of the pointer of a message
    /// (`GetDevicePointerInfo` of the reference, whose out parameters
    /// these are).
    pub(crate) struct DevicePointerInfo {
        pub device: PointerDeviceKind,
        pub info: POINTER_INFO,
        pub point: RawPointerPoint,
        pub modifiers: RawInputModifiers,
        pub timestamp: u64,
    }

    /// A raw pointer point at a position, with the defaults of the rest.
    fn raw_point(position: Point) -> RawPointerPoint {
        let mut point = RawPointerPoint::new();
        point.position = position;
        point
    }

    /// A screen pixel point as a point of the client area of a window.
    fn point_to_client(hwnd: isize, scaling: f64, point: PixelPoint) -> Point {
        let p = screen_to_client(hwnd, POINT { x: point.x, y: point.y });
        Point::new(f64::from(p.x), f64::from(p.y)) / scaling
    }

    /// A screen point as a point of the client area of a window, with the
    /// precision it has.
    fn point_to_client_precise(hwnd: isize, scaling: f64, point: Point) -> Point {
        let p = client_to_screen(hwnd, POINT { x: 0, y: 0 });
        Point::new(point.x - f64::from(p.x), point.y - f64::from(p.y)) / scaling
    }

    /// Get the location of the pointer in screen coordinates with HIMETRIC sub-pixel precision
    /// when supported, falling back to the integer pixel location on platforms that do not
    /// implement `GetPointerDeviceRects` (e.g. Wine/Proton).
    fn get_himetric_location(info: &POINTER_INFO) -> Point {
        thread_local! {
            static IS_GET_POINTER_DEVICE_RECTS_AVAILABLE: bool = has_get_pointer_device_rects();
        }

        let pixel_location = Point::new(f64::from(info.pt_pixel_location_x), f64::from(info.pt_pixel_location_y));
        if !IS_GET_POINTER_DEVICE_RECTS_AVAILABLE.with(|available| *available) {
            return pixel_location;
        }

        match get_pointer_device_rects(info.source_device) {
            Some((pointer_device_rect, display_rect)) => himetric_location(info, pointer_device_rect, display_rect),
            // Deviation (DEVIATIONS.md, Windows platform backend): a call
            // that fails leaves rectangles of zeros in the reference and
            // a location that is not a number.
            None => pixel_location,
        }
    }

    fn mouse_raw_pointer_point(hwnd: isize, scaling: f64, pointer_info: &POINTER_INFO) -> RawPointerPoint {
        let point = point_to_client(
            hwnd,
            scaling,
            PixelPoint::new(pointer_info.pt_pixel_location_x, pointer_info.pt_pixel_location_y),
        );
        raw_point(point)
    }

    fn touch_raw_pointer_point(hwnd: isize, scaling: f64, info: &POINTER_TOUCH_INFO) -> RawPointerPoint {
        let himetric_location = get_himetric_location(&info.pointer_info);
        let point = point_to_client_precise(hwnd, scaling, himetric_location);

        let mut pointer_point = raw_point(point);
        // POINTER_PEN_INFO.pressure is normalized to a range between 0 and 1024, with 512 as a default.
        // But in our API we use range from 0.0 to 1.0.
        pointer_point.pressure = info.pressure as f32 / 1024.0;

        // See https://learn.microsoft.com/en-us/windows/win32/inputmsg/touch-mask-constants
        // > TOUCH_MASK_CONTACTAREA: rcContact of the POINTER_TOUCH_INFO structure is valid.
        if (info.touch_mask & TouchMask::TOUCH_MASK_CONTACTAREA.bits()) != 0 {
            // See https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-pointer_touch_info
            // > The predicted screen coordinates of the contact area, in pixels. By default, if the device does not report a contact area, this field defaults to a 0-by-0 rectangle centered around the pointer location.
            let left_top_position =
                point_to_client(hwnd, scaling, PixelPoint::new(info.rc_contact_left, info.rc_contact_top));
            let bottom_right_position =
                point_to_client(hwnd, scaling, PixelPoint::new(info.rc_contact_right, info.rc_contact_bottom));

            // Why not use ptPixelLocationX and ptPixelLocationY to as leftTopPosition?
            // Because ptPixelLocationX and ptPixelLocationY will be the center of the contact area.
            pointer_point.set_contact_rect(Rect::from_points(left_top_position, bottom_right_position));
        }

        pointer_point
    }

    fn pen_raw_pointer_point(hwnd: isize, scaling: f64, info: &POINTER_PEN_INFO) -> RawPointerPoint {
        let himetric_location = get_himetric_location(&info.pointer_info);
        let point = point_to_client_precise(hwnd, scaling, himetric_location);
        let mut pointer_point = raw_point(point);
        // POINTER_PEN_INFO.pressure is normalized to a range between 0 and 1024, with 512 as a default.
        // But in our API we use range from 0.0 to 1.0.
        pointer_point.pressure = info.pressure as f32 / 1024.0;
        pointer_point.twist = info.rotation as f32;
        pointer_point.x_tilt = info.tilt_x as f32;
        pointer_point.y_tilt = info.tilt_y as f32;
        pointer_point
    }

    /// The points the mouse went through between two moves.
    fn create_intermediate_points(
        hwnd: isize,
        scaling: f64,
        move_point: MOUSEMOVEPOINT,
        prev_move_point: MOUSEMOVEPOINT,
    ) -> Vec<RawPointerPoint> {
        // To understand some of this code, please check MS docs:
        // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmousemovepointsex#remarks

        // empty "time" as otherwise WinAPI will always fail
        let move_point_copy = MOUSEMOVEPOINT { time: 0, ..move_point };
        let history = get_mouse_move_points_ex(move_point_copy, MOUSE_HISTORY_SIZE);

        intermediate_mouse_points(&history, move_point, prev_move_point)
            .into_iter()
            .map(|point| raw_point(point_to_client(hwnd, scaling, point)))
            .collect()
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

                    // The automation provider of the window and the input
                    // method are released here by the reference: they
                    // arrive with their stages.

                    self.release_drop_target();

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
                    self.refresh_icon();
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

                WindowsMessage::WM_GETICON => {
                    if self.has_icon() {
                        let request_icon = w_param as i32;
                        let mut request_dpi = l_param as u32;

                        if request_dpi == 0 {
                            request_dpi = self.dpi();
                        }

                        return self.load_icon(request_icon, request_dpi);
                    }
                }

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

                        // Prepare points for the IntermediatePoints call.
                        let p = client_to_screen(
                            self.hwnd(),
                            POINT { x: (point.x * self.scaling()) as i32, y: (point.y * self.scaling()) as i32 },
                        );
                        let curr_point = mouse_move_point(p.x, p.y, timestamp);
                        let prev_point = self.replace_last_wm_mouse_point(curr_point);

                        let args = RawPointerEventArgs::new(
                            self.mouse_input_device(),
                            timestamp,
                            self.owner(),
                            RawPointerEventType::Move,
                            point,
                            get_mouse_modifiers(w_param, WindowsKeyboardDevice::modifiers()),
                        );
                        let (hwnd, scaling) = (self.hwnd(), self.scaling());
                        let points: Box<dyn FnOnce() -> Option<Vec<RawPointerPoint>>> =
                            Box::new(move || Some(create_intermediate_points(hwnd, scaling, curr_point, prev_point)));
                        args.set_intermediate_points(Some(Rc::new(LazyCell::new(points))));
                        e = Some(RawEvent::Other(Rc::new(args)));
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

                WindowsMessage::WM_TOUCH => {
                    if let (false, Some(input)) = (self.wm_pointer_enabled(), self.input_callback()) {
                        let touch_input_count = to_int32(w_param as isize);

                        if let Some(touch_inputs) = get_touch_input_info(l_param, touch_input_count.max(0) as u32) {
                            for touch_input in touch_inputs {
                                let position = self
                                    .point_to_client_impl(PixelPoint::new(touch_input.x / 100, touch_input.y / 100));
                                let mut raw_pointer_point = raw_point(position);

                                // Try to get the touch width and height.
                                // See https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-touchinput
                                // > The width of the touch contact area in hundredths of a pixel in physical screen coordinates. This value is only valid if the dwMask member has the TOUCHEVENTFMASK_CONTACTAREA flag set.
                                const TOUCHEVENTFMASK_CONTACTAREA: u32 = 0x0004; // Known as TOUCHINPUTMASKF_CONTACTAREA in the docs.
                                if (touch_input.mask & TOUCHEVENTFMASK_CONTACTAREA) != 0 {
                                    let center_x = f64::from(touch_input.x) / 100.0;
                                    let center_y = f64::from(touch_input.y) / 100.0;

                                    // The center X add the half width is the right X
                                    let right_x = center_x + f64::from(touch_input.cx_contact) / 100.0 / 2.0;
                                    // The center Y add the half height is the bottom Y
                                    let bottom_y = center_y + f64::from(touch_input.cy_contact) / 100.0 / 2.0;

                                    let bottom_right_position =
                                        self.point_to_client_impl(PixelPoint::new(right_x as i32, bottom_y as i32));

                                    let center_position = position;
                                    let half_width = bottom_right_position.x - center_position.x;
                                    let half_height = bottom_right_position.y - center_position.y;
                                    let left_top_position =
                                        Point::new(center_position.x - half_width, center_position.y - half_height);

                                    raw_pointer_point
                                        .set_contact_rect(Rect::from_points(left_top_position, bottom_right_position));
                                }

                                let device: Rc<dyn IInputDevice> = self.touch_device().clone();
                                input(Rc::new(RawTouchEventArgs::with_point(
                                    device,
                                    u64::from(touch_input.time),
                                    self.owner(),
                                    touch_input_event_type(touch_input.flags),
                                    raw_pointer_point,
                                    WindowsKeyboardDevice::modifiers(),
                                    i64::from(touch_input.id),
                                )));
                            }

                            close_touch_input_handle(l_param);
                            return 0;
                        }
                    }
                }

                WindowsMessage::WM_NCPOINTERDOWN
                | WindowsMessage::WM_NCPOINTERUP
                | WindowsMessage::WM_POINTERDOWN
                | WindowsMessage::WM_POINTERUP
                | WindowsMessage::WM_POINTERUPDATE => {
                    if self.wm_pointer_enabled() {
                        let pointer = self.get_device_pointer_info(w_param, timestamp);
                        let event_type = get_event_type(message, &pointer.info);

                        e = Some(RawEvent::Other(self.create_pointer_args_with_intermediate_points(
                            &pointer,
                            event_type,
                            self.create_lazy_intermediate_points(&pointer.info),
                        )));
                    }
                }

                WindowsMessage::WM_POINTERLEAVE => {
                    if self.wm_pointer_enabled() {
                        let pointer = self.get_device_pointer_info(w_param, timestamp);
                        let event_type = if pointer.device == PointerDeviceKind::Touch {
                            RawPointerEventType::TouchCancel
                        } else {
                            RawPointerEventType::LeaveWindow
                        };
                        e = Some(RawEvent::Other(self.create_pointer_args(&pointer, event_type)));
                    }
                }

                WindowsMessage::WM_POINTERCAPTURECHANGED => {
                    if self.wm_pointer_enabled() {
                        let pointer = self.get_device_pointer_info(w_param, timestamp);
                        let event_type = if pointer.device == PointerDeviceKind::Touch {
                            RawPointerEventType::TouchCancel
                        } else {
                            RawPointerEventType::CancelCapture
                        };
                        e = Some(RawEvent::Other(self.create_pointer_args(&pointer, event_type)));
                    }
                }

                WindowsMessage::WM_POINTERWHEEL | WindowsMessage::WM_POINTERHWHEEL => {
                    if self.wm_pointer_enabled() {
                        let pointer = self.get_device_pointer_info(w_param, timestamp);

                        let val = wheel_delta_from_w_param(w_param);
                        let delta = if message == WindowsMessage::WM_POINTERWHEEL {
                            Vector::new(0.0, val)
                        } else {
                            Vector::new(val, 0.0)
                        };
                        let args = RawMouseWheelEventArgs::new(
                            self.pointer_input_device(pointer.device),
                            pointer.timestamp,
                            self.owner(),
                            pointer.point.position,
                            delta,
                            pointer.modifiers,
                        );
                        args.set_raw_pointer_id(i64::from(pointer.info.pointer_id));
                        e = Some(RawEvent::Other(Rc::new(args)));
                    }
                }

                // WM_POINTERACTIVATE occurs when a pointer activates an inactive window; we should
                // handle this and return PA_ACTIVATE or PA_NOACTIVATE.
                // WM_POINTERDEVICECHANGE notifies about changes in the settings of a monitor that has
                // a digitizer attached to it.
                // WM_NCPOINTERUPDATE: NC stands for non-client area - window header and window border;
                // all we need is pointer down and up, so this is skipped for now.
                // WM_POINTERENTER is not handled as WM_MOUSEENTER is not; with a pen there can be a
                // pointer leave or enter inside the window when the pen is lifted above the display.
                // DM_POINTERHITTEST (direct manipulation), WM_TOUCHHITTESTING (the most probable
                // touch target) and WM_PARENTNOTIFY (dialog scenarios) are not handled either.
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

        fn pointer_input_device(&self, device: PointerDeviceKind) -> Rc<dyn IInputDevice> {
            match device {
                PointerDeviceKind::Mouse => self.mouse_input_device(),
                PointerDeviceKind::Touch => self.touch_device().clone(),
                PointerDeviceKind::Pen => self.pen_device().clone(),
            }
        }

        fn create_lazy_intermediate_points(&self, info: &POINTER_INFO) -> Option<IntermediatePoints> {
            let history_count = info.history_count.min(MAX_POINTER_HISTORY_SIZE);
            if history_count <= 1 {
                return None;
            }

            let (pointer_type, pointer_id) = (info.pointer_type, info.pointer_id);
            let (hwnd, scaling) = (self.hwnd(), self.scaling());
            let points: Box<dyn FnOnce() -> Option<Vec<RawPointerPoint>>> = Box::new(move || {
                let mut list = Vec::with_capacity(history_count as usize);

                // Pointers in history are ordered from newest to oldest, so we need to reverse iteration.
                // Also we skip the newest pointer, because original event arguments already contains it.

                if pointer_type == PointerInputType::PT_TOUCH {
                    if let Some(history) = get_pointer_touch_info_history(pointer_id, history_count) {
                        list.extend(history.iter().skip(1).rev().map(|info| touch_raw_pointer_point(hwnd, scaling, info)));
                    }
                } else if pointer_type == PointerInputType::PT_PEN {
                    if let Some(history) = get_pointer_pen_info_history(pointer_id, history_count) {
                        list.extend(history.iter().skip(1).rev().map(|info| pen_raw_pointer_point(hwnd, scaling, info)));
                    }
                } else {
                    // Currently Windows does not return history info for mouse input, but we handle it just for case.
                    if let Some(history) = get_pointer_info_history(pointer_id, history_count) {
                        list.extend(history.iter().skip(1).rev().map(|info| mouse_raw_pointer_point(hwnd, scaling, info)));
                    }
                }
                Some(list)
            });

            Some(Rc::new(LazyCell::new(points)))
        }

        pub(crate) fn create_pointer_args(
            &self,
            pointer: &DevicePointerInfo,
            event_type: RawPointerEventType,
        ) -> Rc<dyn IRawInputEventArgs> {
            self.create_pointer_args_with_intermediate_points(pointer, event_type, None)
        }

        fn create_pointer_args_with_intermediate_points(
            &self,
            pointer: &DevicePointerInfo,
            event_type: RawPointerEventType,
            intermediate_points: Option<IntermediatePoints>,
        ) -> Rc<dyn IRawInputEventArgs> {
            let device = self.pointer_input_device(pointer.device);
            let raw_pointer_id = i64::from(pointer.info.pointer_id);

            if pointer.device == PointerDeviceKind::Touch {
                let args = RawTouchEventArgs::with_point(
                    device,
                    pointer.timestamp,
                    self.owner(),
                    event_type,
                    pointer.point,
                    pointer.modifiers,
                    raw_pointer_id,
                );
                args.set_intermediate_points(intermediate_points);
                Rc::new(args)
            } else {
                let args = RawPointerEventArgs::with_point(
                    device,
                    pointer.timestamp,
                    self.owner(),
                    event_type,
                    pointer.point,
                    pointer.modifiers,
                );
                args.set_raw_pointer_id(raw_pointer_id);
                args.set_intermediate_points(intermediate_points);
                Rc::new(args)
            }
        }

        /// The device, the state, the point and the modifiers of the
        /// pointer of a message; the time of the pointer replaces
        /// `timestamp` when the system has one.
        pub(crate) fn get_device_pointer_info(&self, w_param: usize, timestamp: u64) -> DevicePointerInfo {
            let pointer_id = (to_int32(w_param as isize) & 0xFFFF) as u32;
            let type_ = get_pointer_type(pointer_id);
            let (hwnd, scaling) = (self.hwnd(), self.scaling());

            let mut modifiers = RawInputModifiers::NONE;

            let (device, info, point) = match type_ {
                PointerInputType::PT_PEN => {
                    let pen_info = get_pointer_pen_info(pointer_id);
                    modifiers |= get_pen_modifiers(pen_info.pen_flags);
                    (PointerDeviceKind::Pen, pen_info.pointer_info, pen_raw_pointer_point(hwnd, scaling, &pen_info))
                }
                PointerInputType::PT_TOUCH => {
                    let touch_info = get_pointer_touch_info(pointer_id);
                    (
                        PointerDeviceKind::Touch,
                        touch_info.pointer_info,
                        touch_raw_pointer_point(hwnd, scaling, &touch_info),
                    )
                }
                _ => {
                    let info = get_pointer_info(pointer_id);
                    (PointerDeviceKind::Mouse, info, mouse_raw_pointer_point(hwnd, scaling, &info))
                }
            };

            let timestamp = if info.dw_time != 0 { u64::from(info.dw_time) } else { timestamp };

            modifiers |= get_pointer_input_modifiers(info.pointer_flags, WindowsKeyboardDevice::modifiers());

            DevicePointerInfo { device, info, point, modifiers, timestamp }
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

    fn pointer(pointer_type: u32, flags: PointerFlags, change: u32) -> POINTER_INFO {
        POINTER_INFO { pointer_type, pointer_flags: flags.bits(), button_change_type: change, ..POINTER_INFO::default() }
    }

    #[test]
    fn the_event_of_a_change_of_the_buttons_of_a_pointer() {
        use PointerButtonChangeType as C;
        use RawPointerEventType as E;
        let cases = [
            (C::POINTER_CHANGE_FIRSTBUTTON_DOWN, E::LeftButtonDown, E::TouchBegin),
            (C::POINTER_CHANGE_FIRSTBUTTON_UP, E::LeftButtonUp, E::TouchEnd),
            (C::POINTER_CHANGE_SECONDBUTTON_DOWN, E::RightButtonDown, E::RightButtonDown),
            (C::POINTER_CHANGE_SECONDBUTTON_UP, E::RightButtonUp, E::RightButtonUp),
            (C::POINTER_CHANGE_THIRDBUTTON_DOWN, E::MiddleButtonDown, E::MiddleButtonDown),
            (C::POINTER_CHANGE_THIRDBUTTON_UP, E::MiddleButtonUp, E::MiddleButtonUp),
            (C::POINTER_CHANGE_FOURTHBUTTON_DOWN, E::XButton1Down, E::XButton1Down),
            (C::POINTER_CHANGE_FOURTHBUTTON_UP, E::XButton1Up, E::XButton1Up),
            (C::POINTER_CHANGE_FIFTHBUTTON_DOWN, E::XButton2Down, E::XButton2Down),
            (C::POINTER_CHANGE_FIFTHBUTTON_UP, E::XButton2Up, E::XButton2Up),
            (C::POINTER_CHANGE_NONE, E::Move, E::TouchUpdate),
        ];
        for (change, not_touch, touch) in cases {
            assert_eq!(to_event_type(change, false), not_touch);
            assert_eq!(to_event_type(change, true), touch);
        }
    }

    #[test]
    fn the_event_of_a_pointer_message() {
        use PointerButtonChangeType as C;
        let down = pointer(PointerInputType::PT_MOUSE, PointerFlags::POINTER_FLAG_DOWN, C::POINTER_CHANGE_FIRSTBUTTON_DOWN);
        assert_eq!(get_event_type(WindowsMessage::WM_POINTERDOWN, &down), RawPointerEventType::LeftButtonDown);
        assert_eq!(get_event_type(WindowsMessage::WM_NCPOINTERDOWN, &down), RawPointerEventType::NonClientLeftButtonDown);

        // A touch in the frame is a touch.
        let touch = pointer(PointerInputType::PT_TOUCH, PointerFlags::POINTER_FLAG_DOWN, C::POINTER_CHANGE_FIRSTBUTTON_DOWN);
        assert_eq!(get_event_type(WindowsMessage::WM_NCPOINTERDOWN, &touch), RawPointerEventType::TouchBegin);

        // A cancelled pointer, whatever its buttons say.
        let cancelled = pointer(PointerInputType::PT_PEN, PointerFlags::POINTER_FLAG_CANCELED, C::POINTER_CHANGE_FIRSTBUTTON_UP);
        assert_eq!(get_event_type(WindowsMessage::WM_POINTERUP, &cancelled), RawPointerEventType::CancelCapture);
        let cancelled = pointer(PointerInputType::PT_TOUCH, PointerFlags::POINTER_FLAG_CANCELED, C::POINTER_CHANGE_NONE);
        assert_eq!(get_event_type(WindowsMessage::WM_POINTERUPDATE, &cancelled), RawPointerEventType::TouchCancel);
    }

    #[test]
    fn the_modifiers_of_a_pointer_and_of_a_pen() {
        let flags = PointerFlags::POINTER_FLAG_FIRSTBUTTON | PointerFlags::POINTER_FLAG_FIFTHBUTTON | PointerFlags::POINTER_FLAG_INCONTACT;
        assert_eq!(
            get_pointer_input_modifiers(flags.bits(), RawInputModifiers::SHIFT),
            RawInputModifiers::SHIFT | RawInputModifiers::LEFT_MOUSE_BUTTON | RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON
        );
        let flags = PointerFlags::POINTER_FLAG_SECONDBUTTON | PointerFlags::POINTER_FLAG_THIRDBUTTON | PointerFlags::POINTER_FLAG_FOURTHBUTTON;
        assert_eq!(
            get_pointer_input_modifiers(flags.bits(), RawInputModifiers::NONE),
            RawInputModifiers::RIGHT_MOUSE_BUTTON | RawInputModifiers::MIDDLE_MOUSE_BUTTON | RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON
        );
        assert_eq!(get_pointer_input_modifiers(0, RawInputModifiers::NONE), RawInputModifiers::NONE);

        assert_eq!(get_pen_modifiers(0), RawInputModifiers::NONE);
        assert_eq!(
            get_pen_modifiers((PenFlags::PEN_FLAGS_BARREL | PenFlags::PEN_FLAGS_INVERTED).bits()),
            RawInputModifiers::PEN_BARREL_BUTTON | RawInputModifiers::PEN_INVERTED
        );
        assert_eq!(get_pen_modifiers(PenFlags::PEN_FLAGS_ERASER.bits()), RawInputModifiers::PEN_ERASER);
    }

    #[test]
    fn the_event_of_a_contact_of_a_touch_message() {
        use TouchInputFlags as F;
        assert_eq!(touch_input_event_type(F::TOUCHEVENTF_DOWN.bits()), RawPointerEventType::TouchBegin);
        assert_eq!(touch_input_event_type(F::TOUCHEVENTF_MOVE.bits()), RawPointerEventType::TouchUpdate);
        assert_eq!(touch_input_event_type((F::TOUCHEVENTF_UP | F::TOUCHEVENTF_PRIMARY).bits()), RawPointerEventType::TouchEnd);
        // Up wins over down, as in the reference.
        assert_eq!(touch_input_event_type((F::TOUCHEVENTF_UP | F::TOUCHEVENTF_DOWN).bits()), RawPointerEventType::TouchEnd);
    }

    #[test]
    fn the_location_of_a_pointer_in_the_units_of_its_device() {
        // A digitizer of 20000 by 10000 units mapped to a display of 2000
        // by 1000 pixels whose left edge is at 1920.
        let device = RECT { left: 0, top: 0, right: 20000, bottom: 10000 };
        let display = RECT { left: 1920, top: 0, right: 3920, bottom: 1000 };
        let info = POINTER_INFO { pt_himetric_location_raw_x: 10005, pt_himetric_location_raw_y: 2503, ..POINTER_INFO::default() };
        let location = himetric_location(&info, device, display);
        assert!((location.x - 2920.5).abs() < 1e-9);
        assert!((location.y - 250.3).abs() < 1e-9);
    }

    #[test]
    fn the_point_of_a_mouse_move_for_the_history_of_the_system() {
        assert_eq!(mouse_move_point(100, 200, 5000), MOUSEMOVEPOINT { x: 100, y: 200, time: 5000, dw_extra_info: 0 });
        // A negative coordinate (a monitor left of the primary) is cut to 16 bits.
        assert_eq!(mouse_move_point(-10, 20, 1).x, 65526);
    }

    #[test]
    fn the_points_between_two_mouse_moves() {
        let at = |x, y, time| MOUSEMOVEPOINT { x, y, time, dw_extra_info: 0 };
        // Newest first, as the system returns them; the first is the move
        // itself, the last two are the previous move and one before it.
        let history = [at(50, 50, 400), at(40, 40, 300), at(65526, 30, 250), at(20, 20, 200), at(10, 10, 100)];
        let points = intermediate_mouse_points(&history, at(50, 50, 400), at(20, 20, 200));
        assert_eq!(points, vec![PixelPoint::new(-10, 30), PixelPoint::new(40, 40)]);

        // The point was not found, or it is the only one.
        assert!(intermediate_mouse_points(&[], at(50, 50, 400), at(20, 20, 200)).is_empty());
        assert!(intermediate_mouse_points(&history[..1], at(50, 50, 400), at(20, 20, 200)).is_empty());
    }
}
