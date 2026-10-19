//! `DispatchStubControl` (`TestModels/DispatchStubControl.cs`).

use ferroui_system::events::Event;
use std::cell::RefCell;

/// A control a command binding names, carrying a property to bind against and the default event
/// (`DispatchStubControl`).
pub struct DispatchStubControl {
    click: Event<()>,
    text: RefCell<String>,
}

#[allow(dead_code)]
impl DispatchStubControl {
    /// The control with the text `"a"`.
    pub fn new() -> Self {
        Self { click: Event::new(), text: RefCell::new("a".to_string()) }
    }

    /// Raised to execute a bound command (`Click`): the event the default command binder looks
    /// for.
    pub fn click(&self) -> &Event<()> {
        &self.click
    }

    /// The control's text (`Text`).
    pub fn text(&self) -> String {
        self.text.borrow().clone()
    }

    /// Sets the control's text.
    pub fn set_text(&self, value: String) {
        *self.text.borrow_mut() = value;
    }

    /// Raises `Click`, executing whichever command is bound to it (`PerformClick`).
    pub fn perform_click(&self) {
        self.click.raise(&());
    }
}

impl Default for DispatchStubControl {
    fn default() -> Self {
        Self::new()
    }
}

/// Controls compare by identity, as .NET compares references.
impl PartialEq for DispatchStubControl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
