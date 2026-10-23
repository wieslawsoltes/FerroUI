use super::raw_input_event_args::raw_input_event_args;
use super::RawInputEventArgs;
use crate::input::text_input::TextInputMethodCommit;
use crate::input::{IInputDevice, IInputRoot};
use std::rc::Rc;

/// A raw text input event.
pub struct RawTextInputEventArgs {
    base: RawInputEventArgs,
    text: String,
    input_method_commit: Option<Rc<TextInputMethodCommit>>,
}

raw_input_event_args!(RawTextInputEventArgs: RawInputEventArgs);

impl RawTextInputEventArgs {
    /// Creates raw text input event args. `device` is the keyboard device
    /// the text comes from.
    pub fn new(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            base: RawInputEventArgs::new(device, timestamp, root),
            text: text.into(),
            input_method_commit: None,
        }
    }

    pub(crate) fn for_input_method(
        device: Rc<dyn IInputDevice>,
        timestamp: u64,
        root: Rc<dyn IInputRoot>,
        text: &str,
        commit: Rc<TextInputMethodCommit>,
    ) -> Self {
        Self {
            base: RawInputEventArgs::new(device, timestamp, root),
            text: text.to_owned(),
            input_method_commit: Some(commit),
        }
    }

    /// The captured owner and acceptance of a managed platform commit.
    pub fn input_method_commit(&self) -> Option<&Rc<TextInputMethodCommit>> {
        self.input_method_commit.as_ref()
    }

    /// The text that was input.
    #[inline]
    pub fn text(&self) -> &str {
        &self.text
    }
}
