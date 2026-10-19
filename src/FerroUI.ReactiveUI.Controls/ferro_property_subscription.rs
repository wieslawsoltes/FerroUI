//! `FerroPropertySubscription` (ReactiveUI.Binding's integration, `...PropertySubscription.cs`).

use ferroui_base::{FerroObject, Ref};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::fmt;

/// The object and the handler a property observation detaches when it is disposed
/// (`FerroPropertySubscription`): disposing it removes the property-changed handler from the
/// object; only the first call does anything.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream's record of the sender and the handler
/// is disposed by a `Scope`; here the record is the disposable, and the handler is its token.
pub struct FerroPropertySubscription {
    sender: Ref<FerroObject>,
    token: Cell<Option<u64>>,
}

impl FerroPropertySubscription {
    /// The subscription of the property-changed handler `token` of `sender`.
    pub fn new(sender: Ref<FerroObject>, token: u64) -> Self {
        Self { sender, token: Cell::new(Some(token)) }
    }

    /// The observed object (`Sender`).
    pub fn sender(&self) -> &Ref<FerroObject> {
        &self.sender
    }
}

impl IDisposable for FerroPropertySubscription {
    fn dispose(&self) {
        if let Some(token) = self.token.take() {
            self.sender.remove_property_changed_handler(token);
        }
    }
}

impl fmt::Debug for FerroPropertySubscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FerroPropertySubscription: {:?}", self.sender)
    }
}
