//! `ReactiveCommandAdapter`: a command of ReactiveUI as a command of FerroUI, and the conversion of
//! the command parameters between the two.
//!
//! Not an upstream file. Upstream has one `System.Windows.Input.ICommand`, which both ReactiveUI's
//! commands and the controls' `Command` property use. The port has two: the `ICommand` of
//! `ferroui-reactiveui`, whose parameter is `Option<&dyn Any>`, and the `ICommand` of the controls
//! (`ferroui_base::input::ICommand`), whose parameter is a [`BoxedValue`] (a value that compares).
//! The adapter is the second over the first, so that a `ReactiveCommand` can be the `Command` of a
//! button.

use ferroui_base::input::ICommand;
use ferroui_base::{AnyValue, BoxedValue};
use ferroui_reactiveui::CommandParameter;
use ferroui_system::IDisposable;
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// A command of ReactiveUI as a command of FerroUI: each member forwards to the command it wraps,
/// with the parameter seen as `&dyn Any`.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream sets the command itself as the
/// `Command` of a control; the two `ICommand`s of the port are bridged by this adapter.
pub struct ReactiveCommandAdapter {
    command: Rc<dyn ferroui_reactiveui::ICommand>,
}

impl fmt::Debug for ReactiveCommandAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveCommandAdapter").field("command", &(Rc::as_ptr(&self.command) as *const ())).finish()
    }
}

impl ReactiveCommandAdapter {
    /// The command of FerroUI that runs `command`.
    pub fn new(command: Rc<dyn ferroui_reactiveui::ICommand>) -> Rc<Self> {
        Rc::new(Self { command })
    }

    /// [`new`](Self::new) as the handle the `Command` property of a control holds.
    pub fn command_of(command: Rc<dyn ferroui_reactiveui::ICommand>) -> Rc<dyn ICommand> {
        Self::new(command)
    }

    /// The command of ReactiveUI the adapter runs.
    pub fn command(&self) -> &Rc<dyn ferroui_reactiveui::ICommand> {
        &self.command
    }

    /// Whether the adapter runs `command` (the same object).
    pub fn wraps(&self, command: &Rc<dyn ferroui_reactiveui::ICommand>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.command), Rc::as_ptr(command))
    }
}

impl ICommand for ReactiveCommandAdapter {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        self.command.can_execute(parameter.map(parameter_as_any))
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        self.command.execute(parameter.map(parameter_as_any));
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.command.can_execute_changed(handler)
    }
}

/// A command parameter of ReactiveUI that is not one of the values the property system compares,
/// held by a control's `CommandParameter`: it compares by identity, as an object does upstream.
#[derive(Clone)]
pub struct ReactiveCommandParameter(pub Rc<dyn Any>);

impl PartialEq for ReactiveCommandParameter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl fmt::Debug for ReactiveCommandParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ReactiveCommandParameter").field(&(Rc::as_ptr(&self.0) as *const ())).finish()
    }
}

/// The value a command of ReactiveUI is executed with, from the value of a control's
/// `CommandParameter`: the value itself, or the object of a [`ReactiveCommandParameter`].
pub fn parameter_as_any(parameter: &BoxedValue) -> &dyn Any {
    let value: &dyn Any = (**parameter).as_any();
    match value.downcast_ref::<ReactiveCommandParameter>() {
        Some(wrapped) => &*wrapped.0,
        None => value,
    }
}

/// A command parameter of ReactiveUI as the value of a control's `CommandParameter`: a
/// [`BoxedValue`] (or a handle to an object of FerroUI) as it is; a string, a number, a character
/// or a boolean boxed as itself, so that the control's parameter compares as the value; anything
/// else in a [`ReactiveCommandParameter`].
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream's parameter is the same `object` on
/// both sides; the port's `Rc<dyn Any>` does not compare, so it is converted.
pub fn boxed_parameter(parameter: &CommandParameter) -> Option<BoxedValue> {
    let parameter = parameter.as_ref()?;
    let value: &dyn Any = &**parameter;
    if let Some(boxed) = value.downcast_ref::<BoxedValue>() {
        return Some(boxed.clone());
    }
    if let Some(object) = value.downcast_ref::<ferroui_base::Ref<ferroui_base::FerroObject>>() {
        return Some(Rc::new(object.clone()) as BoxedValue);
    }
    Some(box_primitive(value).unwrap_or_else(|| Rc::new(ReactiveCommandParameter(parameter.clone())) as BoxedValue))
}

/// A string, a number, a character or a boolean boxed as itself.
fn box_primitive(value: &dyn Any) -> Option<BoxedValue> {
    fn boxed<T: Clone + PartialEq + 'static>(value: &dyn Any) -> Option<BoxedValue> {
        value.downcast_ref::<T>().map(|value| Rc::new(value.clone()) as Rc<dyn AnyValue>)
    }
    boxed::<String>(value)
        .or_else(|| boxed::<&'static str>(value))
        .or_else(|| boxed::<bool>(value))
        .or_else(|| boxed::<char>(value))
        .or_else(|| boxed::<i8>(value))
        .or_else(|| boxed::<i16>(value))
        .or_else(|| boxed::<i32>(value))
        .or_else(|| boxed::<i64>(value))
        .or_else(|| boxed::<isize>(value))
        .or_else(|| boxed::<u8>(value))
        .or_else(|| boxed::<u16>(value))
        .or_else(|| boxed::<u32>(value))
        .or_else(|| boxed::<u64>(value))
        .or_else(|| boxed::<usize>(value))
        .or_else(|| boxed::<f32>(value))
        .or_else(|| boxed::<f64>(value))
}
