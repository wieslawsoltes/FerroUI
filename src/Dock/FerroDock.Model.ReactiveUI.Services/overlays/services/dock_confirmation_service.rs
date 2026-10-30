// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::threading::task_action::spawn_catching;
use crate::threading::ServiceDispatcher;
use ferrodock_model::collections::ReadOnlyObservableCollection;
use ferrodock_model::services::{
    ConfirmationRequest, IDockConfirmationService, IDockGlobalConfirmationService,
};
use ferroui_reactiveui::{reactive_object, IReactiveObject, IReactiveObjectExt, ReactiveObject};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::threading::{tasks::Task, SynchronizationContext};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Source per-host confirmation stack, dispatching mutation to its captured context.
pub struct DockConfirmationService {
    reactive: ReactiveObject,
    global: Option<Rc<dyn IDockGlobalConfirmationService>>,
    dispatcher: ServiceDispatcher,
    requests: ObservableCollection<Rc<ConfirmationRequest>>,
    read_only: ReadOnlyObservableCollection<Rc<ConfirmationRequest>>,
    active: RefCell<Option<Rc<ConfirmationRequest>>>,
    runtime: Weak<Self>,
}
reactive_object!(DockConfirmationService);
impl DockConfirmationService {
    /// Creates a service with an optional global scope service and notification context.
    pub fn new(
        global: Option<Rc<dyn IDockGlobalConfirmationService>>,
        context: Option<Arc<dyn SynchronizationContext>>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|runtime| {
            let requests = ObservableCollection::new();
            Self {
                reactive: ReactiveObject::new(),
                global,
                dispatcher: ServiceDispatcher::new(context, None),
                read_only: ReadOnlyObservableCollection::new(requests.clone()),
                requests,
                active: RefCell::new(None),
                runtime: runtime.clone(),
            }
        })
    }
    /// Gets the same live read-only collection.
    pub fn confirmations(&self) -> ReadOnlyObservableCollection<Rc<ConfirmationRequest>> {
        self.read_only.clone()
    }
    /// Gets the currently active request.
    pub fn active_confirmation(&self) -> Option<Rc<ConfirmationRequest>> {
        self.active.borrow().clone()
    }
    /// Gets whether any request is present.
    pub fn has_confirmations(&self) -> bool {
        self.requests.count() > 0
    }
    /// Creates a request/global scope eagerly, adds it through InvokeAsync, then
    /// awaits its own completion. Confirmation insertion has no completed-task guard.
    pub fn confirm_async(
        &self,
        title: String,
        message: String,
        confirm_text: String,
        cancel_text: String,
    ) -> Task<bool> {
        let service = self.runtime.upgrade().expect("live confirmation service");
        spawn_catching(async move {
            let weak = service.runtime.clone();
            let request = ConfirmationRequest::new(
                title,
                message,
                confirm_text,
                cancel_text,
                Rc::new(move |request, result| {
                    if let Some(service) = weak.upgrade() {
                        service.close(request, result);
                    }
                }),
            );
            request.set_global_handle(service.global.as_ref().map(|global| global.begin(None)));
            let target = service.clone();
            let item = request.clone();
            service
                .dispatcher
                .invoke_async(move || {
                    target.requests.add(item.clone());
                    target.raise_and_set_if_changed(
                        &target.active,
                        Some(item),
                        "ActiveConfirmation",
                    );
                    target.raise_property_changed("HasConfirmations");
                })
                .await?;
            request.task().await
        })
    }
    /// Uses the source Confirm/Cancel default labels.
    pub fn confirm_async_default(&self, title: String, message: String) -> Task<bool> {
        self.confirm_async(title, message, "Confirm".into(), "Cancel".into())
    }
    /// Closes the supplied request on the dispatcher, even when it is not in this collection.
    pub fn close(&self, request: Rc<ConfirmationRequest>, result: bool) {
        if self.dispatcher.check_access() {
            self.close_on_dispatcher(request, result);
            return;
        }
        let service = self.runtime.upgrade().expect("live confirmation service");
        self.dispatcher
            .post(move || service.close_on_dispatcher(request, result));
    }
    /// Cancels a snapshot in insertion order; requests opened reentrantly remain open.
    pub fn cancel_all(&self) {
        if self.dispatcher.check_access() {
            self.cancel_all_on_dispatcher();
            return;
        }
        let service = self.runtime.upgrade().expect("live confirmation service");
        self.dispatcher
            .post(move || service.cancel_all_on_dispatcher());
    }
    fn close_on_dispatcher(&self, request: Rc<ConfirmationRequest>, result: bool) {
        if !self.requests.contains(&request) {
            if let Some(handle) = request.global_handle() {
                handle.dispose();
            }
            request.complete(result);
            return;
        }
        self.requests.remove(&request);
        if let Some(handle) = request.global_handle() {
            handle.dispose();
        }
        request.complete(result);
        let count = self.requests.count();
        let active = if count == 0 {
            None
        } else {
            Some(self.requests.get(count - 1))
        };
        self.raise_and_set_if_changed(&self.active, active, "ActiveConfirmation");
        self.raise_property_changed("HasConfirmations");
    }
    fn cancel_all_on_dispatcher(&self) {
        if self.requests.count() == 0 {
            return;
        }
        for request in self.requests.to_vec() {
            self.close_on_dispatcher(request, false);
        }
    }
}
impl IDockConfirmationService for DockConfirmationService {
    fn confirmations(&self) -> ReadOnlyObservableCollection<Rc<ConfirmationRequest>> {
        self.confirmations()
    }
    fn active_confirmation(&self) -> Option<Rc<ConfirmationRequest>> {
        self.active_confirmation()
    }
    fn has_confirmations(&self) -> bool {
        self.has_confirmations()
    }
    fn confirm_async(
        &self,
        title: String,
        message: String,
        confirm_text: String,
        cancel_text: String,
    ) -> Task<bool> {
        self.confirm_async(title, message, confirm_text, cancel_text)
    }
    fn close(&self, request: Rc<ConfirmationRequest>, result: bool) {
        self.close(request, result);
    }
    fn cancel_all(&self) {
        self.cancel_all();
    }
}
impl PartialEq for DockConfirmationService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
