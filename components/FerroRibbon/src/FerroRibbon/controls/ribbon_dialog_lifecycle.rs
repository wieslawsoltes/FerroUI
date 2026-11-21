//! Concrete native ribbon modal lifecycle. ContentDialog adaptation: Microsoft MIT.
//! Counted closing deferrals: Uno repository Apache-2.0. Both original licenses
//! No generic DeferralManager, managed task, or platform facade is shipped here.
use super::{RibbonDialogHost, RibbonDialogOutcome};
use crate::compat::ContentDialogButton;
use ferroui_base::{
    BoxedValue, WeakRef,
    metadata::MarkupDelegate,
    reactive::{Disposable, IDisposable},
    utilities::HandlerList,
};
use ferroui_controls::Control;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct RibbonDialogClosingEventArgs {
    result: RibbonDialogOutcome,
    button: ContentDialogButton,
    cancel: Cell<bool>,
    outstanding: Cell<usize>,
    completed_synchronously: Cell<bool>,
    host: WeakRef<RibbonDialogHost>,
    generation: u64,
}
impl PartialEq for RibbonDialogClosingEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl RibbonDialogClosingEventArgs {
    pub fn result(&self) -> RibbonDialogOutcome {
        self.result
    }
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
    pub fn get_deferral(self: &Rc<Self>) -> RibbonDialogClosingDeferral {
        self.outstanding.set(
            self.outstanding
                .get()
                .checked_add(1)
                .expect("dialog deferral count overflow"),
        );
        RibbonDialogClosingDeferral {
            args: self.clone(),
            completed: Cell::new(false),
        }
    }
    fn release(&self, event_raise_completed: bool) {
        let count = self
            .outstanding
            .get()
            .checked_sub(1)
            .expect("dialog closing raise completes once");
        self.outstanding.set(count);
        if count == 0 {
            self.completed_synchronously.set(event_raise_completed);
            if let Some(host) = self
                .host
                .upgrade()
                .filter(|host| host.dialog_generation() == self.generation)
            {
                if !self.cancel.get() {
                    host.accept_close(self.button);
                }
                if host.dialog_generation() == self.generation {
                    host.lifecycle.hiding.set(false);
                }
            }
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonDialogDeferralError {
    AlreadyCompleted,
}
impl std::fmt::Display for RibbonDialogDeferralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Dialog closing deferral already completed.")
    }
}
impl std::error::Error for RibbonDialogDeferralError {}
/// Each explicit closing handle must complete; dropping one does not imply acceptance.
pub struct RibbonDialogClosingDeferral {
    args: Rc<RibbonDialogClosingEventArgs>,
    completed: Cell<bool>,
}
impl RibbonDialogClosingDeferral {
    pub fn complete(&self) -> Result<(), RibbonDialogDeferralError> {
        if self.completed.replace(true) {
            return Err(RibbonDialogDeferralError::AlreadyCompleted);
        }
        self.args.release(false);
        Ok(())
    }
}
pub struct RibbonDialogClosedEventArgs {
    result: RibbonDialogOutcome,
}
impl RibbonDialogClosedEventArgs {
    pub fn result(&self) -> RibbonDialogOutcome {
        self.result
    }
}
impl PartialEq for RibbonDialogClosedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
pub struct RibbonDialogOpenedEventArgs;
impl PartialEq for RibbonDialogOpenedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RibbonDialogClosingEventArgs {namespace:"FerroRibbon.Controls",this:Rc<RibbonDialogClosingEventArgs>,handles:[Rc<RibbonDialogClosingEventArgs>],properties:[Result:RibbonDialogOutcome{get:|args:&Rc<RibbonDialogClosingEventArgs>|args.result()},Cancel:bool{get:|args:&Rc<RibbonDialogClosingEventArgs>|args.cancel(),set:|args:&Rc<RibbonDialogClosingEventArgs>,value|args.set_cancel(value)}]});
ferroui_base::ferro_markup_type!(class RibbonDialogClosedEventArgs {namespace:"FerroRibbon.Controls",this:Rc<RibbonDialogClosedEventArgs>,handles:[Rc<RibbonDialogClosedEventArgs>],properties:[Result:RibbonDialogOutcome{get:|args:&Rc<RibbonDialogClosedEventArgs>|args.result()}]});
ferroui_base::ferro_markup_type!(class RibbonDialogOpenedEventArgs {namespace:"FerroRibbon.Controls",this:Rc<RibbonDialogOpenedEventArgs>,handles:[Rc<RibbonDialogOpenedEventArgs>]});
type Closing = dyn Fn(&Rc<RibbonDialogClosingEventArgs>);
type Closed = dyn Fn(&Rc<RibbonDialogClosedEventArgs>);
type Opened = dyn Fn(&Rc<RibbonDialogOpenedEventArgs>);
#[derive(Default)]
pub(super) struct DialogLifecycle {
    pub(super) hiding: Cell<bool>,
    showing: Cell<bool>,
    generation: Cell<u64>,
    closing: HandlerList<Closing>,
    closed: HandlerList<Closed>,
    opened: HandlerList<Opened>,
    markup: RefCell<Vec<(usize, MarkupDelegate, u64)>>,
}
impl RibbonDialogHost {
    pub fn is_dialog_closing(&self) -> bool {
        self.lifecycle.hiding.get()
    }
    pub fn is_dialog_showing(&self) -> bool {
        self.lifecycle.showing.get()
    }
    pub(crate) fn end_dialog(&self) {
        self.lifecycle.showing.set(false);
    }
    pub(crate) fn cancel_closing_attempt(&self) {
        self.lifecycle.hiding.set(false);
    }
    pub(crate) fn begin_dialog(&self) {
        self.lifecycle.generation.set(self.dialog_generation() + 1);
        self.lifecycle.hiding.set(false);
        self.lifecycle.showing.set(true);
    }
    pub(super) fn dialog_generation(&self) -> u64 {
        self.lifecycle.generation.get()
    }
    pub(crate) fn notify_dialog_opened(&self) {
        let args = Rc::new(RibbonDialogOpenedEventArgs);
        for (_, handler) in self.lifecycle.opened.snapshot().iter() {
            handler(&args);
        }
    }
    pub fn hide(&self) -> bool {
        if self.lifecycle.hiding.get() || !self.lifecycle.showing.get() {
            return false;
        }
        self.request_close(ContentDialogButton::None)
    }
    pub(crate) fn request_close(&self, button: ContentDialogButton) -> bool {
        self.lifecycle.hiding.set(true);
        let args = Rc::new(RibbonDialogClosingEventArgs {
            result: RibbonDialogOutcome::from_button(button),
            button,
            cancel: Cell::new(false),
            outstanding: Cell::new(1),
            completed_synchronously: Cell::new(false),
            host: self.to_ref().downgrade(),
            generation: self.dialog_generation(),
        });
        for (_, handler) in self.lifecycle.closing.snapshot().iter() {
            handler(&args);
        }
        args.release(true);
        args.completed_synchronously.get() && !args.cancel.get()
    }
    fn accept_close(&self, button: ContentDialogButton) {
        self.lifecycle.showing.set(false);
        self.invoke_close_action(button);
        let completions = self.after_closed.take();
        let args = Rc::new(RibbonDialogClosedEventArgs {
            result: RibbonDialogOutcome::from_button(button),
        });
        for (_, handler) in self.lifecycle.closed.snapshot().iter() {
            handler(&args);
        }
        // The original Hide posts completion only after every Closed subscriber
        // returned. A handler may pump the dispatcher; no result may be ready yet.
        for complete in completions {
            ferroui_base::threading::Dispatcher::ui_thread().post_local(
                complete,
                ferroui_base::threading::DispatcherPriority::NORMAL,
            );
        }
    }
    pub(crate) fn after_dialog_closed(&self, complete: impl FnOnce() + 'static) {
        self.after_closed.borrow_mut().push(Box::new(complete));
    }
    pub(super) fn force_close_dialog(&self) {
        if !self.is_dialog_showing() {
            return;
        }
        // A forced dismissal retires pending button and Closing callbacks.
        let generation = self.dialog_generation() + 1;
        self.lifecycle.generation.set(generation);
        self.lifecycle.hiding.set(true);
        self.accept_close(ContentDialogButton::None);
        if self.dialog_generation() == generation {
            self.lifecycle.hiding.set(false);
        }
    }
    pub fn dialog_closing(
        &self,
        handler: impl Fn(&Rc<RibbonDialogClosingEventArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.lifecycle.closing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(host) = weak.upgrade() {
                host.lifecycle.closing.remove(token);
            }
        })
    }
    pub fn dialog_closed(
        &self,
        handler: impl Fn(&Rc<RibbonDialogClosedEventArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.lifecycle.closed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(host) = weak.upgrade() {
                host.lifecycle.closed.remove(token);
            }
        })
    }
    pub fn dialog_opened(
        &self,
        handler: impl Fn(&Rc<RibbonDialogOpenedEventArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.lifecycle.opened.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(host) = weak.upgrade() {
                host.lifecycle.opened.remove(token);
            }
        })
    }
    fn add_lifecycle_markup(&self, index: usize, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let weak = self.to_ref().downgrade();
        let invoke = move |value: BoxedValue| {
            if let Some(host) = weak.upgrade() {
                callback
                    .try_invoke(&[Some(Control::boxed(host)), Some(value)])
                    .expect("dialog lifecycle markup handler");
            }
        };
        let token = match index {
            0 => self
                .lifecycle
                .closing
                .add(Rc::new(move |args| invoke(Rc::new(args.clone())))),
            1 => self
                .lifecycle
                .closed
                .add(Rc::new(move |args| invoke(Rc::new(args.clone())))),
            _ => self
                .lifecycle
                .opened
                .add(Rc::new(move |args| invoke(Rc::new(args.clone())))),
        };
        self.lifecycle
            .markup
            .borrow_mut()
            .push((index, handler, token));
    }
    fn remove_lifecycle_markup(&self, index: usize, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut entries = self.lifecycle.markup.borrow_mut();
            entries
                .iter()
                .rposition(|(i, h, _)| *i == index && Some(h) == handler.as_ref())
                .map(|at| entries.remove(at))
        };
        if let Some((_, _, token)) = removed {
            match index {
                0 => {
                    self.lifecycle.closing.remove(token);
                }
                1 => {
                    self.lifecycle.closed.remove(token);
                }
                _ => {
                    self.lifecycle.opened.remove(token);
                }
            }
        }
    }
    pub fn add_closing(&self, handler: Option<MarkupDelegate>) {
        self.add_lifecycle_markup(0, handler);
    }
    pub fn remove_closing(&self, handler: Option<MarkupDelegate>) {
        self.remove_lifecycle_markup(0, handler);
    }
    pub fn add_closed(&self, handler: Option<MarkupDelegate>) {
        self.add_lifecycle_markup(1, handler);
    }
    pub fn remove_closed(&self, handler: Option<MarkupDelegate>) {
        self.remove_lifecycle_markup(1, handler);
    }
    pub fn add_opened(&self, handler: Option<MarkupDelegate>) {
        self.add_lifecycle_markup(2, handler);
    }
    pub fn remove_opened(&self, handler: Option<MarkupDelegate>) {
        self.remove_lifecycle_markup(2, handler);
    }
}
