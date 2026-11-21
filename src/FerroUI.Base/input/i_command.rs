use crate::reactive::IDisposable;
use crate::BoxedValue;
use std::rc::Rc;

/// Defines a command.
///
/// Commands compare by reference.
pub trait ICommand {
    /// Defines the method that determines whether the command can execute
    /// in its current state. `parameter` is data used by the command; if
    /// the command does not require data, it can be `None`.
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool;

    /// Defines the method to be called when the command is invoked.
    fn execute(&self, parameter: Option<&BoxedValue>);

    /// Occurs when changes occur that affect whether or not the command
    /// should execute. Disposing the returned handle unsubscribes.
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;

    /// Registers an enablement handler and publishes its removal handle.
    ///
    /// The default implementation publishes after the legacy registration
    /// returns. Implementors whose add accessor can run application code
    /// can override this method to publish immediately after inserting the
    /// handler, so a reentrant caller can remove it during that accessor.
    /// Call `publish` once, synchronously. It may dispose the handle immediately,
    /// so release any borrows of the handler list before calling it.
    fn register_can_execute_changed(
        &self,
        handler: Rc<dyn Fn()>,
        publish: &mut dyn FnMut(Rc<dyn IDisposable>),
    ) {
        publish(self.can_execute_changed(handler));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactive::Disposable;
    use std::cell::Cell;

    struct LegacyCommand;

    impl ICommand for LegacyCommand {
        fn can_execute(&self, _: Option<&BoxedValue>) -> bool { true }
        fn execute(&self, _: Option<&BoxedValue>) {}
        fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            handler();
            Disposable::empty()
        }
    }

    #[test]
    fn default_registration_preserves_synchronous_legacy_handlers() {
        let sender: &dyn ICommand = &LegacyCommand;
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        let mut publications = 0;
        sender.register_can_execute_changed(Rc::new(move || seen.set(seen.get() + 1)), &mut |token| {
            assert_eq!(calls.get(), 1);
            publications += 1;
            token.dispose();
        });
        assert_eq!(publications, 1);
    }
}

impl PartialEq for dyn ICommand {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const dyn ICommand, other as *const dyn ICommand)
    }
}
