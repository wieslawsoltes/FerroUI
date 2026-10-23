//! Port of `ViewModels/SettingsViewModel.cs`.

use super::{identity_eq, ViewModelBase};
use std::cell::Cell;

/// The settings page: three switches.
pub struct SettingsViewModel {
    use_dark_theme: Cell<bool>,
    enable_animations: Cell<bool>,
    receive_release_notifications: Cell<bool>,
}

identity_eq!(SettingsViewModel);

impl SettingsViewModel {
    pub fn new() -> Self {
        Self { use_dark_theme: Cell::new(true), enable_animations: Cell::new(true), receive_release_notifications: Cell::new(true) }
    }

    pub fn use_dark_theme(&self) -> bool {
        self.use_dark_theme.get()
    }

    pub fn set_use_dark_theme(&self, value: bool) {
        self.use_dark_theme.set(value);
    }

    pub fn enable_animations(&self) -> bool {
        self.enable_animations.get()
    }

    pub fn set_enable_animations(&self, value: bool) {
        self.enable_animations.set(value);
    }

    pub fn receive_release_notifications(&self) -> bool {
        self.receive_release_notifications.get()
    }

    pub fn set_receive_release_notifications(&self, value: bool) {
        self.receive_release_notifications.set(value);
    }
}

impl Default for SettingsViewModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewModelBase for SettingsViewModel {
    fn title(&self) -> String {
        "Settings".to_string()
    }
}
