//! `Primitives/IFinalMeasureSelector.cs`: lets a presenter decide which of its elements need the
//! final pass of its two-pass measure.

use ferroui_base::Ref;
use ferroui_controls::Control;

/// Selects the elements of a presenter that are measured again in the final measure pass
/// (`IFinalMeasureSelector`).
pub trait IFinalMeasureSelector {
    /// Whether `element`, at `index`, needs the final measure pass.
    fn needs_final_measure(&self, element: &Ref<Control>, index: i32) -> bool;
}
