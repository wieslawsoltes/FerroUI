use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::ICommand;
use ferroui_system::{IDisposable, ObservableError, events::Event};
use futures_channel::oneshot;
use futures_util::FutureExt;
use std::{
    any::Any,
    cell::Cell,
    future::Future,
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

pub type RibbonCommandAction = dyn Fn(Option<&dyn Any>);
pub type RibbonCommandPredicate = dyn Fn(Option<&dyn Any>) -> bool;
pub type RibbonAsyncCommandFuture = Pin<Box<dyn Future<Output = Result<(), ObservableError>>>>;
/// Application-provided local future executor. It must retain submitted work
/// through completion and surface an unhandled Err. Core owns no runtime.
pub type RibbonCommandExecutor = dyn Fn(RibbonAsyncCommandFuture);
pub type RibbonAsyncCommandAction = dyn Fn(Option<&dyn Any>) -> RibbonAsyncCommandFuture;
pub type RibbonCommandErrorHandler = dyn Fn(&Rc<RibbonCommandErrorEventArgs>);

/// Original relay algorithm using the existing UI-independent command contract.
pub struct RibbonRelayCommand {
    execute: Rc<RibbonCommandAction>,
    can_execute: Option<Rc<RibbonCommandPredicate>>,
    can_execute_changed: Rc<Event<()>>,
}

impl RibbonRelayCommand {
    pub fn new(execute: impl Fn() + 'static) -> Rc<Self> {
        Self::new_with_can_execute(execute, None)
    }
    pub fn new_with_can_execute(
        execute: impl Fn() + 'static,
        can_execute: Option<Rc<dyn Fn() -> bool>>,
    ) -> Rc<Self> {
        Self::new_with_parameter(
            move |_| execute(),
            can_execute
                .map(|f| Rc::new(move |_: Option<&dyn Any>| f()) as Rc<RibbonCommandPredicate>),
        )
    }
    pub fn new_with_parameter(
        execute: impl Fn(Option<&dyn Any>) + 'static,
        can_execute: Option<Rc<RibbonCommandPredicate>>,
    ) -> Rc<Self> {
        Rc::new(Self::construct(Rc::new(execute), can_execute))
    }
    /// Base field initialization for the original derived generic command.
    pub fn construct(
        execute: Rc<RibbonCommandAction>,
        can_execute: Option<Rc<RibbonCommandPredicate>>,
    ) -> Self {
        Self {
            execute,
            can_execute,
            can_execute_changed: Rc::new(Event::new()),
        }
    }
    pub fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        self.can_execute.as_ref().is_none_or(|f| f(parameter))
    }
    pub fn execute(&self, parameter: Option<&dyn Any>) {
        if self.can_execute(parameter) {
            (self.execute)(parameter);
        }
    }
    pub fn notify_can_execute_changed(&self) {
        self.can_execute_changed.raise(&());
    }
    pub fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.can_execute_changed, Rc::new(move |_| handler()))
    }
}
impl PartialEq for RibbonRelayCommand {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ICommand for RibbonRelayCommand {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        Self::can_execute(self, parameter)
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        Self::execute(self, parameter);
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Self::can_execute_changed(self, handler)
    }
}

/// The `p is T t ? t : default` conversion used by the source generic command.
/// Nullable/reference parameters use Option<T>; value parameters retain their default value.
pub trait RibbonCommandParameter: Clone + 'static {
    fn from_command_parameter(parameter: Option<&dyn Any>) -> Self;
}
macro_rules! value_parameters {
    ($($t:ty),* $(,)?) => {$(impl RibbonCommandParameter for $t {
        fn from_command_parameter(parameter: Option<&dyn Any>) -> Self {
            parameter.and_then(|p|p.downcast_ref::<Self>()).copied().unwrap_or_default()
        }
    })*};
}
value_parameters!(bool, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64, char);
impl<T: Clone + 'static> RibbonCommandParameter for Option<T> {
    fn from_command_parameter(parameter: Option<&dyn Any>) -> Self {
        parameter.and_then(|p| p.downcast_ref::<T>()).cloned()
    }
}

/// Source RibbonRelayCommand<T>; `Of` disambiguates CLR arity in Rust.
/// The same embedded base/event state is used by direct and interface calls.
pub struct RibbonRelayCommandOf<T: RibbonCommandParameter> {
    base: RibbonRelayCommand,
    marker: PhantomData<fn(T)>,
}
impl<T: RibbonCommandParameter> RibbonRelayCommandOf<T> {
    pub fn new(
        execute: impl Fn(T) + 'static,
        can_execute: Option<Rc<dyn Fn(T) -> bool>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: RibbonRelayCommand::construct(
                Rc::new(move |parameter| execute(T::from_command_parameter(parameter))),
                can_execute.map(|f| {
                    Rc::new(move |p: Option<&dyn Any>| f(T::from_command_parameter(p)))
                        as Rc<RibbonCommandPredicate>
                }),
            ),
            marker: PhantomData,
        })
    }
}
impl<T: RibbonCommandParameter> Deref for RibbonRelayCommandOf<T> {
    type Target = RibbonRelayCommand;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl<T: RibbonCommandParameter> PartialEq for RibbonRelayCommandOf<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<T: RibbonCommandParameter> ICommand for RibbonRelayCommandOf<T> {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        self.base.can_execute(parameter)
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        self.base.execute(parameter);
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.base.can_execute_changed(handler)
    }
}

/// Shared source error-event args. Every handler sees the same mutable Handled flag.
pub struct RibbonCommandErrorEventArgs {
    exception: ObservableError,
    handled: Cell<bool>,
}
impl RibbonCommandErrorEventArgs {
    pub fn new(exception: ObservableError) -> Rc<Self> {
        Rc::new(Self {
            exception,
            handled: Cell::new(false),
        })
    }
    pub fn exception(&self) -> ObservableError {
        self.exception.clone()
    }
    pub fn handled(&self) -> bool {
        self.handled.get()
    }
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}
impl PartialEq for RibbonCommandErrorEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub struct RibbonAsyncCommand {
    executor: Rc<RibbonCommandExecutor>,
    execute: Rc<RibbonAsyncCommandAction>,
    can_execute: Option<Rc<RibbonCommandPredicate>>,
    is_running: Cell<bool>,
    failed: Rc<Event<Rc<RibbonCommandErrorEventArgs>>>,
    can_execute_changed: Rc<Event<()>>,
    this: std::rc::Weak<Self>,
}
impl RibbonAsyncCommand {
    pub fn new<Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        executor: Rc<RibbonCommandExecutor>,
        execute: impl Fn() -> Fut + 'static,
    ) -> Rc<Self> {
        Self::new_with_can_execute(executor, execute, None)
    }
    pub fn new_with_can_execute<Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        executor: Rc<RibbonCommandExecutor>,
        execute: impl Fn() -> Fut + 'static,
        can_execute: Option<Rc<dyn Fn() -> bool>>,
    ) -> Rc<Self> {
        Self::new_with_parameter(
            executor,
            move |_| execute(),
            can_execute
                .map(|f| Rc::new(move |_: Option<&dyn Any>| f()) as Rc<RibbonCommandPredicate>),
        )
    }
    pub fn new_with_parameter<Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        executor: Rc<RibbonCommandExecutor>,
        execute: impl Fn(Option<&dyn Any>) -> Fut + 'static,
        can_execute: Option<Rc<RibbonCommandPredicate>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            executor,
            execute: Rc::new(move |parameter| Box::pin(execute(parameter))),
            can_execute,
            is_running: Cell::new(false),
            failed: Rc::new(Event::new()),
            can_execute_changed: Rc::new(Event::new()),
            this: this.clone(),
        })
    }
    pub fn is_running(&self) -> bool {
        self.is_running.get()
    }
    pub fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        !self.is_running.get() && self.can_execute.as_ref().is_none_or(|f| f(parameter))
    }
    pub fn notify_can_execute_changed(&self) {
        self.can_execute_changed.raise(&());
    }
    pub fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        subscribe(&self.can_execute_changed, Rc::new(move |_| handler()))
    }
    pub fn failed(&self, handler: Rc<RibbonCommandErrorHandler>) -> Rc<dyn IDisposable> {
        subscribe(&self.failed, handler)
    }

    /// Starts through the injected executor and returns a standard Rust future.
    /// Dropping that future abandons observation, not execution. Borrowed parameters
    /// are used only by the synchronous factory; its future must own retained data.
    pub fn execute_async(&self, parameter: Option<&dyn Any>) -> RibbonAsyncCommandFuture {
        let start = catch(|| {
            if !self.can_execute(parameter) {
                return false;
            }
            self.is_running.set(true);
            // Before the try/finally, exactly as in the source.
            self.notify_can_execute_changed();
            true
        });
        match start {
            Ok(false) => return Box::pin(std::future::ready(Ok(()))),
            Err(error) => return Box::pin(std::future::ready(Err(error))),
            Ok(true) => {}
        }
        let mut future = match catch(|| (self.execute)(parameter)) {
            Ok(future) => future,
            Err(error) => return Box::pin(std::future::ready(self.finish(Err(error)))),
        };
        // A source async callback runs through its first await before returning.
        // Poll once, without creating a runtime. Pending work gets the executor's
        // real waker on submission; completed work needs no scheduled continuation.
        match catch(|| {
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
        }) {
            Ok(Poll::Ready(outcome)) => return Box::pin(std::future::ready(self.finish(outcome))),
            Err(error) => return Box::pin(std::future::ready(self.finish(Err(error)))),
            Ok(Poll::Pending) => {}
        }
        let (sender, receiver) = oneshot::channel();
        let completion = CommandCompletion {
            command: self.this.upgrade().expect("live async command"),
            sender: Some(sender),
        };
        (self.executor)(Box::pin(async move {
            let outcome = std::panic::AssertUnwindSafe(future)
                .catch_unwind()
                .await
                .unwrap_or_else(|payload| Err(panic_error(payload)));
            completion.finish(outcome);
            Ok(())
        }));
        Box::pin(async move { receiver.await.unwrap_or_else(|_| Err(canceled())) })
    }
    fn finish(&self, outcome: Result<(), ObservableError>) -> Result<(), ObservableError> {
        let outcome = match outcome {
            Ok(()) => Ok(()),
            Err(error) => {
                let args = RibbonCommandErrorEventArgs::new(error.clone());
                match catch(|| self.failed.raise(&args)) {
                    Err(handler_error) => Err(handler_error),
                    Ok(()) if args.handled() => Ok(()),
                    Ok(()) => Err(error),
                }
            }
        };
        self.is_running.set(false);
        // Finally's exception replaces the exception already in flight.
        catch(|| self.notify_can_execute_changed())?;
        outcome
    }
    pub fn execute(&self, parameter: Option<&dyn Any>) {
        let completion = self.execute_async(parameter);
        (self.executor)(completion);
    }
}
impl PartialEq for RibbonAsyncCommand {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ICommand for RibbonAsyncCommand {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        Self::can_execute(self, parameter)
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        Self::execute(self, parameter);
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Self::can_execute_changed(self, handler)
    }
}

fn subscribe<A: 'static>(event: &Rc<Event<A>>, handler: Rc<dyn Fn(&A)>) -> Rc<dyn IDisposable> {
    let token = event.add(handler);
    let event = Rc::downgrade(event);
    Disposable::create(move || {
        if let Some(event) = event.upgrade() {
            event.remove(token);
        }
    })
}
fn catch<T>(action: impl FnOnce() -> T) -> Result<T, ObservableError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).map_err(panic_error)
}

#[derive(Debug)]
struct CallbackPanicError(String);
impl std::fmt::Display for CallbackPanicError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for CallbackPanicError {}
fn panic_error(payload: Box<dyn Any + Send>) -> ObservableError {
    let message = if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        "Non-string panic in ribbon command callback".to_owned()
    };
    Rc::new(CallbackPanicError(message))
}
fn canceled() -> ObservableError {
    Rc::new(std::io::Error::new(
        std::io::ErrorKind::Interrupted,
        "Async command execution was cancelled before completion",
    ))
}

// Owns only one command invocation. Executor shutdown/drop completes its normal
// Failed/finally path; no task state machine, waker storage or executor lives here.
struct CommandCompletion {
    command: Rc<RibbonAsyncCommand>,
    sender: Option<oneshot::Sender<Result<(), ObservableError>>>,
}
impl CommandCompletion {
    fn finish(mut self, outcome: Result<(), ObservableError>) {
        let sender = self.sender.take().expect("single command completion");
        let outcome = self.command.finish(outcome);
        // Abandoning observation must not skip Failed/finally or retain the command.
        let _ = sender.send(outcome);
    }
}
impl Drop for CommandCompletion {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            let outcome = self.command.finish(Err(canceled()));
            let _ = sender.send(outcome);
        }
    }
}
