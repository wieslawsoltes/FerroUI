//! Port of `InputElement/Actions/ReleasePointerCaptureAction.cs`.

use super::super::pointer_event_args_of;
use ferroui_base::input::InputElement;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};

/// Releases the pointer capture.
#[repr(C)]
pub struct ReleasePointerCaptureAction {
    base: StyledElementAction,
}

ferro_class!(ReleasePointerCaptureAction: StyledElementAction);
ferro_class_info!(ReleasePointerCaptureAction { new: ReleasePointerCaptureAction::new });
ferroui_base::ferro_impl_classes!(ReleasePointerCaptureAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for ReleasePointerCaptureAction {
    /// Executes the action: the pointer of the pointer event args given as the parameter releases
    /// its capture.
    ///
    /// Returns null after executed.
    fn execute(_this: &Self, _sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let pointer_event_args = pointer_event_args_of(parameter)?;

        pointer_event_args.source().and_then(|source| source.cast::<InputElement>())?;

        pointer_event_args.pointer().capture(None);

        None
    }
}

impl ReleasePointerCaptureAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
