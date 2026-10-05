use ferroui_base::data::BindingBase;
use std::ops::Deref;
use std::rc::Rc;

/// A binding held as the value of a property (a property that is assigned
/// a binding rather than bound with it). Equality is identity.
#[derive(Clone)]
pub struct AssignedBinding(pub Rc<dyn BindingBase>);

impl AssignedBinding {
    /// Wraps a binding.
    pub fn new(binding: Rc<dyn BindingBase>) -> Self {
        Self(binding)
    }
}

impl PartialEq for AssignedBinding {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl Deref for AssignedBinding {
    type Target = dyn BindingBase;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl From<Rc<dyn BindingBase>> for AssignedBinding {
    fn from(value: Rc<dyn BindingBase>) -> Self {
        Self(value)
    }
}
