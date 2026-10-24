// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See LICENSE in the project root for license information.

use super::repeater_test_hooks::RepeaterTestHooks;
use ferroui_base::animation::IGlobalClock;
use ferroui_base::reactive::{AnonymousObserver, Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type WorkCallback = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

struct WorkInfo {
    priority: i32,
    id: u64,
    callback: WorkCallback,
}
struct SchedulerState {
    pending: Vec<WorkInfo>,
    queued: bool,
    next_id: u64,
    start_time: Option<i64>,
    budget_ms: f64,
}
ferroui_base::ferro_thread_local! {
    static STATE: RefCell<SchedulerState> = const { RefCell::new(SchedulerState {
        pending: Vec::new(), queued: false, next_id: 0, start_time: None, budget_ms: 40.0,
    }) };
}

/// Schedules phased binding work against the framework's rendering clock and
/// yields after the same 40 ms work budget as the source implementation.
pub struct BuildTreeScheduler;
impl BuildTreeScheduler {
    /// Registers work for a rendering frame. Disposing the returned handle
    /// cancels it until invocation starts, including from another callback in
    /// the same frame. Dropping the handle does not cancel the work.
    pub fn register_work(priority: i32, callback: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        assert!(
            priority >= 0,
            "Build-tree work requires a nonnegative phase priority."
        );
        // The frame takes a snapshot before invoking user code. Keep the
        // callback independently cancellable while it is in that snapshot,
        // as well as while it is in the pending queue.
        let callback: WorkCallback = Rc::new(RefCell::new(Some(Box::new(callback))));
        let cancellation = Rc::downgrade(&callback);
        let (id, queue) = STATE.with(|state| {
            let mut state = state.borrow_mut();
            let id = state.next_id;
            state.next_id = state.next_id.wrapping_add(1);
            state.pending.push(WorkInfo {
                priority,
                id,
                callback,
            });
            let queue = !state.queued;
            state.queued = true;
            (id, queue)
        });
        if queue {
            queue_render_callback(Self::on_rendering);
        }
        Disposable::create(move || {
            let callback = cancellation
                .upgrade()
                .and_then(|callback| callback.borrow_mut().take());
            let removed = STATE.with(|state| {
                let mut state = state.borrow_mut();
                state
                    .pending
                    .iter()
                    .position(|work| work.id == id)
                    .map(|index| state.pending.remove(index))
            });
            drop(removed);
            // Captured values may run user destructors. Release them after
            // leaving the scheduler state and callback borrows.
            drop(callback);
        })
    }
    pub fn should_yield() -> bool {
        let now = Dispatcher::ui_thread().now();
        STATE.with(|state| {
            let state = state.borrow();
            state
                .start_time
                .is_some_and(|start| (now - start) as f64 > state.budget_ms)
        })
    }
    pub fn pending_work_count() -> usize {
        STATE.with(|state| state.borrow().pending.len())
    }
    pub fn budget_milliseconds() -> f64 {
        STATE.with(|state| state.borrow().budget_ms)
    }
    pub fn set_budget_milliseconds(value: f64) {
        assert!(
            value.is_finite() && value >= 0.0,
            "The build-tree budget must be finite and nonnegative."
        );
        STATE.with(|state| state.borrow_mut().budget_ms = value);
    }
    fn on_rendering() {
        let now = Dispatcher::ui_thread().now();
        let mut work = STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.start_time = Some(now);
            std::mem::take(&mut state.pending)
        });
        // Pop from the end to avoid shifting the unprocessed callbacks.
        work.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| b.id.cmp(&a.id)));
        while !work.is_empty() && !Self::should_yield() {
            let item = work.pop().expect("Work is nonempty.");
            let callback = item.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        }
        // A callback can cancel a later item and exhaust the budget before we
        // reach it. Empty work must not request another rendering frame.
        work.retain(|item| item.callback.borrow().is_some());
        let queue = STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.pending.extend(work);
            state.start_time = None;
            state.queued = !state.pending.is_empty();
            state.queued
        });
        if queue {
            queue_render_callback(Self::on_rendering);
        } else {
            RepeaterTestHooks::notify_build_tree_completed();
        }
    }
}

/// Runs after the next rendering pulse. A dispatcher without a rendering clock
/// still supports offscreen layout and unit tests through its render queue.
pub(crate) fn queue_render_callback(callback: impl FnOnce() + 'static) {
    if let Some(clock) = FerroLocator::current().get_service::<dyn IGlobalClock>() {
        struct RenderCallback {
            callback: RefCell<Option<Box<dyn FnOnce()>>>,
            subscription: RefCell<Option<Rc<dyn IDisposable>>>,
            fired: Cell<bool>,
        }
        let state = Rc::new(RenderCallback {
            callback: RefCell::new(Some(Box::new(callback))),
            subscription: RefCell::new(None),
            fired: Cell::new(false),
        });
        let observed = state.clone();
        let subscription = clock.subscribe(Rc::new(AnonymousObserver::new(move |_| {
            if observed.fired.replace(true) {
                return;
            }
            let subscription = observed.subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            let callback = observed.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        })));
        if state.fired.get() {
            subscription.dispose();
        } else {
            *state.subscription.borrow_mut() = Some(subscription);
        }
    } else {
        Dispatcher::ui_thread().post_local(callback, DispatcherPriority::RENDER);
    }
}
