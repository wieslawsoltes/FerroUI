//! Scoped ownership of FerroUI's real platform input-method enablement property.
use ferroui_base::{
    data::BindingPriority,
    input::{InputElement, InputMethod},
    reactive::IDisposable,
};
use std::rc::Rc;

/// Disable the platform IME while a custom keyboard owns a native editing target.
/// Dropping the guard removes only its priority value: existing bindings, local values
/// and other owners remain intact. This also suppresses mobile system keyboards.
/// Native raw key/text events and direct editor operations remain available; platform
/// IME composition is disabled for this target until all its guards are released.
pub struct SystemInputMethodGuard {
    value: Rc<dyn IDisposable>,
}
impl SystemInputMethodGuard {
    pub fn new(target: &InputElement) -> Self {
        let value = target
            .set_value_with_priority(
                InputMethod::is_input_method_enabled_property(),
                false,
                BindingPriority::Animation,
            )
            .expect("non-local native input-method override must have a removable handle");
        Self { value }
    }
}
impl Drop for SystemInputMethodGuard {
    fn drop(&mut self) {
        self.value.dispose();
    }
}
