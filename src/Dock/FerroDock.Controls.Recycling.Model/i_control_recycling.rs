// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use std::any::Any;
use std::rc::Rc;

/// A cache and builder of recycled content, independent of a UI framework.
pub trait IControlRecycling {
    /// Exposes an optional native adapter for registered object projections.
    /// This is the safe counterpart of the source runtime's interface cast.
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
    /// Returns whether a content-provided identifier may be used as the cache key.
    fn try_to_use_id_as_key(&self) -> bool;
    /// Sets whether a content-provided identifier may be used as the cache key.
    fn set_try_to_use_id_as_key(&self, value: bool);
    /// Gets a cached control for the supplied key; `None` means no cached value.
    fn try_get_value(&self, data: Option<Rc<dyn Any>>) -> Option<Rc<dyn Any>>;
    /// Adds or replaces a control under the supplied data key.
    fn add(&self, data: Rc<dyn Any>, control: Rc<dyn Any>);
    /// Builds or reuses content for data, an existing control, and a parent.
    fn build(
        &self,
        data: Option<Rc<dyn Any>>,
        existing: Option<Rc<dyn Any>>,
        parent: Option<Rc<dyn Any>>,
    ) -> Option<Rc<dyn Any>>;
    /// Clears cached controls.
    fn clear(&self);
}

impl PartialEq for dyn IControlRecycling {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
