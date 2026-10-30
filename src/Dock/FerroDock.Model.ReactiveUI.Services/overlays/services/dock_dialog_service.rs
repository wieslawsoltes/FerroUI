// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::threading::task_action::spawn_catching;
use crate::threading::ServiceDispatcher;
use ferrodock_model::collections::ReadOnlyObservableCollection;
use ferrodock_model::services::{
    DialogContent, DialogRequest, IDockDialogService, IDockGlobalDialogService,
};
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{reactive_object, IReactiveObject, IReactiveObjectExt, ReactiveObject};
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::threading::{tasks::Task, SynchronizationContext};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Source per-host dialog stack; typed results use DockDialogServiceExt::show_async.
pub struct DockDialogService {
    reactive: ReactiveObject,
    global: Option<Rc<dyn IDockGlobalDialogService>>,
    dispatcher: ServiceDispatcher,
    requests: ObservableCollection<Rc<DialogRequest>>,
    read_only: ReadOnlyObservableCollection<Rc<DialogRequest>>,
    active: RefCell<Option<Rc<DialogRequest>>>,
    runtime: Weak<Self>,
}
reactive_object!(DockDialogService);
impl DockDialogService {
    /// Creates a dialog service with optional global scope and notification context.
    pub fn new(
        global: Option<Rc<dyn IDockGlobalDialogService>>,
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
    /// Gets the same live read-only collection of requests.
    pub fn dialogs(&self) -> ReadOnlyObservableCollection<Rc<DialogRequest>> {
        self.read_only.clone()
    }
    /// Gets the current active dialog.
    pub fn active_dialog(&self) -> Option<Rc<DialogRequest>> {
        self.active.borrow().clone()
    }
    /// Gets whether any dialogs are present.
    pub fn has_dialogs(&self) -> bool {
        self.requests.count() > 0
    }
    /// Performs ShowAsync<object>'s lifecycle and returns its original nullable object.
    /// The generic extension applies the source type/default cast after this task.
    pub fn show_async_boxed(
        &self,
        content: DialogContent,
        title: Option<String>,
    ) -> Task<Option<BoxedValue>> {
        let service = self.runtime.upgrade().expect("live dialog service");
        spawn_catching(async move {
            let weak = service.runtime.clone();
            let request = DialogRequest::new(
                content.value(),
                title,
                Rc::new(move |request, result| {
                    if let Some(service) = weak.upgrade() {
                        service.close(request, result);
                    }
                }),
            );
            request.set_global_handle(service.global.as_ref().map(|global| global.begin(None)));
            if let Some(content) = content.as_dialog_content() {
                let weak = service.runtime.clone();
                let item = Rc::downgrade(&request);
                content.set_close_action(Rc::new(move |result| {
                    if let (Some(service), Some(item)) = (weak.upgrade(), item.upgrade()) {
                        service.close(item, result);
                    }
                }));
            }
            let target = service.clone();
            let item = request.clone();
            service
                .dispatcher
                .invoke_async(move || {
                    if item.task().is_completed() {
                        return;
                    }
                    target.requests.add(item.clone());
                    target.raise_and_set_if_changed(&target.active, Some(item), "ActiveDialog");
                    target.raise_property_changed("HasDialogs");
                })
                .await?;
            request.task().await
        })
    }
    /// Closes the supplied request on the dispatcher, including foreign/removed requests.
    pub fn close(&self, request: Rc<DialogRequest>, result: Option<BoxedValue>) {
        if self.dispatcher.check_access() {
            self.close_on_dispatcher(request, result);
            return;
        }
        let service = self.runtime.upgrade().expect("live dialog service");
        self.dispatcher
            .post(move || service.close_on_dispatcher(request, result));
    }
    /// Cancels the current snapshot in insertion order with null results.
    pub fn cancel_all(&self) {
        if self.dispatcher.check_access() {
            self.cancel_all_on_dispatcher();
            return;
        }
        let service = self.runtime.upgrade().expect("live dialog service");
        self.dispatcher
            .post(move || service.cancel_all_on_dispatcher());
    }
    fn close_on_dispatcher(&self, request: Rc<DialogRequest>, result: Option<BoxedValue>) {
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
        self.raise_and_set_if_changed(&self.active, active, "ActiveDialog");
        self.raise_property_changed("HasDialogs");
    }
    fn cancel_all_on_dispatcher(&self) {
        if self.requests.count() == 0 {
            return;
        }
        for request in self.requests.to_vec() {
            self.close_on_dispatcher(request, None);
        }
    }
}
impl IDockDialogService for DockDialogService {
    fn dialogs(&self) -> ReadOnlyObservableCollection<Rc<DialogRequest>> {
        self.dialogs()
    }
    fn active_dialog(&self) -> Option<Rc<DialogRequest>> {
        self.active_dialog()
    }
    fn has_dialogs(&self) -> bool {
        self.has_dialogs()
    }
    fn show_async_boxed(
        &self,
        content: DialogContent,
        title: Option<String>,
    ) -> Task<Option<BoxedValue>> {
        self.show_async_boxed(content, title)
    }
    fn close(&self, request: Rc<DialogRequest>, result: Option<BoxedValue>) {
        self.close(request, result);
    }
    fn cancel_all(&self) {
        self.cancel_all();
    }
}
impl PartialEq for DockDialogService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
