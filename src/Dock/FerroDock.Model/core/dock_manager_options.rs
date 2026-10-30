// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use std::cell::Cell;
use std::rc::Rc;

/// Dock manager options shared across docking surfaces.
#[derive(Debug)]
pub struct DockManagerOptions {
    is_docking_enabled: Cell<bool>,
}

impl Default for DockManagerOptions {
    fn default() -> Self {
        Self {
            is_docking_enabled: Cell::new(true),
        }
    }
}

impl DockManagerOptions {
    /// Creates shared options with docking enabled.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Returns whether docking interactions are enabled.
    pub fn is_docking_enabled(&self) -> bool {
        self.is_docking_enabled.get()
    }

    /// Sets whether docking interactions are enabled.
    pub fn set_is_docking_enabled(&self, value: bool) {
        self.is_docking_enabled.set(value);
    }
}

impl PartialEq for DockManagerOptions {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docking_is_enabled_by_default_and_updates_across_surfaces() {
        let options = DockManagerOptions::new();
        let second_surface = options.clone();
        assert!(options.is_docking_enabled());
        second_surface.set_is_docking_enabled(false);
        assert!(!options.is_docking_enabled());
        options.set_is_docking_enabled(true);
        assert!(second_surface.is_docking_enabled());
        assert_eq!(options, second_surface);
        assert_ne!(options, DockManagerOptions::new());
    }
}
