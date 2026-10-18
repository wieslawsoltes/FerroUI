//! `CurrentThreadScheduler` (`CurrentThreadScheduler.cs`).

use super::local_scheduler::{sleep, SystemStopwatch, SystemStopwatchProvider};
use super::{IScheduler, IStopwatch, IStopwatchProvider, ScheduledItem, Scheduler, ScheduledAction};
use crate::internal::PriorityQueue;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

ferroui_system::ferro_thread_local! {
    /// Whether work of the scheduler is running on the thread (`_running`).
    static RUNNING: Cell<bool> = const { Cell::new(false) };
    /// The work queued on the thread while work of the scheduler runs (`_threadLocalQueue`), by due
    /// time on the clock of the thread.
    static QUEUE: RefCell<Option<PriorityQueue<Duration, Rc<ScheduledItem<Duration>>>>> = const { RefCell::new(None) };
    /// The clock of the thread (`_clock`), started the first time the scheduler needs the time.
    static CLOCK: RefCell<Option<SystemStopwatch>> = const { RefCell::new(None) };
}

fn time() -> Duration {
    CLOCK.with(|clock| clock.borrow_mut().get_or_insert_with(SystemStopwatch::start).elapsed())
}

/// Queues work on the current thread (`CurrentThreadScheduler`): the first work runs at once, and
/// work scheduled while it runs is queued and runs after it, by due time and then in the order it
/// was scheduled (the trampoline). Delayed work blocks the thread until it is due.
pub struct CurrentThreadScheduler {
    _private: (),
}

impl CurrentThreadScheduler {
    /// The scheduler (`CurrentThreadScheduler.Instance`).
    pub fn instance() -> Rc<dyn IScheduler> {
        ferroui_system::ferro_thread_local! {
            static INSTANCE: Rc<dyn IScheduler> = Rc::new(CurrentThreadScheduler { _private: () });
        }
        INSTANCE.with(Rc::clone)
    }

    /// Whether the caller has to schedule its work to keep the order of the trampoline: true when
    /// no work of the scheduler is running on the thread (`IsScheduleRequired`).
    pub fn is_schedule_required() -> bool {
        !RUNNING.with(Cell::get)
    }
}

/// Clears the state of the thread when the outermost work returns or panics.
struct EndRun;

impl Drop for EndRun {
    fn drop(&mut self) {
        QUEUE.with(|queue| queue.borrow_mut().take());
        RUNNING.with(|running| running.set(false));
    }
}

fn run_trampoline() {
    let scheduler = CurrentThreadScheduler::instance();
    loop {
        let next = QUEUE.with(|queue| queue.borrow_mut().as_mut().and_then(|q| q.pop()));
        let Some((due, item)) = next else { break };
        if item.is_canceled() {
            continue;
        }
        let wait = due.saturating_sub(time());
        sleep(wait);
        if !item.is_canceled() {
            item.invoke(&scheduler);
        }
    }
}

impl IScheduler for CurrentThreadScheduler {
    fn now(&self) -> SystemTime {
        Scheduler::now()
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_after(Duration::ZERO, action)
    }

    fn schedule_after(&self, due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        if !RUNNING.with(|running| running.replace(true)) {
            let _end = EndRun;
            sleep(due_time);
            let started = action(&CurrentThreadScheduler::instance());
            if QUEUE.with(|queue| queue.borrow().is_some()) {
                run_trampoline();
            }
            return started;
        }
        let due = time() + due_time;
        let item = ScheduledItem::new(due, action);
        QUEUE.with(|queue| queue.borrow_mut().get_or_insert_with(PriorityQueue::new).push(due, item.clone()));
        item
    }

    fn schedule_at(&self, due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule_after(Scheduler::relative_to(self, due_time), action)
    }

    fn as_stopwatch_provider(&self) -> Option<&dyn IStopwatchProvider> {
        Some(&SystemStopwatchProvider)
    }
}
