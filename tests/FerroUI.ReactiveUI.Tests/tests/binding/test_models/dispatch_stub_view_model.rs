//! `DispatchStubViewModel` (`TestModels/DispatchStubViewModel.cs`).
//!
//! `Signal` is left out: only a test of a refused call (not ported, expression trees) names it.
//! `Run` and `Confirm`, plain properties upstream, raise `PropertyChanged` here as the other
//! properties do; the tests set them before they bind. Upstream's `Confirm` starts as `null!`, an
//! `Option` here.

use ferroui_reactiveui::{ICommand, Interaction};
use std::rc::Rc;

notifying_model! {
    /// A view model whose setters raise `PropertyChanged` on every write (`DispatchStubViewModel`).
    pub struct DispatchStubViewModel (changing: false, check_equality: false) {
        caption / set_caption: String = "a".to_string(),
        parameter / set_parameter: String = "a".to_string(),
        run / set_run: Option<Rc<dyn ICommand>> = None,
        confirm / set_confirm: Option<Interaction<String, bool>> = None,
    }
}
