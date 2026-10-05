use super::{ActivatedEventArgs, ActivationKind, IActivatableLifetime};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use std::rc::{Rc, Weak};

/// The base of the activatable lifetimes of the platforms: it owns the
/// activated and deactivated events and raises them on the UI thread.
///
/// A platform lifetime holds one of these, forwards its
/// [`IActivatableLifetime`] events to it and provides its own
/// `try_leave_background` / `try_enter_background`; used on its own, both
/// requests are refused.
pub struct ActivatableLifetimeBase {
    this: Weak<ActivatableLifetimeBase>,
    activated: HandlerList<dyn Fn(&ActivatedEventArgs)>,
    deactivated: HandlerList<dyn Fn(&ActivatedEventArgs)>,
}

impl ActivatableLifetimeBase {
    /// Creates a lifetime base without handlers.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            activated: HandlerList::new(),
            deactivated: HandlerList::new(),
        })
    }

    /// Raises the activated event with arguments of the given kind.
    pub fn on_activated_kind(&self, kind: ActivationKind) {
        self.on_activated(ActivatedEventArgs::new(kind))
    }

    /// Raises the activated event.
    pub fn on_activated(&self, event_args: ActivatedEventArgs) {
        Self::raise(&self.activated, event_args)
    }

    /// Raises the deactivated event with arguments of the given kind.
    pub fn on_deactivated_kind(&self, kind: ActivationKind) {
        self.on_deactivated(ActivatedEventArgs::new(kind))
    }

    /// Raises the deactivated event.
    pub fn on_deactivated(&self, event_args: ActivatedEventArgs) {
        Self::raise(&self.deactivated, event_args)
    }

    fn raise(handlers: &HandlerList<dyn Fn(&ActivatedEventArgs)>, event_args: ActivatedEventArgs) {
        let handlers = handlers.snapshot();
        // A send on the UI thread runs the callback before returning.
        let _ = Dispatcher::ui_thread().invoke_local(move || {
            for (_, handler) in handlers.iter() {
                handler(&event_args);
            }
        });
    }

    fn subscribe(
        &self,
        select: fn(&ActivatableLifetimeBase) -> &HandlerList<dyn Fn(&ActivatedEventArgs)>,
        handler: Rc<dyn Fn(&ActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        let token = select(self).add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                select(&this).remove(token);
            }
        })
    }
}

impl IActivatableLifetime for ActivatableLifetimeBase {
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.activated, handler)
    }

    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe(|this| &this.deactivated, handler)
    }

    fn try_leave_background(&self) -> bool {
        false
    }

    fn try_enter_background(&self) -> bool {
        false
    }
}
