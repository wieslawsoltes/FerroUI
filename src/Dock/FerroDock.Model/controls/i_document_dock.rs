// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::BoxedValue;
use ferroui_reactiveui::ICommand;
use std::rc::Rc;

/// The `IDocumentDock` docking contract.
pub trait IDocumentDock: IDock + ILocalTarget {
    /// Gets or sets if document dock can create new documents.
    fn can_create_document(&self) -> bool;
    /// Sets `CanCreateDocument`.
    fn set_can_create_document(&self, value: bool);
    /// Gets or sets command to create new document.
    fn create_document(&self) -> Option<Rc<dyn ICommand>>;
    /// Sets `CreateDocument`.
    fn set_create_document(&self, value: Option<Rc<dyn ICommand>>);
    /// Gets or sets if the window can be dragged by clicking on the tab strip.
    fn enable_window_drag(&self) -> bool;
    /// Sets `EnableWindowDrag`.
    fn set_enable_window_drag(&self, value: bool);
    /// Gets or sets document layout mode.
    fn layout_mode(&self) -> DocumentLayoutMode;
    /// Sets `LayoutMode`.
    fn set_layout_mode(&self, value: DocumentLayoutMode);
    /// Gets or sets document tabs layout.
    fn tabs_layout(&self) -> DocumentTabLayout;
    /// Sets `TabsLayout`.
    fn set_tabs_layout(&self, value: DocumentTabLayout);
    /// Gets or sets when document close buttons are displayed.
    fn close_button_show_mode(&self) -> DocumentCloseButtonShowMode;
    /// Sets `CloseButtonShowMode`.
    fn set_close_button_show_mode(&self, value: DocumentCloseButtonShowMode);
    /// Gets or sets placeholder content shown when the document host has no visible dockables.
    fn empty_content(&self) -> Option<BoxedValue>;
    /// Sets `EmptyContent`.
    fn set_empty_content(&self, value: Option<BoxedValue>);
    /// Gets or sets command to cascade MDI documents.
    fn cascade_documents(&self) -> Option<Rc<dyn ICommand>>;
    /// Sets `CascadeDocuments`.
    fn set_cascade_documents(&self, value: Option<Rc<dyn ICommand>>);
    /// Gets or sets command to tile MDI documents horizontally.
    fn tile_documents_horizontal(&self) -> Option<Rc<dyn ICommand>>;
    /// Sets `TileDocumentsHorizontal`.
    fn set_tile_documents_horizontal(&self, value: Option<Rc<dyn ICommand>>);
    /// Gets or sets command to tile MDI documents vertically.
    fn tile_documents_vertical(&self) -> Option<Rc<dyn ICommand>>;
    /// Sets `TileDocumentsVertical`.
    fn set_tile_documents_vertical(&self, value: Option<Rc<dyn ICommand>>);
    /// Gets or sets command to restore MDI documents to normal state.
    fn restore_documents(&self) -> Option<Rc<dyn ICommand>>;
    /// Sets `RestoreDocuments`.
    fn set_restore_documents(&self, value: Option<Rc<dyn ICommand>>);
    /// Adds the specified document to this dock and activates it.
    fn add_document(&self, document: Option<Rc<dyn IDockable>>);
    /// Adds the specified tool to this dock and activates it.
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>);
}

impl PartialEq for dyn IDocumentDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
