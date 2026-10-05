//! Conversions between the interop structs/enums and the toolkit types.

use crate::frn_string::FrnString;
use crate::interop::*;
use ferroui_base::input::platform::ClipboardError;
use ferroui_base::input::raw::{RawDragEventType, RawKeyEventType, RawPointerEventType};
use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers, StandardCursorType};
use ferroui_base::{PixelPoint, PixelRect, Point, Rect, Size};
use ferroui_controls::{WindowDecorations, WindowResizeReason, WindowState};
use ferroui_microcom::{ComPtr, HResult};

pub(crate) fn to_ferro_point(pt: FrnPoint) -> Point {
    Point::new(pt.x, pt.y)
}

pub(crate) fn to_ferro_pixel_point(pt: FrnPoint) -> PixelPoint {
    PixelPoint::new(pt.x as i32, pt.y as i32)
}

pub(crate) fn to_frn_point(pt: Point) -> FrnPoint {
    FrnPoint { x: pt.x, y: pt.y }
}

pub(crate) fn pixel_point_to_frn_point(pt: PixelPoint) -> FrnPoint {
    FrnPoint { x: pt.x as f64, y: pt.y as f64 }
}

pub(crate) fn to_frn_rect(rect: Rect) -> FrnRect {
    FrnRect { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
}

pub(crate) fn to_frn_size(size: Size) -> FrnSize {
    FrnSize { width: size.width, height: size.height }
}

/// Wraps a string into a COM string object for native code.
#[allow(dead_code)]
pub(crate) fn to_frn_string(s: Option<&str>) -> Option<ComPtr<IFrnString>> {
    s.map(|s| IFrnString::from_impl(FrnString::new(s)))
}

pub(crate) fn to_ferro_size(size: FrnSize) -> Size {
    Size::new(size.width, size.height)
}

#[allow(dead_code)]
pub(crate) fn to_ferro_rect(rect: FrnRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}

pub(crate) fn to_ferro_pixel_rect(rect: FrnRect) -> PixelRect {
    PixelRect::new(rect.x as i32, rect.y as i32, rect.width as i32, rect.height as i32)
}

// --- enum conversions (plain casts in the reference implementation) ---------

/// The key of a native key event; unknown values map to [`Key::None`].
pub(crate) fn to_key(key: FrnKey) -> Key {
    Key::from_value(key.0).unwrap_or(Key::None)
}

/// The physical key of a native key event; unknown values map to
/// [`PhysicalKey::None`].
pub(crate) fn to_physical_key(key: FrnPhysicalKey) -> PhysicalKey {
    PhysicalKey::from_value(key.0).unwrap_or(PhysicalKey::None)
}

/// The modifiers of a native input event. The bit values are shared, so
/// every bit is carried over.
pub(crate) fn to_raw_input_modifiers(modifiers: FrnInputModifiers) -> RawInputModifiers {
    RawInputModifiers::from_bits_retain(modifiers.0)
}

pub(crate) fn to_raw_key_event_type(type_: FrnRawKeyEventType) -> RawKeyEventType {
    if type_ == FrnRawKeyEventType::KeyUp {
        RawKeyEventType::KeyUp
    } else {
        RawKeyEventType::KeyDown
    }
}

/// The pointer event type of a native mouse event, or `None` for a value
/// the toolkit does not know.
pub(crate) fn to_raw_pointer_event_type(type_: FrnRawMouseEventType) -> Option<RawPointerEventType> {
    Some(match type_.0 {
        0 => RawPointerEventType::LeaveWindow,
        1 => RawPointerEventType::LeftButtonDown,
        2 => RawPointerEventType::LeftButtonUp,
        3 => RawPointerEventType::RightButtonDown,
        4 => RawPointerEventType::RightButtonUp,
        5 => RawPointerEventType::MiddleButtonDown,
        6 => RawPointerEventType::MiddleButtonUp,
        7 => RawPointerEventType::XButton1Down,
        8 => RawPointerEventType::XButton1Up,
        9 => RawPointerEventType::XButton2Down,
        10 => RawPointerEventType::XButton2Up,
        11 => RawPointerEventType::Move,
        12 => RawPointerEventType::Wheel,
        13 => RawPointerEventType::NonClientLeftButtonDown,
        14 => RawPointerEventType::TouchBegin,
        15 => RawPointerEventType::TouchUpdate,
        16 => RawPointerEventType::TouchEnd,
        17 => RawPointerEventType::TouchCancel,
        18 => RawPointerEventType::Magnify,
        19 => RawPointerEventType::Rotate,
        20 => RawPointerEventType::Swipe,
        _ => return None,
    })
}

pub(crate) fn to_raw_drag_event_type(type_: FrnDragEventType) -> RawDragEventType {
    match type_.0 {
        1 => RawDragEventType::DragOver,
        2 => RawDragEventType::DragLeave,
        3 => RawDragEventType::Drop,
        _ => RawDragEventType::DragEnter,
    }
}

pub(crate) fn to_window_resize_reason(reason: FrnPlatformResizeReason) -> WindowResizeReason {
    match reason.0 {
        1 => WindowResizeReason::User,
        2 => WindowResizeReason::Application,
        3 => WindowResizeReason::Layout,
        4 => WindowResizeReason::DpiChange,
        _ => WindowResizeReason::Unspecified,
    }
}

pub(crate) fn to_frn_resize_reason(reason: WindowResizeReason) -> FrnPlatformResizeReason {
    FrnPlatformResizeReason(reason as i32)
}

pub(crate) fn to_window_state(state: FrnWindowState) -> WindowState {
    match state.0 {
        1 => WindowState::Minimized,
        2 => WindowState::Maximized,
        3 => WindowState::FullScreen,
        _ => WindowState::Normal,
    }
}

pub(crate) fn to_frn_window_state(state: WindowState) -> FrnWindowState {
    FrnWindowState(state as i32)
}

pub(crate) fn to_frn_decorations(decorations: WindowDecorations) -> SystemDecorations {
    SystemDecorations(decorations as i32)
}

pub(crate) fn to_frn_cursor_type(cursor_type: StandardCursorType) -> FrnStandardCursorType {
    FrnStandardCursorType(cursor_type as i32)
}

/// Unwraps the result of a native call.
///
/// A failed `HRESULT` is what the reference implementation surfaces as a COM
/// exception, so it is a panic here; inside a native callback the panic is
/// carried to the run loop frame (see `callback_base`).
pub(crate) trait ComResultExt<T> {
    #[track_caller]
    fn check(self) -> T;
}

impl<T> ComResultExt<T> for Result<T, HResult> {
    #[track_caller]
    fn check(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => panic!("Native call failed: {error}"),
        }
    }
}

/// A failed `HRESULT` of a native clipboard call is what the reference
/// implementation surfaces as a COM exception from the clipboard operation
/// that made the call: it becomes the error of that operation.
pub(crate) trait ClipboardResultExt<T> {
    fn clipboard_check(self) -> Result<T, ClipboardError>;
}

impl<T> ClipboardResultExt<T> for Result<T, HResult> {
    fn clipboard_check(self) -> Result<T, ClipboardError> {
        self.map_err(to_clipboard_error)
    }
}

/// The clipboard error of a failed native clipboard call.
pub(crate) fn to_clipboard_error(error: HResult) -> ClipboardError {
    ClipboardError::platform(error.0 as i32, format!("Native call failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_clipboard_calls_become_platform_errors() {
        use ferroui_base::input::platform::ClipboardErrorKind;

        assert_eq!(Ok(3), Ok::<i32, HResult>(3).clipboard_check());

        let error = Err::<i32, HResult>(HResult::FAIL).clipboard_check().expect_err("the failed call");
        assert_eq!(ClipboardErrorKind::Platform, error.kind());
        assert_eq!(Some(HResult::FAIL.0 as i32), error.code());
        assert_eq!(format!("Native call failed: {}", HResult::FAIL), error.message());
    }

    #[test]
    fn struct_conversions() {
        assert_eq!(to_ferro_point(FrnPoint { x: 1.5, y: -2.25 }), Point::new(1.5, -2.25));
        // Pixel conversions truncate toward zero, like the reference casts.
        assert_eq!(to_ferro_pixel_point(FrnPoint { x: 10.9, y: -3.9 }), PixelPoint::new(10, -3));
        assert_eq!(to_frn_point(Point::new(4.0, 5.0)), FrnPoint { x: 4.0, y: 5.0 });
        assert_eq!(pixel_point_to_frn_point(PixelPoint::new(7, -8)), FrnPoint { x: 7.0, y: -8.0 });
        assert_eq!(
            to_frn_rect(Rect::new(1.0, 2.0, 3.0, 4.0)),
            FrnRect { x: 1.0, y: 2.0, width: 3.0, height: 4.0 }
        );
        assert_eq!(to_frn_size(Size::new(3.0, 4.0)), FrnSize { width: 3.0, height: 4.0 });
        assert_eq!(to_ferro_size(FrnSize { width: 3.0, height: 4.0 }), Size::new(3.0, 4.0));
        assert_eq!(
            to_ferro_rect(FrnRect { x: 1.0, y: 2.0, width: 3.0, height: 4.0 }),
            Rect::new(1.0, 2.0, 3.0, 4.0)
        );
        assert_eq!(
            to_ferro_pixel_rect(FrnRect { x: 0.9, y: 25.0, width: 1728.7, height: 1117.2 }),
            PixelRect::new(0, 25, 1728, 1117)
        );
    }

    #[test]
    fn keys_share_their_numeric_values() {
        assert_eq!(to_key(FrnKey::FrnKeyNone), Key::None);
        assert_eq!(to_key(FrnKey::FrnKeyReturn), Key::Return);
        assert_eq!(to_key(FrnKey::FrnKeyEnter), Key::Return);
        assert_eq!(to_key(FrnKey::FrnKeyEscape), Key::Escape);
        assert_eq!(to_key(FrnKey::FrnKeySpace), Key::Space);
        assert_eq!(to_key(FrnKey::FrnKeyLeft), Key::Left);
        assert_eq!(to_key(FrnKey::FrnKeyD0), Key::D0);
        assert_eq!(to_key(FrnKey::FrnKeyA), Key::A);
        assert_eq!(to_key(FrnKey::FrnKeyZ), Key::Z);
        assert_eq!(to_key(FrnKey::FrnKeyF1), Key::F1);
        assert_eq!(to_key(FrnKey::FrnKeyLeftShift), Key::LeftShift);
        assert_eq!(to_key(FrnKey::FrnKeyLWin), Key::LWin);
        assert_eq!(to_key(FrnKey(-12345)), Key::None);

        // Every value the toolkit knows maps to itself.
        for value in 0..400 {
            if let Some(key) = Key::from_value(value) {
                assert_eq!(to_key(FrnKey(value)).value(), key.value());
            }
            if let Some(key) = PhysicalKey::from_value(value) {
                assert_eq!(to_physical_key(FrnPhysicalKey(value)).value(), key.value());
            }
        }
    }

    #[test]
    fn physical_keys_share_their_numeric_values() {
        assert_eq!(to_physical_key(FrnPhysicalKey::FrnPhysicalKeyNone), PhysicalKey::None);
        assert_eq!(to_physical_key(FrnPhysicalKey::FrnPhysicalKeyA), PhysicalKey::A);
        assert_eq!(to_physical_key(FrnPhysicalKey::FrnPhysicalKeyEnter), PhysicalKey::Enter);
        assert_eq!(to_physical_key(FrnPhysicalKey::FrnPhysicalKeySpace), PhysicalKey::Space);
        assert_eq!(to_physical_key(FrnPhysicalKey::FrnPhysicalKeyArrowLeft), PhysicalKey::ArrowLeft);
        assert_eq!(to_physical_key(FrnPhysicalKey(-1)), PhysicalKey::None);
    }

    #[test]
    fn modifiers_share_their_bits() {
        assert_eq!(to_raw_input_modifiers(FrnInputModifiers::FrnInputModifiersNone), RawInputModifiers::NONE);
        assert_eq!(to_raw_input_modifiers(FrnInputModifiers::Alt), RawInputModifiers::ALT);
        assert_eq!(to_raw_input_modifiers(FrnInputModifiers::Control), RawInputModifiers::CONTROL);
        assert_eq!(to_raw_input_modifiers(FrnInputModifiers::Shift), RawInputModifiers::SHIFT);
        assert_eq!(to_raw_input_modifiers(FrnInputModifiers::Windows), RawInputModifiers::META);
        assert_eq!(to_raw_input_modifiers(FrnInputModifiers::LeftMouseButton), RawInputModifiers::LEFT_MOUSE_BUTTON);
        assert_eq!(to_raw_input_modifiers(FrnInputModifiers::RightMouseButton), RawInputModifiers::RIGHT_MOUSE_BUTTON);
        assert_eq!(
            to_raw_input_modifiers(FrnInputModifiers::MiddleMouseButton),
            RawInputModifiers::MIDDLE_MOUSE_BUTTON
        );
        assert_eq!(
            to_raw_input_modifiers(FrnInputModifiers::XButton1MouseButton),
            RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON
        );
        assert_eq!(
            to_raw_input_modifiers(FrnInputModifiers::XButton2MouseButton),
            RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON
        );
        assert_eq!(
            to_raw_input_modifiers(FrnInputModifiers::Windows | FrnInputModifiers::Shift),
            RawInputModifiers::META | RawInputModifiers::SHIFT
        );
    }

    #[test]
    fn pointer_event_types() {
        use RawPointerEventType as T;
        let expected = [
            (FrnRawMouseEventType::LeaveWindow, T::LeaveWindow),
            (FrnRawMouseEventType::LeftButtonDown, T::LeftButtonDown),
            (FrnRawMouseEventType::LeftButtonUp, T::LeftButtonUp),
            (FrnRawMouseEventType::RightButtonDown, T::RightButtonDown),
            (FrnRawMouseEventType::RightButtonUp, T::RightButtonUp),
            (FrnRawMouseEventType::MiddleButtonDown, T::MiddleButtonDown),
            (FrnRawMouseEventType::MiddleButtonUp, T::MiddleButtonUp),
            (FrnRawMouseEventType::XButton1Down, T::XButton1Down),
            (FrnRawMouseEventType::XButton1Up, T::XButton1Up),
            (FrnRawMouseEventType::XButton2Down, T::XButton2Down),
            (FrnRawMouseEventType::XButton2Up, T::XButton2Up),
            (FrnRawMouseEventType::Move, T::Move),
            (FrnRawMouseEventType::Wheel, T::Wheel),
            (FrnRawMouseEventType::NonClientLeftButtonDown, T::NonClientLeftButtonDown),
            (FrnRawMouseEventType::TouchBegin, T::TouchBegin),
            (FrnRawMouseEventType::TouchUpdate, T::TouchUpdate),
            (FrnRawMouseEventType::TouchEnd, T::TouchEnd),
            (FrnRawMouseEventType::TouchCancel, T::TouchCancel),
            (FrnRawMouseEventType::Magnify, T::Magnify),
            (FrnRawMouseEventType::Rotate, T::Rotate),
            (FrnRawMouseEventType::Swipe, T::Swipe),
        ];
        for (native, toolkit) in expected {
            assert_eq!(to_raw_pointer_event_type(native), Some(toolkit));
        }
        assert_eq!(to_raw_pointer_event_type(FrnRawMouseEventType(21)), None);
        assert_eq!(to_raw_pointer_event_type(FrnRawMouseEventType(-1)), None);
    }

    #[test]
    fn drag_event_types() {
        assert_eq!(to_raw_drag_event_type(FrnDragEventType::Enter), RawDragEventType::DragEnter);
        assert_eq!(to_raw_drag_event_type(FrnDragEventType::Over), RawDragEventType::DragOver);
        assert_eq!(to_raw_drag_event_type(FrnDragEventType::Leave), RawDragEventType::DragLeave);
        assert_eq!(to_raw_drag_event_type(FrnDragEventType::Drop), RawDragEventType::Drop);
    }

    #[test]
    fn key_event_types() {
        assert_eq!(to_raw_key_event_type(FrnRawKeyEventType::KeyDown), RawKeyEventType::KeyDown);
        assert_eq!(to_raw_key_event_type(FrnRawKeyEventType::KeyUp), RawKeyEventType::KeyUp);
    }

    #[test]
    fn window_enums() {
        assert_eq!(to_window_resize_reason(FrnPlatformResizeReason::ResizeUnspecified), WindowResizeReason::Unspecified);
        assert_eq!(to_window_resize_reason(FrnPlatformResizeReason::ResizeUser), WindowResizeReason::User);
        assert_eq!(to_window_resize_reason(FrnPlatformResizeReason::ResizeApplication), WindowResizeReason::Application);
        assert_eq!(to_window_resize_reason(FrnPlatformResizeReason::ResizeLayout), WindowResizeReason::Layout);
        assert_eq!(to_window_resize_reason(FrnPlatformResizeReason::ResizeDpiChange), WindowResizeReason::DpiChange);
        for reason in [
            WindowResizeReason::Unspecified,
            WindowResizeReason::User,
            WindowResizeReason::Application,
            WindowResizeReason::Layout,
            WindowResizeReason::DpiChange,
        ] {
            assert_eq!(to_window_resize_reason(to_frn_resize_reason(reason)), reason);
        }

        for state in [WindowState::Normal, WindowState::Minimized, WindowState::Maximized, WindowState::FullScreen] {
            assert_eq!(to_window_state(to_frn_window_state(state)), state);
        }
        assert_eq!(to_frn_window_state(WindowState::FullScreen), FrnWindowState::FullScreen);

        assert_eq!(to_frn_decorations(WindowDecorations::None), SystemDecorations::SystemDecorationsNone);
        assert_eq!(to_frn_decorations(WindowDecorations::BorderOnly), SystemDecorations::SystemDecorationsBorderOnly);
        assert_eq!(to_frn_decorations(WindowDecorations::Full), SystemDecorations::SystemDecorationsFull);
    }

    #[test]
    fn cursor_types() {
        assert_eq!(to_frn_cursor_type(StandardCursorType::Arrow), FrnStandardCursorType::CursorArrow);
        assert_eq!(to_frn_cursor_type(StandardCursorType::Ibeam), FrnStandardCursorType::CursorIbeam);
        assert_eq!(to_frn_cursor_type(StandardCursorType::Hand), FrnStandardCursorType::CursorHand);
        assert_eq!(to_frn_cursor_type(StandardCursorType::BottomSide), FrnStandardCursorType::CursorBottomSize);
        assert_eq!(
            to_frn_cursor_type(StandardCursorType::BottomRightCorner),
            FrnStandardCursorType::CursorBottomRightCorner
        );
        assert_eq!(to_frn_cursor_type(StandardCursorType::DragLink), FrnStandardCursorType::CursorDragLink);
        assert_eq!(to_frn_cursor_type(StandardCursorType::None), FrnStandardCursorType::CursorNone);
    }

    #[test]
    #[should_panic(expected = "Native call failed")]
    fn failed_native_calls_panic() {
        let result: Result<(), HResult> = Err(HResult::FAIL);
        result.check();
    }
}
