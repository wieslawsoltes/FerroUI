//! Port of `Validation/NumericUpDownValidationBehavior.cs`.

use super::property_validation_behavior::PropertyValidationBehavior;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StaticType, StyledElementImpl,
};
use ferroui_base::utilities::Decimal;
use ferroui_controls::NumericUpDown;
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::any::TypeId;

/// Validation behavior for [`NumericUpDown`] value.
#[repr(C)]
pub struct NumericUpDownValidationBehavior {
    base: PropertyValidationBehavior,
}

ferro_class!(NumericUpDownValidationBehavior: PropertyValidationBehavior);
ferro_class_info!(NumericUpDownValidationBehavior { new: NumericUpDownValidationBehavior::new });
ferroui_base::ferro_impl_classes!(NumericUpDownValidationBehavior: StyledElementImpl, StyledElementBehaviorImpl, DisposingBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for NumericUpDownValidationBehavior {
        /// The constructor body: the property the behavior validates.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_property(Some(NumericUpDown::value_property().as_property()));
        }
    }
}

impl NumericUpDownValidationBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PropertyValidationBehavior::construct(<NumericUpDown as StaticType>::TYPE, TypeId::of::<Option<Decimal>>()) }
    }

    /// Initializes a new instance of the [`NumericUpDownValidationBehavior`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
