use super::ISetterInstance;

/// The instance of a setter of a direct property with a binding: the binding
/// has been applied to the control directly.
pub(crate) struct DirectPropertySetterBindingInstance;

impl ISetterInstance for DirectPropertySetterBindingInstance {}
