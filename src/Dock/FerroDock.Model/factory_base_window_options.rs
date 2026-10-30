// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Applies optional window options through the source options object.
    pub fn apply_window_options(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        if let Some(options) = options {
            options.apply_to(window.as_deref());
        }
    }

    /// Captures the source root's window for DockableWindow ownership before movement.
    pub fn prepare_window_options_for_dockable(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        let Some(options) = options else { return };
        if options.owner_mode() != DockWindowOwnerMode::DockableWindow
            || options.parent_window().is_some()
        {
            return;
        }
        if let Some(window) = self
            .runtime()
            .find_root(dockable, None)
            .and_then(|root| root.window())
        {
            options.set_parent_window(Some(window));
        }
    }
}
