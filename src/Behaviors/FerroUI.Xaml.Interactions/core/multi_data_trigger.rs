//! Port of `Core/MultiDataTrigger.cs`.

use super::MultiDataTriggerBehavior;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;

/// A behavior that performs actions when all bound data conditions are satisfied (the short name
/// of [`MultiDataTriggerBehavior`]).
#[repr(C)]
pub struct MultiDataTrigger {
    base: MultiDataTriggerBehavior,
}

ferro_class!(MultiDataTrigger: MultiDataTriggerBehavior);
ferro_class_info!(MultiDataTrigger { new: MultiDataTrigger::new });
ferroui_base::ferro_impl_classes!(MultiDataTrigger: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

impl MultiDataTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: MultiDataTriggerBehavior::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
