// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::collections::IEnumerable;
use crate::core::*;
use crate::BoxedValue;
use std::rc::Rc;

/// Supports ItemsSource-based document generation and its source metadata.
pub trait IItemsSourceDock {
    /// Gets the source collection.
    fn items_source(&self) -> Option<Rc<dyn IEnumerable>>;
    /// Gets the generated-container lifecycle implementation.
    fn item_container_generator(&self) -> Option<Rc<dyn IDockItemContainerGenerator>>;
    /// Gets the container theme instance or resource key.
    fn document_item_container_theme(&self) -> Option<BoxedValue>;
    /// Sets the container theme instance or resource key.
    fn set_document_item_container_theme(&self, value: Option<BoxedValue>);
    /// Gets the generated document template selector.
    fn document_item_template_selector(&self) -> Option<Rc<dyn IDocumentItemTemplateSelector>>;
    /// Sets the generated document template selector.
    fn set_document_item_template_selector(
        &self,
        value: Option<Rc<dyn IDocumentItemTemplateSelector>>,
    );
    /// Gets whether unregistering removes the source item; None uses global settings.
    fn can_update_items_source_on_unregister(&self) -> Option<bool>;
    /// Sets the unregister source-removal policy.
    fn set_can_update_items_source_on_unregister(&self, value: Option<bool>);
    /// Tests whether this dock generated the supplied document.
    fn is_document_from_items_source(&self, document: Rc<dyn IDockable>) -> bool;
    /// Removes the supplied nullable source item and reports success.
    fn remove_item_from_source(&self, item: Option<BoxedValue>) -> bool;
}
impl PartialEq for dyn IItemsSourceDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
