// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::threading::{ActionDisposable, ServiceDispatcher};
use ferrodock_model::services::IDockGlobalBusyService;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt, ReactiveObject};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Global busy service shared by overlays in different windows.
///
/// Like FerroUI's ReactiveUI objects, this service belongs to its creating thread.
/// A scope ends when its handle is explicitly disposed, matching the service contract.
pub struct DockGlobalBusyService {
    reactive: ReactiveObject,
    default_message: String,
    dispatcher: ServiceDispatcher,
    busy_count: Cell<i32>,
    is_busy: RefCell<bool>,
    message: RefCell<Option<String>>,
    self_weak: Weak<Self>,
}

reactive_object!(DockGlobalBusyService);

impl DockGlobalBusyService {
    /// Creates a shared service using the optional default message and notification context.
    /// An absent or whitespace-only default message becomes `"Working..."`.
    pub fn new(
        default_message: Option<String>,
        synchronization_context: Option<Arc<dyn SynchronizationContext>>,
    ) -> Rc<Self> {
        let default_message = default_message
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| "Working...".into());
        Rc::new_cyclic(|self_weak| Self {
            reactive: ReactiveObject::new(),
            default_message,
            dispatcher: ServiceDispatcher::new(synchronization_context, None),
            busy_count: Cell::new(0),
            is_busy: RefCell::new(false),
            message: RefCell::new(None),
            self_weak: self_weak.clone(),
        })
    }

    /// Returns whether at least one busy scope is active.
    pub fn is_busy(&self) -> bool {
        *self.is_busy.borrow()
    }

    /// Gets the current global busy message.
    pub fn message(&self) -> Option<String> {
        self.message.borrow().clone()
    }

    /// Begins a busy scope. The latest scope supplies the displayed message.
    /// Disposing an inner scope does not restore an earlier message.
    pub fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable> {
        self.busy_count.set(self.busy_count.get().wrapping_add(1));
        self.update_state(
            true,
            Some(message.unwrap_or_else(|| self.default_message.clone())),
        );

        let this = self
            .self_weak
            .upgrade()
            .expect("a service method keeps its instance alive");
        ActionDisposable::new(move || {
            let count = this.busy_count.get().wrapping_sub(1);
            this.busy_count.set(count);
            if count <= 0 {
                this.busy_count.set(0);
                this.update_state(false, None);
            }
        })
    }

    fn update_state(&self, is_busy: bool, message: Option<String>) {
        let this = self
            .self_weak
            .upgrade()
            .expect("a service method keeps its instance alive");
        self.dispatcher.post(move || {
            this.raise_and_set_if_changed(&this.is_busy, is_busy, "IsBusy");
            this.raise_and_set_if_changed(&this.message, message, "Message");
        });
    }
}

impl IDockGlobalBusyService for DockGlobalBusyService {
    fn is_busy(&self) -> bool {
        self.is_busy()
    }
    fn message(&self) -> Option<String> {
        self.message()
    }
    fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable> {
        self.begin(message)
    }
}

impl PartialEq for DockGlobalBusyService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_reactive::concurrency::{HistoricalScheduler, ImmediateScheduler};
    use ferroui_reactiveui::RxSchedulers;
    use ferroui_system::component_model::INotifyPropertyChanged;
    use ferroui_system::threading::{current_synchronization_context, set_synchronization_context};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Environment {
        context: Option<Arc<dyn SynchronizationContext>>,
        scheduler: Rc<dyn ferroui_reactive::concurrency::IScheduler>,
    }
    impl Environment {
        fn new() -> Self {
            let this = Self {
                context: current_synchronization_context(),
                scheduler: RxSchedulers::main_thread_scheduler(),
            };
            set_synchronization_context(None);
            RxSchedulers::set_main_thread_scheduler(ImmediateScheduler::instance());
            this
        }
    }
    impl Drop for Environment {
        fn drop(&mut self) {
            set_synchronization_context(self.context.take());
            RxSchedulers::set_main_thread_scheduler(self.scheduler.clone());
        }
    }

    #[test]
    fn nested_scopes_keep_the_latest_message_until_the_last_scope_ends() {
        let _environment = Environment::new();
        let service: Rc<dyn IDockGlobalBusyService> = DockGlobalBusyService::new(None, None);
        assert!(!service.is_busy());
        assert_eq!(service.message(), None);
        let outer = service.begin(Some("First".into()));
        let inner = service.begin(Some("Second".into()));
        assert!(service.is_busy());
        inner.dispose();
        assert!(service.is_busy());
        assert_eq!(service.message().as_deref(), Some("Second"));
        outer.dispose();
        assert!(!service.is_busy());
        assert_eq!(service.message(), None);
        inner.dispose();
        let next = service.begin(None);
        assert!(service.is_busy());
        next.dispose();
        assert!(!service.is_busy());
    }

    #[test]
    fn scopes_may_be_disposed_out_of_order() {
        let _environment = Environment::new();
        let service = DockGlobalBusyService::new(None, None);
        let first = service.begin(None);
        let second = service.begin(Some("Second".into()));
        first.dispose();
        first.dispose();
        assert!(service.is_busy());
        second.dispose();
        assert!(!service.is_busy());
    }

    #[test]
    fn constructor_defaults_and_explicit_empty_scope_messages_are_preserved() {
        let _environment = Environment::new();
        for text in [None, Some(""), Some(" \t\n\u{00a0}")] {
            let service = DockGlobalBusyService::new(text.map(String::from), None);
            let scope = service.begin(None);
            assert_eq!(service.message().as_deref(), Some("Working..."));
            scope.dispose();
        }
        let service = DockGlobalBusyService::new(Some(" Custom ".into()), None);
        let scope = service.begin(None);
        assert_eq!(service.message().as_deref(), Some(" Custom "));
        let empty = service.begin(Some("".into()));
        assert_eq!(service.message().as_deref(), Some(""));
        empty.dispose();
        scope.dispose();
    }

    #[test]
    fn notifications_use_original_property_names_and_skip_unchanged_values() {
        let _environment = Environment::new();
        let service = DockGlobalBusyService::new(None, None);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let seen = changes.clone();
        service
            .property_changed()
            .add(Rc::new(move |name| seen.borrow_mut().push(name.to_owned())));
        let first = service.begin(None);
        let second = service.begin(None);
        first.dispose();
        second.dispose();
        assert_eq!(
            *changes.borrow(),
            ["IsBusy", "Message", "IsBusy", "Message"]
        );
    }

    #[test]
    fn queued_state_updates_preserve_begin_and_dispose_order() {
        let _environment = Environment::new();
        let scheduler = HistoricalScheduler::historical();
        RxSchedulers::set_main_thread_scheduler(scheduler.clone());
        let service = DockGlobalBusyService::new(None, None);
        let changes = Rc::new(RefCell::new(Vec::new()));
        let seen = changes.clone();
        let weak = Rc::downgrade(&service);
        service.property_changed().add(Rc::new(move |name| {
            let service = weak.upgrade().unwrap();
            seen.borrow_mut()
                .push((name.to_owned(), service.is_busy(), service.message()));
        }));
        let scope = service.begin(None);
        scope.dispose();
        assert!(!service.is_busy());
        assert!(changes.borrow().is_empty());
        scheduler.start();
        assert_eq!(changes.borrow().len(), 4);
        assert_eq!(changes.borrow()[0], ("IsBusy".into(), true, None));
        assert_eq!(
            changes.borrow()[1],
            ("Message".into(), true, Some("Working...".into()))
        );
        assert_eq!(
            changes.borrow()[2],
            ("IsBusy".into(), false, Some("Working...".into()))
        );
        assert_eq!(changes.borrow()[3], ("Message".into(), false, None));
    }

    #[test]
    fn a_scope_keeps_the_service_alive_until_disposed() {
        let _environment = Environment::new();
        let service = DockGlobalBusyService::new(None, None);
        let weak = Rc::downgrade(&service);
        let scope = service.begin(None);
        drop(service);
        assert!(weak.upgrade().is_some());
        scope.dispose();
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn dropping_a_scope_does_not_end_it() {
        let _environment = Environment::new();
        let service = DockGlobalBusyService::new(None, None);
        drop(service.begin(None));
        assert!(service.is_busy());
    }

    #[test]
    fn explicit_synchronization_context_receives_the_notifications() {
        #[derive(Default)]
        struct InlineContext {
            posts: AtomicUsize,
        }
        impl SynchronizationContext for InlineContext {
            fn post(&self, work: Box<dyn FnOnce() + Send>) {
                work();
            }
            fn send(&self, work: Box<dyn FnOnce() + Send>) {
                work();
            }
            fn post_local(&self, work: Box<dyn FnOnce()>) {
                self.posts.fetch_add(1, Ordering::Relaxed);
                work();
            }
            fn send_local(&self, work: Box<dyn FnOnce()>) {
                work();
            }
        }
        let _environment = Environment::new();
        let context = Arc::new(InlineContext::default());
        let service = DockGlobalBusyService::new(None, Some(context.clone()));
        let scope = service.begin(None);
        assert!(service.is_busy());
        scope.dispose();
        assert!(!service.is_busy());
        assert_eq!(context.posts.load(Ordering::Relaxed), 2);
    }
}
