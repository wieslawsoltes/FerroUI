//! `ReactiveUIBindingExtensions.BindCommand.cs`: `BindCommand`, with the runtime of
//! `RuntimeCommandBindingFallback` (`Fallback/RuntimeCommandBindingFallback.cs`) and of the code the
//! source generator writes for it.

use super::reactive_ui_binding_extensions_bind_interaction::{follow_view_models, values_on, view_models_of};
use super::ReactiveUIBindingExtensions;
use crate::{BindingAffinity, BindingErrors, BindingSchedulers};
use crate::binding::command_binding::{bind_command_to_event, control_event, CommandBinderService, ControlEventSubscription};
use crate::binding::interfaces::{CommandParameter, IViewForOf};
use crate::binding::observables::CombineLatestObservable;
use crate::reactive_command::{CombinedReactiveCommand, ICommand, ReactiveCommand};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable, SerialDisposable};
use ferroui_reactive::linq::Observable;
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

/// The value of a property that holds a command: a [`ReactiveCommand`], a
/// [`CombinedReactiveCommand`], an `Rc<dyn ICommand>`, or an `Option` of one (a property not
/// assigned yet).
pub trait CommandValue: Clone + PartialEq + 'static {
    /// The command the property holds, if any.
    fn command(&self) -> Option<Rc<dyn ICommand>>;
}

impl<P: Clone + Default + 'static, R: Clone + 'static> CommandValue for ReactiveCommand<P, R> {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        Some(Rc::new(self.clone()))
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> CommandValue for Option<ReactiveCommand<P, R>> {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.as_ref().and_then(CommandValue::command)
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> CommandValue for CombinedReactiveCommand<P, R> {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        Some(Rc::new(self.clone()))
    }
}

impl<P: Clone + Default + 'static, R: Clone + 'static> CommandValue for Option<CombinedReactiveCommand<P, R>> {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.as_ref().and_then(CommandValue::command)
    }
}

impl CommandValue for Rc<dyn ICommand> {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        Some(self.clone())
    }
}

impl CommandValue for Option<Rc<dyn ICommand>> {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.clone()
    }
}

/// The value of a property of a view that holds a control: an `Rc` of the control, or an `Option`
/// of one. The control compares as its type does; a control that compares by identity reports
/// each new control.
pub trait ControlValue: Clone + PartialEq + 'static {
    /// The type of the control (`TControl`).
    type Control: 'static;

    /// The control the property holds, if any.
    fn control(&self) -> Option<Rc<Self::Control>>;
}

impl<C: PartialEq + 'static> ControlValue for Rc<C> {
    type Control = C;

    fn control(&self) -> Option<Rc<C>> {
        Some(self.clone())
    }
}

impl<C: PartialEq + 'static> ControlValue for Option<Rc<C>> {
    type Control = C;

    fn control(&self) -> Option<Rc<C>> {
        self.clone()
    }
}

/// The event of a control that executes a bound command (`string? toEvent`).
///
/// [`ToEvent::event`] names the event and reads it from the control, as upstream's source
/// generator resolves a literal event name at compile time: the command binds to it directly,
/// unless a registered binder has a higher affinity for the control than the binding to an event
/// ([`BindingAffinity::DEFAULT_EVENT`]). [`ToEvent::named`] only names it, and the registered
/// binder with the highest affinity for the control binds it
/// ([`ICreatesCommandBinding::bind_command_to_object_event`](crate::ICreatesCommandBinding::bind_command_to_object_event)).
pub struct ToEvent<C> {
    name: String,
    subscription: Option<ControlEventSubscription<C>>,
}

impl<C> Clone for ToEvent<C> {
    fn clone(&self) -> Self {
        Self { name: self.name.clone(), subscription: self.subscription.clone() }
    }
}

impl<C> fmt::Debug for ToEvent<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToEvent").field("name", &self.name).field("resolved", &self.subscription.is_some()).finish()
    }
}

impl<C: 'static> ToEvent<C> {
    /// The event named `name`, bound by a registered binder.
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), subscription: None }
    }

    /// The event named `name` that `event` reads from the control.
    pub fn event<A: ?Sized + 'static>(name: impl Into<String>, event: fn(&C) -> &Event<A>) -> Self {
        Self { name: name.into(), subscription: Some(control_event(event)) }
    }

    /// The event named `name` that `subscription` adds a handler to.
    pub fn subscription(name: impl Into<String>, subscription: ControlEventSubscription<C>) -> Self {
        Self { name: name.into(), subscription: Some(subscription) }
    }

    /// The name of the event.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Binds one command to one control (`RuntimeCommandBindingFallback.Bind`, and the event binding of
/// the generated code); nothing is bound while either is missing, or when no binder supports the
/// control.
fn bind_one<C: 'static>(
    command: Option<Rc<dyn ICommand>>,
    control: Option<Rc<C>>,
    command_parameter: &Rc<dyn IObservable<CommandParameter>>,
    to_event: Option<&ToEvent<C>>,
) -> Rc<dyn IDisposable> {
    let (Some(command), Some(control)) = (command, control) else {
        return Disposable::empty();
    };
    let target: Rc<dyn Any> = control.clone();
    let control_type = TypeId::of::<C>();
    let bound = match to_event {
        Some(ToEvent { name, subscription: Some(subscription) }) => {
            match CommandBinderService::get_binder_above(control_type, BindingAffinity::DEFAULT_EVENT, true) {
                Some(binder) => binder.bind_command_to_object_event(Some(command), Some(target), command_parameter.clone(), name),
                None => {
                    let subscription = subscription.clone();
                    Some(bind_command_to_event(command, command_parameter, move |handler| subscription(&control, handler)))
                }
            }
        }
        Some(ToEvent { name, subscription: None }) => CommandBinderService::get_binder_for(control_type, true)
            .and_then(|binder| binder.bind_command_to_object_event(Some(command), Some(target), command_parameter.clone(), name)),
        None => CommandBinderService::get_binder_for(control_type, false)
            .and_then(|binder| binder.bind_command_to_object(Some(command), Some(target), command_parameter.clone())),
    };
    bound.unwrap_or_else(Disposable::empty)
}

/// Keeps the latest command bound to the latest control, rebinding when either changes, on the
/// thread of the view (`RuntimeCommandBindingFallback.BindCommand`). Disposing the result unbinds
/// the command and ends the observation.
fn bind_commands_to_controls<C: 'static>(
    view: Rc<dyn Any>,
    commands: Rc<dyn IObservable<Option<Rc<dyn ICommand>>>>,
    controls: Rc<dyn IObservable<Option<Rc<C>>>>,
    command_parameter: Rc<dyn IObservable<CommandParameter>>,
    to_event: Option<ToEvent<C>>,
    binding_expression: String,
) -> Rc<dyn IDisposable> {
    let binding = SerialDisposable::new();
    let current = binding.clone();

    // Each rebind touches the control, so it is delivered on the view's thread.
    let pairs = BindingSchedulers::observe_on_view_thread(CombineLatestObservable::create_tuple((commands, controls)), Some(view));
    let observation = BindingErrors::subscribe(
        pairs,
        move |(command, control): (Option<Rc<dyn ICommand>>, Option<Rc<C>>)| {
            // The previous binding ends first, so that a binder restoring the state of a control
            // restores it before the next one reads it.
            current.set_disposable(None);
            current.set_disposable(Some(bind_one(command, control, &command_parameter, to_event.as_ref())));
        },
        binding_expression,
    );
    CompositeDisposable::from_disposables([observation, binding as Rc<dyn IDisposable>])
}

/// The commands a sequence of command values holds.
fn commands_of<X: CommandValue>(values: Rc<dyn IObservable<Option<X>>>) -> Rc<dyn IObservable<Option<Rc<dyn ICommand>>>> {
    values.select(|value: Option<X>| value.and_then(|value| value.command()))
}

/// The controls `path` reads from `view`.
fn controls_of<V: 'static, CV: ControlValue>(view: &Rc<V>, path: PropertyPath<Rc<V>, CV>) -> Rc<dyn IObservable<Option<Rc<CV::Control>>>> {
    view.when_any_value(path).select(|value: CV| value.control())
}

/// The values of an observable as command parameters.
fn parameters_of<P: 'static>(parameter: Rc<dyn IObservable<P>>) -> Rc<dyn IObservable<CommandParameter>> {
    parameter.select(|value: P| Some(Rc::new(value) as Rc<dyn Any>))
}

/// The values of a property as command parameters, `None` while there is none.
fn optional_parameters_of<P: 'static>(parameter: Rc<dyn IObservable<Option<P>>>) -> Rc<dyn IObservable<CommandParameter>> {
    parameter.select(|value: Option<P>| value.map(|value| Rc::new(value) as Rc<dyn Any>))
}

/// The text of a binding, for its errors.
fn expression_of<A, B, C, D>(command: &PropertyPath<A, B>, control: &PropertyPath<C, D>) -> String
where
    A: 'static,
    B: 'static,
    C: 'static,
    D: 'static,
{
    format!("{} -> {}", command.path(), control.path())
}

/// `BindCommand`: binds the command a property of a view model holds to the control a property of
/// a view holds, rebinding when either changes. Implemented for every `Rc` of a view.
///
/// Upstream's source generator roots the command at the view's `ViewModel` when the view declares
/// one: the binding follows the view model the view displays, and binds nothing while there is
/// none. [`bind_command`](Self::bind_command) and its `_with_` forms do that for a view that
/// declares its view model (`IViewForOf<VM>`), observed as the property `ViewModel` of the view; the `_on`
/// forms root the command at a given view model, as for a view that does not declare one, and bind
/// nothing without a view model.
///
/// The command binds to `to_event` ([`ToEvent`]); without one, to the default event of the
/// control, through the registered binder with the highest affinity for the type of the control
/// ([`CommandBinderService`]). The command executes with the latest value of the command
/// parameter, if it can execute with it.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream overloads `BindCommand` by the source of the
/// parameter (none, an observable, an expression) and resolves the root and the event at compile
/// time; here each is a method of its own, the event is a [`ToEvent`], and the view-rooted forms
/// leave out upstream's `viewModel` argument, which the generated code does not read either. The
/// expressions are [`PropertyPath`]s. A control no binder supports binds nothing, as upstream's
/// run-time path does (the generated code throws when it finds no event at compile time).
pub trait BindCommandMixins<V: 'static> {
    /// Binds the command `command` reads from the view's view model to the control `control` reads
    /// from the view (`BindCommand(viewModel, propertyName, controlName, toEvent)`).
    fn bind_command<VM, X, CV>(&self, command: PropertyPath<Rc<VM>, X>, control: PropertyPath<Rc<V>, CV>, to_event: Option<ToEvent<CV::Control>>) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: CommandValue,
        CV: ControlValue;

    /// [`bind_command`](Self::bind_command) with the values of `parameter` as the command
    /// parameter (`BindCommand(..., IObservable<TParam?> withParameter, toEvent)`).
    fn bind_command_with_parameter<VM, X, CV, P>(
        &self,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: Rc<dyn IObservable<P>>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: 'static;

    /// [`bind_command`](Self::bind_command) with the property `parameter` of the view's view model
    /// as the command parameter, observed so that the control sees each of its values
    /// (`BindCommand(..., Expression<Func<TViewModel, TParam?>> withParameter, toEvent)`).
    fn bind_command_with_parameter_path<VM, X, CV, P>(
        &self,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: PropertyPath<Rc<VM>, P>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: Clone + PartialEq + 'static;

    /// [`bind_command`](Self::bind_command) rooted at `view_model`.
    fn bind_command_on<VM, X, CV>(
        &self,
        view_model: Option<Rc<VM>>,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: CommandValue,
        CV: ControlValue;

    /// [`bind_command_with_parameter`](Self::bind_command_with_parameter) rooted at `view_model`.
    fn bind_command_with_parameter_on<VM, X, CV, P>(
        &self,
        view_model: Option<Rc<VM>>,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: Rc<dyn IObservable<P>>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: 'static;

    /// [`bind_command_with_parameter_path`](Self::bind_command_with_parameter_path) rooted at
    /// `view_model`.
    fn bind_command_with_parameter_path_on<VM, X, CV, P>(
        &self,
        view_model: Option<Rc<VM>>,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: PropertyPath<Rc<VM>, P>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: Clone + PartialEq + 'static;
}

impl<V: 'static> BindCommandMixins<V> for Rc<V> {
    fn bind_command<VM, X, CV>(&self, command: PropertyPath<Rc<VM>, X>, control: PropertyPath<Rc<V>, CV>, to_event: Option<ToEvent<CV::Control>>) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
    {
        self.bind_command_with_parameter(command, control, Observable::empty::<()>(), to_event)
    }

    fn bind_command_with_parameter<VM, X, CV, P>(
        &self,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: Rc<dyn IObservable<P>>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: 'static,
    {
        let expression = expression_of(&command, &control);
        let commands = commands_of(follow_view_models(view_models_of::<V, VM>(self), command));
        let controls = controls_of(self, control);
        bind_commands_to_controls(self.clone(), commands, controls, parameters_of(parameter), to_event, expression)
    }

    fn bind_command_with_parameter_path<VM, X, CV, P>(
        &self,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: PropertyPath<Rc<VM>, P>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        V: IViewForOf<VM>,
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: Clone + PartialEq + 'static,
    {
        let expression = expression_of(&command, &control);
        let view_models = view_models_of::<V, VM>(self);
        let commands = commands_of(follow_view_models(view_models.clone(), command));
        let parameters = optional_parameters_of(follow_view_models(view_models, parameter));
        let controls = controls_of(self, control);
        bind_commands_to_controls(self.clone(), commands, controls, parameters, to_event, expression)
    }

    fn bind_command_on<VM, X, CV>(
        &self,
        view_model: Option<Rc<VM>>,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
    {
        self.bind_command_with_parameter_on(view_model, command, control, Observable::empty::<()>(), to_event)
    }

    fn bind_command_with_parameter_on<VM, X, CV, P>(
        &self,
        view_model: Option<Rc<VM>>,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: Rc<dyn IObservable<P>>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: 'static,
    {
        let expression = expression_of(&command, &control);
        let Some(values) = values_on(view_model, command) else {
            return Disposable::empty();
        };
        let controls = controls_of(self, control);
        bind_commands_to_controls(self.clone(), commands_of(values), controls, parameters_of(parameter), to_event, expression)
    }

    fn bind_command_with_parameter_path_on<VM, X, CV, P>(
        &self,
        view_model: Option<Rc<VM>>,
        command: PropertyPath<Rc<VM>, X>,
        control: PropertyPath<Rc<V>, CV>,
        parameter: PropertyPath<Rc<VM>, P>,
        to_event: Option<ToEvent<CV::Control>>,
    ) -> Rc<dyn IDisposable>
    where
        VM: 'static,
        X: CommandValue,
        CV: ControlValue,
        P: Clone + PartialEq + 'static,
    {
        let expression = expression_of(&command, &control);
        // The parameter lives on the same view model, so it is only observable once there is one.
        let Some(values) = values_on(view_model.clone(), command) else {
            return Disposable::empty();
        };
        let parameters = match values_on(view_model, parameter) {
            Some(parameters) => optional_parameters_of(parameters),
            None => Observable::never::<CommandParameter>(),
        };
        let controls = controls_of(self, control);
        bind_commands_to_controls(self.clone(), commands_of(values), controls, parameters, to_event, expression)
    }
}
