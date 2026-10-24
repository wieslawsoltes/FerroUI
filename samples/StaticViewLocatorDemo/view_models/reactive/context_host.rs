//! Port of `ViewModels/ContextHost.cs`: an object that holds a view model, shown by the fallback
//! hook of the locator.

use super::SettingsViewModel;
use std::rc::Rc;

/// A view model wrapped in another object, which has no view of its own.
pub struct ContextHost {
    context: Rc<SettingsViewModel>,
}

impl PartialEq for ContextHost {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ContextHost {
    pub fn new(context: Rc<SettingsViewModel>) -> Self {
        Self { context }
    }

    /// The view model the host holds (`Context`).
    pub fn context(&self) -> &Rc<SettingsViewModel> {
        &self.context
    }
}
