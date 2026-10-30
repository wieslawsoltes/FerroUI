// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::core::IToolItemsSourceDock;
use crate::BoxedValue;
use std::rc::Rc;

/// Selects generated tool template content; None retains dock defaults.
pub trait IToolItemTemplateSelector {
    /// Selects content for the item at the source index, or -1 when unknown.
    fn select_template(
        &self,
        dock: Rc<dyn IToolItemsSourceDock>,
        item: BoxedValue,
        index: i32,
    ) -> Option<BoxedValue>;
}
impl PartialEq for dyn IToolItemTemplateSelector {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
