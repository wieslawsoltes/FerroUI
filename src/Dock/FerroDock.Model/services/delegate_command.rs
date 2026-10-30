// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_reactiveui::ICommand;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::Cell;
use std::rc::{Rc, Weak};

type Execute = Rc<dyn Fn(Option<&dyn Any>)>;
type CanExecute = Rc<dyn Fn(Option<&dyn Any>) -> bool>;

pub(crate) struct DelegateCommand {
    execute: Execute,
    can_execute: Option<CanExecute>,
    changed: Rc<Event<()>>,
}
impl DelegateCommand {
    pub(crate) fn new(execute: Execute, can_execute: Option<CanExecute>) -> Rc<Self> {
        Rc::new(Self {
            execute,
            can_execute,
            changed: Rc::new(Event::new()),
        })
    }
    // Source internal API, not called by the two immutable request commands.
    #[allow(dead_code)]
    pub(crate) fn raise_can_execute_changed(&self) {
        self.changed.raise(&());
    }
}
struct Subscription {
    event: Weak<Event<()>>,
    token: Cell<Option<u64>>,
}
impl IDisposable for Subscription {
    fn dispose(&self) {
        if let Some(token) = self.token.take() {
            if let Some(event) = self.event.upgrade() {
                event.remove(token);
            }
        }
    }
}
impl ICommand for DelegateCommand {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        self.can_execute
            .as_ref()
            .is_none_or(|can_execute| can_execute(parameter))
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        (self.execute)(parameter);
    }
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(move |_| handler()));
        Rc::new(Subscription {
            event: Rc::downgrade(&self.changed),
            token: Cell::new(Some(token)),
        })
    }
}
