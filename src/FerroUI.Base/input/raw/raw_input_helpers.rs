use super::RawPointerEventType;
use crate::input::{KeyModifiers, PointerUpdateKind, RawInputModifiers};

impl RawInputModifiers {
    /// The keyboard modifiers contained in the raw modifiers.
    #[inline]
    pub fn to_key_modifiers(self) -> KeyModifiers {
        KeyModifiers::from_bits_truncate((self & RawInputModifiers::KEYBOARD_MASK).bits())
    }
}

impl RawPointerEventType {
    /// The kind of pointer state change the raw event type represents.
    pub fn to_update_kind(self) -> PointerUpdateKind {
        match self {
            RawPointerEventType::LeftButtonDown => PointerUpdateKind::LeftButtonPressed,
            RawPointerEventType::LeftButtonUp => PointerUpdateKind::LeftButtonReleased,
            RawPointerEventType::RightButtonDown => PointerUpdateKind::RightButtonPressed,
            RawPointerEventType::RightButtonUp => PointerUpdateKind::RightButtonReleased,
            RawPointerEventType::MiddleButtonDown => PointerUpdateKind::MiddleButtonPressed,
            RawPointerEventType::MiddleButtonUp => PointerUpdateKind::MiddleButtonReleased,
            RawPointerEventType::XButton1Down => PointerUpdateKind::XButton1Pressed,
            RawPointerEventType::XButton1Up => PointerUpdateKind::XButton1Released,
            RawPointerEventType::XButton2Down => PointerUpdateKind::XButton2Pressed,
            RawPointerEventType::XButton2Up => PointerUpdateKind::XButton2Released,
            RawPointerEventType::TouchBegin => PointerUpdateKind::LeftButtonPressed,
            RawPointerEventType::TouchEnd => PointerUpdateKind::LeftButtonReleased,
            _ => PointerUpdateKind::Other,
        }
    }
}
