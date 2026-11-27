//! Native ribbon modal button protocol. Source: Uno ContentDialog.Process* methods
//! and ContentDialogButtonClickEventArgs/Deferral (Microsoft, MIT; retained in docs).
//! Uses FerroUI commands and ordered event snapshots; no general dialog runtime.
use super::RibbonDialogHost;
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

pub struct RibbonDialogButtonClickEventArgs {
    pub button: ContentDialogButton,
    cancel: Cell<bool>,
    deferred: Cell<bool>,
    host: WeakRef<RibbonDialogHost>,
    generation: u64,
}
impl PartialEq for RibbonDialogButtonClickEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl RibbonDialogButtonClickEventArgs {
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
    /// Explicit completion follows the source; dropping a pending handle does not
    /// accept or cancel. The handle retains the args, but never roots the host.
    pub fn get_deferral(self: &Rc<Self>) -> RibbonDialogButtonDeferral {
        self.deferred.set(true);
        RibbonDialogButtonDeferral(self.clone())
    }
    fn complete(&self) {
        if let Some(host) = self
            .host
            .upgrade()
            .filter(|host| host.dialog_generation() == self.generation)
        {
            if self.cancel.get() {
                host.cancel_closing_attempt();
            } else {
                host.complete_button(self.button);
            }
        }
    }
}
pub struct RibbonDialogButtonDeferral(Rc<RibbonDialogButtonClickEventArgs>);
impl RibbonDialogButtonDeferral {
    // The original Complete is repeatable; do not invent an idempotence guarantee.
    pub fn complete(&self) {
        self.0.complete();
    }
}
ferroui_base::ferro_markup_type!(class RibbonDialogButtonClickEventArgs {
    namespace:"FerroRibbon.Controls",this:Rc<RibbonDialogButtonClickEventArgs>,handles:[Rc<RibbonDialogButtonClickEventArgs>],
    properties:[Cancel:bool{get:|args:&Rc<RibbonDialogButtonClickEventArgs>|args.cancel(),set:|args:&Rc<RibbonDialogButtonClickEventArgs>,value|args.set_cancel(value)}]
});
type Handler = dyn Fn(&Rc<RibbonDialogButtonClickEventArgs>);
#[derive(Default)]
pub(super) struct DialogButtonEvents {
    handlers: [HandlerList<Handler>; 3],
    markup: RefCell<Vec<(usize, MarkupDelegate, u64)>>,
}
fn index(button: ContentDialogButton) -> Option<usize> {
    match button {
        ContentDialogButton::Primary => Some(0),
        ContentDialogButton::Secondary => Some(1),
        ContentDialogButton::Close => Some(2),
        _ => None,
    }
}
impl RibbonDialogHost {
    pub(crate) fn process_button(&self, button: ContentDialogButton) {
        if self.is_dialog_closing() {
            return;
        }
        let Some(index) = index(button) else {
            return;
        };
        let args = Rc::new(RibbonDialogButtonClickEventArgs {
            button,
            cancel: Cell::new(false),
            deferred: Cell::new(false),
            host: self.to_ref().downgrade(),
            generation: self.dialog_generation(),
        });
        for (_, handler) in self.button_events.handlers[index].snapshot().iter() {
            handler(&args);
        }
        if !args.deferred.get() {
            args.complete();
        }
    }
    pub fn button_click(
        &self,
        button: ContentDialogButton,
        handler: impl Fn(&Rc<RibbonDialogButtonClickEventArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let index = index(button).expect("a dialog button is required");
        let token = self.button_events.handlers[index].add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(host) = weak.upgrade() {
                host.button_events.handlers[index].remove(token);
            }
        })
    }
    fn add_button_markup(&self, index: usize, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let weak = self.to_ref().downgrade();
        let token = self.button_events.handlers[index].add(Rc::new(move |args| {
            if let Some(host) = weak.upgrade() {
                callback
                    .try_invoke(&[
                        Some(Control::boxed(host)),
                        Some(Rc::new(args.clone()) as BoxedValue),
                    ])
                    .expect("dialog button markup event");
            }
        }));
        self.button_events
            .markup
            .borrow_mut()
            .push((index, handler, token));
    }
    fn remove_button_markup(&self, index: usize, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut entries = self.button_events.markup.borrow_mut();
            entries
                .iter()
                .rposition(|(i, h, _)| *i == index && Some(h) == handler.as_ref())
                .map(|at| entries.remove(at))
        };
        if let Some((_, _, token)) = removed {
            self.button_events.handlers[index].remove(token);
        }
    }
    pub fn add_primary_button_click(&self, handler: Option<MarkupDelegate>) {
        self.add_button_markup(0, handler);
    }
    pub fn remove_primary_button_click(&self, handler: Option<MarkupDelegate>) {
        self.remove_button_markup(0, handler);
    }
    pub fn add_secondary_button_click(&self, handler: Option<MarkupDelegate>) {
        self.add_button_markup(1, handler);
    }
    pub fn remove_secondary_button_click(&self, handler: Option<MarkupDelegate>) {
        self.remove_button_markup(1, handler);
    }
    pub fn add_close_button_click(&self, handler: Option<MarkupDelegate>) {
        self.add_button_markup(2, handler);
    }
    pub fn remove_close_button_click(&self, handler: Option<MarkupDelegate>) {
        self.remove_button_markup(2, handler);
    }
}
