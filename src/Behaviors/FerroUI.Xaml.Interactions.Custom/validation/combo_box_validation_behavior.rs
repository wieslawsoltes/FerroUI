//! Port of `Validation/ComboBoxValidationBehavior.cs`.

use super::property_validation_behavior::PropertyValidationBehavior;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StaticType, StyledElementImpl,
};
use ferroui_base::BoxedValue;
use ferroui_controls::primitives::SelectingItemsControl;
use ferroui_controls::ComboBox;
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::any::TypeId;

/// Validation behavior for [`ComboBox`] selected item.
#[repr(C)]
pub struct ComboBoxValidationBehavior {
    base: PropertyValidationBehavior,
}

ferro_class!(ComboBoxValidationBehavior: PropertyValidationBehavior);
ferro_class_info!(ComboBoxValidationBehavior { new: ComboBoxValidationBehavior::new });
ferroui_base::ferro_impl_classes!(ComboBoxValidationBehavior: StyledElementImpl, StyledElementBehaviorImpl, DisposingBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ComboBoxValidationBehavior {
        /// The constructor body: the property the behavior validates.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_property(Some(SelectingItemsControl::selected_item_property().as_property()));
        }
    }
}

impl ComboBoxValidationBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PropertyValidationBehavior::construct(<ComboBox as StaticType>::TYPE, TypeId::of::<Option<BoxedValue>>()) }
    }

    /// Initializes a new instance of the [`ComboBoxValidationBehavior`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
