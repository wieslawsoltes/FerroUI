//! Port of `Core/CallMethodAction.cs`.

use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::helpers::member_lookup::type_name_of;
use ferroui_xaml_interactivity::helpers::MethodHelper;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that calls a method on a specified object when invoked.
///
/// The method is a public method of the target that takes no parameters, or two parameters of
/// which the first is the sender; it is found by name in the markup metadata of the type of the
/// target (see [`MethodHelper`]).
#[repr(C)]
pub struct CallMethodAction {
    base: StyledElementAction,
    method_helper: MethodHelper,
}

ferro_class!(CallMethodAction: StyledElementAction);
ferro_class_info!(CallMethodAction {
    new: CallMethodAction::new,
    markup: {
        property_attributes: [
            TargetObject: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(CallMethodAction: StyledElementImpl);

ferro_properties! {
    impl CallMethodAction {
        /// Identifies the `MethodName` property.
        pub fn method_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<CallMethodAction, _>("MethodName", None)
        }

        /// Identifies the `TargetObject` property.
        pub fn target_object_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<CallMethodAction, _>("TargetObject", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for CallMethodAction {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::method_name_property().as_property() {
                let method_name = change.get_new_value::<Option<String>>();
                this.method_helper.update_method_descriptors(method_name.as_deref());
            }

            if change.property() == Self::target_object_property().as_property() {
                let new_value = change.get_new_value::<Option<BoxedValue>>();
                if let Some(new_value) = new_value {
                    this.method_helper.update_target_type(&new_value, this.method_name().as_deref());
                }
            }
        }
    }
}

impl StyledElementActionImpl for CallMethodAction {
    /// Executes the action.
    ///
    /// `sender` is the object that is passed to the action by the behavior. Generally this is the
    /// associated object of the behavior or a target object. The value of `parameter` is
    /// determined by the caller.
    ///
    /// Returns true if the method is called; else false.
    ///
    /// # Panics
    /// Panics when [`target_object`](CallMethodAction::target_object) is set and has no method of
    /// the name with the expected signature (the argument exception of the reference).
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(method_name) = this.method_name().filter(|name| !name.is_empty()) else {
            return Some(Rc::new(false));
        };

        let target = match this.target_object() {
            Some(target_object) => Some(target_object),
            None => sender.cloned(),
        };
        let Some(target) = target else {
            return Some(Rc::new(false));
        };

        this.method_helper.update_target_type(&target, Some(&method_name));

        let Some(method_descriptor) = this.method_helper.find_best_method(parameter) else {
            if this.target_object().is_some() {
                panic!(
                    "Cannot find method named {} on object of type {} that matches the expected signature.",
                    method_name,
                    type_name_of(&target)
                );
            }

            return Some(Rc::new(false));
        };

        let method = method_descriptor.method_info();
        let result = match method_descriptor.parameter_count() {
            0 => (method.invoke)(&[Some(target.clone())]),
            2 => (method.invoke)(&[Some(target.clone()), Some(target.clone()), parameter.cloned()]),
            _ => return Some(Rc::new(false)),
        };

        // The exception of the invoked method propagates (`TargetInvocationException`).
        if let Err(error) = result {
            panic!("{error}");
        }

        Some(Rc::new(true))
    }
}

impl CallMethodAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct(), method_helper: MethodHelper::new() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the name of the method to invoke. This is a property of the object model.
    pub fn method_name(&self) -> Option<String> {
        self.get_value(Self::method_name_property())
    }

    pub fn set_method_name(&self, value: Option<String>) {
        self.set_value(Self::method_name_property(), value)
    }

    /// Gets or sets the object that exposes the method of interest. Markup resolves a name given
    /// for it to the named element. This is a property of the object model.
    pub fn target_object(&self) -> Option<BoxedValue> {
        self.get_value(Self::target_object_property())
    }

    pub fn set_target_object(&self, value: Option<BoxedValue>) {
        self.set_value(Self::target_object_property(), value)
    }
}
