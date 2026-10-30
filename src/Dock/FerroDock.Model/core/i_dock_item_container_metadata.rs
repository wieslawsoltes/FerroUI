// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::BoxedValue;
/// Original optional presentation metadata for generated dockables.
pub trait IDockItemContainerMetadata {
    /// Resolves sibling dockable membership without reflection when implemented.
    fn as_dockable(self: std::rc::Rc<Self>) -> Option<std::rc::Rc<dyn super::IDockable>> {
        None
    }
    /// Gets the generated container theme instance or resource key.
    fn item_container_theme(&self) -> Option<BoxedValue>;
    /// Sets the generated container theme instance or resource key.
    fn set_item_container_theme(&self, value: Option<BoxedValue>);
    /// Gets the original template-selector metadata.
    fn item_template_selector(&self) -> Option<BoxedValue>;
    /// Sets the original template-selector metadata.
    fn set_item_template_selector(&self, value: Option<BoxedValue>);
}
impl PartialEq for dyn IDockItemContainerMetadata {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
