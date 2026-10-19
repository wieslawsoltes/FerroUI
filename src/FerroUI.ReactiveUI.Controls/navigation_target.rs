//! `NavigationTarget` (`NavigationTarget.cs`): a pending navigation target.

use crate::view_model_value::same_view_model_object;
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// A pending navigation target (`NavigationTarget`): the view model to display and the optional
/// view contract.
#[derive(Clone)]
pub struct NavigationTarget {
    /// The view model to display (`ViewModel`).
    pub view_model: Option<Rc<dyn Any>>,
    /// The optional view contract (`Contract`).
    pub contract: Option<String>,
}

impl NavigationTarget {
    /// A navigation target.
    pub fn new(view_model: Option<Rc<dyn Any>>, contract: Option<String>) -> Self {
        Self { view_model, contract }
    }
}

/// The equality of the record struct: the same view model object and the same contract.
impl PartialEq for NavigationTarget {
    fn eq(&self, other: &Self) -> bool {
        same_view_model_object(self.view_model.as_ref(), other.view_model.as_ref()) && self.contract == other.contract
    }
}

impl fmt::Debug for NavigationTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavigationTarget")
            .field("view_model", &self.view_model.as_ref().map(|view_model| Rc::as_ptr(view_model) as *const ()))
            .field("contract", &self.contract)
            .finish()
    }
}
