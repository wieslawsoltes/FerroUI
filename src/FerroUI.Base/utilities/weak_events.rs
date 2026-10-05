use crate::input::ICommand;
use crate::reactive::IDisposable;
use crate::{ObjectType, Ref};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Events that are subscribed to weakly: the subscription does not keep
/// the subscriber alive, and it does not outlive the subscriber either.
pub struct WeakEvents;

impl WeakEvents {
    /// Subscribes `subscriber` to the "can execute changed" event of
    /// `command`. `handler` is called with the subscriber each time the
    /// event is raised.
    ///
    /// The subscriber keeps the returned handle (normally in a field). The
    /// subscription ends when the handle is disposed, when the handle is
    /// dropped (so with the subscriber), or, if the handle is kept alive
    /// elsewhere, the first time the event is raised after the subscriber
    /// is gone. The command never holds a strong reference to the
    /// subscriber.
    pub fn command_can_execute_changed<T: ObjectType>(
        command: &Rc<dyn ICommand>,
        subscriber: &Ref<T>,
        handler: fn(&T),
    ) -> Rc<dyn IDisposable> {
        let subscription = Rc::new(WeakEventSubscription { inner: RefCell::new(None) });

        let weak_subscriber = subscriber.downgrade();
        let weak_subscription: Weak<WeakEventSubscription> = Rc::downgrade(&subscription);
        let inner = command.can_execute_changed(Rc::new(move || match weak_subscriber.upgrade() {
            Some(subscriber) => handler(&subscriber),
            None => {
                if let Some(subscription) = weak_subscription.upgrade() {
                    subscription.dispose();
                }
            }
        }));
        *subscription.inner.borrow_mut() = Some(inner);

        subscription
    }
}

/// The handle of a weak event subscription: unsubscribes when disposed and
/// when dropped.
struct WeakEventSubscription {
    inner: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl IDisposable for WeakEventSubscription {
    fn dispose(&self) {
        let inner = self.inner.borrow_mut().take();
        if let Some(inner) = inner {
            inner.dispose();
        }
    }
}

impl Drop for WeakEventSubscription {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KeyBinding;
    use crate::reactive::Disposable;
    use crate::BoxedValue;
    use std::cell::Cell;

    /// A command that counts its subscribers.
    struct CountingCommand {
        this: Weak<CountingCommand>,
        handlers: RefCell<Vec<(u64, Rc<dyn Fn()>)>>,
        next_token: Cell<u64>,
    }

    impl CountingCommand {
        fn new() -> Rc<Self> {
            Rc::new_cyclic(|this| Self { this: this.clone(), handlers: RefCell::new(Vec::new()), next_token: Cell::new(0) })
        }

        fn subscription_count(&self) -> usize {
            self.handlers.borrow().len()
        }

        fn raise(&self) {
            let handlers: Vec<Rc<dyn Fn()>> = self.handlers.borrow().iter().map(|(_, h)| h.clone()).collect();
            for handler in handlers {
                handler();
            }
        }
    }

    impl ICommand for CountingCommand {
        fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
            true
        }

        fn execute(&self, _parameter: Option<&BoxedValue>) {}

        fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
            let token = self.next_token.get();
            self.next_token.set(token + 1);
            self.handlers.borrow_mut().push((token, handler));
            let weak = self.this.clone();
            Disposable::create(move || {
                if let Some(this) = weak.upgrade() {
                    this.handlers.borrow_mut().retain(|(t, _)| *t != token);
                }
            })
        }
    }

    thread_local! {
        static CALLS: Cell<i32> = const { Cell::new(0) };
    }

    fn on_event(_subscriber: &KeyBinding) {
        CALLS.with(|calls| calls.set(calls.get() + 1));
    }

    #[test]
    fn handler_is_called_while_the_subscriber_is_alive() {
        CALLS.with(|calls| calls.set(0));
        let command = CountingCommand::new();
        let as_command: Rc<dyn ICommand> = command.clone();
        let subscriber = KeyBinding::new();

        let _subscription = WeakEvents::command_can_execute_changed(&as_command, &subscriber, on_event);
        command.raise();
        command.raise();

        assert_eq!(1, command.subscription_count());
        assert_eq!(2, CALLS.with(Cell::get));
    }

    #[test]
    fn subscription_does_not_keep_the_subscriber_alive() {
        let command = CountingCommand::new();
        let as_command: Rc<dyn ICommand> = command.clone();
        let subscriber = KeyBinding::new();
        let weak = subscriber.downgrade();

        let _subscription = WeakEvents::command_can_execute_changed(&as_command, &subscriber, on_event);
        drop(subscriber);

        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn dropping_the_handle_unsubscribes() {
        let command = CountingCommand::new();
        let as_command: Rc<dyn ICommand> = command.clone();
        let subscriber = KeyBinding::new();

        let subscription = WeakEvents::command_can_execute_changed(&as_command, &subscriber, on_event);
        assert_eq!(1, command.subscription_count());
        drop(subscription);

        assert_eq!(0, command.subscription_count());
    }

    #[test]
    fn disposing_the_handle_unsubscribes_once() {
        let command = CountingCommand::new();
        let as_command: Rc<dyn ICommand> = command.clone();
        let subscriber = KeyBinding::new();

        let subscription = WeakEvents::command_can_execute_changed(&as_command, &subscriber, on_event);
        subscription.dispose();
        subscription.dispose();

        assert_eq!(0, command.subscription_count());
    }

    #[test]
    fn subscription_removes_itself_on_the_next_raise_after_the_subscriber_is_gone() {
        CALLS.with(|calls| calls.set(0));
        let command = CountingCommand::new();
        let as_command: Rc<dyn ICommand> = command.clone();
        let subscriber = KeyBinding::new();

        // The handle is kept alive by someone other than the subscriber.
        let subscription = WeakEvents::command_can_execute_changed(&as_command, &subscriber, on_event);
        drop(subscriber);
        assert_eq!(1, command.subscription_count());

        command.raise();

        assert_eq!(0, command.subscription_count());
        assert_eq!(0, CALLS.with(Cell::get));
        drop(subscription);
    }
}
