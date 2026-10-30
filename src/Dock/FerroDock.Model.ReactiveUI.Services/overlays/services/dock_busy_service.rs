// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::threading::task_action::{invoke_task, panic_error};
use crate::threading::{ActionDisposable, ServiceDispatcher};
use ferrodock_model::services::{BusyAction, IDockBusyService, IDockGlobalBusyService};
use ferroui_reactiveui::{
    reactive_object, ICommand, IReactiveObjectExt, ReactiveCommand, ReactiveObject,
    ReactiveUIBindingExtensions,
};
use ferroui_system::prop;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::{IDisposable, ObservableError};
use std::cell::{Cell, OnceCell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Per-host busy scopes and source ReactiveUI reload command.
/// Notifications belong to the captured synchronization context or Rx scheduler.
pub struct DockBusyService {
    reactive: ReactiveObject,
    global: Option<Rc<dyn IDockGlobalBusyService>>,
    dispatcher: ServiceDispatcher,
    reload_command: OnceCell<Rc<ReactiveCommand<(), ()>>>,
    busy_count: Cell<i32>,
    reload_handler: RefCell<Option<BusyAction>>,
    is_busy: RefCell<bool>,
    message: RefCell<Option<String>>,
    is_reload_visible: RefCell<bool>,
    can_reload: RefCell<bool>,
    runtime: Weak<Self>,
}
reactive_object!(DockBusyService);
impl DockBusyService {
    /// Creates the service, optionally linking scopes to a global busy service.
    /// Reload visibility starts true; CanReload starts false until a handler is set.
    pub fn new(
        global_busy_service: Option<Rc<dyn IDockGlobalBusyService>>,
        synchronization_context: Option<Arc<dyn SynchronizationContext>>,
    ) -> Rc<Self> {
        let service = Rc::new_cyclic(|runtime| Self {
            reactive: ReactiveObject::new(),
            global: global_busy_service,
            dispatcher: ServiceDispatcher::new(synchronization_context, None),
            reload_command: OnceCell::new(),
            busy_count: Cell::new(0),
            reload_handler: RefCell::new(None),
            is_busy: RefCell::new(false),
            message: RefCell::new(None),
            is_reload_visible: RefCell::new(true),
            can_reload: RefCell::new(false),
            runtime: runtime.clone(),
        });
        let can_reload = service.when_any_value(prop!(DockBusyService => can_reload));
        let weak = Rc::downgrade(&service);
        let command = ReactiveCommand::create_from_task_with(
            move || {
                weak.upgrade().map_or_else(
                    || Task::from_result(()),
                    |service| service.execute_reload_async(),
                )
            },
            Some(can_reload),
            None,
        );
        assert!(
            service.reload_command.set(Rc::new(command)).is_ok(),
            "reload command initialized once"
        );
        service
    }
    /// Gets whether this host has an active busy scope.
    pub fn is_busy(&self) -> bool {
        *self.is_busy.borrow()
    }
    /// Gets the latest busy message.
    pub fn message(&self) -> Option<String> {
        self.message.borrow().clone()
    }
    /// Gets reload button visibility independently of handler availability.
    pub fn is_reload_visible(&self) -> bool {
        *self.is_reload_visible.borrow()
    }
    /// Sets reload button visibility, notifying only when changed.
    pub fn set_is_reload_visible(&self, value: bool) {
        self.raise_and_set_if_changed(&self.is_reload_visible, value, "IsReloadVisible");
    }
    /// Gets whether a reload handler is installed.
    pub fn can_reload(&self) -> bool {
        *self.can_reload.borrow()
    }
    /// Gets the same source ReactiveCommand handle on every call.
    pub fn reload_command(&self) -> Rc<dyn ICommand> {
        self.reload_command
            .get()
            .expect("initialized reload command")
            .clone()
    }
    /// Begins a scope. Global Begin precedes local count/notification changes.
    /// Latest messages are retained until the final scope ends, without a message stack.
    pub fn begin(&self, message: String) -> Rc<dyn IDisposable> {
        let global = self.global.as_ref().map(|service| service.begin(None));
        self.busy_count.set(self.busy_count.get().wrapping_add(1));
        self.update_state(true, Some(message));
        let service = self.runtime.upgrade().expect("service is alive");
        ActionDisposable::new(move || {
            let count = service.busy_count.get().wrapping_sub(1);
            service.busy_count.set(count);
            if count <= 0 {
                service.busy_count.set(0);
                service.update_state(false, None);
            }
            if let Some(global) = global {
                global.dispose();
            }
        })
    }
    /// Runs work eagerly, retaining its scope until completion. Cleanup occurs
    /// before the returned task ends, and a cleanup failure supersedes a work failure.
    pub fn run_async(&self, message: String, action: BusyAction) -> Task<()> {
        let handle = match catch_unwind(AssertUnwindSafe(|| self.begin(message))) {
            Ok(handle) => handle,
            Err(payload) => return Task::from_exception(panic_error(payload)),
        };
        let completion = TaskCompletionSource::new();
        let result = completion.task();
        let work = invoke_task(|| action());
        work.continue_with(move |work| Self::finish_run(completion, handle, work.result()));
        result
    }
    fn finish_run(
        completion: TaskCompletionSource<()>,
        handle: Rc<dyn IDisposable>,
        outcome: Result<(), ObservableError>,
    ) {
        let outcome = match catch_unwind(AssertUnwindSafe(|| handle.dispose())) {
            Ok(()) => outcome,
            Err(payload) => Err(panic_error(payload)),
        };
        match outcome {
            Ok(()) => {
                completion.try_set_result(());
            }
            Err(error) => {
                completion.try_set_exception(error);
            }
        }
    }
    /// Posts a message assignment even when no scope is active.
    pub fn update_message(&self, message: Option<String>) {
        let service = self.runtime.upgrade().expect("service is alive");
        self.dispatcher.post(move || {
            service.raise_and_set_if_changed(&service.message, message, "Message");
        });
    }
    /// Replaces the reload handler on the dispatcher; handler identity is its shared Rc.
    pub fn set_reload_handler(&self, handler: Option<BusyAction>) {
        if self.dispatcher.check_access() {
            self.set_reload_handler_core(handler);
            return;
        }
        let service = self.runtime.upgrade().expect("service is alive");
        self.dispatcher
            .post(move || service.set_reload_handler_core(handler));
    }
    fn set_reload_handler_core(&self, handler: Option<BusyAction>) {
        let same = match (self.reload_handler.borrow().as_ref(), handler.as_ref()) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.reload_handler.replace(handler);
        let can_reload = self.reload_handler.borrow().is_some();
        self.raise_and_set_if_changed(&self.can_reload, can_reload, "CanReload");
    }
    fn update_state(&self, is_busy: bool, message: Option<String>) {
        let service = self.runtime.upgrade().expect("service is alive");
        self.dispatcher.post(move || {
            service.raise_and_set_if_changed(&service.is_busy, is_busy, "IsBusy");
            service.raise_and_set_if_changed(&service.message, message, "Message");
        });
    }
    fn execute_reload_async(&self) -> Task<()> {
        let handler = self.reload_handler.borrow().clone();
        handler.map_or_else(
            || Task::from_result(()),
            |handler| invoke_task(|| handler()),
        )
    }
}
impl IDockBusyService for DockBusyService {
    fn is_busy(&self) -> bool {
        self.is_busy()
    }
    fn message(&self) -> Option<String> {
        self.message()
    }
    fn is_reload_visible(&self) -> bool {
        self.is_reload_visible()
    }
    fn set_is_reload_visible(&self, value: bool) {
        self.set_is_reload_visible(value);
    }
    fn can_reload(&self) -> bool {
        self.can_reload()
    }
    fn reload_command(&self) -> Rc<dyn ICommand> {
        self.reload_command()
    }
    fn begin(&self, message: String) -> Rc<dyn IDisposable> {
        self.begin(message)
    }
    fn run_async(&self, message: String, action: BusyAction) -> Task<()> {
        self.run_async(message, action)
    }
    fn update_message(&self, message: Option<String>) {
        self.update_message(message);
    }
    fn set_reload_handler(&self, handler: Option<BusyAction>) {
        self.set_reload_handler(handler);
    }
}
impl PartialEq for DockBusyService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
#[cfg(test)]
#[path = "dock_busy_service_tests.rs"]
mod tests;
