//! An event binding of `{x:Bind}` (step X4 of docs/porting/x-bind.md): `Click="{x:Bind OnClick}"`,
//! `Click="{x:Bind Model.OnClick}"`, `Click="{x:Bind local:Handlers.OnClick}"`, after the
//! generated handlers of WinUI's compiler (NOTICE.md of the crate: `BoundEventAssignment.cs`,
//! the generated code of `EventTests.xaml`).

use super::{XBindDiagnostic, XBindings};
use ferroui_base::data::{CompiledBindingPath, PathEventHandler};
use ferroui_base::metadata::{MarkupDelegate, MarkupValue};
use ferroui_base::reactive::Subscription;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Subscribes the handler of an event binding to its event: the subscription, released when it
/// is dropped (`None` for an event that offers no way to remove a handler), or why the handler
/// could not be subscribed.
pub(super) type SubscribeEvent = Box<dyn Fn(MarkupDelegate) -> Result<Option<Subscription>, String>>;

/// The event binding of one target: the method the path ends with, called on the value of the
/// elements before it, which is read from the root source when the event is raised (WinUI's
/// generated handler walks the path then, and calls nothing when a step is null).
///
/// The handler is subscribed when the bindings of the root are initialized (or updated) and
/// released when they stop tracking; [`XBindings`] holds the binding, the binding holds the
/// subscription guard, and the handler on the target holds the binding weakly.
pub(super) struct XBindEvent {
    this: Weak<XBindEvent>,
    bindings: Weak<XBindings>,
    /// The name of the event, for diagnostics.
    event: String,
    path: CompiledBindingPath,
    handler: Rc<PathEventHandler>,
    subscribe: SubscribeEvent,
    subscription: RefCell<Option<Subscription>>,
    /// Whether the handler is subscribed: an event that cannot remove its handler keeps it, and
    /// the handler does nothing while this is off.
    subscribed: Cell<bool>,
    error: RefCell<Option<String>>,
}

impl XBindEvent {
    pub fn new(
        bindings: &Rc<XBindings>,
        event: String,
        path: CompiledBindingPath,
        handler: Rc<PathEventHandler>,
        subscribe: SubscribeEvent,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            bindings: Rc::downgrade(bindings),
            event,
            path,
            handler,
            subscribe,
            subscription: RefCell::new(None),
            subscribed: Cell::new(false),
            error: RefCell::new(None),
        })
    }

    /// Subscribes the handler (`Initialize`, `Update`); a subscribed handler stays as it is.
    pub fn start(&self) {
        if self.subscribed.get() {
            return;
        }
        let weak = self.this.clone();
        let handler = MarkupDelegate::new(move |arguments| {
            if let Some(this) = weak.upgrade() {
                this.invoke(arguments);
            }
            None
        });
        match (self.subscribe)(handler) {
            Ok(subscription) => {
                self.subscribed.set(true);
                self.error.borrow_mut().take();
                let previous = self.subscription.replace(subscription);
                drop(previous);
            }
            Err(message) => {
                *self.error.borrow_mut() = Some(message);
            }
        }
    }

    /// Releases the handler (`StopTracking`).
    pub fn stop(&self) {
        self.subscribed.set(false);
        // The guard is dropped after the borrow is released: removing a handler can run code.
        let subscription = self.subscription.borrow_mut().take();
        drop(subscription);
    }

    /// Whether the handler is subscribed to the event.
    pub fn is_subscribed(&self) -> bool {
        self.subscribed.get()
    }

    /// Calls the method with the arguments of the event: on the value of the path before it,
    /// read from the root source now. A root source without a value (a template instance
    /// without an item) or a null on the path calls nothing; a failure of the method is the
    /// diagnostic of the binding.
    fn invoke(&self, arguments: &[MarkupValue]) {
        if !self.subscribed.get() {
            return;
        }
        let Some(bindings) = self.bindings.upgrade().filter(|bindings| bindings.is_initialized()) else {
            return;
        };
        let root = match bindings.data_root() {
            Ok(Some(root)) => root,
            Ok(None) => return,
            Err(message) => {
                *self.error.borrow_mut() = Some(message);
                return;
            }
        };
        let owner = match self.path.read_once(&root) {
            Ok(Some(owner)) => owner,
            // WinUI's generated handler checks every step of the path for null.
            Ok(None) | Err(_) => return,
        };
        let result = (self.handler.invoke)(&owner, arguments);
        *self.error.borrow_mut() = result.err().map(|error| error.to_string());
    }

    /// The failure of the binding: a handler that could not be subscribed, or the last call of
    /// the method that failed.
    pub fn diagnostic(&self) -> Option<XBindDiagnostic> {
        let message = self.error.borrow().clone()?;
        Some(XBindDiagnostic { path: format!("{} = {}", self.event, self.path), message })
    }
}
