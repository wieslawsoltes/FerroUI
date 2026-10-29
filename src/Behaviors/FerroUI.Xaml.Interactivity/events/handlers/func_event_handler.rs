//! Port of `Events/Handlers/FuncEventHandler.cs` (`FuncAddEventHandler<TTarget, TEventArgs>`).

use crate::events::{AddEventHandlerCallback, IAddEventHandler};
use crate::DisposableAction;
use ferroui_base::data::core::WeakValue;
use ferroui_base::interactivity::{CancelRoutedEventArgs, IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::EventArgs;
use ferroui_base::BoxedValue;
use std::cell::RefCell;
use std::rc::Rc;

/// The event args of an event as the untyped value an [`IAddEventHandler`] passes on (the args
/// are an object in the reference).
pub trait EventArgsValue {
    /// The args as an untyped value.
    fn to_event_args_value(&self) -> BoxedValue;
}

impl EventArgsValue for EventArgs {
    fn to_event_args_value(&self) -> BoxedValue {
        Rc::new(*self)
    }
}

/// Routed event args are passed as a shared handle to the live args.
impl EventArgsValue for RoutedEventArgs {
    fn to_event_args_value(&self) -> BoxedValue {
        Rc::new(self.share())
    }
}

/// Routed event args are passed as a shared handle to the live args.
impl EventArgsValue for CancelRoutedEventArgs {
    fn to_event_args_value(&self) -> BoxedValue {
        Rc::new(self.share())
    }
}

/// The handler [`FuncAddEventHandler`] gives to the subscription function: the sender and the
/// typed event args (`EventHandler<TEventArgs>`).
pub type EventHandler<TEventArgs> = Rc<dyn Fn(Option<&BoxedValue>, &TEventArgs)>;

/// An [`IAddEventHandler`] for the event `name` of sources of type `TTarget`, from a function
/// that subscribes a handler and a function that removes it.
///
/// `TTarget` is the handle of the source (`Ref<Button>`, `Rc<Model>`). Deviation (DEVIATIONS.md,
/// Behaviors): the reference removes a handler by delegate identity; a subscription of the port
/// returns a token (`TToken`: a routed event handler token, a disposable), which is what `remove`
/// takes.
pub struct FuncAddEventHandler<TTarget, TEventArgs, TToken = Rc<dyn IDisposable>> {
    name: String,
    add: Box<dyn Fn(&TTarget, EventHandler<TEventArgs>) -> TToken>,
    remove: Rc<dyn Fn(&TTarget, TToken)>,
}

impl<TTarget, TEventArgs, TToken> FuncAddEventHandler<TTarget, TEventArgs, TToken>
where
    TTarget: Clone + 'static,
    TEventArgs: EventArgsValue + 'static,
    TToken: 'static,
{
    /// Creates the handler of the event `name`: `add` subscribes a handler to a source, `remove`
    /// removes it with the token `add` returned.
    pub fn new(
        name: &str,
        add: impl Fn(&TTarget, EventHandler<TEventArgs>) -> TToken + 'static,
        remove: impl Fn(&TTarget, TToken) + 'static,
    ) -> Self {
        Self { name: name.to_string(), add: Box::new(add), remove: Rc::new(remove) }
    }

    /// The source as a `TTarget` (`source is TTarget target`).
    fn target_of(source: &BoxedValue) -> Option<TTarget> {
        from_markup_value::<TTarget>(&Some(source.clone()))
    }
}

impl<TTarget, TEventArgs, TToken> IAddEventHandler for FuncAddEventHandler<TTarget, TEventArgs, TToken>
where
    TTarget: Clone + 'static,
    TEventArgs: EventArgsValue + 'static,
    TToken: 'static,
{
    fn matches(&self, source: &BoxedValue, event_name: &str) -> bool {
        Self::target_of(source).is_some() && event_name == self.name
    }

    fn add_handler(
        &self,
        source: &BoxedValue,
        event_name: &str,
        handler: AddEventHandlerCallback,
    ) -> Option<Rc<dyn IDisposable>> {
        let target = Self::target_of(source)?;
        if event_name != self.name {
            return None;
        }

        let event_handler: EventHandler<TEventArgs> =
            Rc::new(move |sender: Option<&BoxedValue>, e: &TEventArgs| handler(sender, &e.to_event_args_value()));
        let token = (self.add)(&target, event_handler);

        // The subscription does not keep the source alive: it is the source that holds its
        // handlers.
        let source = WeakValue::new(source);
        let token = RefCell::new(Some(token));
        let remove = self.remove.clone();
        Some(DisposableAction::create(move || {
            let Some(token) = token.borrow_mut().take() else {
                return;
            };
            if let Some(target) = source.upgrade().and_then(|source| Self::target_of(&source)) {
                remove(&target, token);
            }
        }))
    }
}
