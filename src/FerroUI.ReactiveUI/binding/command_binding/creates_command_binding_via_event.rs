//! `CreatesCommandBindingViaEvent`: binds a command to an event of a control.
//!
//! Not an upstream file of ReactiveUI.Binding: ReactiveUI had this binder, which found the event by
//! reflection; ReactiveUI.Binding writes the binding to an event at compile time
//! (`DefaultEventBindingPlugin`, `CommandEventBindingEmitter`). Here the events of a control type
//! are registered by name with accessors, and the binder is registered in the service locator as a
//! `dyn ICreatesCommandBinding`.

use crate::binding::bindings::BindingAffinity;
use crate::binding::interfaces::{CommandEventSubscription, CommandParameter, ICreatesCommandBinding};
use crate::reactive_command::ICommand;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::ObservableExt;
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// The names of the events a control raises when it is used, in order of preference, which a
/// command binds to when no event is named (`defaultEventsToBind`).
pub const DEFAULT_COMMAND_EVENTS: [&str; 3] = ["Click", "TouchUpInside", "MouseUp"];

/// Adds a handler to an event of a control of type `C`; disposing the result removes it.
pub type ControlEventSubscription<C> = Rc<dyn Fn(&Rc<C>, Rc<dyn Fn()>) -> Rc<dyn IDisposable>>;

/// A subscription to the event that `event` reads from a control.
pub fn control_event<C: 'static, A: ?Sized + 'static>(event: fn(&C) -> &Event<A>) -> ControlEventSubscription<C> {
    Rc::new(move |control: &Rc<C>, handler: Rc<dyn Fn()>| {
        let token = event(control).add(Rc::new(move |_: &A| handler()));
        let control = control.clone();
        Disposable::create(move || {
            event(&control).remove(token);
        })
    })
}

/// Executes `command` with the latest value of `command_parameter` whenever the event that
/// `subscribe` adds a handler to is raised, if the command can execute with it; disposing the
/// result removes the handler and ends the subscription of the parameter.
pub fn bind_command_to_event(
    command: Rc<dyn ICommand>,
    command_parameter: &Rc<dyn IObservable<CommandParameter>>,
    subscribe: impl FnOnce(Rc<dyn Fn()>) -> Rc<dyn IDisposable>,
) -> Rc<dyn IDisposable> {
    let latest: Rc<RefCell<CommandParameter>> = Rc::new(RefCell::new(None));
    let latest_for_parameter = latest.clone();
    let parameters = command_parameter.subscribe_next(move |parameter| *latest_for_parameter.borrow_mut() = parameter);
    let handler = subscribe(Rc::new(move || {
        let parameter = latest.borrow().clone();
        if !command.can_execute(parameter.as_deref()) {
            return;
        }
        command.execute(parameter.as_deref());
    }));
    CompositeDisposable::from_disposables([handler, parameters])
}

/// Binds a command to an event of a control of type `C` (`CreatesCommandBindingViaEvent`):
/// executed with the latest command parameter each time the event is raised, if the command can
/// execute with it.
///
/// The events are registered by name ([`with_event`](Self::with_event)). Without an event name,
/// the command binds to the first of [`DEFAULT_COMMAND_EVENTS`] that is registered, then to the
/// first registered event.
///
/// The affinity is [`BindingAffinity::EXPLICIT`] for a named event and
/// [`BindingAffinity::DEFAULT_EVENT`] for a default event, for a control of type `C` only.
pub struct CreatesCommandBindingViaEvent<C> {
    events: Vec<(String, ControlEventSubscription<C>)>,
}

impl<C> fmt::Debug for CreatesCommandBindingViaEvent<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreatesCommandBindingViaEvent").field("events", &self.events.iter().map(|(name, _)| name).collect::<Vec<_>>()).finish()
    }
}

impl<C: 'static> Default for CreatesCommandBindingViaEvent<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: 'static> CreatesCommandBindingViaEvent<C> {
    /// A binder with no event.
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// Registers the event named `name` that `event` reads from a control.
    pub fn with_event<A: ?Sized + 'static>(mut self, name: impl Into<String>, event: fn(&C) -> &Event<A>) -> Self {
        self.events.push((name.into(), control_event(event)));
        self
    }

    /// Registers the event named `name` that `subscription` adds a handler to.
    pub fn with_event_subscription(mut self, name: impl Into<String>, subscription: ControlEventSubscription<C>) -> Self {
        self.events.push((name.into(), subscription));
        self
    }

    /// The event named `name`.
    fn event(&self, name: &str) -> Option<&ControlEventSubscription<C>> {
        self.events.iter().find(|(event_name, _)| event_name == name).map(|(_, event)| event)
    }

    /// The event bound when none is named.
    fn default_event(&self) -> Option<&ControlEventSubscription<C>> {
        DEFAULT_COMMAND_EVENTS.iter().find_map(|name| self.event(name)).or_else(|| self.events.first().map(|(_, event)| event))
    }

    /// Binds `command` to `event` of `target`, a control of type `C`.
    fn bind(
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        event: &ControlEventSubscription<C>,
    ) -> Option<Rc<dyn IDisposable>> {
        let command = command?;
        let control = target?.downcast::<C>().ok()?;
        let event = event.clone();
        Some(bind_command_to_event(command, &command_parameter, move |handler| event(&control, handler)))
    }
}

impl<C: 'static> ICreatesCommandBinding for CreatesCommandBindingViaEvent<C> {
    fn get_affinity_for_object(&self, target_type: TypeId, has_event_target: bool) -> i32 {
        if target_type != TypeId::of::<C>() {
            return 0;
        }
        if has_event_target {
            return if self.events.is_empty() { 0 } else { BindingAffinity::EXPLICIT };
        }
        if self.default_event().is_some() {
            BindingAffinity::DEFAULT_EVENT
        } else {
            0
        }
    }

    fn bind_command_to_object(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
    ) -> Option<Rc<dyn IDisposable>> {
        let event = self.default_event()?;
        Self::bind(command, target, command_parameter, event)
    }

    fn bind_command_to_object_event(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        event_name: &str,
    ) -> Option<Rc<dyn IDisposable>> {
        let event = self.event(event_name)?;
        Self::bind(command, target, command_parameter, event)
    }

    fn bind_command_to_object_with_handlers(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        subscription: CommandEventSubscription,
    ) -> Option<Rc<dyn IDisposable>> {
        let command = command?;
        target?;
        Some(bind_command_to_event(command, &command_parameter, move |handler| subscription(handler)))
    }
}
