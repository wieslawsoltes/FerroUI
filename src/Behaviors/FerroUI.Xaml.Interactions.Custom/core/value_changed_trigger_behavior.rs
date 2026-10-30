//! Port of `Core/ValueChangedTriggerBehavior.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTrigger};
use std::rc::Rc;

/// A behavior that performs actions when the bound data produces new value.
///
/// The actions run in a job of the dispatcher after each change of [`binding`](Self::binding)
/// to a value that is not null, with the change as their parameter.
#[repr(C)]
pub struct ValueChangedTriggerBehavior {
    base: StyledElementTrigger,
}

ferro_class!(ValueChangedTriggerBehavior: StyledElementTrigger);
ferro_class_info!(ValueChangedTriggerBehavior { new: ValueChangedTriggerBehavior::new });
ferroui_base::ferro_impl_classes!(ValueChangedTriggerBehavior: StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl ValueChangedTriggerBehavior {
        /// Identifies the `Binding` property.
        pub fn binding_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ValueChangedTriggerBehavior, _>("Binding", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ValueChangedTriggerBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::binding_property().as_property() {
                this.on_value_changed(change);
            }
        }
    }
}

impl ValueChangedTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the bound object that the [`ValueChangedTriggerBehavior`] will listen to. This
    /// is a property of the object model.
    pub fn binding(&self) -> Option<BoxedValue> {
        self.get_value(Self::binding_property())
    }

    pub fn set_binding(&self, value: Option<BoxedValue>) {
        self.set_value(Self::binding_property(), value)
    }

    fn on_value_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.sender().downcast_ref::<ValueChangedTriggerBehavior>().is_none() {
            return;
        }

        let behavior: Ref<ValueChangedTriggerBehavior> = self.to_ref();
        // The change is the parameter of the actions (the property changed event args of the
        // reference).
        let parameter: BoxedValue = Rc::new(change.to_owned_args());
        Dispatcher::ui_thread().post_local(
            move || {
                behavior.execute(Some(&parameter));
            },
            DispatcherPriority::DEFAULT,
        );
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        let Some(associated_object) = self.associated_object() else {
            return;
        };

        if !self.is_enabled() {
            return;
        }

        let binding = self.binding();
        if binding.is_some() {
            let sender: BoxedValue = Rc::new(associated_object);
            Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
        }
    }
}
