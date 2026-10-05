use super::raw::RawPointerEventArgs;
use super::{IInputDevice, IPointer};
use std::rc::Rc;

/// Represents a pointer device: a mouse, a touch surface or a pen.
pub trait IPointerDevice: IInputDevice {
    /// Gets a pointer for specific event args.
    ///
    /// If the pointer doesn't exist or wasn't yet created this method will
    /// return `None`.
    fn try_get_pointer(&self, ev: &RawPointerEventArgs) -> Option<Rc<dyn IPointer>>;
}
