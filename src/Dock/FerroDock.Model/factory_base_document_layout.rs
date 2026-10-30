// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::core::*;
use std::rc::Rc;

impl FactoryBase {
    /// Sets tabs layout only when the supplied item is a document dock.
    pub fn set_document_dock_tabs_layout(
        &self,
        dockable: Rc<dyn IDockable>,
        layout: DocumentTabLayout,
    ) {
        if let Some(dock) = dockable.as_document_dock() {
            dock.set_tabs_layout(layout);
        }
    }
    /// Dispatches the virtual tabs layout setter with Left.
    pub fn set_document_dock_tabs_layout_left(&self, dockable: Rc<dyn IDockable>) {
        self.runtime()
            .set_document_dock_tabs_layout(dockable, DocumentTabLayout::Left);
    }
    /// Dispatches the virtual tabs layout setter with Top.
    pub fn set_document_dock_tabs_layout_top(&self, dockable: Rc<dyn IDockable>) {
        self.runtime()
            .set_document_dock_tabs_layout(dockable, DocumentTabLayout::Top);
    }
    /// Dispatches the virtual tabs layout setter with Right.
    pub fn set_document_dock_tabs_layout_right(&self, dockable: Rc<dyn IDockable>) {
        self.runtime()
            .set_document_dock_tabs_layout(dockable, DocumentTabLayout::Right);
    }
    /// Sets layout mode only when the supplied item is a document dock.
    pub fn set_document_dock_layout_mode(
        &self,
        dockable: Rc<dyn IDockable>,
        mode: DocumentLayoutMode,
    ) {
        if let Some(dock) = dockable.as_document_dock() {
            dock.set_layout_mode(mode);
        }
    }
    /// Dispatches the virtual layout mode setter with Tabbed.
    pub fn set_document_dock_layout_mode_tabbed(&self, dockable: Rc<dyn IDockable>) {
        self.runtime()
            .set_document_dock_layout_mode(dockable, DocumentLayoutMode::Tabbed);
    }
    /// Dispatches the virtual layout mode setter with Mdi.
    pub fn set_document_dock_layout_mode_mdi(&self, dockable: Rc<dyn IDockable>) {
        self.runtime()
            .set_document_dock_layout_mode(dockable, DocumentLayoutMode::Mdi);
    }
}
