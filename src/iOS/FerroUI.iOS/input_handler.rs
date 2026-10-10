//! The input of a view: touches of fingers, of a pencil and of an
//! indirect pointer become raw pointer events of the top-level, the
//! presses of a keyboard or of a remote become raw key events and text,
//! and the scroll events of a pointing device become wheel events that
//! go on with inertia when the scrolling stops.
//!
//! The swipe gestures of a remote, which the reference handles on tvOS
//! only, are not ported (`docs/porting/ios-platform.md`).

use ferroui_base::input::raw::{RawKeyEventType, RawPointerEventType};
use ferroui_base::input::{KeyDeviceType, PhysicalKey, RawInputModifiers};
use ferroui_base::{Point, Vector};

/// The device a touch belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TouchDeviceKind {
    /// A finger on the screen.
    Touch,
    /// A pencil.
    Pen,
    /// A pointer that is not on the screen (a mouse or a trackpad).
    Mouse,
}

/// `UITouchTypeDirect` and the other values of `UITouchType`.
pub mod touch_type {
    pub const DIRECT: isize = 0;
    pub const INDIRECT: isize = 1;
    pub const STYLUS: isize = 2;
    pub const INDIRECT_POINTER: isize = 3;
}

/// `UITouchPhaseBegan` and the other values of `UITouchPhase`.
pub mod touch_phase {
    pub const BEGAN: isize = 0;
    pub const MOVED: isize = 1;
    pub const STATIONARY: isize = 2;
    pub const ENDED: isize = 3;
    pub const CANCELLED: isize = 4;
}

/// The bits of `UIKeyModifierFlags`.
pub mod key_modifier_flags {
    pub const SHIFT: isize = 1 << 17;
    pub const CONTROL: isize = 1 << 18;
    pub const ALTERNATE: isize = 1 << 19;
    pub const COMMAND: isize = 1 << 20;
}

/// The device of a touch of the given `UITouchType`; none for an indirect
/// touch (the trackpad of a remote), which is handled with gestures.
pub fn device_kind(touch_type: isize) -> Option<TouchDeviceKind> {
    match touch_type {
        touch_type::INDIRECT => None,
        touch_type::STYLUS => Some(TouchDeviceKind::Pen),
        touch_type::INDIRECT_POINTER => Some(TouchDeviceKind::Mouse),
        _ => Some(TouchDeviceKind::Touch),
    }
}

/// The raw event of a touch of a device in the given `UITouchPhase`.
/// `is_right_click` tells whether the secondary button is in the button
/// mask of the event.
pub fn pointer_event_type(device: TouchDeviceKind, phase: isize, is_right_click: bool) -> RawPointerEventType {
    match (device, phase) {
        (TouchDeviceKind::Touch, touch_phase::BEGAN) => RawPointerEventType::TouchBegin,
        (TouchDeviceKind::Touch, touch_phase::ENDED) => RawPointerEventType::TouchEnd,
        (TouchDeviceKind::Touch, touch_phase::CANCELLED) => RawPointerEventType::TouchCancel,
        (TouchDeviceKind::Touch, _) => RawPointerEventType::TouchUpdate,

        (_, touch_phase::BEGAN) if is_right_click => RawPointerEventType::RightButtonDown,
        (_, touch_phase::BEGAN) => RawPointerEventType::LeftButtonDown,
        (_, touch_phase::ENDED) if is_right_click => RawPointerEventType::RightButtonUp,
        (_, touch_phase::ENDED) => RawPointerEventType::LeftButtonUp,
        (_, touch_phase::CANCELLED) => RawPointerEventType::LeaveWindow,
        (_, _) => RawPointerEventType::Move,
    }
}

/// The pressure of a touch. On iOS "1.0 represents the force of an average
/// touch", where the framework expects 0.5 for "average". A touch whose
/// maximum possible force is 0 has no force and gets the average.
pub fn pressure(force: f64, maximum_possible_force: f64) -> f32 {
    if maximum_possible_force == 0.0 {
        0.5
    } else {
        force as f32 / 2.0
    }
}

/// The modifiers of `UIKeyModifierFlags`.
pub fn convert_modifier_keys(ui_modifier: Option<isize>) -> RawInputModifiers {
    let mut modifier = RawInputModifiers::NONE;
    if let Some(flags) = ui_modifier {
        if flags & key_modifier_flags::SHIFT != 0 {
            modifier |= RawInputModifiers::SHIFT;
        }
        if flags & key_modifier_flags::ALTERNATE != 0 {
            modifier |= RawInputModifiers::ALT;
        }
        if flags & key_modifier_flags::CONTROL != 0 {
            modifier |= RawInputModifiers::CONTROL;
        }
        if flags & key_modifier_flags::COMMAND != 0 {
            modifier |= RawInputModifiers::META;
        }
    }

    modifier
}

/// The timestamp of an event (seconds since the system started) in
/// milliseconds; 0 without an event.
pub fn ts(event_timestamp: Option<f64>) -> u64 {
    match event_timestamp {
        None => 0,
        Some(timestamp) => (timestamp * 1000.0) as u64,
    }
}

/// Whether a touch in the given phase is over.
pub fn is_final_phase(phase: isize) -> bool {
    phase == touch_phase::CANCELLED || phase == touch_phase::ENDED
}

/// `UIPressPhaseBegan` and the other values of `UIPressPhase`.
pub mod press_phase {
    pub const BEGAN: isize = 0;
    pub const CHANGED: isize = 1;
    pub const STATIONARY: isize = 2;
    pub const ENDED: isize = 3;
    pub const CANCELLED: isize = 4;
}

/// `UIPressTypeUpArrow` and the other values of `UIPressType` the handler
/// knows.
pub mod press_type {
    pub const UP_ARROW: isize = 0;
    pub const DOWN_ARROW: isize = 1;
    pub const LEFT_ARROW: isize = 2;
    pub const RIGHT_ARROW: isize = 3;
    pub const SELECT: isize = 4;
    pub const MENU: isize = 5;
    pub const PLAY_PAUSE: isize = 6;
    pub const PAGE_UP: isize = 30;
    pub const PAGE_DOWN: isize = 31;
}

/// `UIGestureRecognizerStateBegan` and the other values of
/// `UIGestureRecognizerState`.
pub mod gesture_recognizer_state {
    pub const POSSIBLE: isize = 0;
    pub const BEGAN: isize = 1;
    pub const CHANGED: isize = 2;
    pub const ENDED: isize = 3;
    pub const CANCELLED: isize = 4;
    pub const FAILED: isize = 5;
}

/// The key table: the physical key of a usage of the keyboard page of the
/// HID usage tables (`UIKeyboardHIDUsage`). The name of a row is the name
/// of the constant of UIKit, which the build for iOS compares with the
/// number.
macro_rules! keys {
    ($($name:ident = $usage:literal => $key:ident,)*) => {
        /// The physical key of a `UIKeyboardHIDUsage`, for the usages the
        /// table of the reference has.
        pub fn physical_key_of_hid_usage(usage: isize) -> Option<PhysicalKey> {
            match usage {
                $($usage => Some(PhysicalKey::$key),)*
                _ => None,
            }
        }

        /// The usages of the key table.
        pub const KEY_TABLE_USAGES: &[isize] = &[$($usage),*];

        #[cfg(target_os = "ios")]
        const _: () = {
            use objc2_ui_kit::UIKeyboardHIDUsage;
            $(assert!(UIKeyboardHIDUsage::$name.0 == $usage);)*
        };
    };
}

keys! {
    KeyboardA = 0x04 => A,
    KeyboardB = 0x05 => B,
    KeyboardC = 0x06 => C,
    KeyboardD = 0x07 => D,
    KeyboardE = 0x08 => E,
    KeyboardF = 0x09 => F,
    KeyboardG = 0x0A => G,
    KeyboardH = 0x0B => H,
    KeyboardI = 0x0C => I,
    KeyboardJ = 0x0D => J,
    KeyboardK = 0x0E => K,
    KeyboardL = 0x0F => L,
    KeyboardM = 0x10 => M,
    KeyboardN = 0x11 => N,
    KeyboardO = 0x12 => O,
    KeyboardP = 0x13 => P,
    KeyboardQ = 0x14 => Q,
    KeyboardR = 0x15 => R,
    KeyboardS = 0x16 => S,
    KeyboardT = 0x17 => T,
    KeyboardU = 0x18 => U,
    KeyboardV = 0x19 => V,
    KeyboardW = 0x1A => W,
    KeyboardX = 0x1B => X,
    KeyboardY = 0x1C => Y,
    KeyboardZ = 0x1D => Z,
    Keyboard1 = 0x1E => Digit1,
    Keyboard2 = 0x1F => Digit2,
    Keyboard3 = 0x20 => Digit3,
    Keyboard4 = 0x21 => Digit4,
    Keyboard5 = 0x22 => Digit5,
    Keyboard6 = 0x23 => Digit6,
    Keyboard7 = 0x24 => Digit7,
    Keyboard8 = 0x25 => Digit8,
    Keyboard9 = 0x26 => Digit9,
    Keyboard0 = 0x27 => Digit0,
    KeyboardReturnOrEnter = 0x28 => Enter,
    KeyboardEscape = 0x29 => Escape,
    // See KeyboardDeleteForward for an actual Delete.
    KeyboardDeleteOrBackspace = 0x2A => Backspace,
    KeyboardTab = 0x2B => Tab,
    KeyboardSpacebar = 0x2C => Space,
    KeyboardHyphen = 0x2D => NumPadSubtract,
    KeyboardEqualSign = 0x2E => NumPadEqual,
    KeyboardOpenBracket = 0x2F => BracketLeft,
    KeyboardCloseBracket = 0x30 => BracketRight,
    KeyboardBackslash = 0x31 => Backslash,
    KeyboardSemicolon = 0x33 => Semicolon,
    KeyboardQuote = 0x34 => Quote,
    KeyboardComma = 0x36 => Comma,
    KeyboardPeriod = 0x37 => Period,
    KeyboardSlash = 0x38 => Slash,
    KeyboardCapsLock = 0x39 => CapsLock,
    KeyboardF1 = 0x3A => F1,
    KeyboardF2 = 0x3B => F2,
    KeyboardF3 = 0x3C => F3,
    KeyboardF4 = 0x3D => F4,
    KeyboardF5 = 0x3E => F5,
    KeyboardF6 = 0x3F => F6,
    KeyboardF7 = 0x40 => F7,
    KeyboardF8 = 0x41 => F8,
    KeyboardF9 = 0x42 => F9,
    KeyboardF10 = 0x43 => F10,
    KeyboardF11 = 0x44 => F11,
    KeyboardF12 = 0x45 => F12,
    KeyboardPrintScreen = 0x46 => PrintScreen,
    KeyboardScrollLock = 0x47 => ScrollLock,
    KeyboardPause = 0x48 => Pause,
    KeyboardInsert = 0x49 => Insert,
    KeyboardHome = 0x4A => Home,
    KeyboardPageUp = 0x4B => PageUp,
    KeyboardDeleteForward = 0x4C => Delete,
    KeyboardEnd = 0x4D => End,
    KeyboardPageDown = 0x4E => PageDown,
    KeyboardRightArrow = 0x4F => ArrowRight,
    KeyboardLeftArrow = 0x50 => ArrowLeft,
    KeyboardDownArrow = 0x51 => ArrowDown,
    KeyboardUpArrow = 0x52 => ArrowUp,
    KeypadNumLock = 0x53 => NumLock,
    KeypadSlash = 0x54 => Slash,
    KeypadAsterisk = 0x55 => NumPadMultiply,
    KeypadHyphen = 0x56 => NumPadSubtract,
    KeypadPlus = 0x57 => NumPadAdd,
    KeypadEnter = 0x58 => Enter,
    Keypad1 = 0x59 => NumPad1,
    Keypad2 = 0x5A => NumPad2,
    Keypad3 = 0x5B => NumPad3,
    Keypad4 = 0x5C => NumPad4,
    Keypad5 = 0x5D => NumPad5,
    Keypad6 = 0x5E => NumPad6,
    Keypad7 = 0x5F => NumPad7,
    Keypad8 = 0x60 => NumPad8,
    Keypad9 = 0x61 => NumPad9,
    Keypad0 = 0x62 => NumPad0,
    KeypadPeriod = 0x63 => Period,
    KeyboardNonUSBackslash = 0x64 => IntlBackslash,
    KeyboardF13 = 0x68 => F13,
    KeyboardF14 = 0x69 => F14,
    KeyboardF15 = 0x6A => F15,
    KeyboardF16 = 0x6B => F16,
    KeyboardF17 = 0x6C => F17,
    KeyboardF18 = 0x6D => F18,
    KeyboardF19 = 0x6E => F19,
    KeyboardF20 = 0x6F => F20,
    KeyboardF21 = 0x70 => F21,
    KeyboardF22 = 0x71 => F22,
    KeyboardF23 = 0x72 => F23,
    KeyboardF24 = 0x73 => F24,
    KeyboardSelect = 0x77 => Space,
    KeyboardMute = 0x7F => AudioVolumeMute,
    KeyboardVolumeUp = 0x80 => AudioVolumeUp,
    KeyboardVolumeDown = 0x81 => AudioVolumeDown,
    KeypadComma = 0x85 => NumPadComma,
    KeyboardLeftControl = 0xE0 => ControlLeft,
    KeyboardLeftShift = 0xE1 => ShiftLeft,
    KeyboardLeftAlt = 0xE2 => AltLeft,
    KeyboardLeftGUI = 0xE3 => MetaLeft,
    KeyboardRightControl = 0xE4 => ControlRight,
    KeyboardRightShift = 0xE5 => ShiftRight,
    KeyboardRightAlt = 0xE6 => AltRight,
    KeyboardRightGUI = 0xE7 => MetaRight,
}

/// What a press says of its key (`UIKey`): the usage, the modifier flags
/// and the characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PressKey<'a> {
    pub key_code: isize,
    pub modifier_flags: isize,
    pub characters: &'a str,
}

/// A press as the raw key event describes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslatedPress {
    pub physical_key: PhysicalKey,
    pub modifier: RawInputModifiers,
    pub characters: Option<String>,
    pub key_device_type: KeyDeviceType,
}

/// Translates a press: by its key when it has one that the key table
/// knows (a keyboard, very likely), by its type otherwise (a remote, very
/// likely).
pub fn translate_press(key: Option<PressKey<'_>>, press_type: isize) -> TranslatedPress {
    if let Some(ui_key) = key {
        if let Some(physical_key) = physical_key_of_hid_usage(ui_key.key_code) {
            return TranslatedPress {
                physical_key,
                modifier: convert_modifier_keys(Some(ui_key.modifier_flags)),
                // The characters of a key that has none are the name of
                // its input constant ("UIKeyInputUpArrow").
                characters: (!ui_key.characters.starts_with("UIKey")).then(|| ui_key.characters.to_string()),
                key_device_type: KeyDeviceType::Keyboard,
            };
        }
    }

    TranslatedPress {
        physical_key: match press_type {
            press_type::UP_ARROW => PhysicalKey::ArrowUp,
            press_type::DOWN_ARROW => PhysicalKey::ArrowDown,
            press_type::LEFT_ARROW => PhysicalKey::ArrowLeft,
            press_type::RIGHT_ARROW => PhysicalKey::ArrowRight,
            press_type::SELECT => PhysicalKey::Space,
            press_type::MENU => PhysicalKey::ContextMenu,
            press_type::PLAY_PAUSE => PhysicalKey::MediaPlayPause,
            press_type::PAGE_UP => PhysicalKey::PageUp,
            press_type::PAGE_DOWN => PhysicalKey::PageDown,
            _ => PhysicalKey::None,
        },
        modifier: RawInputModifiers::NONE,
        characters: None,
        key_device_type: KeyDeviceType::Remote,
    }
}

/// The raw key event of a press in the given `UIPressPhase`.
pub fn key_event_type(phase: isize) -> RawKeyEventType {
    match phase {
        press_phase::BEGAN | press_phase::CHANGED | press_phase::STATIONARY => RawKeyEventType::KeyDown,
        _ => RawKeyEventType::KeyUp,
    }
}

/// Whether a press that was not handled as a key is text: only when it
/// began, and when it has characters.
pub fn press_is_text_input(handled: bool, phase: isize, characters: Option<&str>) -> bool {
    !handled && phase == press_phase::BEGAN && characters.is_some_and(|characters| !characters.is_empty())
}

/// The rate the velocity of inertia scrolling falls by at every frame.
pub const DECELERATION_RATE: f64 = 0.95;

/// The wheel delta of active scrolling at a velocity (points per second).
/// A much more sensitive scaling than the one of inertia, to match AppKit:
/// macOS uses small deltas, so the divisor is large.
pub fn active_scroll_delta(velocity: Point) -> Vector {
    let scale_factor = 3000.0;
    Vector::new(velocity.x / scale_factor, velocity.y / scale_factor)
}

/// The state of scrolling: where it began, and the velocity of the
/// inertia that follows it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MomentumScrolling {
    cached_scroll_location: Option<Point>,
    momentum_velocity_x: f64,
    momentum_velocity_y: f64,
}

impl MomentumScrolling {
    /// Scrolling began at a location, which the events keep until the
    /// inertia stops.
    pub fn begin(&mut self, location: Point) {
        self.stop();
        self.cached_scroll_location = Some(location);
    }

    /// The location scrolling began at.
    pub fn cached_scroll_location(&self) -> Option<Point> {
        self.cached_scroll_location
    }

    /// Scrolling ended at a velocity (points per second): inertia starts.
    pub fn start_inertia(&mut self, velocity: Point) {
        let scale_factor = 800.0;
        self.momentum_velocity_x = velocity.x / scale_factor;
        self.momentum_velocity_y = velocity.y / scale_factor;
    }

    /// Forgets the velocity and the location.
    pub fn stop(&mut self) {
        self.momentum_velocity_x = 0.0;
        self.momentum_velocity_y = 0.0;
        self.cached_scroll_location = None;
    }

    /// A frame of inertia: the location and the delta of the wheel event
    /// to send, or nothing when the inertia is over (the state is then
    /// stopped).
    pub fn update(&mut self) -> Option<(Point, Vector)> {
        self.momentum_velocity_x *= DECELERATION_RATE;
        self.momentum_velocity_y *= DECELERATION_RATE;

        let current_magnitude = (self.momentum_velocity_x * self.momentum_velocity_x
            + self.momentum_velocity_y * self.momentum_velocity_y)
            .sqrt();

        match self.cached_scroll_location {
            Some(location) if current_magnitude >= 0.0001 => {
                Some((location, Vector::new(self.momentum_velocity_x, self.momentum_velocity_y)))
            }
            _ => {
                self.stop();
                None
            }
        }
    }
}

#[cfg(target_os = "ios")]
pub(crate) use uikit::InputHandler;

#[cfg(target_os = "ios")]
mod uikit {
    use super::*;
    use crate::extensions::to_point;
    use crate::ferro_view::{FerroView, TopLevelImpl};
    use ferroui_base::input::raw::{
        IRawInputEventArgs, RawKeyEventArgs, RawMouseWheelEventArgs, RawPointerPoint, RawTextInputEventArgs,
        RawTouchEventArgs,
    };
    use ferroui_base::input::{
        IInputDevice, IntermediatePoints, KeyboardDevice, MouseDevice, PenDevice, TouchDevice,
    };
    use ferroui_controls::platform::ITopLevelImpl;
    use objc2::rc::{Retained, Weak};
    use objc2::{sel, MainThreadOnly, Message};
    use objc2_foundation::{NSProcessInfo, NSRunLoop, NSRunLoopCommonModes, NSSet};
    use objc2_quartz_core::CADisplayLink;
    use objc2_ui_kit::{
        UIEvent, UIEventButtonMask, UIPanGestureRecognizer, UIPress, UIPressesEvent, UITouch,
    };
    use std::cell::{Cell, LazyCell, RefCell};
    use std::collections::HashMap;
    use std::rc::Rc;

    thread_local! {
        /// The identifier of the next touch, across the views of the
        /// application.
        static NEXT_TOUCH_POINT_ID: Cell<i64> = const { Cell::new(1) };
    }

    const _: () = {
        use objc2_ui_kit::{UIKeyModifierFlags, UITouchPhase, UITouchType};
        // The constants of this file are those of UIKit.
        assert!(touch_type::DIRECT == UITouchType::Direct.0);
        assert!(touch_type::INDIRECT == UITouchType::Indirect.0);
        assert!(touch_type::STYLUS == UITouchType::Pencil.0);
        assert!(touch_type::INDIRECT_POINTER == UITouchType::IndirectPointer.0);
        assert!(touch_phase::BEGAN == UITouchPhase::Began.0);
        assert!(touch_phase::MOVED == UITouchPhase::Moved.0);
        assert!(touch_phase::STATIONARY == UITouchPhase::Stationary.0);
        assert!(touch_phase::ENDED == UITouchPhase::Ended.0);
        assert!(touch_phase::CANCELLED == UITouchPhase::Cancelled.0);
        assert!(key_modifier_flags::SHIFT == UIKeyModifierFlags::Shift.0);
        assert!(key_modifier_flags::CONTROL == UIKeyModifierFlags::Control.0);
        assert!(key_modifier_flags::ALTERNATE == UIKeyModifierFlags::Alternate.0);
        assert!(key_modifier_flags::COMMAND == UIKeyModifierFlags::Command.0);
    };

    const _: () = {
        use objc2_ui_kit::{UIGestureRecognizerState, UIPressPhase, UIPressType};
        assert!(press_phase::BEGAN == UIPressPhase::Began.0);
        assert!(press_phase::CHANGED == UIPressPhase::Changed.0);
        assert!(press_phase::STATIONARY == UIPressPhase::Stationary.0);
        assert!(press_phase::ENDED == UIPressPhase::Ended.0);
        assert!(press_phase::CANCELLED == UIPressPhase::Cancelled.0);
        assert!(press_type::UP_ARROW == UIPressType::UpArrow.0);
        assert!(press_type::DOWN_ARROW == UIPressType::DownArrow.0);
        assert!(press_type::LEFT_ARROW == UIPressType::LeftArrow.0);
        assert!(press_type::RIGHT_ARROW == UIPressType::RightArrow.0);
        assert!(press_type::SELECT == UIPressType::Select.0);
        assert!(press_type::MENU == UIPressType::Menu.0);
        assert!(press_type::PLAY_PAUSE == UIPressType::PlayPause.0);
        assert!(press_type::PAGE_UP == UIPressType::PageUp.0);
        assert!(press_type::PAGE_DOWN == UIPressType::PageDown.0);
        assert!(gesture_recognizer_state::POSSIBLE == UIGestureRecognizerState::Possible.0);
        assert!(gesture_recognizer_state::BEGAN == UIGestureRecognizerState::Began.0);
        assert!(gesture_recognizer_state::CHANGED == UIGestureRecognizerState::Changed.0);
        assert!(gesture_recognizer_state::ENDED == UIGestureRecognizerState::Ended.0);
        assert!(gesture_recognizer_state::CANCELLED == UIGestureRecognizerState::Cancelled.0);
        assert!(gesture_recognizer_state::FAILED == UIGestureRecognizerState::Failed.0);
    };

    /// The milliseconds since the system started: the clock of the
    /// timestamps of the events of UIKit.
    fn tick_count() -> u64 {
        (NSProcessInfo::processInfo().systemUptime() * 1000.0) as u64
    }

    /// Translates the touches of a view to the input of its top-level.
    pub(crate) struct InputHandler {
        view: Weak<FerroView>,
        tl: Rc<TopLevelImpl>,
        touch_device: Rc<TouchDevice>,
        mouse_device: Rc<MouseDevice>,
        pen_device: Rc<PenDevice>,
        /// The identifiers of the touches that are down, by the address of
        /// the touch object, which UIKit keeps for the life of a touch.
        known_touches: RefCell<HashMap<usize, i64>>,
        momentum: Cell<MomentumScrolling>,
        momentum_display_link: RefCell<Option<Retained<CADisplayLink>>>,
    }

    impl InputHandler {
        pub(crate) fn new(view: Weak<FerroView>, tl: Rc<TopLevelImpl>) -> Self {
            Self {
                view,
                tl,
                touch_device: TouchDevice::new(),
                mouse_device: MouseDevice::new(),
                pen_device: PenDevice::new(true),
                known_touches: RefCell::new(HashMap::new()),
                momentum: Cell::new(MomentumScrolling::default()),
                momentum_display_link: RefCell::new(None),
            }
        }

        fn invoke_input(&self, args: Rc<dyn IRawInputEventArgs>) {
            if let Some(input) = self.tl.input() {
                input(args);
            }
        }

        /// Handles the touches of a `touchesBegan:withEvent:` and of the
        /// three methods that follow it.
        pub(crate) fn handle(&self, touches: &NSSet<UITouch>, evt: Option<&UIEvent>) {
            let Some(view) = self.view.load() else {
                return;
            };

            for t in touches.iter() {
                let Some(kind) = device_kind(t.r#type().0) else {
                    // Ignore indirect input, like the trackpad of a remote
                    // controller, which is handled with gestures.
                    continue;
                };

                let key = Retained::as_ptr(&t) as usize;
                let id = *self.known_touches.borrow_mut().entry(key).or_insert_with(|| {
                    NEXT_TOUCH_POINT_ID.with(|next| {
                        let id = next.get();
                        next.set(id + 1);
                        id
                    })
                });

                let device: Rc<dyn IInputDevice> = match kind {
                    TouchDeviceKind::Pen => self.pen_device.clone(),
                    TouchDeviceKind::Mouse => self.mouse_device.clone(),
                    TouchDeviceKind::Touch => self.touch_device.clone(),
                };

                let modifiers = convert_modifier_keys(evt.map(|evt| evt.modifierFlags().0));
                let is_right_click = evt.is_some_and(|evt| evt.buttonMask().contains(UIEventButtonMask::Secondary));
                let phase = t.phase().0;

                let to_pointer_point = {
                    let view = view.clone();
                    // The pressure of every point is the one of the touch
                    // that is handled, as in the reference.
                    let pressure = pressure(t.force(), t.maximumPossibleForce());
                    move |touch: &UITouch| {
                        let mut point = RawPointerPoint::new();
                        point.position = to_point(touch.locationInView(Some(&view)));
                        point.pressure = pressure;
                        point
                    }
                };

                let ev = RawTouchEventArgs::with_point(
                    device,
                    ts(evt.map(|evt| evt.timestamp())),
                    view.input_root(),
                    pointer_event_type(kind, phase, is_right_click),
                    to_pointer_point(&t),
                    modifiers,
                    id,
                );

                if let Some(this_event) = evt {
                    let this_event = this_event.retain();
                    let touch = t.clone();
                    let intermediate_points: IntermediatePoints = Rc::new(LazyCell::new(Box::new(move || {
                        let coalesced = this_event.coalescedTouchesForTouch(&touch);
                        let coalesced: Vec<Retained<UITouch>> =
                            coalesced.map(|coalesced| coalesced.iter().collect()).unwrap_or_default();
                        // Skip the last one, as it is the point that was
                        // processed already.
                        let count = coalesced.len().saturating_sub(1);
                        Some(coalesced[..count].iter().map(|touch| to_pointer_point(touch)).collect())
                    })));
                    ev.set_intermediate_points(Some(intermediate_points));
                }

                let args: Rc<dyn IRawInputEventArgs> = Rc::new(ev);
                if let Some(input) = self.tl.input() {
                    input(args);
                }

                if is_final_phase(phase) {
                    self.known_touches.borrow_mut().remove(&key);
                }
            }
        }

        /// Handles the presses of a `pressesBegan:withEvent:` and of the
        /// three methods that follow it; tells whether one was handled.
        ///
        /// # Panics
        /// Panics when the platform has no keyboard device (`use_ios`).
        pub(crate) fn handle_presses(&self, presses: &NSSet<UIPress>, evt: Option<&UIPressesEvent>) -> bool {
            let Some(view) = self.view.load() else {
                return false;
            };
            let mtm = view.mtm();
            let keyboard_device: Rc<dyn IInputDevice> = match KeyboardDevice::instance() {
                Some(keyboard_device) => keyboard_device,
                None => panic!("The keyboard device of the platform is not registered."),
            };
            let timestamp = ts(evt.map(|evt| evt.timestamp()));

            let mut handled = false;
            for p in presses.iter() {
                let ui_key = p.key(mtm);
                let characters = ui_key.as_ref().map(|ui_key| ui_key.characters().to_string());
                let key = ui_key.as_ref().zip(characters.as_ref()).map(|(ui_key, characters)| PressKey {
                    key_code: ui_key.keyCode().0,
                    modifier_flags: ui_key.modifierFlags().0,
                    characters,
                });
                let press = translate_press(key, p.r#type().0);
                let phase = p.phase().0;

                let key = press.physical_key.to_qwerty_key();
                if key == ferroui_base::input::Key::None {
                    continue;
                }

                let ev = Rc::new(RawKeyEventArgs::new(
                    keyboard_device.clone(),
                    timestamp,
                    view.input_root(),
                    key_event_type(phase),
                    key,
                    press.modifier,
                    press.physical_key,
                    press.characters.clone(),
                    press.key_device_type,
                ));

                self.invoke_input(ev.clone());
                handled |= ev.handled();

                if press_is_text_input(ev.handled(), phase, press.characters.as_deref()) {
                    let raw_text_event = Rc::new(RawTextInputEventArgs::new(
                        keyboard_device.clone(),
                        timestamp,
                        view.input_root(),
                        press.characters.unwrap_or_default(),
                    ));
                    self.invoke_input(raw_text_event.clone());
                    handled |= raw_text_event.handled();
                }
            }

            handled
        }

        /// Handles the pan gesture that only takes scroll events.
        pub(crate) fn handle_scroll_wheel(&self, recognizer: &UIPanGestureRecognizer) {
            let Some(view) = self.view.load() else {
                return;
            };

            match recognizer.state().0 {
                gesture_recognizer_state::BEGAN => {
                    // Scrolling started: stop any previous inertia
                    // scrolling and cache the current scroll location.
                    self.stop_momentum_scrolling();
                    let mut momentum = self.momentum.get();
                    momentum.begin(to_point(recognizer.locationInView(Some(&view))));
                    self.momentum.set(momentum);
                }
                gesture_recognizer_state::CHANGED => {
                    // While scrolling is active, the scroll events are
                    // sent.
                    self.send_active_scroll_event(recognizer, &view);
                }
                gesture_recognizer_state::ENDED => {
                    // When scrolling stops, inertia scrolling starts.
                    // `update_inertia_scrolling` checks when the inertia
                    // stops and calls `stop_momentum_scrolling`.
                    self.start_inertia_scrolling(recognizer, &view);
                }
                gesture_recognizer_state::CANCELLED | gesture_recognizer_state::FAILED => {
                    // If the gesture is cancelled or failed, stop.
                    self.stop_momentum_scrolling();
                }
                _ => {}
            }
        }

        fn send_wheel_event(&self, view: &FerroView, position: Point, delta: Vector) {
            let device: Rc<dyn IInputDevice> = self.mouse_device.clone();
            self.invoke_input(Rc::new(RawMouseWheelEventArgs::new(
                device,
                tick_count(),
                view.input_root(),
                position,
                delta,
                RawInputModifiers::NONE,
            )));
        }

        fn send_active_scroll_event(&self, recognizer: &UIPanGestureRecognizer, view: &FerroView) {
            let velocity = to_point(recognizer.velocityInView(Some(view)));
            let position = self.momentum.get().cached_scroll_location().unwrap_or_default();
            self.send_wheel_event(view, position, active_scroll_delta(velocity));
        }

        fn start_inertia_scrolling(&self, recognizer: &UIPanGestureRecognizer, view: &FerroView) {
            let velocity = to_point(recognizer.velocityInView(Some(view)));
            let mut momentum = self.momentum.get();
            momentum.start_inertia(velocity);
            self.momentum.set(momentum);

            // SAFETY: the view responds to the selector (the class of the
            // view declares `ferroUpdateInertiaScrolling:`, which takes
            // the link, the one argument a display link passes); the link
            // retains its target until it is invalidated, which
            // `stop_momentum_scrolling` does. The link is created on the
            // main thread, added to the run loop of the main thread and
            // invalidated there; the mode is a constant of Foundation.
            let link = unsafe {
                let link = CADisplayLink::displayLinkWithTarget_selector(view, sel!(ferroUpdateInertiaScrolling:));
                link.addToRunLoop_forMode(&NSRunLoop::mainRunLoop(), NSRunLoopCommonModes);
                link
            };
            let previous = self.momentum_display_link.borrow_mut().replace(link);
            if let Some(previous) = previous {
                // The reference overwrites the field; a link that was
                // still running would run on with the view retained, so
                // it is invalidated here.
                previous.invalidate();
            }
        }

        fn stop_momentum_scrolling(&self) {
            let link = self.momentum_display_link.borrow_mut().take();
            if let Some(link) = link {
                // Invalidating removes the link from all run loops.
                link.invalidate();
            }

            let mut momentum = self.momentum.get();
            momentum.stop();
            self.momentum.set(momentum);
        }

        /// A frame of inertia scrolling, from the display link.
        pub(crate) fn update_inertia_scrolling(&self) {
            let mut momentum = self.momentum.get();
            let event = momentum.update();
            self.momentum.set(momentum);

            match (event, self.view.load()) {
                // The pan gesture goes on reporting the location of the
                // pointer where it would be if it moved with the current
                // velocity, though the pointer on the screen does not
                // move. The location is cached when scrolling starts and
                // kept until the inertia stops.
                (Some((location, delta)), Some(view)) => self.send_wheel_event(&view, location, delta),
                _ => self.stop_momentum_scrolling(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;

    #[test]
    fn fingers_pencils_and_pointers_have_their_devices() {
        assert_eq!(Some(TouchDeviceKind::Touch), device_kind(touch_type::DIRECT));
        assert_eq!(None, device_kind(touch_type::INDIRECT));
        assert_eq!(Some(TouchDeviceKind::Pen), device_kind(touch_type::STYLUS));
        assert_eq!(Some(TouchDeviceKind::Mouse), device_kind(touch_type::INDIRECT_POINTER));
        // A type this version does not know is a touch.
        assert_eq!(Some(TouchDeviceKind::Touch), device_kind(17));
    }

    #[test]
    fn the_phases_of_a_finger_are_the_touch_events() {
        let event = |phase| pointer_event_type(TouchDeviceKind::Touch, phase, false);
        assert_eq!(RawPointerEventType::TouchBegin, event(touch_phase::BEGAN));
        assert_eq!(RawPointerEventType::TouchUpdate, event(touch_phase::MOVED));
        assert_eq!(RawPointerEventType::TouchUpdate, event(touch_phase::STATIONARY));
        assert_eq!(RawPointerEventType::TouchEnd, event(touch_phase::ENDED));
        assert_eq!(RawPointerEventType::TouchCancel, event(touch_phase::CANCELLED));
        // The secondary button means nothing for a finger.
        assert_eq!(RawPointerEventType::TouchBegin, pointer_event_type(TouchDeviceKind::Touch, touch_phase::BEGAN, true));
    }

    #[test]
    fn the_phases_of_a_pencil_and_of_a_pointer_are_button_events() {
        for device in [TouchDeviceKind::Pen, TouchDeviceKind::Mouse] {
            let event = |phase, right| pointer_event_type(device, phase, right);
            assert_eq!(RawPointerEventType::LeftButtonDown, event(touch_phase::BEGAN, false));
            assert_eq!(RawPointerEventType::RightButtonDown, event(touch_phase::BEGAN, true));
            assert_eq!(RawPointerEventType::Move, event(touch_phase::MOVED, false));
            assert_eq!(RawPointerEventType::Move, event(touch_phase::STATIONARY, true));
            assert_eq!(RawPointerEventType::LeftButtonUp, event(touch_phase::ENDED, false));
            assert_eq!(RawPointerEventType::RightButtonUp, event(touch_phase::ENDED, true));
            assert_eq!(RawPointerEventType::LeaveWindow, event(touch_phase::CANCELLED, false));
        }
    }

    #[test]
    fn the_average_force_is_half_the_pressure_range() {
        assert_eq!(0.5, pressure(1.0, 6.67));
        assert_eq!(1.0, pressure(2.0, 6.67));
        assert_eq!(0.0, pressure(0.0, 6.67));
        // A screen that measures no force.
        assert_eq!(0.5, pressure(0.0, 0.0));
    }

    #[test]
    fn the_modifier_flags_are_the_raw_modifiers() {
        assert_eq!(RawInputModifiers::NONE, convert_modifier_keys(None));
        assert_eq!(RawInputModifiers::NONE, convert_modifier_keys(Some(0)));
        assert_eq!(RawInputModifiers::SHIFT, convert_modifier_keys(Some(key_modifier_flags::SHIFT)));
        assert_eq!(RawInputModifiers::ALT, convert_modifier_keys(Some(key_modifier_flags::ALTERNATE)));
        assert_eq!(RawInputModifiers::CONTROL, convert_modifier_keys(Some(key_modifier_flags::CONTROL)));
        assert_eq!(RawInputModifiers::META, convert_modifier_keys(Some(key_modifier_flags::COMMAND)));
        assert_eq!(
            RawInputModifiers::SHIFT | RawInputModifiers::META,
            convert_modifier_keys(Some(key_modifier_flags::SHIFT | key_modifier_flags::COMMAND | 1 << 16))
        );
    }

    #[test]
    fn the_timestamp_is_in_milliseconds() {
        assert_eq!(0, ts(None));
        assert_eq!(1500, ts(Some(1.5)));
        assert_eq!(86_400_123, ts(Some(86_400.1234)));
    }

    #[test]
    fn a_touch_is_over_when_it_ended_or_was_cancelled() {
        assert!(is_final_phase(touch_phase::ENDED));
        assert!(is_final_phase(touch_phase::CANCELLED));
        assert!(!is_final_phase(touch_phase::BEGAN));
        assert!(!is_final_phase(touch_phase::MOVED));
        assert!(!is_final_phase(touch_phase::STATIONARY));
    }

    #[test]
    fn the_key_table_has_the_keys_of_the_reference() {
        assert_eq!(120, KEY_TABLE_USAGES.len());
        assert_eq!(Some(PhysicalKey::A), physical_key_of_hid_usage(0x04));
        assert_eq!(Some(PhysicalKey::Z), physical_key_of_hid_usage(0x1D));
        assert_eq!(Some(PhysicalKey::Digit1), physical_key_of_hid_usage(0x1E));
        assert_eq!(Some(PhysicalKey::Digit0), physical_key_of_hid_usage(0x27));
        assert_eq!(Some(PhysicalKey::Enter), physical_key_of_hid_usage(0x28));
        assert_eq!(Some(PhysicalKey::Backspace), physical_key_of_hid_usage(0x2A));
        assert_eq!(Some(PhysicalKey::Delete), physical_key_of_hid_usage(0x4C));
        assert_eq!(Some(PhysicalKey::F12), physical_key_of_hid_usage(0x45));
        assert_eq!(Some(PhysicalKey::F24), physical_key_of_hid_usage(0x73));
        assert_eq!(Some(PhysicalKey::ArrowUp), physical_key_of_hid_usage(0x52));
        assert_eq!(Some(PhysicalKey::MetaLeft), physical_key_of_hid_usage(0xE3));
        assert_eq!(Some(PhysicalKey::MetaRight), physical_key_of_hid_usage(0xE7));
        // As in the reference: the hyphen and the equal sign of the main
        // block are the keys of the numeric pad, the slash and the period
        // of the numeric pad are the keys of the main block, and "select"
        // is the space bar.
        assert_eq!(Some(PhysicalKey::NumPadSubtract), physical_key_of_hid_usage(0x2D));
        assert_eq!(Some(PhysicalKey::NumPadEqual), physical_key_of_hid_usage(0x2E));
        assert_eq!(Some(PhysicalKey::Slash), physical_key_of_hid_usage(0x54));
        assert_eq!(Some(PhysicalKey::Period), physical_key_of_hid_usage(0x63));
        assert_eq!(Some(PhysicalKey::Space), physical_key_of_hid_usage(0x77));
        // Not in the table: the non-US pound, the grave accent, the
        // application key, the error codes.
        for usage in [0x00, 0x01, 0x32, 0x35, 0x65, 0x66, 0x67, 0x74, 0x82, 0xE8, 0xFFFF] {
            assert_eq!(None, physical_key_of_hid_usage(usage), "{usage:#x}");
        }
        // Every key of the table has a key of the QWERTY layout.
        for usage in KEY_TABLE_USAGES {
            let physical_key = physical_key_of_hid_usage(*usage).unwrap();
            assert_ne!(ferroui_base::input::Key::None, physical_key.to_qwerty_key(), "{usage:#x}");
        }
    }

    #[test]
    fn a_press_with_a_key_of_the_table_is_a_keyboard_press() {
        let press = translate_press(
            Some(PressKey { key_code: 0x04, modifier_flags: key_modifier_flags::SHIFT, characters: "A" }),
            press_type::SELECT,
        );
        assert_eq!(PhysicalKey::A, press.physical_key);
        assert_eq!(RawInputModifiers::SHIFT, press.modifier);
        assert_eq!(Some("A".to_string()), press.characters);
        assert_eq!(KeyDeviceType::Keyboard, press.key_device_type);

        // A key without characters reports the name of its constant.
        let press = translate_press(
            Some(PressKey { key_code: 0x52, modifier_flags: 0, characters: "UIKeyInputUpArrow" }),
            press_type::UP_ARROW,
        );
        assert_eq!(PhysicalKey::ArrowUp, press.physical_key);
        assert_eq!(None, press.characters);
        assert_eq!(KeyDeviceType::Keyboard, press.key_device_type);
    }

    #[test]
    fn a_press_without_a_key_of_the_table_is_a_press_of_a_remote() {
        let of_type = |press_type| translate_press(None, press_type);
        assert_eq!(PhysicalKey::ArrowUp, of_type(press_type::UP_ARROW).physical_key);
        assert_eq!(PhysicalKey::ArrowDown, of_type(press_type::DOWN_ARROW).physical_key);
        assert_eq!(PhysicalKey::ArrowLeft, of_type(press_type::LEFT_ARROW).physical_key);
        assert_eq!(PhysicalKey::ArrowRight, of_type(press_type::RIGHT_ARROW).physical_key);
        assert_eq!(PhysicalKey::Space, of_type(press_type::SELECT).physical_key);
        assert_eq!(PhysicalKey::ContextMenu, of_type(press_type::MENU).physical_key);
        assert_eq!(PhysicalKey::MediaPlayPause, of_type(press_type::PLAY_PAUSE).physical_key);
        assert_eq!(PhysicalKey::PageUp, of_type(press_type::PAGE_UP).physical_key);
        assert_eq!(PhysicalKey::PageDown, of_type(press_type::PAGE_DOWN).physical_key);
        assert_eq!(PhysicalKey::None, of_type(32).physical_key);
        assert_eq!(KeyDeviceType::Remote, of_type(press_type::SELECT).key_device_type);
        assert_eq!(RawInputModifiers::NONE, of_type(press_type::SELECT).modifier);

        // A key the table does not have (the grave accent) falls back to
        // the type of the press, with nothing of the key.
        let press = translate_press(
            Some(PressKey { key_code: 0x35, modifier_flags: key_modifier_flags::SHIFT, characters: "~" }),
            press_type::MENU,
        );
        assert_eq!(PhysicalKey::ContextMenu, press.physical_key);
        assert_eq!(RawInputModifiers::NONE, press.modifier);
        assert_eq!(None, press.characters);
        assert_eq!(KeyDeviceType::Remote, press.key_device_type);
    }

    #[test]
    fn the_phases_of_a_press_are_key_down_until_it_ends() {
        assert_eq!(RawKeyEventType::KeyDown, key_event_type(press_phase::BEGAN));
        assert_eq!(RawKeyEventType::KeyDown, key_event_type(press_phase::CHANGED));
        assert_eq!(RawKeyEventType::KeyDown, key_event_type(press_phase::STATIONARY));
        assert_eq!(RawKeyEventType::KeyUp, key_event_type(press_phase::ENDED));
        assert_eq!(RawKeyEventType::KeyUp, key_event_type(press_phase::CANCELLED));
        assert_eq!(RawKeyEventType::KeyUp, key_event_type(17));
    }

    #[test]
    fn text_follows_a_key_down_that_was_not_handled() {
        assert!(press_is_text_input(false, press_phase::BEGAN, Some("a")));
        assert!(!press_is_text_input(true, press_phase::BEGAN, Some("a")));
        assert!(!press_is_text_input(false, press_phase::CHANGED, Some("a")));
        assert!(!press_is_text_input(false, press_phase::ENDED, Some("a")));
        assert!(!press_is_text_input(false, press_phase::BEGAN, Some("")));
        assert!(!press_is_text_input(false, press_phase::BEGAN, None));
    }

    #[test]
    fn active_scrolling_scales_the_velocity_down() {
        assert_eq!(Vector::new(1.0, -0.5), active_scroll_delta(Point::new(3000.0, -1500.0)));
        assert_eq!(Vector::new(0.0, 0.0), active_scroll_delta(Point::new(0.0, 0.0)));
    }

    #[test]
    fn inertia_decelerates_until_it_stops() {
        let mut momentum = MomentumScrolling::default();
        // No inertia without a location.
        momentum.start_inertia(Point::new(800.0, 0.0));
        assert_eq!(None, momentum.update());

        momentum.begin(Point::new(10.0, 20.0));
        assert_eq!(Some(Point::new(10.0, 20.0)), momentum.cached_scroll_location());
        momentum.start_inertia(Point::new(800.0, -1600.0));
        let (location, delta) = momentum.update().unwrap();
        assert_eq!(Point::new(10.0, 20.0), location);
        assert_eq!(Vector::new(0.95, -1.9), delta);
        let (location, delta) = momentum.update().unwrap();
        assert_eq!(Point::new(10.0, 20.0), location);
        assert!((delta.x - 0.9025).abs() < 1e-12 && (delta.y + 1.805).abs() < 1e-12);

        // The velocity falls below the threshold after a number of
        // frames, and the state is then as it was at the start.
        let mut frames = 2;
        while momentum.update().is_some() {
            frames += 1;
            assert!(frames < 1000);
        }
        assert!((150..250).contains(&frames), "{frames}");
        assert_eq!(MomentumScrolling::default(), momentum);
    }

    #[test]
    fn a_new_scroll_stops_the_inertia_of_the_last_one() {
        let mut momentum = MomentumScrolling::default();
        momentum.begin(Point::new(1.0, 1.0));
        momentum.start_inertia(Point::new(800.0, 800.0));
        momentum.begin(Point::new(5.0, 6.0));
        assert_eq!(Some(Point::new(5.0, 6.0)), momentum.cached_scroll_location());
        // The velocity of the last scroll is gone.
        assert_eq!(None, momentum.update());
    }
}
