//! Port of `Debugging/BreakAction.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that triggers a debugger break.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference calls `Debugger.Break()`. Safe, portable
/// Rust has no call that breaks into an attached debugger, and the framework has no service for
/// it; the action does not stop the program. It still returns `true` when it is enabled, so a
/// breakpoint set on [`BreakAction::execute`](StyledElementActionImpl::execute) stops where the
/// reference breaks.
#[repr(C)]
pub struct BreakAction {
    base: StyledElementAction,
}

ferro_class!(BreakAction: StyledElementAction);
ferro_class_info!(BreakAction { new: BreakAction::new });
ferroui_base::ferro_impl_classes!(BreakAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for BreakAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        // Deviation (DEVIATIONS.md, Behaviors): no debugger break (see the type).
        Some(Rc::new(true))
    }
}

impl BreakAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
