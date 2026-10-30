//! Port of `FerroObject/Action.cs`.

use crate::IAction;
use ferroui_base::{ferro_class, ferro_properties, BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, StyledProperty};

/// A base class for action that calls a method on a specified object when invoked.
///
/// The class is abstract: a deriving class implements [`ActionImpl::execute`].
#[repr(C)]
pub struct Action {
    base: FerroObject,
}

ferro_class! {
    Action: FerroObject, virtuals ActionImpl: FerroObjectImpl {
        /// Executes the action.
        ///
        /// `sender` is the object that is passed to the action by the behavior. Generally this is
        /// the associated object of the behavior or a target object. The value of `parameter` is
        /// determined by the caller. An example of parameter usage is EventTriggerBehavior,
        /// which passes the event args as a parameter to its actions.
        ///
        /// Returns the result of the action.
        fn execute(this, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue>;
    }
}

ferroui_base::ferro_impl_classes!(Action: FerroObjectImpl);

impl ActionImpl for Action {
    fn execute(_this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        panic!("Action is abstract: 'execute' must be implemented by the deriving class")
    }
}

ferro_properties! {
    impl Action {
        /// Identifies the `IsEnabled` property.
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Action, _>("IsEnabled", true)
        }
    }
}

impl Action {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<Action>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Gets or sets a value indicating whether this instance is enabled.
    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value)
    }
}

impl IAction for Action {
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Action::execute(self, sender, parameter)
    }
}
