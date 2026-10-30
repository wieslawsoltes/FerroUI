// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::BoxedValue;
use std::rc::Rc;

/// Creates and manages ItemsSource-generated document/tool containers.
pub trait IDockItemContainerGenerator {
    /// Creates a document container or skips the item; -1 denotes an unknown index.
    fn create_document_container(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        item: BoxedValue,
        index: i32,
    ) -> Option<Rc<dyn IDockable>>;
    /// Prepares a generated document container before layout insertion.
    fn prepare_document_container(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: BoxedValue,
        index: i32,
    );
    /// Clears a document container, with its source item when known.
    fn clear_document_container(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: Option<BoxedValue>,
    );
    /// Creates a tool container or skips the item; -1 denotes an unknown index.
    fn create_tool_container(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        item: BoxedValue,
        index: i32,
    ) -> Option<Rc<dyn IDockable>>;
    /// Prepares a generated tool container before layout insertion.
    fn prepare_tool_container(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: BoxedValue,
        index: i32,
    );
    /// Clears a tool container, with its source item when known.
    fn clear_tool_container(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        container: Rc<dyn IDockable>,
        item: Option<BoxedValue>,
    );
}
impl PartialEq for dyn IDockItemContainerGenerator {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
