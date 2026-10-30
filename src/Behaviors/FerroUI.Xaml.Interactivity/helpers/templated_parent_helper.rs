//! Port of `Helpers/TemplatedParentHelper.cs`.

use ferroui_base::{FerroObject, Ref, StyledElement};

/// Sets the templated parent of a styled element.
///
/// The reference reaches the non-public setter of the property through reflection; the framework
/// exposes it ([`StyledElement::set_templated_parent`]), so this is a direct call.
pub struct TemplatedParentHelper;

impl TemplatedParentHelper {
    /// Sets the templated parent of `target`.
    pub fn set_templated_parent(target: &StyledElement, value: Option<Ref<FerroObject>>) {
        target.set_templated_parent(value);
    }
}
