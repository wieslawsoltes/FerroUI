//! Port of `Validation/TextBoxValidationBehavior.cs`.

use super::property_validation_behavior::PropertyValidationBehavior;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::TextBox;
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::any::TypeId;

/// Validation behavior for [`TextBox`] text.
#[repr(C)]
pub struct TextBoxValidationBehavior {
    base: PropertyValidationBehavior,
}

ferro_class!(TextBoxValidationBehavior: PropertyValidationBehavior);
ferro_class_info!(TextBoxValidationBehavior { new: TextBoxValidationBehavior::new });
ferroui_base::ferro_impl_classes!(TextBoxValidationBehavior: StyledElementImpl, StyledElementBehaviorImpl, DisposingBehaviorImpl);

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for TextBoxValidationBehavior {
        /// The constructor body: the property the behavior validates.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_property(Some(TextBox::text_property().as_property()));
        }
    }
}

impl TextBoxValidationBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: PropertyValidationBehavior::construct(<TextBox as StaticType>::TYPE, TypeId::of::<Option<String>>()) }
    }

    /// Initializes a new instance of the [`TextBoxValidationBehavior`] class.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
