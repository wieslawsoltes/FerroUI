// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Optional report that a host has finished closing.
/// Hosts that can cancel or defer exit return false until closure completes.
pub trait IHostWindowCloseState {
    /// Resolves an optional native receiver without a UI dependency in the model.
    #[doc(hidden)]
    fn boxed_interface(
        self: std::rc::Rc<Self>,
        _target: std::any::TypeId,
    ) -> Option<crate::BoxedValue> {
        None
    }
    /// Returns whether the host is closed. Hidden and canceled hosts return false.
    fn is_closed(&self) -> bool;
}

impl PartialEq for dyn IHostWindowCloseState {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

#[cfg(test)]
mod tests {
    use super::IHostWindowCloseState;
    use std::{any::TypeId, cell::Cell, rc::Rc};

    struct CloseState(Cell<bool>);
    impl IHostWindowCloseState for CloseState {
        fn is_closed(&self) -> bool {
            self.0.get()
        }
    }

    #[test]
    fn close_state_is_a_retained_identity_view_and_plain_receivers_have_no_native_cast() {
        let original = Rc::new(CloseState(Cell::new(false)));
        let weak = Rc::downgrade(&original);
        let first: Rc<dyn IHostWindowCloseState> = original.clone();
        let same = first.clone();
        let other: Rc<dyn IHostWindowCloseState> = Rc::new(CloseState(Cell::new(false)));
        assert!(first.as_ref() == same.as_ref());
        assert!(first.as_ref() != other.as_ref());
        assert!(!first.is_closed());
        original.0.set(true);
        assert!(same.is_closed());
        assert!(first
            .clone()
            .boxed_interface(TypeId::of::<CloseState>())
            .is_none());
        drop(original);
        drop(first);
        assert!(weak.upgrade().is_some());
        drop(same);
        assert!(weak.upgrade().is_none());
    }
}
