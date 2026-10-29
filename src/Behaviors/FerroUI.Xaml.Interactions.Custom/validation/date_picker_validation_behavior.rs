//! Port of `Validation/DatePickerValidationBehavior.cs`.

use super::property_validation_behavior::PropertyValidationBehavior;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StaticType, StyledElementImpl,
};
use ferroui_base::utilities::DateTimeOffset;
use ferroui_controls::DatePicker;
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::any::TypeId;

/// Validation behavior for [`DatePicker`] selected date.
#[repr(C)]
pub struct DatePickerValidationBehavior {
    base: PropertyValidationBehavior,
}

ferro_class!(DatePickerValidationBehavior: PropertyValidationBehavior);
ferro_class_info!(DatePickerValidationBehavior { new: DatePickerValidationBehavior::new });
ferroui_base::ferro_impl_classes!(DatePickerValidationBehavior: StyledElementImpl, StyledElementBehaviorImpl, DisposingBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for DatePickerValidationBehavior {
        /// The constructor body: the property the behavior validates.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_property(Some(DatePicker::selected_date_property().as_property()));
        }
    }
}

impl DatePickerValidationBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PropertyValidationBehavior::construct(<DatePicker as StaticType>::TYPE, TypeId::of::<Option<DateTimeOffset>>()) }
    }

    /// Initializes a new instance of the [`DatePickerValidationBehavior`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
