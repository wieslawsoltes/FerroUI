//! Port of `Core/InvokeCommandAction.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{InvokeCommandActionBase, StyledElementActionImpl};
use std::rc::Rc;

/// Executes a specified command (`ICommand`) when invoked.
#[repr(C)]
pub struct InvokeCommandAction {
    base: InvokeCommandActionBase,
}

ferro_class!(InvokeCommandAction: InvokeCommandActionBase);
ferro_class_info!(InvokeCommandAction { new: InvokeCommandAction::new });
ferroui_base::ferro_impl_classes!(InvokeCommandAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for InvokeCommandAction {
    /// Executes the action.
    ///
    /// `sender` is the object that is passed to the action by the behavior. Generally this is the
    /// associated object of the behavior or a target object. The value of `parameter` is
    /// determined by the caller.
    ///
    /// Returns true if the command is successfully executed; else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let command = this.command();
        let Some(command) = command.filter(|_| this.is_enabled()) else {
            return Some(Rc::new(false));
        };

        let resolved_parameter = this.resolve_parameter(parameter);

        if !command.can_execute(resolved_parameter.as_ref()) {
            return Some(Rc::new(false));
        }

        command.execute(resolved_parameter.as_ref());

        Some(Rc::new(true))
    }
}

impl InvokeCommandAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InvokeCommandActionBase::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
