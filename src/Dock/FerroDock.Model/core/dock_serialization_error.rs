// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use std::{error::Error, fmt, rc::Rc};
/// A source serialization failure retaining the original error instance.
#[derive(Clone, Debug)]
pub struct DockSerializationError(Rc<dyn Error>);
impl DockSerializationError {
    /// Retains the supplied failure without replacing its cause or message.
    pub fn new(error: impl Error + 'static) -> Self {
        Self(Rc::new(error))
    }
    /// Gets the original concrete error.
    pub fn cause(&self) -> &(dyn Error + 'static) {
        self.0.as_ref()
    }
    /// Compares failure identity across propagation and cloned handles.
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl fmt::Display for DockSerializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Error for DockSerializationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}
/// Result preserving the source distinction between failure and a null value.
pub type SerializationResult<T> = Result<T, DockSerializationError>;
