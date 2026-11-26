//! WinUI's `XamlUICommand`, `ExecuteRequestedEventArgs` and `CanExecuteRequestedEventArgs`
//! (`dxaml/xcp/dxaml/lib/XamlUICommand_Partial.cpp` and `CommandingHelpers.cpp` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::commanding_helpers;
use super::KeyboardAccelerator;
use crate::IconSource;
use ferroui_base::collections::FerroList;
use ferroui_base::input::{ICommand, KeyGesture};
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The arguments of [`XamlUICommand::execute_requested`] (WinUI's `ExecuteRequestedEventArgs`).
#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteRequestedEventArgs {
    parameter: Option<BoxedValue>,
}

impl ExecuteRequestedEventArgs {
    /// The parameter the command runs with: the `CommandParameter` of its source.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
}

ferro_markup_type!(class ExecuteRequestedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ExecuteRequestedEventArgs],
    properties: [
        Parameter: Option<BoxedValue> { get: |args: &ExecuteRequestedEventArgs| args.parameter() },
    ],
});

/// The arguments of [`XamlUICommand::can_execute_requested`] (WinUI's
/// `CanExecuteRequestedEventArgs`): a handler sets [`set_can_execute`](Self::set_can_execute)
/// to `false` when the command cannot run with the parameter.
#[derive(Debug)]
pub struct CanExecuteRequestedEventArgs {
    parameter: Option<BoxedValue>,
    can_execute: Cell<bool>,
}

impl CanExecuteRequestedEventArgs {
    /// The parameter the command would run with.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }

    /// Whether the command can run; `true` until a handler sets it.
    pub fn can_execute(&self) -> bool {
        self.can_execute.get()
    }

    pub fn set_can_execute(&self, value: bool) {
        self.can_execute.set(value);
    }
}

impl Clone for CanExecuteRequestedEventArgs {
    fn clone(&self) -> Self {
        Self { parameter: self.parameter.clone(), can_execute: Cell::new(self.can_execute.get()) }
    }
}

impl PartialEq for CanExecuteRequestedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.parameter == other.parameter && self.can_execute.get() == other.can_execute.get()
    }
}

ferro_markup_type!(class CanExecuteRequestedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [CanExecuteRequestedEventArgs],
    properties: [
        Parameter: Option<BoxedValue> { get: |args: &CanExecuteRequestedEventArgs| args.parameter() },
        CanExecute: bool { get: |args: &CanExecuteRequestedEventArgs| args.can_execute() },
    ],
});

type ExecuteRequestedHandler = dyn Fn(&Ref<XamlUICommand>, &ExecuteRequestedEventArgs);
type CanExecuteRequestedHandler = dyn Fn(&Ref<XamlUICommand>, &CanExecuteRequestedEventArgs);

/// A command that carries the user experience of the command with it (WinUI's
/// `XamlUICommand`): a [`label`](Self::label), an [`icon_source`](Self::icon_source), the
/// [`keyboard_accelerators`](Self::keyboard_accelerators), an [`access_key`](Self::access_key)
/// and a [`description`](Self::description), which each command source it is set on takes for
/// the properties it has not set itself.
///
/// Running the command raises [`execute_requested`](Self::execute_requested) and then runs the
/// wrapped [`command`](Self::command); it can run when no handler of
/// [`can_execute_requested`](Self::can_execute_requested) says no and the wrapped command can.
/// [`notify_can_execute_changed`](Self::notify_can_execute_changed) tells the sources to ask
/// again.
///
/// A command property holds the command as an `Rc<dyn ICommand>` ([`as_command`](Self::as_command));
/// markup converts the command object itself (`Command="{StaticResource SaveCommand}"`).
///
/// The sources that take its properties are `Button` (the label as its content), the command
/// bar buttons (`CommandBarButton` and `CommandBarToggleButton`, WinUI's `AppBarButton` and
/// `AppBarToggleButton`: the label and the icon), `MenuItem` (WinUI's `MenuFlyoutItem`: the
/// header and the icon) and `SwipeItem` (the text and the icon source); the controls also take
/// the first enabled keyboard accelerator as their `HotKey` (and a menu item as its
/// `InputGesture`), the access key as their `AutomationProperties.AccessKey` and the description
/// as their `AutomationProperties.HelpText` and tool tip. None of this code runs in an
/// application that creates no `XamlUICommand`.
#[repr(C)]
pub struct XamlUICommand {
    base: FerroObject,
    keyboard_accelerators: FerroList<Ref<KeyboardAccelerator>>,
    execute_requested: HandlerList<ExecuteRequestedHandler>,
    can_execute_requested: HandlerList<CanExecuteRequestedHandler>,
    can_execute_changed: HandlerList<dyn Fn()>,
    /// The `ICommand` of the command while a property holds it: one value, so that command
    /// properties compare equal for the same command.
    handle: RefCell<Weak<UICommandHandle>>,
}

ferro_class!(XamlUICommand: FerroObject);
ferro_class_info!(XamlUICommand {
    new: XamlUICommand::new,
    interfaces: [Rc<dyn ICommand> => |command: Ref<XamlUICommand>| command.as_command()],
    markup: {
        namespace: "FerroUI.Controls",
        properties: [
            KeyboardAccelerators: FerroList<Ref<KeyboardAccelerator>> { get: XamlUICommand::keyboard_accelerators },
        ],
    },
});
ferro_impl_classes!(XamlUICommand: FerroObjectImpl);

ferro_properties! { impl XamlUICommand {
    /// Defines the `Label` property: the label of the command's sources. Default empty.
    pub fn label_property() -> StyledProperty<String> {
        FerroProperty::register::<XamlUICommand, _>("Label", String::new())
    }

    /// Defines the `IconSource` property: the icon of the command's sources. Default `None`.
    pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
        FerroProperty::register::<XamlUICommand, _>("IconSource", None)
    }

    /// Defines the `AccessKey` property: the access key of the command's sources. Default empty.
    pub fn access_key_property() -> StyledProperty<String> {
        FerroProperty::register::<XamlUICommand, _>("AccessKey", String::new())
    }

    /// Defines the `Description` property: the help text and tool tip of the command's sources.
    /// Default empty.
    pub fn description_property() -> StyledProperty<String> {
        FerroProperty::register::<XamlUICommand, _>("Description", String::new())
    }

    /// Defines the `Command` property: the command run after `ExecuteRequested`. Default `None`.
    pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
        FerroProperty::register::<XamlUICommand, _>("Command", None)
    }
} }

/// The `ICommand` of an [`XamlUICommand`]: it keeps the command alive while a command property
/// holds it.
struct UICommandHandle(Ref<XamlUICommand>);

impl ICommand for UICommandHandle {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        self.0.can_execute(parameter)
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        self.0.execute(parameter);
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.add_can_execute_changed(handler)
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

impl XamlUICommand {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        commanding_helpers::install();
        Self {
            base: FerroObject::construct(),
            keyboard_accelerators: FerroList::new(),
            execute_requested: HandlerList::new(),
            can_execute_requested: HandlerList::new(),
            can_execute_changed: HandlerList::new(),
            handle: RefCell::new(Weak::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a command with a label.
    pub fn with_label(label: impl Into<String>) -> Ref<Self> {
        let command = Self::new();
        command.set_label(label);
        command
    }

    /// The command as the `ICommand` of a command property. While a property holds it, every
    /// call returns the same value.
    pub fn as_command(&self) -> Rc<dyn ICommand> {
        if let Some(handle) = self.handle.borrow().upgrade() {
            return handle;
        }
        let handle = Rc::new(UICommandHandle(self.to_ref()));
        *self.handle.borrow_mut() = Rc::downgrade(&handle);
        handle
    }

    /// The `XamlUICommand` of a command property's value, if it is one.
    pub fn from_command(command: &Rc<dyn ICommand>) -> Option<Ref<XamlUICommand>> {
        command.as_any()?.downcast_ref::<UICommandHandle>().map(|handle| handle.0.clone())
    }

    pub fn label(&self) -> String {
        self.get_value(Self::label_property())
    }

    pub fn set_label(&self, value: impl Into<String>) {
        self.set_value(Self::label_property(), value.into());
    }

    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }

    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        self.set_value(Self::icon_source_property(), value);
    }

    /// The keyboard accelerators of the command (WinUI's `KeyboardAccelerators`).
    pub fn keyboard_accelerators(&self) -> FerroList<Ref<KeyboardAccelerator>> {
        self.keyboard_accelerators.clone()
    }

    /// The key gesture of the first enabled keyboard accelerator: the `HotKey` its sources take.
    pub fn first_key_gesture(&self) -> Option<KeyGesture> {
        self.keyboard_accelerators.snapshot().iter().find(|accelerator| accelerator.is_enabled())?.key_gesture()
    }

    pub fn access_key(&self) -> String {
        self.get_value(Self::access_key_property())
    }

    pub fn set_access_key(&self, value: impl Into<String>) {
        self.set_value(Self::access_key_property(), value.into());
    }

    pub fn description(&self) -> String {
        self.get_value(Self::description_property())
    }

    pub fn set_description(&self, value: impl Into<String>) {
        self.set_value(Self::description_property(), value.into());
    }

    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value);
    }

    /// Adds a handler of `ExecuteRequested`, raised when the command runs. Dropping the returned
    /// guard removes it; [`Subscription::detach`] keeps it for the life of the command.
    pub fn execute_requested(
        &self,
        handler: impl Fn(&Ref<XamlUICommand>, &ExecuteRequestedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.execute_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.execute_requested.remove(token);
            }
        }))
    }

    /// Adds a handler of `CanExecuteRequested`, raised when a source asks whether the command can
    /// run. Dropping the returned guard removes it.
    pub fn can_execute_requested(
        &self,
        handler: impl Fn(&Ref<XamlUICommand>, &CanExecuteRequestedEventArgs) + 'static,
    ) -> Subscription {
        let token = self.can_execute_requested.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.can_execute_requested.remove(token);
            }
        }))
    }

    /// Adds a handler of `CanExecuteChanged`. Dropping the returned guard removes it.
    pub fn can_execute_changed(&self, handler: impl Fn() + 'static) -> Subscription {
        Subscription::new(self.add_can_execute_changed(Rc::new(handler)))
    }

    fn add_can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.can_execute_changed.remove(token);
            }
        })
    }

    /// Raises `CanExecuteChanged`: the sources of the command ask again whether it can run
    /// (`NotifyCanExecuteChanged`).
    pub fn notify_can_execute_changed(&self) {
        for (_, handler) in self.can_execute_changed.snapshot().iter() {
            handler();
        }
    }

    /// Whether the command can run with `parameter` (`CanExecute`): `CanExecuteRequested` with
    /// `CanExecute` true, then the wrapped command, if any.
    pub fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        let args = CanExecuteRequestedEventArgs { parameter: parameter.cloned(), can_execute: Cell::new(true) };
        let this = self.to_ref();
        for (_, handler) in self.can_execute_requested.snapshot().iter() {
            handler(&this, &args);
        }
        let mut can_execute = args.can_execute();
        if let Some(command) = self.command() {
            can_execute = can_execute && command.can_execute(parameter);
        }
        can_execute
    }

    /// Runs the command with `parameter` (`Execute`): raises `ExecuteRequested`, then runs the
    /// wrapped command, if any.
    pub fn execute(&self, parameter: Option<&BoxedValue>) {
        let args = ExecuteRequestedEventArgs { parameter: parameter.cloned() };
        let this = self.to_ref();
        for (_, handler) in self.execute_requested.snapshot().iter() {
            handler(&this, &args);
        }
        if let Some(command) = self.command() {
            command.execute(parameter);
        }
    }
}
