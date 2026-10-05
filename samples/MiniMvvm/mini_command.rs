//! Port of `MiniCommand.cs`.

use ferroui_base::input::ICommand;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{
    Dispatcher, DispatcherPriority, DispatcherTask, DispatcherTimer, FerroSynchronizationContext,
};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

/// Starts `future` the way calling an asynchronous method does:
/// synchronously up to its first pending await, then resumed through the
/// synchronization context of the calling thread (the dispatcher's context
/// when the thread has none). The returned task is the `Task` of the
/// method; dropping it does not cancel the future (`_ = FooAsync()`,
/// `async void`).
///
/// # Panics
/// Panics when called from a thread other than the dispatcher thread.
pub fn start_async<T: 'static>(future: impl Future<Output = T> + 'static) -> DispatcherTask<T> {
    let context = FerroSynchronizationContext::current().unwrap_or_else(|| {
        FerroSynchronizationContext::with_dispatcher(&Dispatcher::current_dispatcher(), DispatcherPriority::NORMAL)
    });
    context.to_task_scheduler().start_local(future)
}

/// The state of a [`Delay`]: whether its time has passed, and who waits.
#[derive(Default)]
struct DelayState {
    elapsed: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// The future [`delay`] returns.
pub struct Delay(Rc<DelayState>);

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.elapsed.get() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// `Task.Delay(duration)` for the asynchronous methods of the samples:
/// completes when a dispatcher timer of `duration` has ticked.
///
/// # Panics
/// Panics when called from a thread other than the dispatcher thread.
pub fn delay(duration: Duration) -> Delay {
    let state = Rc::new(DelayState::default());
    let timer_state = state.clone();
    DispatcherTimer::run_once(
        move || {
            timer_state.elapsed.set(true);
            let waker = timer_state.waker.borrow_mut().take();
            if let Some(waker) = waker {
                waker.wake();
            }
        },
        duration,
        DispatcherPriority::DEFAULT,
    );
    Delay(state)
}

/// The task of an asynchronous command callback (`Task`).
pub type MiniCommandTask = Pin<Box<dyn Future<Output = ()>>>;

enum Callback {
    /// `Action<T>`.
    Sync(Box<dyn Fn(Option<&BoxedValue>)>),
    /// `Func<T, Task>`.
    Async(Box<dyn Fn(Option<&BoxedValue>) -> MiniCommandTask>),
}

/// A command that runs a closure; it cannot execute while its closure runs.
///
/// The generic class `MiniCommand<T>` and its abstract base of the managed
/// original are one type here: the typed factory ([`create_with`](Self::create_with))
/// performs the cast `(T)parameter` the generic class performs.
pub struct MiniCommand {
    callback: Callback,
    busy: Cell<bool>,
    can_execute_changed: RefCell<Vec<(u64, Rc<dyn Fn()>)>>,
    next_token: Cell<u64>,
    this: Weak<MiniCommand>,
}

impl MiniCommand {
    fn new(callback: Callback) -> Rc<MiniCommand> {
        Rc::new_cyclic(|this| MiniCommand {
            callback,
            busy: Cell::new(false),
            can_execute_changed: RefCell::new(Vec::new()),
            next_token: Cell::new(0),
            this: this.clone(),
        })
    }

    /// `MiniCommand.Create(Action cb)`.
    pub fn create(cb: impl Fn() + 'static) -> Rc<MiniCommand> {
        Self::new(Callback::Sync(Box::new(move |_| cb())))
    }

    /// `MiniCommand.Create<TArg>(Action<TArg> cb)`: the parameter is the
    /// value of type `TArg` the command is executed with.
    ///
    /// # Panics
    /// Executing the command with a parameter that is not a `TArg` panics
    /// (the invalid cast of the managed original).
    pub fn create_with<TArg: Clone + 'static>(cb: impl Fn(TArg) + 'static) -> Rc<MiniCommand> {
        Self::new(Callback::Sync(Box::new(move |parameter| cb(cast::<TArg>(parameter)))))
    }

    /// `MiniCommand.CreateFromTask(Func<Task> cb)`.
    pub fn create_from_task<Fut: Future<Output = ()> + 'static>(cb: impl Fn() -> Fut + 'static) -> Rc<MiniCommand> {
        Self::new(Callback::Async(Box::new(move |_| Box::pin(cb()))))
    }

    /// `new MiniCommand<T>(Func<T, Task> cb)`.
    pub fn create_from_task_with<TArg: Clone + 'static, Fut: Future<Output = ()> + 'static>(
        cb: impl Fn(TArg) -> Fut + 'static,
    ) -> Rc<MiniCommand> {
        Self::new(Callback::Async(Box::new(move |parameter| Box::pin(cb(cast::<TArg>(parameter))))))
    }

    /// The command as the contract controls take.
    pub fn as_command(self: &Rc<Self>) -> Rc<dyn ICommand> {
        self.clone()
    }

    fn is_busy(&self) -> bool {
        self.busy.get()
    }

    fn set_busy(&self, value: bool) {
        self.busy.set(value);
        let handlers: Vec<Rc<dyn Fn()>> = self.can_execute_changed.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler();
        }
    }
}

/// `(T)parameter`: a markup cast, so that a parameter a binding read from a
/// typed property of a markup class converts as well as one boxed as `T`.
fn cast<T: Clone + 'static>(parameter: Option<&BoxedValue>) -> T {
    match from_markup_value::<T>(&parameter.cloned()) {
        Some(value) => value,
        None => panic!("Unable to cast the command parameter to type '{}'.", std::any::type_name::<T>()),
    }
}

/// Clears the busy flag when the callback has finished, also when it
/// panicked (the `finally` of the managed original).
struct BusyScope(Rc<MiniCommand>);

impl Drop for BusyScope {
    fn drop(&mut self) {
        self.0.set_busy(false);
    }
}

impl ICommand for MiniCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        !self.busy.get()
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if self.is_busy() {
            return;
        }
        let Some(this) = self.this.upgrade() else { return };
        self.set_busy(true);
        let scope = BusyScope(this);
        match &self.callback {
            Callback::Sync(cb) => cb(parameter),
            Callback::Async(cb) => {
                // `async void`: the command is busy until the task ends.
                let task = cb(parameter);
                drop(start_async(async move {
                    let _scope = scope;
                    task.await;
                }));
                return;
            }
        }
        drop(scope);
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.can_execute_changed.borrow_mut().push((token, handler));
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.can_execute_changed.borrow_mut().retain(|(t, _)| *t != token);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream library has no tests.
    use super::*;

    #[test]
    fn create_runs_the_callback_and_reports_busy_around_it() {
        let runs = Rc::new(Cell::new(0));
        let sink = runs.clone();
        let command = MiniCommand::create(move || sink.set(sink.get() + 1));
        let changes = Rc::new(Cell::new(0));
        let changed = changes.clone();
        let subscription = command.can_execute_changed(Rc::new(move || changed.set(changed.get() + 1)));

        assert!(command.can_execute(None));
        command.execute(None);
        assert_eq!(1, runs.get());
        // Busy was set and cleared.
        assert_eq!(2, changes.get());
        assert!(command.can_execute(None));

        subscription.dispose();
        command.execute(None);
        assert_eq!(2, changes.get());
    }

    #[test]
    fn create_with_casts_the_parameter() {
        let seen = Rc::new(RefCell::new(String::new()));
        let sink = seen.clone();
        let command = MiniCommand::create_with::<String>(move |value| *sink.borrow_mut() = value);
        let parameter: BoxedValue = Rc::new(String::from("recent"));
        command.execute(Some(&parameter));
        assert_eq!("recent", *seen.borrow());
    }

    #[test]
    fn a_command_cannot_execute_while_its_callback_runs() {
        let observed = Rc::new(Cell::new(true));
        let slot: Rc<RefCell<Option<Rc<MiniCommand>>>> = Rc::new(RefCell::new(None));
        let (sink, inner) = (observed.clone(), slot.clone());
        let command = MiniCommand::create(move || {
            let command = inner.borrow().clone().expect("the command");
            sink.set(command.can_execute(None));
        });
        *slot.borrow_mut() = Some(command.clone());
        command.execute(None);
        assert!(!observed.get());
        *slot.borrow_mut() = None;
    }
}
