//! `FerroScheduler` (ReactiveUI.Primitives, the scheduler of the user-interface thread in the
//! `Reactive` flavour of the integration, `Concurrency/...Scheduler.cs`), with the queue and the
//! drain it embeds (`CoalescingDispatchState.cs`) and its work items (`DispatchWorkItem.cs`,
//! `DispatchWorkState.cs`), and the installation of the dispatcher's synchronization context as
//! the context of the user-interface thread.

use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer, FerroSynchronizationContext};
use ferroui_reactive::concurrency::{IScheduler, ScheduledAction, Scheduler};
use ferroui_reactive::disposables::{Disposable, StableCompositeDisposable};
use ferroui_reactiveui::IThreadAffineSequencer;
use ferroui_system::threading::{set_synchronization_context, SynchronizationContext};
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

ferroui_base::ferro_thread_local! {
    /// The shared scheduler of the user-interface thread (`Instance`).
    static INSTANCE: RefCell<Option<Rc<FerroScheduler>>> = const { RefCell::new(None) };
    /// The schedulers created for the dispatchers other than the one of [`FerroScheduler::instance`]
    /// (`ByDispatcher`).
    static BY_DISPATCHER: RefCell<Vec<Rc<FerroScheduler>>> = const { RefCell::new(Vec::new()) };
}

/// The scheduler of the user-interface thread: it runs its work on the thread of a
/// [`Dispatcher`], at a priority, and coalesces the work scheduled before the dispatcher runs it
/// into one job of the dispatcher (a drain).
///
/// Work runs in the jobs of the dispatcher, never inline in [`schedule`](IScheduler::schedule);
/// disposing scheduled work that has not started drops it, and disposing work that has started
/// disposes what the work returned. Delayed work waits on a [`DispatcherTimer`].
///
/// [`RxSchedulers::set_main_thread_scheduler`](ferroui_reactiveui::RxSchedulers::set_main_thread_scheduler)
/// takes [`FerroScheduler::instance`] when the application uses ReactiveUI.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): the work is not `Send`, as the observables of
/// the port are not (`docs/porting/reactive.md`, section 5.1): the scheduler posts its drains with
/// [`Dispatcher::post_local`] and is used on the thread of its dispatcher, where upstream may
/// schedule from any thread. The shared instance and the schedulers of other dispatchers are kept
/// per thread. The constructor that takes the posting and the timers of a test (internal upstream)
/// is not ported.
pub struct FerroScheduler {
    this: Weak<FerroScheduler>,
    dispatcher: Arc<Dispatcher>,
    priority: DispatcherPriority,
    /// The work waiting for a drain (`_ready`).
    ready: RefCell<VecDeque<Rc<DispatchWorkItem>>>,
    /// Whether a drain is posted and has not started (`_drainPosted`).
    drain_posted: Cell<bool>,
}

impl fmt::Debug for FerroScheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FerroScheduler").field("dispatcher", &self.dispatcher).field("priority", &self.priority).finish()
    }
}

impl FerroScheduler {
    /// A scheduler of the dispatcher of the user-interface thread at
    /// [`DispatcherPriority::BACKGROUND`] (`new FerroScheduler(Dispatcher.UIThread)`). Most code
    /// takes the shared one, [`instance`](Self::instance).
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Rc<Self> {
        Self::with_dispatcher(&Dispatcher::ui_thread())
    }

    /// A scheduler of `dispatcher` at [`DispatcherPriority::BACKGROUND`] (`FerroScheduler(Dispatcher)`).
    pub fn with_dispatcher(dispatcher: &Arc<Dispatcher>) -> Rc<Self> {
        Self::with_priority(dispatcher, DispatcherPriority::BACKGROUND)
    }

    /// A scheduler of `dispatcher` that posts its drains and starts its timers at `priority`
    /// (`FerroScheduler(Dispatcher, DispatcherPriority)`).
    pub fn with_priority(dispatcher: &Arc<Dispatcher>, priority: DispatcherPriority) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            dispatcher: dispatcher.clone(),
            priority,
            ready: RefCell::new(VecDeque::new()),
            drain_posted: Cell::new(false),
        })
    }

    /// The shared scheduler of the dispatcher of the user-interface thread, at
    /// [`DispatcherPriority::BACKGROUND`] (`Instance`). A new one is made when the dispatcher of
    /// the user-interface thread is replaced (the dispatcher of a unit test).
    pub fn instance() -> Rc<Self> {
        let dispatcher = Dispatcher::ui_thread();
        let current = INSTANCE.with(|instance| instance.borrow().clone());
        if let Some(current) = current {
            if Arc::ptr_eq(&current.dispatcher, &dispatcher) {
                return current;
            }
        }
        let created = Self::with_dispatcher(&dispatcher);
        let previous = INSTANCE.with(|instance| instance.borrow_mut().replace(created.clone()));
        drop(previous);
        created
    }

    /// The scheduler of `dispatcher`, created once per dispatcher: [`instance`](Self::instance)
    /// for its own dispatcher, otherwise one scheduler per dispatcher at
    /// [`DispatcherPriority::BACKGROUND`] (`For(Dispatcher)`).
    pub fn for_dispatcher(dispatcher: &Arc<Dispatcher>) -> Rc<Self> {
        let instance = Self::instance();
        if Arc::ptr_eq(&instance.dispatcher, dispatcher) {
            return instance;
        }
        let existing = BY_DISPATCHER.with(|schedulers| {
            schedulers.borrow().iter().find(|scheduler| Arc::ptr_eq(&scheduler.dispatcher, dispatcher)).cloned()
        });
        if let Some(existing) = existing {
            return existing;
        }
        let created = Self::with_dispatcher(dispatcher);
        BY_DISPATCHER.with(|schedulers| schedulers.borrow_mut().push(created.clone()));
        created
    }

    /// The dispatcher the scheduler runs its work on (`Dispatcher`).
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    /// The priority of the drains and of the timers of delayed work (`Priority`).
    pub fn priority(&self) -> DispatcherPriority {
        self.priority
    }

    /// Whether the calling thread is the thread of the dispatcher, which may run the work of the
    /// scheduler inline (`CheckAccess`).
    pub fn check_access(&self) -> bool {
        self.dispatcher.check_access()
    }

    /// Makes the synchronization context of the dispatcher of the user-interface thread the
    /// synchronization context of that thread (`SynchronizationContext.SetSynchronizationContext`),
    /// which also makes it the thread poster of the thread
    /// ([`ferroui_system::threading::tasks::set_thread_poster`]): a task of the user-interface
    /// thread woken from another thread is resumed through the dispatcher.
    ///
    /// The context is the dispatcher's [`FerroSynchronizationContext`] of the thread (installed if
    /// it has none yet), at [`DispatcherPriority::NORMAL`] when the thread has none. Nothing changes
    /// when that context is already the current one.
    ///
    /// # Panics
    /// Panics when the calling thread is not the user-interface thread.
    // Deviation (DEVIATIONS.md, ReactiveUI controls): the framework of the reference installs its
    // context itself; the base crate of FerroUI keeps its own current context, so the integration
    // installs it as the context of `ferroui-system`, which the tasks and the reactive libraries
    // read.
    pub fn install_on_ui_thread() {
        let dispatcher = Dispatcher::ui_thread();
        dispatcher.verify_access();
        FerroSynchronizationContext::install_if_needed();
        let context = FerroSynchronizationContext::current()
            .unwrap_or_else(|| FerroSynchronizationContext::with_dispatcher(&dispatcher, DispatcherPriority::NORMAL));
        let context: Arc<dyn SynchronizationContext> = context;
        if <dyn SynchronizationContext>::is_current(&context) {
            return;
        }
        set_synchronization_context(Some(context));
    }

    /// The scheduler as the scheduler handed to its work.
    fn this(&self) -> Rc<dyn IScheduler> {
        self.this.upgrade().expect("the scheduler is alive while it schedules")
    }

    /// Queues work for the next drain and posts the drain if none is posted (`Enqueue`).
    fn enqueue(&self, item: Rc<DispatchWorkItem>) {
        self.ready.borrow_mut().push_back(item);
        self.post_drain();
    }

    /// Posts a drain when work is waiting and no drain is posted (`PostDrain`).
    fn post_drain(&self) {
        if self.ready.borrow().is_empty() || self.drain_posted.replace(true) {
            return;
        }
        let this = self.this.upgrade().expect("the scheduler is alive while it schedules");
        // The gate is reopened when posting fails (a call from another thread), as upstream
        // resets it when the post throws.
        let reopen = ReopenDrain { scheduler: self, posted: Cell::new(false) };
        self.dispatcher.post_local(move || this.run_drain(), self.priority);
        reopen.posted.set(true);
    }

    /// Runs one drain: the work that was waiting when it started, in order; a drain is posted
    /// again when work remains, also when a work panics (`RunDrain`).
    fn run_drain(&self) {
        self.drain_posted.set(false);
        let _repost = RepostDrain { scheduler: self };
        let mut remaining = self.ready.borrow().len();
        while remaining > 0 {
            let item = self.ready.borrow_mut().pop_front();
            let Some(item) = item else {
                break;
            };
            remaining -= 1;
            item.run();
        }
    }

    /// Starts a timer of the dispatcher that runs `item` once when `due_time` has passed
    /// (`StartDispatcherTimer`); disposing the result stops the timer.
    fn start_dispatcher_timer(&self, item: Rc<DispatchWorkItem>, due_time: Duration) -> Rc<dyn IDisposable> {
        let timer = DispatcherTimer::with_interval(due_time, self.priority, &self.dispatcher);
        let _ = timer.tick(move |timer: &DispatcherTimer| {
            timer.stop();
            item.run();
        });
        timer.start();
        Disposable::create(move || timer.stop())
    }
}

/// Posts a drain again when work remains at the end of a drain, also while a panic of a work
/// unwinds (the `finally` of `RunDrain`).
struct RepostDrain<'a> {
    scheduler: &'a FerroScheduler,
}

impl Drop for RepostDrain<'_> {
    fn drop(&mut self) {
        let remains = !self.scheduler.ready.borrow().is_empty();
        if remains {
            self.scheduler.post_drain();
        }
    }
}

/// Reopens the gate of the drains when the post of a drain did not complete (the `catch` of
/// `PostDrain`).
struct ReopenDrain<'a> {
    scheduler: &'a FerroScheduler,
    posted: Cell<bool>,
}

impl Drop for ReopenDrain<'_> {
    fn drop(&mut self) {
        if !self.posted.get() {
            self.scheduler.drain_posted.set(false);
        }
    }
}

impl IScheduler for FerroScheduler {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    /// Queues `action` for the next drain of the dispatcher (`Schedule<TState>(TState, Func<...>)`).
    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let item = DispatchWorkItem::new(self.this(), action);
        self.enqueue(item.clone());
        item
    }

    /// Queues `action` for the next drain when it is due now, otherwise starts a timer of the
    /// dispatcher for it (`Schedule<TState>(TState, TimeSpan, Func<...>)`).
    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let item = DispatchWorkItem::new(self.this(), action);
        if due_time.is_zero() {
            self.enqueue(item.clone());
            return item;
        }
        let timer = self.start_dispatcher_timer(item.clone(), due_time);
        StableCompositeDisposable::create(timer, item)
    }

    /// [`schedule_after`](IScheduler::schedule_after) with the time from now to `due_time`, zero
    /// when it is past (`LocalScheduler.Schedule<TState>(TState, DateTimeOffset, ...)`).
    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        let due = due_time.duration_since(self.now()).unwrap_or(Duration::ZERO);
        self.schedule_after(due, action)
    }
}

impl IThreadAffineSequencer for FerroScheduler {
    fn check_access(&self) -> bool {
        self.dispatcher.check_access()
    }
}

/// One scheduled work: the action and the scheduler handed to it, what the action returned once
/// it ran, and whether the work was disposed (`DispatchWorkItem<TState>` and its
/// `DispatchWorkState<TState>`).
struct DispatchWorkItem {
    scheduler: Rc<dyn IScheduler>,
    action: RefCell<Option<ScheduledAction>>,
    /// What the action returned (`_disposable`).
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    is_disposed: Cell<bool>,
}

impl DispatchWorkItem {
    fn new(scheduler: Rc<dyn IScheduler>, action: ScheduledAction) -> Rc<Self> {
        Rc::new(Self { scheduler, action: RefCell::new(Some(action)), disposable: RefCell::new(None), is_disposed: Cell::new(false) })
    }

    /// Runs the action unless the work was disposed, and disposes what it returned when the work
    /// was disposed while it ran (`Run`).
    fn run(&self) {
        if self.is_disposed.get() {
            return;
        }
        let action = self.action.borrow_mut().take();
        let Some(action) = action else {
            return;
        };
        let disposable = action(&self.scheduler);
        if self.is_disposed.get() {
            disposable.dispose();
            return;
        }
        *self.disposable.borrow_mut() = Some(disposable);
    }
}

impl IDisposable for DispatchWorkItem {
    /// Drops the action when it has not run, and disposes what it returned when it has; only the
    /// first call does anything (`Dispose`).
    fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }
        let action = self.action.borrow_mut().take();
        drop(action);
        let started = self.disposable.borrow_mut().take();
        if let Some(started) = started {
            started.dispose();
        }
    }
}
