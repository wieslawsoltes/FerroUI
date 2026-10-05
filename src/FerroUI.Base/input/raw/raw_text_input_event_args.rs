use super::raw_input_event_args::raw_input_event_args;
use super::RawInputEventArgs;
use crate::input::{IInputDevice, IInputRoot};
use std::rc::Rc;

/// A raw text input event.
pub struct RawTextInputEventArgs {
    base: RawInputEventArgs,
    text: String,
}

raw_input_event_args!(RawTextInputEventArgs: RawInputEventArgs);

impl RawTextInputEventArgs {
    /// Creates raw text input event args. `device` is the keyboard device
    /// the text comes from.
    pub fn new(device: Rc<dyn IInputDevice>, timestamp: u64, root: Rc<dyn IInputRoot>, text: impl Into<String>) -> Self {
        Self { base: RawInputEventArgs::new(device, timestamp, root), text: text.into() }
    }

    /// The text that was input.
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }
}
