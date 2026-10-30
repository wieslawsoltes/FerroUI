// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::pin::is_valid_size;
use super::FactoryBase;
use crate::core::*;
use crate::DockCapabilityResolver;
use std::rc::Rc;

const DEFAULT_WIDTH: f64 = 300.0;
const DEFAULT_HEIGHT: f64 = 400.0;
const MIN_TRACKED_SIZE: f64 = 16.0;

fn floating_size(item: f64, owner: f64, fallback: f64) -> f64 {
    if is_valid_size(item) && item > MIN_TRACKED_SIZE {
        item
    } else if is_valid_size(owner) && owner > MIN_TRACKED_SIZE {
        owner
    } else {
        fallback
    }
}

impl FactoryBase {
    /// Dispatches floating with no window options.
    pub fn float_dockable(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.runtime().float_dockable_with_options(dockable, None);
    }

    /// Floats one dockable using source pointer/bounds fallback and capability policy.
    pub fn float_dockable_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        if !DockCapabilityResolver::is_enabled(
            dockable.clone(),
            DockCapability::Float,
            DockCapabilityResolver::resolve_operation_dock(dockable.clone()),
        ) {
            return;
        }
        let dockable = dockable.expect("source floating dockable.Owner access");
        let Some(dock) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        self.unpin_dockable(Some(dockable.clone()));
        let (owner_pointer_x, owner_pointer_y) = dock.get_pointer_screen_position();
        let (mut x, mut y) = dockable.get_pointer_screen_position();
        if x.is_nan() {
            x = owner_pointer_x;
        }
        if y.is_nan() {
            y = owner_pointer_y;
        }
        let (owner_x, owner_y, owner_width, owner_height) = dock.get_visible_bounds();
        let (item_x, item_y, item_width, item_height) = dockable.get_visible_bounds();
        if x.is_nan() {
            x = if !item_x.is_nan() {
                item_x
            } else if !owner_x.is_nan() {
                owner_x
            } else {
                0.0
            };
        }
        if y.is_nan() {
            y = if !item_y.is_nan() {
                item_y
            } else if !owner_y.is_nan() {
                owner_y
            } else {
                0.0
            };
        }
        let width = floating_size(item_width, owner_width, DEFAULT_WIDTH);
        let height = floating_size(item_height, owner_height, DEFAULT_HEIGHT);
        self.hooks()
            .prepare_window_options_for_dockable(Some(dockable.clone()), options.clone());
        self.runtime().split_to_window_with_options(
            Some(dock),
            Some(dockable),
            x,
            y,
            width,
            height,
            options,
        );
    }

    /// Dispatches whole-pane floating with no window options.
    pub fn float_all_dockables(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.runtime()
            .float_all_dockables_with_options(dockable, None);
    }

    /// Floats a snapshot of all visible siblings, preserving source container metadata and collapse policy.
    pub fn float_all_dockables_with_options(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        if !DockCapabilityResolver::is_enabled(
            dockable.clone(),
            DockCapability::Float,
            DockCapabilityResolver::resolve_operation_dock(dockable.clone()),
        ) {
            return;
        }
        let dockable = dockable.expect("source floating dockable.Owner access");
        let Some(dock) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        if dock.visible_dockables().is_none() {
            return;
        }
        let Some(source_root) = self
            .runtime()
            .find_root(Some(dock.clone()), Some(Rc::new(|_| true)))
        else {
            return;
        };
        let root = self
            .hooks()
            .resolve_window_collection_root(Some(source_root));
        self.hooks()
            .prepare_window_options_for_dockable(Some(dockable), options.clone());
        let (mut x, mut y) = dock.get_pointer_screen_position();
        let (owner_x, owner_y, owner_width, owner_height) = dock.get_visible_bounds();
        if x.is_nan() {
            x = if !owner_x.is_nan() { owner_x } else { 0.0 };
        }
        if y.is_nan() {
            y = if !owner_y.is_nan() { owner_y } else { 0.0 };
        }
        let width = floating_size(f64::NAN, owner_width, DEFAULT_WIDTH);
        let height = floating_size(f64::NAN, owner_height, DEFAULT_HEIGHT);
        let target: Rc<dyn IDock> = if (dock.clone() as Rc<dyn IDockable>)
            .as_document_dock()
            .is_some()
        {
            self.runtime().create_document_dock()
        } else if (dock.clone() as Rc<dyn IDockable>).as_tool_dock().is_some() {
            self.runtime().create_tool_dock()
        } else {
            self.runtime().create_dock_dock()
        };
        target.set_title(dock.title());
        target.set_id(dock.id());
        target.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        if let Some(source) = (dock.clone() as Rc<dyn IDockable>).as_document_dock() {
            if let Some(target) = (target.clone() as Rc<dyn IDockable>).as_document_dock() {
                target.set_can_create_document(source.can_create_document());
                target.set_enable_window_drag(source.enable_window_drag());
                Self::copy_document_extras(source, target);
            }
        }
        if let Some(source) = (dock.clone() as Rc<dyn IDockable>).as_tool_dock() {
            if let Some(target) = (target.clone() as Rc<dyn IDockable>).as_tool_dock() {
                target.set_alignment(source.alignment());
                target.set_is_expanded(source.is_expanded());
                target.set_auto_hide(source.auto_hide());
                target.set_grip_mode(source.grip_mode());
                Self::copy_tool_extras(source, target);
            }
        }
        for item in dock
            .visible_dockables()
            .expect("source visible list exists")
            .to_vec()
        {
            self.runtime()
                .remove_dockable(item.clone(), dock.is_collapsable());
            self.runtime()
                .add_dockable(Some(target.clone()), item.clone());
            self.runtime().on_dockable_moved(item.clone());
            target.set_active_dockable(item);
        }
        if let Some(window) = self
            .runtime()
            .create_window_from_with_options(Some(target.clone()), options)
        {
            self.runtime().add_window(root, Some(window.clone()));
            window.set_x(x);
            window.set_y(y);
            window.set_width(width);
            window.set_height(height);
            window.present(window.is_modal());
            if window.layout().is_some() {
                self.runtime().set_focused_dockable(
                    window.layout().map(|layout| layout as Rc<dyn IDock>),
                    target.active_dockable(),
                );
            }
        }
    }

    /// Moves the dockable into the first document dock when the source capability permits it.
    pub fn dock_as_document(&self, dockable: Rc<dyn IDockable>) {
        if !DockCapabilityResolver::is_enabled(
            Some(dockable.clone()),
            DockCapability::DockAsDocument,
            DockCapabilityResolver::resolve_operation_dock(Some(dockable.clone())),
        ) {
            return;
        }
        let Some(source) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        let Some(root) = self
            .runtime()
            .find_root(Some(source.clone()), Some(Rc::new(|_| true)))
        else {
            return;
        };
        let Some(target) = self
            .runtime()
            .find_dockable(
                Some(root),
                Some(Rc::new(|item| {
                    item.and_then(|item| item.as_document_dock()).is_some()
                })),
            )
            .and_then(|item| item.as_dock())
        else {
            return;
        };
        let last = target.visible_dockables().and_then(|items| {
            let count = items.count();
            (count > 0).then(|| items.get(count - 1)).flatten()
        });
        self.runtime().move_dockable_between_docks(
            Some(source),
            Some(target),
            Some(dockable),
            last,
        );
    }

    /// Creates a document dock to the right, copies source document metadata and moves the item into it.
    pub fn new_horizontal_document_dock(&self, dockable: Rc<dyn IDockable>) {
        self.new_document_dock(dockable, DockOperation::Right);
    }
    /// Creates a document dock below, copies source document metadata and moves the item into it.
    pub fn new_vertical_document_dock(&self, dockable: Rc<dyn IDockable>) {
        self.new_document_dock(dockable, DockOperation::Bottom);
    }

    fn new_document_dock(&self, dockable: Rc<dyn IDockable>, operation: DockOperation) {
        let Some(dock) = dockable.owner().and_then(|owner| owner.as_dock()) else {
            return;
        };
        let target = self.runtime().create_document_dock();
        target.set_title("IDocumentDock".into());
        target.set_visible_dockables(Some((self.create_dockable_list)(Vec::new())));
        if let Some(source) = (dock.clone() as Rc<dyn IDockable>).as_document_dock() {
            target.set_id(source.id());
            target.set_can_create_document(source.can_create_document());
            target.set_enable_window_drag(source.enable_window_drag());
            target.set_layout_mode(source.layout_mode());
            Self::copy_document_extras(source, target.clone());
        }
        self.runtime()
            .split_to_dock(dock.clone(), target.clone(), operation);
        self.runtime()
            .move_dockable_between_docks(Some(dock), Some(target), Some(dockable), None);
    }
}
