// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// The `IDockableControl` docking contract.
pub trait IDockableControl: 'static {
    /// Boxes the concrete reference for optional presentation-side class casts.
    /// The model contract remains independent of the UI framework.
    #[doc(hidden)]
    fn boxed_reference(self: std::rc::Rc<Self>) -> crate::BoxedValue {
        std::rc::Rc::new(self)
    }
}

impl PartialEq for dyn IDockableControl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
