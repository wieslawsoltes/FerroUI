// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::threading::{ActionDisposable, ServiceDispatcher};
use ferrodock_model::services::IDockGlobalConfirmationService;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt, ReactiveObject};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Global confirmation service shared by overlays in different windows.
///
/// Like FerroUI's ReactiveUI objects, this service belongs to its creating thread.
/// A scope ends when its handle is explicitly disposed, matching the service contract.
pub struct DockGlobalConfirmationService {
    reactive: ReactiveObject,
    default_message: String,
    dispatcher: ServiceDispatcher,
    confirmation_count: Cell<i32>,
    is_confirmation_open: RefCell<bool>,
    message: RefCell<Option<String>>,
    self_weak: Weak<Self>,
}

reactive_object!(DockGlobalConfirmationService);

impl DockGlobalConfirmationService {
    /// Creates a shared service using the optional default message and notification context.
    /// An absent or whitespace-only default message becomes `"Confirmation open in another window."`.
    pub fn new(
        default_message: Option<String>,
        synchronization_context: Option<Arc<dyn SynchronizationContext>>,
    ) -> Rc<Self> {
        let default_message = default_message
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| "Confirmation open in another window.".into());
        Rc::new_cyclic(|self_weak| Self {
            reactive: ReactiveObject::new(),
            default_message,
            dispatcher: ServiceDispatcher::new(synchronization_context, None),
            confirmation_count: Cell::new(0),
            is_confirmation_open: RefCell::new(false),
            message: RefCell::new(None),
            self_weak: self_weak.clone(),
        })
    }

    /// Returns whether at least one confirmation scope is active.
    pub fn is_confirmation_open(&self) -> bool {
        *self.is_confirmation_open.borrow()
    }

    /// Gets the current global confirmation message.
    pub fn message(&self) -> Option<String> {
        self.message.borrow().clone()
    }

    /// Begins a confirmation scope. The latest scope supplies the displayed message.
    /// Disposing an inner scope does not restore an earlier message.
    pub fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable> {
        self.confirmation_count
            .set(self.confirmation_count.get().wrapping_add(1));
        self.update_state(
            true,
            Some(message.unwrap_or_else(|| self.default_message.clone())),
        );

        let this = self
            .self_weak
            .upgrade()
            .expect("a service method keeps its instance alive");
        ActionDisposable::new(move || {
            let count = this.confirmation_count.get().wrapping_sub(1);
            this.confirmation_count.set(count);
            if count <= 0 {
                this.confirmation_count.set(0);
                this.update_state(false, None);
            }
        })
    }

    fn update_state(&self, is_confirmation_open: bool, message: Option<String>) {
        let this = self
            .self_weak
            .upgrade()
            .expect("a service method keeps its instance alive");
        self.dispatcher.post(move || {
            this.raise_and_set_if_changed(
                &this.is_confirmation_open,
                is_confirmation_open,
                "IsConfirmationOpen",
            );
            this.raise_and_set_if_changed(&this.message, message, "Message");
        });
    }
}

impl IDockGlobalConfirmationService for DockGlobalConfirmationService {
    fn is_confirmation_open(&self) -> bool {
        self.is_confirmation_open()
    }
    fn message(&self) -> Option<String> {
        self.message()
    }
    fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable> {
        self.begin(message)
    }
}

impl PartialEq for DockGlobalConfirmationService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
