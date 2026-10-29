//! Port of `Core/Condition.cs`.

use crate::ComparisonConditionType;
use ferroui_base::data::{BindingBase, BindingExpressionBase};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty, StyledPropertyOptions,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents a reusable comparison that can be used by triggers and behaviors.
///
/// The left operand is either the value of a [`binding`](Self::binding) (the binding itself is
/// assigned to the property, then bound by the condition) or of a [`property`](Self::property)
/// of a source; the right operand is [`value`](Self::value).
#[repr(C)]
pub struct Condition {
    base: FerroObject,
    binding_subscription: RefCell<Option<Rc<dyn BindingExpressionBase>>>,
}

ferro_class!(Condition: FerroObject);
ferro_class_info!(Condition {
    new: Condition::new,
    markup: {
        property_attributes: [
            Binding: [AssignBinding],
        ],
    },
});

ferro_properties! {
    impl Condition {
        /// Identifies the `Binding` property.
        pub fn binding_property() -> StyledProperty<Option<Rc<dyn BindingBase>>> {
            FerroProperty::register_with::<Condition, _>("Binding", StyledPropertyOptions::new(None).assign_binding(true))
        }

        /// Identifies the `BindingValue` property (internal in the reference): the value of the
        /// binding.
        pub fn binding_value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Condition, _>("BindingValue", None)
        }

        /// Identifies the `ComparisonCondition` property.
        pub fn comparison_condition_property() -> StyledProperty<ComparisonConditionType> {
            FerroProperty::register::<Condition, _>("ComparisonCondition", ComparisonConditionType::Equal)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Condition, _>("Value", None)
        }

        /// Identifies the `Property` property.
        pub fn property_property() -> StyledProperty<Option<&'static FerroProperty>> {
            FerroProperty::register::<Condition, _>("Property", None)
        }

        /// Identifies the `SourceName` property.
        pub fn source_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Condition, _>("SourceName", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for Condition {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::binding_property().as_property() {
                let new_binding = change.get_new_value::<Option<Rc<dyn BindingBase>>>();
                if new_binding.is_some() && this.property().is_some() {
                    panic!("Condition cannot use both Property and Binding.");
                }

                let subscription = this.binding_subscription.borrow_mut().take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }

                match new_binding {
                    Some(new_binding) => {
                        let subscription = this.bind_binding(Self::binding_value_property().as_property(), &*new_binding);
                        *this.binding_subscription.borrow_mut() = Some(subscription);
                    }
                    None => this.set_value(Self::binding_value_property(), None),
                }
            }

            if change.property() == Self::property_property().as_property()
                && change.get_new_value::<Option<&'static FerroProperty>>().is_some()
                && this.binding().is_some()
            {
                panic!("Condition cannot use both Property and Binding.");
            }
        }
    }
}

impl Condition {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), binding_subscription: RefCell::new(None) }
    }

    /// Creates a condition.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the bound object to compare. This is a property of the object model, to which
    /// markup assigns a binding as it is.
    pub fn binding(&self) -> Option<Rc<dyn BindingBase>> {
        self.get_value(Self::binding_property())
    }

    pub fn set_binding(&self, value: Option<Rc<dyn BindingBase>>) {
        self.set_value(Self::binding_property(), value)
    }

    /// Gets or sets the type of comparison that is performed.
    pub fn comparison_condition(&self) -> ComparisonConditionType {
        self.get_value(Self::comparison_condition_property())
    }

    pub fn set_comparison_condition(&self, value: ComparisonConditionType) {
        self.set_value(Self::comparison_condition_property(), value)
    }

    /// Gets or sets the value that is used during comparison.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value that is used during comparison (named apart from the setter of every
    /// property of the object, as the range controls of the framework name theirs).
    pub fn set_condition_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    /// Gets or sets the property that will be monitored for changes.
    pub fn property(&self) -> Option<&'static FerroProperty> {
        self.get_value(Self::property_property())
    }

    pub fn set_property(&self, value: Option<&'static FerroProperty>) {
        self.set_value(Self::property_property(), value)
    }

    /// Gets or sets the name of the element that supplies [`property`](Self::property). When
    /// null, the associated object is used.
    pub fn source_name(&self) -> Option<String> {
        self.get_value(Self::source_name_property())
    }

    pub fn set_source_name(&self, value: Option<String>) {
        self.set_value(Self::source_name_property(), value)
    }

    /// The value of the binding (internal in the reference).
    pub fn binding_value(&self) -> Option<BoxedValue> {
        self.get_value(Self::binding_value_property())
    }

    pub fn set_binding_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::binding_value_property(), value)
    }
}
