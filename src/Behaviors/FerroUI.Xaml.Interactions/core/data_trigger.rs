//! Port of `Core/DataTrigger.cs`.

use super::DataTriggerBehavior;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;

/// A behavior that performs actions when the bound data meets a specified condition (the short
/// name of [`DataTriggerBehavior`]).
#[repr(C)]
pub struct DataTrigger {
    base: DataTriggerBehavior,
}

ferro_class!(DataTrigger: DataTriggerBehavior);
ferro_class_info!(DataTrigger { new: DataTrigger::new });
ferroui_base::ferro_impl_classes!(DataTrigger: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

impl DataTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DataTriggerBehavior::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
