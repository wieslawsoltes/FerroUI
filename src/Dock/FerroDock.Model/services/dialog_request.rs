// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::delegate_command::DelegateCommand;
use crate::BoxedValue;
use ferroui_reactiveui::ICommand;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The close callback supplied to a dialog request.
pub type DialogCloseAction = Rc<dyn Fn(Rc<DialogRequest>, Option<BoxedValue>)>;

/// Dialog content, optional title, close command and one-shot result task.
pub struct DialogRequest {
    content: BoxedValue,
    title: Option<String>,
    close: Rc<dyn ICommand>,
    global_handle: RefCell<Option<Rc<dyn IDisposable>>>,
    completion: TaskCompletionSource<Option<BoxedValue>>,
}
impl DialogRequest {
    /// Creates a request retaining the content object. The native ICommand parameter
    /// is a borrowed canonical BoxedValue handle so reference identity is preserved.
    pub fn new(content: BoxedValue, title: Option<String>, close: DialogCloseAction) -> Rc<Self> {
        Rc::new_cyclic(|runtime: &Weak<Self>| {
            let weak = runtime.clone();
            let command = DelegateCommand::new(
                Rc::new(move |parameter| {
                    if let Some(request) = weak.upgrade() {
                        let result = parameter.map(|value| value.downcast_ref::<BoxedValue>()
                        .expect("DialogRequest.Close requires a canonical BoxedValue command parameter").clone());
                        close(request, result);
                    }
                }),
                None,
            );
            Self {
                content,
                title,
                close: command,
                global_handle: RefCell::new(None),
                completion: TaskCompletionSource::new(),
            }
        })
    }
    /// Gets the original shared content object.
    pub fn content(&self) -> BoxedValue {
        self.content.clone()
    }
    /// Gets the optional title, retaining null versus empty.
    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }
    /// Gets the close command. Pass Some(&boxed_value) or None to its Execute.
    pub fn close(&self) -> Rc<dyn ICommand> {
        self.close.clone()
    }
    /// Gets the optional global scope handle.
    pub fn global_handle(&self) -> Option<Rc<dyn IDisposable>> {
        self.global_handle.borrow().clone()
    }
    /// Sets the optional global handle without implicit disposal.
    pub fn set_global_handle(&self, value: Option<Rc<dyn IDisposable>>) {
        self.global_handle.replace(value);
    }
    /// Gets the shared result task.
    pub fn task(&self) -> Task<Option<BoxedValue>> {
        self.completion.task()
    }
    /// Completes once with a nullable result, retaining the supplied object identity.
    pub fn complete(&self, result: Option<BoxedValue>) {
        self.completion.try_set_result(result);
    }
}
impl PartialEq for DialogRequest {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
