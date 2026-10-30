//! Port of `StyledElement/StyledElementAction.cs`.

use crate::contract::InterfaceTable;
use crate::helpers::TemplatedParentHelper;
use crate::IAction;
use ferroui_base::{
    ferro_class, ferro_properties, BoxedValue, FerroObject, FerroObjectImpl, FerroProperty, ObjectType, StyledElement,
    StyledElementImpl, StyledProperty,
};

/// A base class for action that calls a method on a specified object when invoked.
///
/// The action is a styled element: under a [`StyledElementTrigger`](crate::StyledElementTrigger)
/// that is in the logical tree it is a logical child of the trigger (one way), so it binds with
/// `$parent` and `TemplateBinding`.
///
/// The class is abstract: a deriving class implements [`StyledElementActionImpl::execute`].
#[repr(C)]
pub struct StyledElementAction {
    base: StyledElement,
}

ferro_class! {
    StyledElementAction: StyledElement, virtuals StyledElementActionImpl: StyledElementImpl {
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

ferroui_base::ferro_impl_classes!(StyledElementAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for StyledElementAction {
    fn execute(_this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        panic!("StyledElementAction is abstract: 'execute' must be implemented by the deriving class")
    }
}

ferro_properties! {
    impl StyledElementAction {
        /// Identifies the `IsEnabled` property.
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<StyledElementAction, _>("IsEnabled", true)
        }
    }
}

impl StyledElementAction {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<StyledElementAction>();
    }

    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElement::construct() }
    }

    /// Gets or sets a value indicating whether this instance is enabled.
    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value)
    }

    /// Initializes the action (an internal member of the reference).
    #[doc(hidden)]
    pub fn initialize(&self) {
        self.initialize_if_needed();
    }

    /// Makes the action a logical child of `parent` (an internal member of the reference).
    #[doc(hidden)]
    pub fn attach_action_to_logical_tree(&self, parent: &StyledElement) {
        // Required for $parent binding in markup.
        self.set_parent(parent.to_ref());

        // Required for TemplateBinding in markup.
        if let Some(templated_parent) = parent.templated_parent() {
            TemplatedParentHelper::set_templated_parent(self, Some(templated_parent));
        }

        if let Some(lifecycle) = <dyn IActionLogicalTreeLifecycle>::of(self) {
            lifecycle.attached_to_action_logical_tree();
        }
    }

    /// Removes the action from the logical tree (an internal member of the reference).
    #[doc(hidden)]
    pub fn detach_action_from_logical_tree(&self, parent: &StyledElement) {
        if let Some(lifecycle) = <dyn IActionLogicalTreeLifecycle>::of(self) {
            lifecycle.detached_from_action_logical_tree();
        }

        self.set_parent(None);

        if self.templated_parent().is_some() || parent.templated_parent().is_some() {
            TemplatedParentHelper::set_templated_parent(self, None);
        }
    }
}

impl IAction for StyledElementAction {
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        StyledElementAction::execute(self, sender, parameter)
    }
}

/// The lifecycle of an action in the logical tree of its trigger (an internal contract of the
/// reference, implemented by [`InvokeCommandActionBase`](crate::InvokeCommandActionBase)).
#[doc(hidden)]
pub trait IActionLogicalTreeLifecycle {
    fn attached_to_action_logical_tree(&self);

    fn detached_from_action_logical_tree(&self);
}

static TABLE: InterfaceTable<dyn IActionLogicalTreeLifecycle> = InterfaceTable::new();

fn cast<'a, T: ObjectType + IActionLogicalTreeLifecycle>(
    object: &'a FerroObject,
) -> Option<&'a (dyn IActionLogicalTreeLifecycle + 'static)> {
    object.downcast_ref::<T>().map(|object| object as &(dyn IActionLogicalTreeLifecycle + 'static))
}

impl dyn IActionLogicalTreeLifecycle {
    /// States that the objects of class `T` (and of the classes deriving from it) implement the
    /// contract. Called from the static constructor of the class.
    pub fn register<T: ObjectType + IActionLogicalTreeLifecycle>() {
        TABLE.register(T::TYPE, cast::<T>);
    }

    /// The object as an implementation of the contract, if its class is one.
    pub fn of(object: &FerroObject) -> Option<&dyn IActionLogicalTreeLifecycle> {
        TABLE.find(object)
    }
}
