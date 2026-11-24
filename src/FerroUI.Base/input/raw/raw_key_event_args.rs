use super::raw_input_event_args::raw_input_event_args;
use super::RawInputEventArgs;
use crate::input::{IInputDevice, IInputRoot, Key, KeyDeviceType, PhysicalKey, PlatformKeyInfo, RawInputModifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The kinds of raw key events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RawKeyEventType {
    KeyDown,
    KeyUp,
}

/// A raw keyboard event.
pub struct RawKeyEventArgs {
    base: RawInputEventArgs,
    key: Cell<Key>,
    modifiers: Cell<RawInputModifiers>,
    type_: Cell<RawKeyEventType>,
    physical_key: Cell<PhysicalKey>,
    key_device_type: Cell<KeyDeviceType>,
    key_symbol: RefCell<Option<String>>,
    key_text: RefCell<Option<String>>,
    alternate_key_text: RefCell<Option<String>>,
    consumed_modifiers: Cell<Option<RawInputModifiers>>,
    platform_key_info: Cell<Option<PlatformKeyInfo>>,
}

raw_input_event_args!(RawKeyEventArgs: RawInputEventArgs);

impl RawKeyEventArgs {
    /// Creates raw key event args.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        type_: RawKeyEventType,
        key: Key,
        modifiers: RawInputModifiers,
        physical_key: PhysicalKey,
        key_symbol: Option<String>,
        key_device_type: KeyDeviceType,
    ) -> Self {
        Self {
            base: RawInputEventArgs::new(device, timestamp, root),
            key: Cell::new(key),
            modifiers: Cell::new(modifiers),
            type_: Cell::new(type_),
            physical_key: Cell::new(physical_key),
            key_device_type: Cell::new(key_device_type),
            key_symbol: RefCell::new(key_symbol),
            key_text: RefCell::new(None),
            alternate_key_text: RefCell::new(None),
            consumed_modifiers: Cell::new(None),
            platform_key_info: Cell::new(None),
        }
    }

    #[inline]
    pub fn key(&self) -> Key {
        self.key.get()
    }

    pub fn set_key(&self, value: Key) {
        self.key.set(value)
    }

    #[inline]
    pub fn modifiers(&self) -> RawInputModifiers {
        self.modifiers.get()
    }

    pub fn set_modifiers(&self, value: RawInputModifiers) {
        self.modifiers.set(value)
    }

    #[inline]
    pub fn type_(&self) -> RawKeyEventType {
        self.type_.get()
    }

    pub fn set_type(&self, value: RawKeyEventType) {
        self.type_.set(value)
    }

    #[inline]
    pub fn physical_key(&self) -> PhysicalKey {
        self.physical_key.get()
    }

    pub fn set_physical_key(&self, value: PhysicalKey) {
        self.physical_key.set(value)
    }

    #[inline]
    pub fn key_device_type(&self) -> KeyDeviceType {
        self.key_device_type.get()
    }

    pub fn set_key_device_type(&self, value: KeyDeviceType) {
        self.key_device_type.set(value)
    }

    pub fn key_symbol(&self) -> Option<String> {
        self.key_symbol.borrow().clone()
    }

    pub fn set_key_symbol(&self, value: Option<String>) {
        *self.key_symbol.borrow_mut() = value;
    }

    /// The text the platform attached to the key, with all modifiers applied
    /// (`KeyEventArgs::key_text`).
    pub fn key_text(&self) -> Option<String> {
        self.key_text.borrow().clone()
    }

    pub fn set_key_text(&self, value: Option<String>) {
        *self.key_text.borrow_mut() = value;
    }

    /// The text with Ctrl, Alt and Meta ignored (`KeyEventArgs::alternate_key_text`).
    pub fn alternate_key_text(&self) -> Option<String> {
        self.alternate_key_text.borrow().clone()
    }

    pub fn set_alternate_key_text(&self, value: Option<String>) {
        *self.alternate_key_text.borrow_mut() = value;
    }

    /// The modifiers the layout used for the key's text (`KeyEventArgs::consumed_modifiers`).
    #[inline]
    pub fn consumed_modifiers(&self) -> Option<RawInputModifiers> {
        self.consumed_modifiers.get()
    }

    pub fn set_consumed_modifiers(&self, value: Option<RawInputModifiers>) {
        self.consumed_modifiers.set(value);
    }

    /// The platform data of the key (`KeyEventArgs::platform_key_info`).
    #[inline]
    pub fn platform_key_info(&self) -> Option<PlatformKeyInfo> {
        self.platform_key_info.get()
    }

    pub fn set_platform_key_info(&self, value: Option<PlatformKeyInfo>) {
        self.platform_key_info.set(value);
    }
}
