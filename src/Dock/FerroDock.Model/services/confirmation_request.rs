// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::delegate_command::DelegateCommand;
use ferroui_reactiveui::ICommand;
use ferroui_system::task::{Shared, SharedSender};
use ferroui_system::{IDisposable, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The callback supplied to a source confirmation request.
pub type ConfirmationCloseAction = Rc<dyn Fn(Rc<ConfirmationRequest>, bool)>;

/// An immutable confirmation description, commands, and one-shot completion task.
pub struct ConfirmationRequest {
    title: String,
    message: String,
    confirm_text: String,
    cancel_text: String,
    confirm: Rc<dyn ICommand>,
    cancel: Rc<dyn ICommand>,
    global_handle: RefCell<Option<Rc<dyn IDisposable>>>,
    completion: RefCell<Option<SharedSender<Result<bool, ObservableError>>>>,
    task: Shared<Result<bool, ObservableError>>,
}
impl ConfirmationRequest {
    /// Creates a request. Commands call the supplied close action with this request
    /// and true/false; they do not complete its task themselves.
    pub fn new(
        title: String,
        message: String,
        confirm_text: String,
        cancel_text: String,
        close: ConfirmationCloseAction,
    ) -> Rc<Self> {
        let (completion, task) = Shared::channel();
        Rc::new_cyclic(|runtime: &Weak<Self>| {
            let weak = runtime.clone();
            let confirm_close = close.clone();
            let confirm = DelegateCommand::new(
                Rc::new(move |_| {
                    if let Some(request) = weak.upgrade() {
                        confirm_close(request, true);
                    }
                }),
                None,
            );
            let weak = runtime.clone();
            let cancel = DelegateCommand::new(
                Rc::new(move |_| {
                    if let Some(request) = weak.upgrade() {
                        close(request, false);
                    }
                }),
                None,
            );
            Self {
                title,
                message,
                confirm_text,
                cancel_text,
                confirm,
                cancel,
                global_handle: RefCell::new(None),
                completion: RefCell::new(Some(completion)),
                task,
            }
        })
    }
    /// Gets the immutable title.
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Gets the immutable message.
    pub fn message(&self) -> &str {
        &self.message
    }
    /// Gets the confirm button label.
    pub fn confirm_text(&self) -> &str {
        &self.confirm_text
    }
    /// Gets the cancel button label.
    pub fn cancel_text(&self) -> &str {
        &self.cancel_text
    }
    /// Gets the confirm command.
    pub fn confirm(&self) -> Rc<dyn ICommand> {
        self.confirm.clone()
    }
    /// Gets the cancel command.
    pub fn cancel(&self) -> Rc<dyn ICommand> {
        self.cancel.clone()
    }
    /// Gets the optional global scope handle; requests do not dispose it themselves.
    pub fn global_handle(&self) -> Option<Rc<dyn IDisposable>> {
        self.global_handle.borrow().clone()
    }
    /// Sets the optional global scope handle without disposing the previous handle.
    pub fn set_global_handle(&self, value: Option<Rc<dyn IDisposable>>) {
        self.global_handle.replace(value);
    }
    /// Gets the shared task completed by Complete: the `Shared` of the result, whose `flatten()`
    /// is a future of it.
    pub fn task(&self) -> Shared<Result<bool, ObservableError>> {
        self.task.clone()
    }
    /// Completes once. Subsequent attempts preserve the original result.
    pub fn complete(&self, result: bool) {
        // Taken out of the cell before it sends: the continuations may reach this request.
        let completion = self.completion.borrow_mut().take();
        if let Some(completion) = completion {
            completion.send(Ok(result));
        }
    }
}
impl PartialEq for ConfirmationRequest {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
