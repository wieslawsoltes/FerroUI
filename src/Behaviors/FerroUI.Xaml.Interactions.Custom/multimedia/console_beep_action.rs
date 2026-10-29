//! Port of `Multimedia/ConsoleBeepAction.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::io::Write;
use std::rc::Rc;

/// An action that plays a system beep.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference calls `Console.Beep()`, a basic beep as a
/// fallback since the framework has no audio API; the framework has no beep service either, so
/// the port writes the bell character to the standard output, which is what the console beep of
/// the reference does on a terminal outside Windows. The action returns whether it was written.
#[repr(C)]
pub struct ConsoleBeepAction {
    base: StyledElementAction,
}

ferro_class!(ConsoleBeepAction: StyledElementAction);
ferro_class_info!(ConsoleBeepAction { new: ConsoleBeepAction::new });
ferroui_base::ferro_impl_classes!(ConsoleBeepAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for ConsoleBeepAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        // Deviation (DEVIATIONS.md, Behaviors): the bell character on the standard output stands
        // for `Console.Beep()`; a failure to write it is the exception the reference catches.
        let mut stdout = std::io::stdout();
        let beeped = stdout.write_all(b"\x07").and_then(|()| stdout.flush()).is_ok();
        Some(Rc::new(beeped))
    }
}

impl ConsoleBeepAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
