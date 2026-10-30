//! Port of `Actions/CallMethodAsyncAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::helpers::member_lookup::{find_methods, type_name_of};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Calls a method of the [`target_object`](Self::target_object) (or of the sender) by name: a
/// method without parameters, or with two parameters (the sender and the parameter of the action)
/// whose first parameter is untyped.
///
/// The method is found among the methods the markup metadata of the target declares (the
/// reflection of the reference).
#[repr(C)]
pub struct CallMethodAsyncAction {
    base: StyledElementAction,
}

ferro_class!(CallMethodAsyncAction: StyledElementAction);
ferro_class_info!(CallMethodAsyncAction {
    new: CallMethodAsyncAction::new,
    markup: {
        property_attributes: [
            TargetObject: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CallMethodAsyncAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl CallMethodAsyncAction {
        /// Identifies the `MethodName` property.
        pub fn method_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<CallMethodAsyncAction, _>("MethodName", None)
        }

        /// Identifies the `TargetObject` property.
        pub fn target_object_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<CallMethodAsyncAction, _>("TargetObject", None)
        }
    }
}

impl StyledElementActionImpl for CallMethodAsyncAction {
    /// Executes the action.
    ///
    /// # Panics
    /// Panics when the target has no method of the name with an expected signature, or when the
    /// call fails (the exceptions of the reference).
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target = if this.get_value(Self::target_object_property()).is_some() {
            this.target_object()
        } else {
            sender.cloned()
        };
        let method_name = this.method_name().filter(|name| !name.is_empty());
        let (Some(target), Some(method_name)) = (target, method_name) else {
            return Some(Rc::new(false));
        };

        let method_info = find_methods(&*target, &method_name).into_iter().find(|method| {
            let parameters = method.parameters;
            parameters.is_empty() || (parameters.len() == 2 && parameters[0]().is_object())
        });
        let Some(method_info) = method_info else {
            panic!(
                "Cannot find method named {} on object of type {} that matches the expected signature.",
                method_name,
                type_name_of(&target)
            );
        };

        let mut arguments = Vec::new();
        if !method_info.is_static {
            arguments.push(Some(target.clone()));
        }
        if method_info.parameters.len() == 2 {
            arguments.push(sender.cloned());
            arguments.push(parameter.cloned());
        }

        // The exception of the invoked method propagates (`TargetInvocationException`).
        // Deviation (DEVIATIONS.md, Behaviors): a task the method returns is not awaited in a job
        // of the dispatcher (the reference awaits it only to observe it); it runs on its own.
        if let Err(error) = (method_info.invoke)(&arguments) {
            panic!("{error}");
        }

        Some(Rc::new(true))
    }
}

impl CallMethodAsyncAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the name of the method to call. This is a property of the object model.
    pub fn method_name(&self) -> Option<String> {
        self.get_value(Self::method_name_property())
    }

    pub fn set_method_name(&self, value: Option<String>) {
        self.set_value(Self::method_name_property(), value)
    }

    /// Gets or sets the object that exposes the method. If not set, the sender is used. Markup
    /// resolves a name given for it to the named element.
    pub fn target_object(&self) -> Option<BoxedValue> {
        self.get_value(Self::target_object_property())
    }

    pub fn set_target_object(&self, value: Option<BoxedValue>) {
        self.set_value(Self::target_object_property(), value)
    }
}
