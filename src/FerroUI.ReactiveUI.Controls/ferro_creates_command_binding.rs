//! `FerroCreatesCommandBinding` (ReactiveUI.Binding's integration, `...CreatesCommandBinding.cs`).

use crate::reactive_command_adapter::{boxed_parameter, ReactiveCommandAdapter};
use crate::FerroObjectHandles;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{ICommand as FerroCommand, InputElement};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent, RoutedEventHandlerToken, RoutedEventRegistry, RoutingStrategies};
use ferroui_base::{BoxedValue, FerroPropertyRegistry, ObjectType, Ref, TypeInfo};
use ferroui_controls::{as_command_source, Button};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{CommandEventSubscription, CommandParameter, ICommand, ICreatesCommandBinding};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Binds commands to the input elements of FerroUI, through a command source's `Command` property
/// or through a routed event (`FerroCreatesCommandBinding`).
///
/// A control that is a command source, such as a button or a menu item, is bound through its
/// `Command` and `CommandParameter` properties (the ones `Button` registers and every button-like
/// control and menu item shares). Any other input element is bound through a routed event that
/// executes the command and keeps `IsEnabled` in step with `can_execute`.
///
/// The control is given as an `Rc<dyn Any>` holding its handle (`Ref<T>`), and its type as the
/// `TypeId` of that handle; [`FerroObjectHandles::object_of`] lists the other forms.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): the command of ReactiveUI is set on the control
/// through a [`ReactiveCommandAdapter`], and the parameters through
/// [`boxed_parameter`](crate::boxed_parameter). A type is a command source when its class
/// registers the `Command` property of buttons (an instance: when it is an `ICommandSource` with
/// that property), as `is ICommandSource` cannot be asked of a type. The event binding has no type
/// of event arguments to match (the trait of the port has no `TEventArgs`), so the event is found
/// by its name alone.
#[derive(Debug, Default, Clone, Copy)]
pub struct FerroCreatesCommandBinding;

impl FerroCreatesCommandBinding {
    /// The affinity of a binding through a named routed event (`EventTargetAffinity`).
    pub const EVENT_TARGET_AFFINITY: i32 = 6;

    /// The affinity of a binding through a command source's `Command` property
    /// (`CommandSourceAffinity`).
    pub const COMMAND_SOURCE_AFFINITY: i32 = 10;

    /// The binder (`new FerroCreatesCommandBinding()`).
    pub fn new() -> Self {
        Self
    }

    /// Whether objects of `class` are command sources: the class registers the `Command` property
    /// of buttons.
    fn is_command_source_class(class: &'static TypeInfo) -> bool {
        FerroPropertyRegistry::instance().is_registered(class, Button::command_property())
    }

    /// The routed event named `event_name` that `class` or one of its base classes registers; the
    /// comparison is case-sensitive (`FindRoutedEvent`). Only an input element registers events
    /// that can be bound.
    fn find_routed_event(class: &'static TypeInfo, event_name: &str) -> Option<RoutedEvent> {
        if !InputElement::TYPE.is_assignable_from(class) {
            return None;
        }
        RoutedEventRegistry::instance().find_by_name(class, event_name)
    }
}

impl ICreatesCommandBinding for FerroCreatesCommandBinding {
    /// Zero for a type that is not the handle of an input element; 6 for an input element bound
    /// through an event; 10 for an input element that is a command source, bound without an event;
    /// otherwise zero (`GetAffinityForObject<T>`).
    fn get_affinity_for_object(&self, target_type: TypeId, has_event_target: bool) -> i32 {
        let Some((class, _)) = TypeInfo::find_by_handle(target_type) else {
            return 0;
        };
        if !InputElement::TYPE.is_assignable_from(class) {
            return 0;
        }
        if has_event_target {
            return Self::EVENT_TARGET_AFFINITY;
        }
        if Self::is_command_source_class(class) {
            Self::COMMAND_SOURCE_AFFINITY
        } else {
            0
        }
    }

    /// Sets `command` as the control's `Command` and binds its `CommandParameter` to
    /// `command_parameter` (`BindCommandToObject<T>`).
    ///
    /// Returns a binding that clears the command, if it is still the one set, and ends the binding
    /// of the parameter when disposed; an empty disposable after clearing the command for a null
    /// command; `None` when there is no target, or when the command is null and the target is not a
    /// command source.
    ///
    /// # Panics
    /// Panics when a command is bound to a target that is not an input element that is a command
    /// source (`InvalidOperationException`).
    fn bind_command_to_object(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
    ) -> Option<Rc<dyn IDisposable>> {
        let target = target?;
        let element = FerroObjectHandles::object_of(&*target)
            .filter(|object| object.is::<InputElement>() && as_command_source(object).is_some() && Self::is_command_source_class(object.get_type()));
        let Some(element) = element else {
            command.as_ref()?;
            panic!("InvalidOperationException: Target must be an InputElement and implement ICommandSource.");
        };

        let Some(command) = command else {
            element.set_current_value(Button::command_property(), None);
            return Some(Disposable::empty());
        };

        // Button's Command property is the property every button-like control and menu item shares.
        let adapter: Rc<dyn FerroCommand> = ReactiveCommandAdapter::command_of(command);
        element.set_current_value(Button::command_property(), Some(adapter.clone()));
        let parameters: Rc<dyn IObservable<Option<BoxedValue>>> = command_parameter.select(|parameter: CommandParameter| boxed_parameter(&parameter));
        let parameter_binding = element.bind(Button::command_parameter_property(), parameters, BindingPriority::LocalValue);
        Some(Disposable::create(move || {
            parameter_binding.dispose();
            let current = element.get_value(Button::command_property());
            let is_ours = current.as_ref().is_some_and(|current| std::ptr::addr_eq(Rc::as_ptr(current), Rc::as_ptr(&adapter)));
            if !is_ours {
                return;
            }
            element.clear_value(Button::command_property());
        }))
    }

    /// Executes `command` whenever the routed event named `event_name` is raised on the control
    /// (`BindCommandToObject<T, TEventArgs>(..., string eventName)`).
    ///
    /// Returns a binding that removes the handler and clears `IsEnabled` when disposed, or `None`
    /// when the command or the target is missing.
    ///
    /// # Panics
    /// Panics when the target is not an input element, or when its class registers no routed event
    /// named `event_name` (`InvalidOperationException`).
    fn bind_command_to_object_event(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        event_name: &str,
    ) -> Option<Rc<dyn IDisposable>> {
        let (command, target) = (command?, target?);
        let Some(element) = FerroObjectHandles::object_of(&*target).and_then(|object| object.cast::<InputElement>()) else {
            panic!("InvalidOperationException: Target must be an InputElement.");
        };
        let class = element.get_type();
        let Some(routed_event) = Self::find_routed_event(class, event_name) else {
            panic!("InvalidOperationException: Routed Event {event_name} not found on {} element.", class.name());
        };
        Some(RoutedEventCommandBinding::attach(element, routed_event, command, command_parameter))
    }

    /// Executes `command` whenever the event that `subscription` adds a handler to is raised
    /// (`BindCommandToObject<T, TEventArgs>(..., addHandler, removeHandler)`); `None` when the
    /// command or the target is missing.
    fn bind_command_to_object_with_handlers(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        subscription: CommandEventSubscription,
    ) -> Option<Rc<dyn IDisposable>> {
        let (command, _target) = (command?, target?);
        Some(HandlerCommandBinding::attach(command, command_parameter, subscription))
    }
}

/// Raises an error of the parameters of a binding again, to the code that pushed it
/// (`ExceptionDispatchInfo.Capture(error).Throw()`).
fn rethrow(error: ObservableError) -> ! {
    panic!("{error}");
}

/// Executes a command when a routed event is raised, and keeps the element's `IsEnabled` in step
/// with `can_execute` (`RoutedEventCommandBinding`).
struct RoutedEventCommandBinding {
    element: Ref<InputElement>,
    routed_event: RoutedEvent,
    command: Rc<dyn ICommand>,
    handler: Cell<Option<RoutedEventHandlerToken>>,
    parameter_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    parameter: RefCell<CommandParameter>,
}

impl RoutedEventCommandBinding {
    /// The binding, with its handler attached to the routed event of `element` (bubbling) and the
    /// parameter observed.
    fn attach(
        element: Ref<InputElement>,
        routed_event: RoutedEvent,
        command: Rc<dyn ICommand>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
    ) -> Rc<dyn IDisposable> {
        let binding = Rc::new(Self {
            element,
            routed_event,
            command,
            handler: Cell::new(None),
            parameter_subscription: RefCell::new(None),
            parameter: RefCell::new(None),
        });
        // The parameters and the handler hold the binding, as upstream's delegates do; disposing
        // the binding releases both.
        let subscription = command_parameter.subscribe(Rc::new(RoutedParameterObserver { binding: binding.clone() }));
        *binding.parameter_subscription.borrow_mut() = Some(subscription);
        let handled = binding.clone();
        let interactive: &Interactive = &binding.element;
        let token = interactive.add_route_handler(
            &binding.routed_event,
            Rc::new(move |_: &Interactive, _: &dyn IRoutedEventArgs| handled.on_event()),
            RoutingStrategies::BUBBLE,
            false,
        );
        binding.handler.set(Some(token));
        binding
    }

    /// Executes the command with the latest parameter when it can execute (`OnEvent`).
    fn on_event(&self) {
        let parameter = self.parameter.borrow().clone();
        if !self.command.can_execute(parameter.as_deref()) {
            return;
        }
        self.command.execute(parameter.as_deref());
    }

    /// Keeps the latest parameter and sets `IsEnabled` to whether the command can execute with it
    /// (`OnNext`).
    fn on_parameter(&self, value: CommandParameter) {
        let can_execute = self.command.can_execute(value.as_deref());
        *self.parameter.borrow_mut() = value;
        self.element.set_current_value(InputElement::is_enabled_property(), can_execute);
    }
}

impl IDisposable for RoutedEventCommandBinding {
    fn dispose(&self) {
        let subscription = self.parameter_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        if let Some(token) = self.handler.take() {
            let interactive: &Interactive = &self.element;
            interactive.remove_handler(&self.routed_event, token);
        }
        self.element.clear_value(InputElement::is_enabled_property());
    }
}

/// The observer of the parameters of a [`RoutedEventCommandBinding`].
struct RoutedParameterObserver {
    binding: Rc<RoutedEventCommandBinding>,
}

impl IObserver<CommandParameter> for RoutedParameterObserver {
    fn on_next(&self, value: CommandParameter) {
        self.binding.on_parameter(value);
    }

    fn on_error(&self, error: ObservableError) {
        rethrow(error);
    }

    fn on_completed(&self) {}
}

/// Executes a command when an event is raised (`HandlerCommandBinding<TEventArgs>`).
struct HandlerCommandBinding {
    command: Rc<dyn ICommand>,
    handler: RefCell<Option<Rc<dyn IDisposable>>>,
    parameter_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    parameter: RefCell<CommandParameter>,
}

impl HandlerCommandBinding {
    /// The binding, with the parameter observed and its handler added through `subscription`.
    fn attach(command: Rc<dyn ICommand>, command_parameter: Rc<dyn IObservable<CommandParameter>>, subscription: CommandEventSubscription) -> Rc<dyn IDisposable> {
        let binding = Rc::new(Self {
            command,
            handler: RefCell::new(None),
            parameter_subscription: RefCell::new(None),
            parameter: RefCell::new(None),
        });
        // The parameters and the handler hold the binding, as upstream's delegates do; disposing
        // the binding releases both.
        let parameters = command_parameter.subscribe(Rc::new(HandlerParameterObserver { binding: binding.clone() }));
        *binding.parameter_subscription.borrow_mut() = Some(parameters);
        let handled = binding.clone();
        let handler = subscription(Rc::new(move || handled.on_event()));
        *binding.handler.borrow_mut() = Some(handler);
        binding
    }

    /// Executes the command with the latest parameter when it can execute (`OnEvent`).
    fn on_event(&self) {
        let parameter = self.parameter.borrow().clone();
        if !self.command.can_execute(parameter.as_deref()) {
            return;
        }
        self.command.execute(parameter.as_deref());
    }
}

impl IDisposable for HandlerCommandBinding {
    fn dispose(&self) {
        let subscription = self.parameter_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let handler = self.handler.borrow_mut().take();
        if let Some(handler) = handler {
            handler.dispose();
        }
    }
}

/// The observer of the parameters of a [`HandlerCommandBinding`].
struct HandlerParameterObserver {
    binding: Rc<HandlerCommandBinding>,
}

impl IObserver<CommandParameter> for HandlerParameterObserver {
    fn on_next(&self, value: CommandParameter) {
        *self.binding.parameter.borrow_mut() = value;
    }

    fn on_error(&self, error: ObservableError) {
        rethrow(error);
    }

    fn on_completed(&self) {}
}

/// The handle type of class `T` as the binder sees a control: what `TypeId` the command binding of
/// a control of class `T` passes ([`ICreatesCommandBinding::get_affinity_for_object`]).
pub fn control_type<T: ObjectType>() -> TypeId {
    T::TYPE.ensure_class_init();
    TypeId::of::<Ref<T>>()
}

/// The handle of `control` as the binder takes a control.
pub fn control_target<T: ObjectType>(control: &Ref<T>) -> Rc<dyn Any> {
    Rc::new(control.clone())
}
