//! `Interaction<TInput, TOutput>` (ReactiveUI.Binding, `Interactions/Interaction.cs`), with the
//! copy-on-write list of its handlers (`Helpers/CopyOnWriteArray.cs`).

use super::{IInteraction, IInteractionContext, IOutputContext, InteractionContext, UnhandledInteractionException};
use crate::PanicException;
use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::linq::Observable;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::fmt;
use std::future::Future;
use std::rc::Rc;

/// A registered handler of an interaction, as the interaction keeps it: it receives the context
/// and gives the task that ends when it has finished (`Func<IInteractionContext<TInput, TOutput>, Task>`).
pub type InteractionHandler<I, O> = Rc<dyn Fn(Rc<dyn IInteractionContext<I, O>>) -> Task<()>>;

/// An interaction between collaborating parts of an application (`Interaction<TInput, TOutput>`).
/// It is a shared handle: clones are the same interaction.
///
/// The handlers run in the reverse order of their registration: the handler registered last has
/// the first chance to handle a question. A handler need not handle it, and leaves it to the next
/// one; [`handle`](Self::handle) fails with an [`UnhandledInteractionException`] when no handler
/// set an output.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's virtual members (`Handle`, `GenerateContext`)
/// and protected `GetHandlers` serve subclasses; Rust has none, so `handle` and
/// [`get_handlers`](Self::get_handlers) are public and the context is always an
/// [`InteractionContext`]. A handler that throws upstream faults the question; here a handler that
/// panics does, its panic caught as a [`PanicException`].
pub struct Interaction<I, O> {
    inner: Rc<InteractionInner<I, O>>,
}

struct InteractionInner<I, O> {
    /// `_handlerScheduler`.
    handler_scheduler: Option<Rc<dyn IScheduler>>,
    /// `_handlers`: replaced rather than changed, so that a question already being handled walks
    /// the set it started with.
    handlers: RefCell<Rc<Vec<InteractionHandler<I, O>>>>,
}

impl<I, O> Clone for Interaction<I, O> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<I, O> PartialEq for Interaction<I, O> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<I, O> fmt::Debug for Interaction<I, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Interaction").field("handlers", &self.inner.handlers.borrow().len()).finish()
    }
}

impl<I: Clone + 'static, O: Clone + 'static> Default for Interaction<I, O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: Clone + 'static, O: Clone + 'static> Interaction<I, O> {
    /// An interaction that invokes its handlers on the thread that asks (`Interaction()`).
    pub fn new() -> Self {
        Self::with_scheduler(None)
    }

    /// An interaction that invokes each handler on `handler_scheduler`, such as the main thread's;
    /// `None` invokes them on the thread that asks (`Interaction(IScheduler?)`).
    pub fn with_scheduler(handler_scheduler: Option<Rc<dyn IScheduler>>) -> Self {
        Self { inner: Rc::new(InteractionInner { handler_scheduler, handlers: RefCell::new(Rc::new(Vec::new())) }) }
    }

    /// Registers a synchronous handler; disposing the result unregisters it
    /// (`RegisterHandler(Action<IInteractionContext<TInput, TOutput>>)`).
    pub fn register_handler(&self, handler: impl Fn(Rc<dyn IInteractionContext<I, O>>) + 'static) -> Rc<dyn IDisposable> {
        self.add_handler(Rc::new(move |context| {
            handler(context);
            Task::from_result(())
        }))
    }

    /// Registers an asynchronous handler: the future it gives runs as a task of the thread, and
    /// the next handler runs when it has ended (`RegisterHandler(Func<IInteractionContext<TInput,
    /// TOutput>, Task>)`). A `Task<()>` is such a future.
    pub fn register_handler_task<Fut: Future<Output = Result<(), ObservableError>> + 'static>(
        &self,
        handler: impl Fn(Rc<dyn IInteractionContext<I, O>>) -> Fut + 'static,
    ) -> Rc<dyn IDisposable> {
        self.add_handler(Rc::new(move |context| Task::spawn_local(handler(context))))
    }

    /// Registers a handler that finishes when the observable it gives ends; its values are ignored,
    /// and its error faults the question (`RegisterHandler<TDontCare>(Func<IInteractionContext<TInput,
    /// TOutput>, IObservable<TDontCare>>)`).
    pub fn register_handler_observable<D: 'static>(
        &self,
        handler: impl Fn(Rc<dyn IInteractionContext<I, O>>) -> Rc<dyn IObservable<D>> + 'static,
    ) -> Rc<dyn IDisposable> {
        self.add_handler(Rc::new(move |context| {
            let completion = TaskCompletionSource::new();
            let _ = handler(context).subscribe(Rc::new(ObservableToTaskObserver { completion: completion.clone() }));
            completion.task()
        }))
    }

    /// Runs the handlers, latest registered first, until one sets an output, and gives a task of
    /// that output; the task fails with an [`UnhandledInteractionException`] when no handler set
    /// one (`Handle(TInput)`).
    pub fn handle(&self, input: I) -> Task<O> {
        let this = self.clone();
        Task::spawn_local(async move {
            let context = this.generate_context(input.clone());
            let handlers = this.inner.handlers.borrow().clone();
            for handler in handlers.iter().rev() {
                this.invoke_handler(handler, &context).await?;
                if context.is_handled() {
                    return Ok(context.get_output());
                }
            }
            Err(Rc::new(UnhandledInteractionException::new(this.clone(), input)) as ObservableError)
        })
    }

    /// Asks the question as an observable: each subscription runs [`handle`](Self::handle) once,
    /// then notifies the output and ends, or fails with an [`UnhandledInteractionException`]
    /// (`WhenHandled(TInput)`). Disposing the subscription drops the output; a handler that is
    /// running still finishes.
    pub fn when_handled(&self, input: I) -> Rc<dyn IObservable<O>> {
        let this = self.clone();
        Observable::from_async(move || this.handle(input.clone()))
    }

    /// A copy of the registered handlers, earliest first (`GetHandlers()`).
    pub fn get_handlers(&self) -> Vec<InteractionHandler<I, O>> {
        self.inner.handlers.borrow().iter().cloned().collect()
    }

    /// The context every handler receives for one question (`GenerateContext(TInput)`).
    fn generate_context(&self, input: I) -> Rc<dyn IOutputContext<I, O>> {
        Rc::new(InteractionContext::new(input))
    }

    /// Invokes a handler on the handler scheduler, or inline when there is none
    /// (`InvokeHandler`); the task ends when the handler has finished.
    fn invoke_handler(&self, handler: &InteractionHandler<I, O>, context: &Rc<dyn IOutputContext<I, O>>) -> Task<()> {
        let handler_context: Rc<dyn IInteractionContext<I, O>> = context.clone();
        let Some(scheduler) = &self.inner.handler_scheduler else {
            return run_handler(handler, handler_context);
        };
        // `ScheduledHandlerCall`: completes with the handler's task once the scheduler has invoked
        // the handler.
        let completion = TaskCompletionSource::new();
        let started = completion.clone();
        let handler = handler.clone();
        let _ = scheduler.schedule_action(move || {
            let running = run_handler(&handler, handler_context);
            let _ = running.continue_with(move |running| {
                if running.is_canceled() {
                    started.try_set_canceled();
                    return;
                }
                match running.result() {
                    Ok(()) => {
                        started.try_set_result(());
                    }
                    Err(error) => {
                        started.try_set_exception(error);
                    }
                }
            });
        });
        completion.task()
    }

    /// Adds a handler to the registered set, and returns the disposable that removes the first
    /// registration of it (`AddHandler`, `RemoveHandler`; `CopyOnWriteArray.Add` and `Remove`).
    fn add_handler(&self, handler: InteractionHandler<I, O>) -> Rc<dyn IDisposable> {
        {
            let mut handlers = self.inner.handlers.borrow_mut();
            let mut updated: Vec<_> = handlers.iter().cloned().collect();
            updated.push(handler.clone());
            *handlers = Rc::new(updated);
        }
        let inner = Rc::downgrade(&self.inner);
        Disposable::create(move || {
            let Some(inner) = inner.upgrade() else { return };
            let mut handlers = inner.handlers.borrow_mut();
            if let Some(index) = handlers.iter().position(|registered| Rc::ptr_eq(registered, &handler)) {
                let mut updated: Vec<_> = handlers.iter().cloned().collect();
                updated.remove(index);
                *handlers = Rc::new(updated);
            }
        })
    }
}

/// Runs a handler: its task, or a faulted task when it panics (the exception a handler throws
/// upstream faults `Handle`).
fn run_handler<I, O>(handler: &InteractionHandler<I, O>, context: Rc<dyn IInteractionContext<I, O>>) -> Task<()> {
    match PanicException::catch(|| handler(context)) {
        Ok(task) => task,
        Err((error, _)) => Task::from_exception(error),
    }
}

impl<I: Clone + 'static, O: Clone + 'static> IInteraction<I, O> for Interaction<I, O> {
    fn register_handler(&self, handler: Rc<dyn Fn(Rc<dyn IInteractionContext<I, O>>)>) -> Rc<dyn IDisposable> {
        Interaction::register_handler(self, move |context| handler(context))
    }

    fn register_handler_task(&self, handler: Rc<dyn Fn(Rc<dyn IInteractionContext<I, O>>) -> Task<()>>) -> Rc<dyn IDisposable> {
        self.add_handler(Rc::new(move |context| handler(context)))
    }

    fn register_handler_observable(
        &self,
        handler: Rc<dyn Fn(Rc<dyn IInteractionContext<I, O>>) -> Rc<dyn IObservable<()>>>,
    ) -> Rc<dyn IDisposable> {
        Interaction::register_handler_observable(self, move |context| handler(context))
    }

    fn handle(&self, input: I) -> Rc<dyn IObservable<O>> {
        self.when_handled(input)
    }
}

/// `ObservableToTaskObserver<T>`: ends the task of a handler when its observable ends or fails.
struct ObservableToTaskObserver {
    completion: TaskCompletionSource<()>,
}

impl<T> IObserver<T> for ObservableToTaskObserver {
    fn on_next(&self, _value: T) {
        // Intentionally empty: only the end and the error matter.
    }

    fn on_error(&self, error: ObservableError) {
        self.completion.try_set_exception(error);
    }

    fn on_completed(&self) {
        self.completion.try_set_result(());
    }
}
