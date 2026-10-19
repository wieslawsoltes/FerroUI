//! `CreatesCommandBindingViaCommandParameter`: binds a command to the `Command` and
//! `CommandParameter` properties of a control.
//!
//! Not an upstream file of ReactiveUI.Binding: ReactiveUI had this binder, which found the
//! properties by reflection; ReactiveUI.Binding writes the binding at compile time
//! (`CommandPropertyBindingPlugin`). Here the properties of a control type are given as paths
//! (to read them) and setters, and the binder is registered in the service locator as a
//! `dyn ICreatesCommandBinding`.

use crate::binding::bindings::BindingAffinity;
use crate::binding::interfaces::{CommandEventSubscription, CommandParameter, ICreatesCommandBinding};
use crate::reactive_command::ICommand;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::ObservableExt;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

/// Binds a command to a control of type `C` that has a `Command` and a `CommandParameter`
/// property (`CreatesCommandBindingViaCommandParameter`): the binding assigns the parameter, then
/// the command, and disposing it restores the values the control had.
///
/// The affinity is [`BindingAffinity::EXPLICIT`] for a control of type `C` when no event is named,
/// and zero otherwise: this binder binds no event.
pub struct CreatesCommandBindingViaCommandParameter<C> {
    command: PropertyPath<Rc<C>, Option<Rc<dyn ICommand>>>,
    set_command: Rc<dyn Fn(&C, Option<Rc<dyn ICommand>>)>,
    command_parameter: PropertyPath<Rc<C>, CommandParameter>,
    set_command_parameter: Rc<dyn Fn(&C, CommandParameter)>,
}

impl<C> fmt::Debug for CreatesCommandBindingViaCommandParameter<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreatesCommandBindingViaCommandParameter").finish_non_exhaustive()
    }
}

impl<C: 'static> CreatesCommandBindingViaCommandParameter<C> {
    /// A binder over the `Command` property (`command` reads it, `set_command` assigns it) and the
    /// `CommandParameter` property of the controls of type `C`.
    pub fn new(
        command: PropertyPath<Rc<C>, Option<Rc<dyn ICommand>>>,
        set_command: impl Fn(&C, Option<Rc<dyn ICommand>>) + 'static,
        command_parameter: PropertyPath<Rc<C>, CommandParameter>,
        set_command_parameter: impl Fn(&C, CommandParameter) + 'static,
    ) -> Self {
        Self { command, set_command: Rc::new(set_command), command_parameter, set_command_parameter: Rc::new(set_command_parameter) }
    }
}

impl<C: 'static> ICreatesCommandBinding for CreatesCommandBindingViaCommandParameter<C> {
    fn get_affinity_for_object(&self, target_type: TypeId, has_event_target: bool) -> i32 {
        if has_event_target || target_type != TypeId::of::<C>() {
            return 0;
        }
        BindingAffinity::EXPLICIT
    }

    fn bind_command_to_object(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
    ) -> Option<Rc<dyn IDisposable>> {
        let command = command?;
        let control = target?.downcast::<C>().ok()?;

        // The values the control had, restored when the binding ends.
        let original_command = self.command.get(&control);
        let original_parameter = self.command_parameter.get(&control);

        // The parameter is assigned first, so that the control never has the command without it.
        let set_parameter = self.set_command_parameter.clone();
        let parameter_control = control.clone();
        let parameters = command_parameter.subscribe_next(move |parameter| set_parameter(&parameter_control, parameter));
        (self.set_command)(&control, Some(command));

        let set_command = self.set_command.clone();
        let restore_parameter = self.set_command_parameter.clone();
        let restore = Disposable::create(move || {
            set_command(&control, original_command);
            restore_parameter(&control, original_parameter);
        });
        Some(CompositeDisposable::from_disposables([parameters, restore]))
    }

    /// `None`: this binder binds no event.
    fn bind_command_to_object_event(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _event_name: &str,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }

    /// `None`: this binder binds no event.
    fn bind_command_to_object_with_handlers(
        &self,
        _command: Option<Rc<dyn ICommand>>,
        _target: Option<Rc<dyn Any>>,
        _command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _subscription: CommandEventSubscription,
    ) -> Option<Rc<dyn IDisposable>> {
        None
    }
}
