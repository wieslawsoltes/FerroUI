//! `ClickCommandBinder` (`TestModels/ClickCommandBinder.cs`).

use super::DispatchStubControl;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{CommandEventSubscription, CommandParameter, ICommand, ICreatesCommandBinding};
use ferroui_system::{IDisposable, IObservable};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// The affinity returned for a control this binder knows how to reach (`SupportedAffinity`).
const SUPPORTED_AFFINITY: i32 = 100;

/// Binds a command to [`DispatchStubControl::click`], tracking the latest parameter
/// (`ClickCommandBinder`). The crate registers no command binder of its own, so a binding has
/// nothing to subscribe its parameter stream with; registering this one lets a test drive a
/// command execution and see the parameter that reached it.
pub struct ClickCommandBinder;

impl ICreatesCommandBinding for ClickCommandBinder {
    fn get_affinity_for_object(&self, target_type: TypeId, _has_event_target: bool) -> i32 {
        if target_type == TypeId::of::<DispatchStubControl>() {
            SUPPORTED_AFFINITY
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
        let command = command?;
        let control = target?.downcast::<DispatchStubControl>().ok()?;

        let latest: Rc<RefCell<CommandParameter>> = Rc::new(RefCell::new(None));
        let sink = latest.clone();
        // The latest value of the parameter stream; its errors are ignored (`LatestParameter`).
        let parameters = command_parameter.subscribe_next_error(move |value: CommandParameter| *sink.borrow_mut() = value, |_| {});

        let on_click: Rc<dyn Fn(&())> = Rc::new(move |_: &()| {
            let parameter = latest.borrow().clone();
            if !command.can_execute(parameter.as_deref()) {
                return;
            }

            command.execute(parameter.as_deref());
        });
        let token = control.click().add(on_click);

        let unbind = Disposable::create(move || {
            control.click().remove(token);
        });
        Some(CompositeDisposable::from_disposables([unbind, parameters]) as Rc<dyn IDisposable>)
    }

    fn bind_command_to_object_event(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _event_name: &str,
    ) -> Option<Rc<dyn IDisposable>> {
        self.bind_command_to_object(command, target, command_parameter)
    }

    fn bind_command_to_object_with_handlers(
        &self,
        command: Option<Rc<dyn ICommand>>,
        target: Option<Rc<dyn Any>>,
        command_parameter: Rc<dyn IObservable<CommandParameter>>,
        _subscription: CommandEventSubscription,
    ) -> Option<Rc<dyn IDisposable>> {
        self.bind_command_to_object(command, target, command_parameter)
    }
}
