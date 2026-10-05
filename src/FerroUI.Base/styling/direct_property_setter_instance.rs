use super::ISetterInstance;

/// The instance of a setter of a direct property with a plain value: the
/// value has been set on the control directly.
pub(crate) struct DirectPropertySetterInstance;

impl ISetterInstance for DirectPropertySetterInstance {}
