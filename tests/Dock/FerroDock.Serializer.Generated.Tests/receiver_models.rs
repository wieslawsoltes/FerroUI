//! Identity-preserving native view of the original fixture model, with no copied state.
use crate::payload_models::Payload;
use ferrodock_model::core::SerializationResult;
use std::rc::Rc;

/// Native metadata receiver retaining the original enclosing payload.
#[derive(Clone)]
pub struct PayloadView {
    model: Rc<Payload>,
}
impl PartialEq for PayloadView {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model)
    }
}
impl PayloadView {
    /// Invokes the original model constructor exactly once.
    pub fn create() -> Self {
        Self {
            model: Payload::new(),
        }
    }
    /// Gets a view of the same current model without copying its fields.
    pub fn from_model(model: &Rc<Payload>) -> Self {
        Self {
            model: model.clone(),
        }
    }
    /// Extracts the exact handle produced by the original constructor.
    pub fn into_model(self) -> Rc<Payload> {
        self.model
    }
    /// Gets the original model identity, independent of this view's address.
    pub fn identity(value: &Self) -> usize {
        Rc::as_ptr(&value.model) as usize
    }
    /// Calls the original typed name getter.
    pub fn name(value: &Self) -> SerializationResult<Option<String>> {
        Payload::try_name(&value.model)
    }
    /// Calls the original typed name setter.
    pub fn set_name(value: &Self, name: Option<String>) -> SerializationResult<()> {
        Payload::try_set_name(&value.model, name)
    }
    /// Gets the original self-reference, preserving its alias.
    pub fn own(value: &Self) -> Option<Rc<Payload>> {
        Payload::own(&value.model)
    }
    /// Sets the original self-reference through its typed method.
    pub fn set_own(value: &Self, own: Option<Rc<Payload>>) -> SerializationResult<()> {
        Payload::try_set_own(&value.model, own)
    }
}
