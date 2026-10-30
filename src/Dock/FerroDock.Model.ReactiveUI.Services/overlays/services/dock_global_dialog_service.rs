// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::threading::{ActionDisposable, ServiceDispatcher};
use ferrodock_model::services::IDockGlobalDialogService;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt, ReactiveObject};
use ferroui_system::threading::SynchronizationContext;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Global dialog service shared by overlays in different windows.
///
/// Like FerroUI's ReactiveUI objects, this service belongs to its creating thread.
/// A scope ends when its handle is explicitly disposed, matching the service contract.
pub struct DockGlobalDialogService {
    reactive: ReactiveObject,
    default_message: String,
    dispatcher: ServiceDispatcher,
    dialog_count: Cell<i32>,
    is_dialog_open: RefCell<bool>,
    message: RefCell<Option<String>>,
    self_weak: Weak<Self>,
}

reactive_object!(DockGlobalDialogService);

impl DockGlobalDialogService {
    /// Creates a shared service using the optional default message and notification context.
    /// An absent or whitespace-only default message becomes `"Dialog open in another window."`.
    pub fn new(
        default_message: Option<String>,
        synchronization_context: Option<Arc<dyn SynchronizationContext>>,
    ) -> Rc<Self> {
        let default_message = default_message
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| "Dialog open in another window.".into());
        Rc::new_cyclic(|self_weak| Self {
            reactive: ReactiveObject::new(),
            default_message,
            dispatcher: ServiceDispatcher::new(synchronization_context, None),
            dialog_count: Cell::new(0),
            is_dialog_open: RefCell::new(false),
            message: RefCell::new(None),
            self_weak: self_weak.clone(),
        })
    }

    /// Returns whether at least one dialog scope is active.
    pub fn is_dialog_open(&self) -> bool {
        *self.is_dialog_open.borrow()
    }

    /// Gets the current global dialog message.
    pub fn message(&self) -> Option<String> {
        self.message.borrow().clone()
    }

    /// Begins a dialog scope. The latest scope supplies the displayed message.
    /// Disposing an inner scope does not restore an earlier message.
    pub fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable> {
        self.dialog_count
            .set(self.dialog_count.get().wrapping_add(1));
        self.update_state(
            true,
            Some(message.unwrap_or_else(|| self.default_message.clone())),
        );

        let this = self
            .self_weak
            .upgrade()
            .expect("a service method keeps its instance alive");
        ActionDisposable::new(move || {
            let count = this.dialog_count.get().wrapping_sub(1);
            this.dialog_count.set(count);
            if count <= 0 {
                this.dialog_count.set(0);
                this.update_state(false, None);
            }
        })
    }

    fn update_state(&self, is_dialog_open: bool, message: Option<String>) {
        let this = self
            .self_weak
            .upgrade()
            .expect("a service method keeps its instance alive");
        self.dispatcher.post(move || {
            this.raise_and_set_if_changed(&this.is_dialog_open, is_dialog_open, "IsDialogOpen");
            this.raise_and_set_if_changed(&this.message, message, "Message");
        });
    }
}

impl IDockGlobalDialogService for DockGlobalDialogService {
    fn is_dialog_open(&self) -> bool {
        self.is_dialog_open()
    }
    fn message(&self) -> Option<String> {
        self.message()
    }
    fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable> {
        self.begin(message)
    }
}

impl PartialEq for DockGlobalDialogService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
