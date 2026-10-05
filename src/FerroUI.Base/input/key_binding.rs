use super::{ICommand, KeyEventArgs, KeyGesture};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// Binds a key gesture to a command.
#[repr(C)]
pub struct KeyBinding {
    base: FerroObject,
}

ferro_class!(KeyBinding: FerroObject);
crate::ferro_class_info!(KeyBinding { new: KeyBinding::new });
ferro_impl_classes!(KeyBinding: FerroObjectImpl);

crate::ferro_properties! { impl KeyBinding {
    ferro_property!(
        /// Defines the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<KeyBinding, _>("Command", None)
        }
    );

    ferro_property!(
        /// Defines the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<KeyBinding, _>("CommandParameter", None)
        }
    );

    ferro_property!(
        /// Defines the `Gesture` property.
        pub fn gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<KeyBinding, _>("Gesture", None)
        }
    );
} }

impl KeyBinding {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The command invoked by the binding.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// The parameter passed to the command.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// The key gesture that invokes the command.
    pub fn gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::gesture_property())
    }

    pub fn set_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::gesture_property(), value)
    }

    /// Executes the command and marks the event handled, if the event
    /// matches the gesture and the command can execute.
    pub fn try_handle(&self, args: &KeyEventArgs) {
        if self.gesture().is_some_and(|gesture| gesture.matches(Some(args))) {
            if let Some(command) = self.command() {
                let parameter = self.command_parameter();
                if command.can_execute(parameter.as_ref()) {
                    args.set_handled(true);
                    command.execute(parameter.as_ref());
                }
            }
        }
    }
}
