//! Test services shared by the tests of the button controls.

#![allow(dead_code)]

use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
use ferroui_base::reactive::IDisposable;
use ferroui_base::FerroLocator;
use std::rc::Rc;

/// A locator scope with a keyboard device, so that elements can be focused.
pub struct FocusScope {
    scope: Rc<dyn IDisposable>,
    /// The keyboard device registered for the scope.
    pub keyboard: Rc<KeyboardDevice>,
}

/// Registers a keyboard device for the lifetime of the returned value.
pub fn focus_scope() -> FocusScope {
    let scope = FerroLocator::enter_scope();
    let keyboard = KeyboardDevice::new();
    FerroLocator::current_mutable().bind::<dyn IKeyboardDevice>().to_constant(keyboard.clone());
    FocusScope { scope, keyboard }
}

impl Drop for FocusScope {
    fn drop(&mut self) {
        self.scope.dispose();
    }
}
