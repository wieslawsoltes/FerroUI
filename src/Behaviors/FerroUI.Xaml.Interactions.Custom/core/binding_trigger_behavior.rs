//! Port of `Core/BindingTriggerBehavior.cs`.

use ferroui_base::data::{BindingBase, BindingExpressionBase};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use ferroui_xaml_interactivity::helpers::ComparisonConditionTypeHelper;
use ferroui_xaml_interactivity::{
    ComparisonConditionType, Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt,
    StyledElementTrigger,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A behavior that performs actions when the bound data meets a specified condition.
///
/// The binding itself is assigned to [`binding`](Self::binding) (markup assigns it as it is) and
/// bound by the behavior to a private property; the comparison of its value with
/// [`value`](Self::value) runs when the behavior is initialized, then in a job of the dispatcher
/// after one of them changed.
#[repr(C)]
pub struct BindingTriggerBehavior {
    base: StyledElementTrigger,
    dispose: RefCell<Option<Rc<dyn BindingExpressionBase>>>,
}

ferro_class!(BindingTriggerBehavior: StyledElementTrigger);
ferro_class_info!(BindingTriggerBehavior {
    new: BindingTriggerBehavior::new,
    markup: {
        property_attributes: [
            Binding: [AssignBinding],
        ],
    },
});
ferroui_base::ferro_impl_classes!(BindingTriggerBehavior: StyledElementImpl);

ferro_properties! {
    impl BindingTriggerBehavior {
        /// Identifies the `Binding` property.
        pub fn binding_property() -> StyledProperty<Option<Rc<dyn BindingBase>>> {
            FerroProperty::register_with::<BindingTriggerBehavior, _>(
                "Binding",
                StyledPropertyOptions::new(None).assign_binding(true),
            )
        }

        /// Identifies the `ComparisonCondition` property.
        pub fn comparison_condition_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<BindingTriggerBehavior, _>("ComparisonCondition", ComparisonConditionType::Equal)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<BindingTriggerBehavior, _>("Value", None)
        }

        /// The value of the binding (a private property of the reference).
        fn binding_value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<BindingTriggerBehavior, _>("BindingValue", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for BindingTriggerBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::comparison_condition_property().as_property()
                || property == Self::value_property().as_property()
                || property == Self::binding_value_property().as_property()
            {
                this.on_value_changed(change);
            }

            if property == Self::binding_property().as_property() {
                let dispose = this.dispose.borrow_mut().take();
                if let Some(dispose) = dispose {
                    dispose.dispose();
                }

                let new_value = change.get_new_value::<Option<Rc<dyn BindingBase>>>();
                if let Some(new_value) = new_value {
                    let dispose = this.bind_binding(Self::binding_value_property().as_property(), &*new_value);
                    *this.dispose.borrow_mut() = Some(dispose);
                }
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for BindingTriggerBehavior {
        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);

            // The reference disposes the binding and keeps the disposed handle.
            let dispose = this.dispose.borrow().clone();
            if let Some(dispose) = dispose {
                dispose.dispose();
            }
        }

        fn on_initialized_event(this: &Self) {
            Self::parent_on_initialized_event(this);

            this.execute(None);
        }
    }
}

impl BindingTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct(), dispose: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the bound object that the [`BindingTriggerBehavior`] will listen to. This is
    /// a property of the object model, to which markup assigns a binding as it is.
    pub fn binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.get_value(Self::binding_property())
    }

    pub fn set_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        self.set_value(Self::binding_property(), value)
    }

    /// Gets or sets the type of comparison to be performed between the value of
    /// [`binding`](Self::binding) and [`value`](Self::value). This is a property of the object
    /// model.
    pub fn comparison_condition(&self) -> ComparisonConditionType {
        self.get_value(Self::comparison_condition_property())
    }

    pub fn set_comparison_condition(&self, value: ComparisonConditionType) {
        self.set_value(Self::comparison_condition_property(), value)
    }

    /// Gets or sets the value to be compared with the value of [`binding`](Self::binding). This
    /// is a property of the object model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value to be compared (named apart from the setter of every property of the
    /// object, as `DataTriggerBehavior` names its own).
    pub fn set_trigger_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    fn binding_value(&self) -> Option<BoxedValue> {
        self.get_value(Self::binding_value_property())
    }

    fn on_value_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.sender().downcast_ref::<BindingTriggerBehavior>().is_none() {
            return;
        }

        let behavior: Ref<BindingTriggerBehavior> = self.to_ref();
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

        // NOTE: In UWP version binding null check is not present but the framework throws an
        // exception as bindings are null when first initialized.
        let binding = self.binding_value();
        if binding.is_some() {
            // Some value has changed--either the binding value, reference value, or the
            // comparison condition. Re-evaluate the equation.
            let is_met = ComparisonConditionTypeHelper::compare(
                binding.as_ref(),
                self.comparison_condition(),
                self.value().as_ref(),
            )
            .unwrap_or_else(|error| panic!("{error}"));
            if is_met {
                let sender: BoxedValue = Rc::new(associated_object);
                Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
            }
        }
    }
}
