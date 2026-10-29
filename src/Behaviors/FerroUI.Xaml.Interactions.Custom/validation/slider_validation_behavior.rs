//! Port of `Validation/SliderValidationBehavior.cs`.

use super::property_validation_behavior::PropertyValidationBehavior;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::primitives::RangeBase;
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::any::TypeId;

/// Validation behavior for range based controls like [`Slider`](ferroui_controls::Slider) value.
#[repr(C)]
pub struct SliderValidationBehavior {
    base: PropertyValidationBehavior,
}

ferro_class!(SliderValidationBehavior: PropertyValidationBehavior);
ferro_class_info!(SliderValidationBehavior { new: SliderValidationBehavior::new });
ferroui_base::ferro_impl_classes!(SliderValidationBehavior: StyledElementImpl, StyledElementBehaviorImpl, DisposingBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for SliderValidationBehavior {
        /// The constructor body: the property the behavior validates.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_property(Some(RangeBase::value_property().as_property()));
        }
    }
}

impl SliderValidationBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PropertyValidationBehavior::construct(<RangeBase as StaticType>::TYPE, TypeId::of::<f64>()) }
    }

    /// Initializes a new instance of the [`SliderValidationBehavior`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
