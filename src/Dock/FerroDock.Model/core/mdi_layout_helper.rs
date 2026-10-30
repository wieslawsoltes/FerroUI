// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::{IDocumentDock, IMdiDocument};
use crate::core::{DockRect, MdiLayoutDefaults, MdiWindowState};
use std::rc::Rc;

/// The source algorithms for arranging MDI documents inside a document dock.
pub struct MdiLayoutHelper;

impl MdiLayoutHelper {
    /// Cascades nonminimized documents within the dock's layout bounds.
    pub fn cascade_documents(dock: &dyn IDocumentDock) {
        let Some(bounds) = Self::try_get_layout_bounds(dock) else {
            return;
        };
        let documents = Self::get_ordered_documents(dock);
        let normal_documents = Self::get_nonminimized_documents(&documents);
        if normal_documents.is_empty() {
            return;
        }
        let mut offset = MdiLayoutDefaults::CASCADE_OFFSET;
        let max_offset_x = Self::get_max_offset(
            bounds.width,
            normal_documents.len(),
            MdiLayoutDefaults::MINIMUM_WIDTH,
        );
        let max_offset_y = Self::get_max_offset(
            bounds.height,
            normal_documents.len(),
            MdiLayoutDefaults::MINIMUM_HEIGHT,
        );
        let max_offset = 0.0_f64.max(max_offset_x.min(max_offset_y));
        if max_offset > 0.0 && offset > max_offset {
            offset = max_offset;
        }
        let count_minus_one = (normal_documents.len() - 1) as f64;
        let width = (bounds.width * MdiLayoutDefaults::DEFAULT_WIDTH_RATIO)
            .max(bounds.width - offset * count_minus_one);
        let height = (bounds.height * MdiLayoutDefaults::DEFAULT_HEIGHT_RATIO)
            .max(bounds.height - offset * count_minus_one);
        let width = Self::clamp(width, MdiLayoutDefaults::MINIMUM_WIDTH, bounds.width);
        let height = Self::clamp(height, MdiLayoutDefaults::MINIMUM_HEIGHT, bounds.height);
        for (index, document) in normal_documents.iter().enumerate() {
            document.set_mdi_state(MdiWindowState::Normal);
            document.set_mdi_bounds(DockRect::new(
                bounds.x + offset * index as f64,
                bounds.y + offset * index as f64,
                width,
                height,
            ));
        }
        Self::update_z_order(&documents);
    }

    /// Tiles nonminimized documents from top to bottom.
    pub fn tile_documents_horizontal(dock: &dyn IDocumentDock) {
        let Some(bounds) = Self::try_get_layout_bounds(dock) else {
            return;
        };
        let documents = Self::get_ordered_documents(dock);
        let normal_documents = Self::get_nonminimized_documents(&documents);
        if normal_documents.is_empty() {
            return;
        }
        let height = bounds.height / normal_documents.len() as f64;
        for (index, document) in normal_documents.iter().enumerate() {
            document.set_mdi_state(MdiWindowState::Normal);
            document.set_mdi_bounds(DockRect::new(
                bounds.x,
                bounds.y + height * index as f64,
                bounds.width,
                height,
            ));
        }
        Self::update_z_order(&documents);
    }

    /// Tiles nonminimized documents from left to right.
    pub fn tile_documents_vertical(dock: &dyn IDocumentDock) {
        let Some(bounds) = Self::try_get_layout_bounds(dock) else {
            return;
        };
        let documents = Self::get_ordered_documents(dock);
        let normal_documents = Self::get_nonminimized_documents(&documents);
        if normal_documents.is_empty() {
            return;
        }
        let width = bounds.width / normal_documents.len() as f64;
        for (index, document) in normal_documents.iter().enumerate() {
            document.set_mdi_state(MdiWindowState::Normal);
            document.set_mdi_bounds(DockRect::new(
                bounds.x + width * index as f64,
                bounds.y,
                width,
                bounds.height,
            ));
        }
        Self::update_z_order(&documents);
    }

    /// Restores every MDI document, then puts the active document last in Z order.
    pub fn restore_documents(dock: &dyn IDocumentDock) {
        let documents = Self::get_ordered_documents(dock);
        for document in &documents {
            document.set_mdi_state(MdiWindowState::Normal);
        }
        Self::update_z_order(&documents);
    }

    fn try_get_layout_bounds(dock: &dyn IDocumentDock) -> Option<DockRect> {
        let (_, _, width, height) = dock.get_visible_bounds();
        if width.is_nan() || height.is_nan() || width <= 0.0 || height <= 0.0 {
            return None;
        }
        let mut minimized_count = 0;
        if let Some(visible) = dock.visible_dockables() {
            for dockable in visible.iter() {
                if let Some(document) = dockable.and_then(|item| item.as_mdi_document()) {
                    if document.mdi_state() == MdiWindowState::Minimized {
                        minimized_count += 1;
                    }
                }
            }
        }
        let reserved_height = if minimized_count > 0 {
            MdiLayoutDefaults::MINIMIZED_HEIGHT + MdiLayoutDefaults::MINIMIZED_SPACING
        } else {
            0.0
        };
        Some(DockRect::new(
            0.0,
            0.0,
            width,
            0.0_f64.max(height - reserved_height),
        ))
    }

    fn get_ordered_documents(dock: &dyn IDocumentDock) -> Vec<Rc<dyn IMdiDocument>> {
        let mut documents = Vec::new();
        if let Some(visible) = dock.visible_dockables() {
            for dockable in visible.to_vec() {
                if let Some(document) = dockable.and_then(|item| item.as_mdi_document()) {
                    documents.push(document);
                }
            }
        }
        if let Some(active) = dock
            .active_dockable()
            .and_then(|active| active.as_mdi_document())
        {
            if let Some(index) = documents
                .iter()
                .position(|document| Rc::ptr_eq(document, &active))
            {
                documents.remove(index);
            }
            documents.push(active);
        }
        documents
    }

    fn get_nonminimized_documents(documents: &[Rc<dyn IMdiDocument>]) -> Vec<Rc<dyn IMdiDocument>> {
        let mut normal = Vec::with_capacity(documents.len());
        for document in documents {
            if document.mdi_state() != MdiWindowState::Minimized {
                normal.push(document.clone());
            }
        }
        normal
    }

    fn update_z_order(documents: &[Rc<dyn IMdiDocument>]) {
        for (index, document) in documents.iter().enumerate() {
            document.set_mdi_z_index(index as i32);
        }
    }

    fn clamp(value: f64, min: f64, max: f64) -> f64 {
        if value < min {
            return min;
        }
        if value > max {
            return max;
        }
        value
    }

    fn get_max_offset(total: f64, count: usize, min: f64) -> f64 {
        if count <= 1 {
            return 0.0;
        }
        let available = total - min;
        if available <= 0.0 {
            return 0.0;
        }
        available / (count - 1) as f64
    }
}
