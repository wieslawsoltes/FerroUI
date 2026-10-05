use super::raw_input_event_args::raw_input_event_args;
use super::RawInputEventArgs;
use crate::input::{IInputDevice, IInputRoot, Key, KeyDeviceType, PhysicalKey, RawInputModifiers};
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
}
