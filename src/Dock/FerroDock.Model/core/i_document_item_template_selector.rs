// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::core::IItemsSourceDock;
use crate::BoxedValue;
use std::rc::Rc;

/// Selects generated document template content; None retains dock defaults.
pub trait IDocumentItemTemplateSelector {
    /// Selects content for the item at the source index, or -1 when unknown.
    fn select_template(
        &self,
        dock: Rc<dyn IItemsSourceDock>,
        item: BoxedValue,
        index: i32,
    ) -> Option<BoxedValue>;
}
impl PartialEq for dyn IDocumentItemTemplateSelector {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
