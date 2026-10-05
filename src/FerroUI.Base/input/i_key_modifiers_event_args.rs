use super::KeyModifiers;

/// Implemented by event args that carry the state of the keyboard modifier
/// keys at the time the event occurred.
pub trait IKeyModifiersEventArgs {
    /// The key modifiers associated with this event.
    fn key_modifiers(&self) -> KeyModifiers;
}
