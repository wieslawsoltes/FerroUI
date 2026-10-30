// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Creates an uninitialized window/root for a supported dockable, preserving source classification priority.
    pub fn create_window_from(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        let dockable = dockable?;
        let target: Option<Rc<dyn IDockable>> = if dockable.clone().as_tool().is_some() {
            let target = self.runtime().create_tool_dock();
            target.set_title("IToolDock".into());
            Self::copy_dock_group(dockable.clone(), target.clone());
            target.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
            if let Some(source) = dockable.owner().and_then(|owner| owner.as_tool_dock()) {
                target.set_id(source.id());
                target.set_alignment(source.alignment());
                target.set_is_expanded(source.is_expanded());
                target.set_auto_hide(source.auto_hide());
                target.set_grip_mode(source.grip_mode());
                Self::copy_tool_extras(source, target.clone());
            }
            if target.visible_dockables().is_some() {
                self.add_visible_dockable(Some(target.clone()), Some(dockable.clone()));
                self.runtime().on_dockable_added(Some(dockable.clone()));
                target.set_active_dockable(Some(dockable.clone()));
            }
            Some(target)
        } else if dockable.clone().as_document().is_some() {
            let target = self.runtime().create_document_dock();
            target.set_title("IDocumentDock".into());
            Self::copy_dock_group(dockable.clone(), target.clone());
            target.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
            if let Some(source) = dockable.owner().and_then(|owner| owner.as_document_dock()) {
                target.set_id(source.id());
                target.set_can_create_document(source.can_create_document());
                target.set_enable_window_drag(source.enable_window_drag());
                Self::copy_document_extras(source, target.clone());
            }
            if target.visible_dockables().is_some() {
                self.add_visible_dockable(Some(target.clone()), Some(dockable.clone()));
                self.runtime().on_dockable_added(Some(dockable.clone()));
                target.set_active_dockable(Some(dockable.clone()));
            }
            Some(target)
        } else if dockable.clone().as_tool_dock().is_some()
            || dockable.clone().as_document_dock().is_some()
            || dockable.clone().as_proportional_dock().is_some()
            || dockable.clone().as_dock_dock().is_some()
        {
            Some(dockable.clone())
        } else {
            let root = dockable.clone().as_root_dock()?;
            root.active_dockable()
        };
        let current_root = self.runtime().find_root(Some(dockable), None);
        let root = self.runtime().create_root_dock();
        if let Some(current) = current_root {
            root.set_enable_adaptive_global_dock_targets(
                current.enable_adaptive_global_dock_targets(),
            );
        }
        root.set_title("IRootDock".into());
        root.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        if root.visible_dockables().is_some() {
            if let Some(target) = target {
                self.add_visible_dockable(Some(root.clone()), Some(target.clone()));
                self.runtime().on_dockable_added(Some(target.clone()));
                root.set_active_dockable(Some(target.clone()));
                root.set_default_dockable(Some(target));
            }
        }
        root.set_owner(None);
        let window = self.runtime().create_dock_window();
        window.set_id("IDockWindow".into());
        window.set_title(String::new());
        window.set_width(f64::NAN);
        window.set_height(f64::NAN);
        window.set_layout(Some(root.clone()));
        root.set_window(Some(window.clone()));
        Some(window)
    }

    /// Captures source ownership options before virtual creation and applies them afterward.
    pub fn create_window_from_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) -> Option<Rc<dyn IDockWindow>> {
        self.hooks()
            .prepare_window_options_for_dockable(dockable.clone(), options.clone());
        let window = self.runtime().create_window_from(dockable);
        if let Some(window) = &window {
            self.hooks()
                .apply_window_options(Some(window.clone()), options);
        }
        window
    }

    /// Dispatches the source coordinate overload with no window options.
    #[allow(clippy::too_many_arguments)]
    pub fn split_to_window(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) {
        self.runtime()
            .split_to_window_with_options(dock, dockable, x, y, width, height, None);
    }

    /// Removes, registers and presents a floating window in its outer workspace root, then requests focus.
    #[allow(clippy::too_many_arguments)]
    pub fn split_to_window_with_options(
        &self,
        dock: Option<Rc<dyn IDock>>,
        dockable: Option<Rc<dyn IDockable>>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        let Some(source_root) = self.runtime().find_root(
            dock.map(|dock| dock as Rc<dyn IDockable>),
            Some(Rc::new(|_| true)),
        ) else {
            return;
        };
        let root = self
            .hooks()
            .resolve_window_collection_root(Some(source_root));
        self.hooks()
            .prepare_window_options_for_dockable(dockable.clone(), options.clone());
        self.runtime().remove_dockable(dockable.clone(), true);
        self.runtime()
            .on_dockable_undocked(dockable.clone(), DockOperation::Window);
        if let Some(window) = self
            .runtime()
            .create_window_from_with_options(dockable.clone(), options)
        {
            self.runtime().add_window(root, Some(window.clone()));
            window.set_x(x);
            window.set_y(y);
            window.set_width(width);
            window.set_height(height);
            window.present(window.is_modal());
            self.runtime()
                .on_dockable_docked(dockable.clone(), DockOperation::Window);
            if window.layout().is_some() {
                self.runtime().set_focused_dockable(
                    window.layout().map(|layout| layout as Rc<dyn IDock>),
                    dockable,
                );
            }
        }
    }
}
