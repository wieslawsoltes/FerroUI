//! Port of `Behaviors/InvokeCommandBehaviorBase.cs`.

use crate::helpers::{CommandCanExecuteIsEnabledBinder, CommandCanExecuteObserver};
use crate::{StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::input::{ICommand, InputElement};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, BoxedValue, DirectProperty, FerroObjectExtensions, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use std::cell::{Cell, OnceCell};
use std::rc::Rc;

/// Invoke command behavior base class: a behavior of a [`Control`] that invokes a command,
/// reports whether the command can execute ([`can_execute_command`](Self::can_execute_command))
/// and can make the control follow it ([`use_command_can_execute_for_is_enabled`](Self::use_command_can_execute_for_is_enabled)).
///
/// The class is abstract.
#[repr(C)]
pub struct InvokeCommandBehaviorBase {
    base: StyledElementBehaviorOfT,
    command_can_execute_observer: OnceCell<CommandCanExecuteObserver>,
    command_can_execute_is_enabled_binder: CommandCanExecuteIsEnabledBinder,
    can_execute_command: Cell<bool>,
    pass_event_args_to_command: Cell<bool>,
}

ferro_class!(InvokeCommandBehaviorBase: StyledElementBehaviorOfT);
ferro_class_info!(InvokeCommandBehaviorBase {
    markup: {
        properties: [
            PassEventArgsToCommand: bool {
                get: |this: &Ref<InvokeCommandBehaviorBase>| this.pass_event_args_to_command(),
                set: |this: &Ref<InvokeCommandBehaviorBase>, value: bool| this.set_pass_event_args_to_command(value)
            },
        ],
    },
});
ferroui_base::ferro_impl_classes!(InvokeCommandBehaviorBase: StyledElementImpl);

ferro_properties! {
    impl InvokeCommandBehaviorBase {
        /// Identifies the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<InvokeCommandBehaviorBase, _>("Command", None)
        }

        /// Identifies the `CanExecuteCommand` property.
        pub fn can_execute_command_property() -> DirectProperty<InvokeCommandBehaviorBase, bool> {
            FerroProperty::register_direct::<InvokeCommandBehaviorBase, _>(
                "CanExecuteCommand",
                |behavior| behavior.can_execute_command(),
                None,
                false,
            )
        }

        /// Identifies the `CanExecuteCommandParameter` property.
        pub fn can_execute_command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<InvokeCommandBehaviorBase, _>("CanExecuteCommandParameter", None)
        }

        /// Identifies the `UseCommandCanExecuteForIsEnabled` property.
        pub fn use_command_can_execute_for_is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<InvokeCommandBehaviorBase, _>("UseCommandCanExecuteForIsEnabled", false)
        }

        /// Identifies the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<InvokeCommandBehaviorBase, _>("CommandParameter", None)
        }

        /// Identifies the `InputConverter` property.
        pub fn input_converter_property() -> StyledProperty<Option<Rc<dyn IValueConverter>>> {
            FerroProperty::register::<InvokeCommandBehaviorBase, _>("InputConverter", None)
        }

        /// Identifies the `InputConverterParameter` property.
        pub fn input_converter_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<InvokeCommandBehaviorBase, _>("InputConverterParameter", None)
        }

        /// Identifies the `InputConverterLanguage` property. The empty string of the default
        /// value means the invariant culture.
        pub fn input_converter_language_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<InvokeCommandBehaviorBase, _>("InputConverterLanguage", Some(String::new()))
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for InvokeCommandBehaviorBase {
        /// The constructor body: the observer reports to the `CanExecuteCommand` property.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            let behavior = this.to_ref().downgrade();
            let _ = this.command_can_execute_observer.set(CommandCanExecuteObserver::new(move |value| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_can_execute_command(value);
                }
            }));
        }

        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::command_property().as_property()
                || property == Self::command_parameter_property().as_property()
                || property == Self::can_execute_command_parameter_property().as_property()
                || property == Self::input_converter_property().as_property()
            {
                this.update_command_can_execute_observer();
            } else if property == Self::use_command_can_execute_for_is_enabled_property().as_property() {
                this.update_command_can_execute_is_enabled_binding();
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for InvokeCommandBehaviorBase {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);

            if let Some(observer) = this.command_can_execute_observer.get() {
                observer.start_with(this.command(), this.resolve_can_execute_parameter(), this.is_can_execute_parameter_known());
            }
            this.update_command_can_execute_is_enabled_binding();
        }

        fn on_detaching(this: &Self) {
            this.command_can_execute_is_enabled_binder.stop();
            if let Some(observer) = this.command_can_execute_observer.get() {
                observer.stop();
            }

            Self::parent_on_detaching(this);
        }
    }
}

impl InvokeCommandBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            command_can_execute_observer: OnceCell::new(),
            command_can_execute_is_enabled_binder: CommandCanExecuteIsEnabledBinder::new(),
            can_execute_command: Cell::new(true),
            pass_event_args_to_command: Cell::new(false),
        }
    }

    /// Gets or sets the command this behavior should invoke.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// Gets a value indicating whether [`command`](Self::command) can execute with the current
    /// can-execute parameter.
    pub fn can_execute_command(&self) -> bool {
        self.can_execute_command.get()
    }

    fn set_can_execute_command(&self, value: bool) {
        self.set_and_raise_cell(Self::can_execute_command_property(), &self.can_execute_command, value);
    }

    /// Gets or sets the parameter that is passed to the `can_execute` of the command. When it is
    /// not set, [`command_parameter`](Self::command_parameter) is used if it is set. It does not
    /// change the parameter passed to the `execute` of the command.
    pub fn can_execute_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::can_execute_command_parameter_property())
    }

    pub fn set_can_execute_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::can_execute_command_parameter_property(), value)
    }

    /// Gets or sets a value indicating whether the `IsEnabled` property of the associated control
    /// should follow [`can_execute_command`](Self::can_execute_command).
    pub fn use_command_can_execute_for_is_enabled(&self) -> bool {
        self.get_value(Self::use_command_can_execute_for_is_enabled_property())
    }

    pub fn set_use_command_can_execute_for_is_enabled(&self, value: bool) {
        self.set_value(Self::use_command_can_execute_for_is_enabled_property(), value)
    }

    /// Gets or sets the parameter that is passed to the `execute` of the command. If this is not
    /// set, the parameter given to the behavior is used.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// Gets or sets the converter that is run on the parameter given to the behavior.
    pub fn input_converter(&self) -> Option<Rc<dyn IValueConverter>> {
        self.get_value(Self::input_converter_property())
    }

    pub fn set_input_converter(&self, value: Option<Rc<dyn IValueConverter>>) {
        self.set_value(Self::input_converter_property(), value)
    }

    /// Gets or sets the parameter that is passed to the `convert` of the
    /// [`input_converter`](Self::input_converter).
    pub fn input_converter_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::input_converter_parameter_property())
    }

    pub fn set_input_converter_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::input_converter_parameter_property(), value)
    }

    /// Gets or sets the language that is passed to the `convert` of the
    /// [`input_converter`](Self::input_converter).
    pub fn input_converter_language(&self) -> Option<String> {
        self.get_value(Self::input_converter_language_property())
    }

    pub fn set_input_converter_language(&self, value: Option<String>) {
        self.set_value(Self::input_converter_language_property(), value)
    }

    /// Specifies whether the event args of the event that triggered this behavior should be passed
    /// to the command as a parameter.
    pub fn pass_event_args_to_command(&self) -> bool {
        self.pass_event_args_to_command.get()
    }

    pub fn set_pass_event_args_to_command(&self, value: bool) {
        if self.pass_event_args_to_command.get() == value {
            return;
        }

        self.pass_event_args_to_command.set(value);
        self.update_command_can_execute_observer();
    }

    fn update_command_can_execute_observer(&self) {
        if let Some(observer) = self.command_can_execute_observer.get() {
            observer.update_with(self.command(), self.resolve_can_execute_parameter(), self.is_can_execute_parameter_known());
        }
    }

    /// Resolves the command parameter that will be supplied to the command: the
    /// [`command_parameter`](Self::command_parameter) if it is set, else the parameter given to
    /// the behavior converted by the [`input_converter`](Self::input_converter), else the
    /// parameter itself if [`pass_event_args_to_command`](Self::pass_event_args_to_command).
    pub fn resolve_parameter(&self, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if self.is_set(Self::command_parameter_property()) {
            self.command_parameter()
        } else if let Some(input_converter) = self.input_converter() {
            let culture = match self.input_converter_language() {
                Some(language) => CultureInfo::get_culture_info(&language),
                None => CultureInfo::current_culture(),
            };
            input_converter
                .convert(parameter, ValueType::object(), self.input_converter_parameter().as_ref(), &culture)
                .ok()
                .flatten()
        } else if self.pass_event_args_to_command() {
            parameter.cloned()
        } else {
            None
        }
    }

    fn resolve_can_execute_parameter(&self) -> Option<BoxedValue> {
        if self.is_set(Self::can_execute_command_parameter_property()) {
            return self.can_execute_command_parameter();
        }

        if self.is_set(Self::command_parameter_property()) {
            self.command_parameter()
        } else {
            None
        }
    }

    fn is_can_execute_parameter_known(&self) -> bool {
        self.is_set(Self::can_execute_command_parameter_property())
            || self.is_set(Self::command_parameter_property())
            || (self.input_converter().is_none() && !self.pass_event_args_to_command())
    }

    fn update_command_can_execute_is_enabled_binding(&self) {
        self.command_can_execute_is_enabled_binder.update(
            self.associated_object_as::<InputElement>(),
            self.use_command_can_execute_for_is_enabled(),
            self.get_observable(Self::can_execute_command_property()),
        );
    }
}
