mod keyboard_settings;
mod settings_enums;
pub use keyboard_settings::*;
pub use settings_enums::*;

// Source KeyboardSettings is a reference class with Object.Equals identity, not value equality.
impl PartialEq for KeyboardSettings {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for KeyboardSettings {}
