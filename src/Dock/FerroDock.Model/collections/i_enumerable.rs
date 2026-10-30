// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::BoxedValue;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Non-generic source enumeration contract missing from FerroUI System 0.3.0.
/// Cursor creation and element nullability follow the supplied source implementation.
pub trait IEnumerable {
    /// Creates the source IEnumerator reference.
    fn get_enumerator(&self) -> Rc<dyn IEnumerator>;
}

/// Complete non-generic source cursor contract, including explicit reset.
pub trait IEnumerator {
    /// Gets the current element. Implementations retain source invalid-position behavior.
    fn current(&self) -> Option<BoxedValue>;
    /// Advances the cursor, returning whether it now has a current element.
    fn move_next(&self) -> bool;
    /// Resets the cursor, or fails when the source enumerator does not support reset.
    fn reset(&self);
    /// Resolves optional disposal implemented by the source cursor.
    fn as_disposable(&self) -> Option<&dyn IDisposable> {
        None
    }
}

impl PartialEq for dyn IEnumerable {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
impl PartialEq for dyn IEnumerator {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
