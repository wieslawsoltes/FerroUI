//! The equivalent of the runtime library's `EventArgs`.

/// The arguments of an event that carries no data: what the handler of a
/// plain `EventHandler` event receives besides the sender.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EventArgs;

impl EventArgs {
    /// The arguments of an event without data (`EventArgs.Empty`).
    pub const EMPTY: EventArgs = EventArgs;
}
