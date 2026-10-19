//! `MessageBusExtensions` (`ReactiveUI.Testing/MessageBusExtensions.cs`).

use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::{IMessageBus, MessageBus};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Restores the message bus when it is dropped: the `using` block of upstream's `With`, which
/// restores it also when the block throws (here, panics).
struct RestoreOnDrop(Rc<dyn IDisposable>);

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Runs code with a message bus as [`MessageBus::current`] (`MessageBusExtensions`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the current message bus is per thread, so upstream's
/// lock (`Monitor.Enter(messageBusGate)`), which keeps the tests of other threads from replacing
/// it in between, is not needed and not ported.
pub trait MessageBusExtensions {
    /// The message bus as the handle that [`MessageBus::current`] holds.
    fn as_message_bus(&self) -> Rc<dyn IMessageBus>;

    /// Makes the message bus [`MessageBus::current`] until the returned disposable is disposed,
    /// which restores the message bus that was current before (`WithMessageBus`).
    fn with_message_bus(&self) -> Rc<dyn IDisposable> {
        let original = MessageBus::current();
        MessageBus::set_current(self.as_message_bus());
        Disposable::create(move || MessageBus::set_current(original))
    }

    /// Runs `block` with the message bus as [`MessageBus::current`] and returns its result; the
    /// message bus current before is restored afterwards, also when `block` panics
    /// (`With<TRet>(Func<TRet>)`, and `With(Action)` with a block that returns `()`).
    ///
    /// Not ported: the `ArgumentException` of a null block - argument validation.
    fn with<R>(&self, block: impl FnOnce() -> R) -> R {
        let _restore = RestoreOnDrop(self.with_message_bus());
        block()
    }
}

impl MessageBusExtensions for Rc<dyn IMessageBus> {
    fn as_message_bus(&self) -> Rc<dyn IMessageBus> {
        self.clone()
    }
}

impl<B: IMessageBus + 'static> MessageBusExtensions for Rc<B> {
    fn as_message_bus(&self) -> Rc<dyn IMessageBus> {
        self.clone()
    }
}
