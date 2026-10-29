//! Port of `Actions/ChangeFerroPropertyAction.cs` (the reference names the class after the
//! property class of its framework, here `FerroProperty`).

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObject,
    FerroObjectImpl, FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::helpers::member_lookup::{object_of, property_value_type, type_name_of};
use ferroui_xaml_interactivity::helpers::TypeConverterHelper;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that sets a registered property ([`target_property`](Self::target_property)) of the
/// [`target_object`](Self::target_object) (or of the sender) to [`value`](Self::value), converted
/// to the type of the property.
#[repr(C)]
pub struct ChangeFerroPropertyAction {
    base: StyledElementAction,
}

ferro_class!(ChangeFerroPropertyAction: StyledElementAction);
ferro_class_info!(ChangeFerroPropertyAction {
    new: ChangeFerroPropertyAction::new,
    markup: {
        property_attributes: [
            TargetObject: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ChangeFerroPropertyAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ChangeFerroPropertyAction {
        /// Identifies the `TargetProperty` property.
        pub fn target_property_property() -> StyledProperty<Option<&'static FerroProperty>> {
            FerroProperty::register::<ChangeFerroPropertyAction, _>("TargetProperty", None)
        }

        /// Identifies the `TargetObject` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_object_property() -> StyledProperty<Option<ElementRef<FerroObject>>> {
            ValueTypes::register_element_ref::<FerroObject>();
            FerroProperty::register::<ChangeFerroPropertyAction, _>("TargetObject", None)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ChangeFerroPropertyAction, _>("Value", None)
        }
    }
}

impl StyledElementActionImpl for ChangeFerroPropertyAction {
    /// Executes the action.
    ///
    /// # Panics
    /// Panics when the property is read-only or the value does not convert to its type (the
    /// argument exceptions of the reference).
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target_object = if this.get_value(Self::target_object_property()).is_some() {
            this.target_object()
        } else {
            sender.and_then(object_of)
        };
        if let (Some(ferro_object), Some(target_property)) = (target_object, this.target_property()) {
            this.update_ferro_property_value(&ferro_object, target_property);
            return Some(Rc::new(true));
        }

        Some(Rc::new(false))
    }
}

impl ChangeFerroPropertyAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the property to change. This is a property of the object model.
    pub fn target_property(&self) -> Option<&'static FerroProperty> {
        self.get_value(Self::target_property_property())
    }

    pub fn set_target_property(&self, value: Option<&'static FerroProperty>) {
        self.set_value(Self::target_property_property(), value)
    }

    /// Gets or sets the value to set. This is a property of the object model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    /// Sets the value to set (named apart from the setter of every property of the object, as
    /// `ChangePropertyAction` names its own).
    pub fn set_action_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::value_property(), value)
    }

    /// Gets or sets the object whose property will be changed. If not set, the sender is used.
    /// Markup resolves a name given for it to the named element.
    pub fn target_object(&self) -> Option<Ref<FerroObject>> {
        ElementRef::resolve(&self.get_value(Self::target_object_property()))
    }

    pub fn set_target_object(&self, value: impl Into<Nullable<FerroObject>>) {
        self.set_value(Self::target_object_property(), ElementRef::from_nullable(value.into().0))
    }

    fn update_ferro_property_value(&self, target_object: &FerroObject, target_property: &'static FerroProperty) {
        Self::validate_target_property(target_property);

        let value = self.value();
        let property_type = property_value_type(target_property);
        let Some(result) = Self::convert(value.as_ref(), property_type) else {
            panic!(
                "Cannot assign value of type {} to property {} of type {}. The {} property can be assigned only values \
                 of type {}.",
                value.as_ref().map_or_else(|| "null".to_string(), type_name_of),
                target_property.name(),
                target_object.get_type().name(),
                target_property.name(),
                target_object.get_type().name()
            );
        };

        let _ = target_object.set_value_untyped(target_property, result.as_any(), BindingPriority::LocalValue);
    }

    /// The value converted to the type of the property: null to the default (or null) of the
    /// type, a value of the type as it is, anything else through its text; `None` when it does not
    /// convert (the format and argument exceptions of the reference).
    fn convert(value: Option<&BoxedValue>, property_type: ValueType) -> Option<BoxedValue> {
        let converted = match value {
            // The result can be null if the type is nullable, or the default value of the type.
            None => None,
            Some(value) => match ValueTypes::try_cast(value, property_type) {
                Some(converted) => Some(converted),
                None => match ValueTypes::try_to_string(&**value) {
                    Some(value_as_string) => TypeConverterHelper::convert(&value_as_string, property_type).ok()?,
                    None => None,
                },
            },
        };

        ValueTypes::try_convert(converted.as_ref(), property_type)
            .flatten()
            .or_else(|| ValueTypes::null_value(property_type).filter(|_| converted.is_none()))
    }

    fn validate_target_property(target_property: &'static FerroProperty) {
        if target_property.is_read_only() {
            panic!("Property {} is read-only.", target_property.name());
        }
    }
}
