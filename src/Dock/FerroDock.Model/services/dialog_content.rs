// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IDockDialogContent;
use crate::BoxedValue;
use std::any::Any;
use std::rc::Rc;

/// An object handle and its optional source dialog-content interface projection.
/// The request always receives the original object, not this metadata carrier.
#[derive(Clone)]
pub struct DialogContent {
    value: BoxedValue,
    dialog_content: Option<Rc<dyn IDockDialogContent>>,
}
impl DialogContent {
    /// Carries an ordinary object, recognizing an explicitly boxed interface handle.
    pub fn new(value: BoxedValue) -> Self {
        let dialog_content = value.downcast_ref::<Rc<dyn IDockDialogContent>>().cloned();
        Self {
            value,
            dialog_content,
        }
    }
    /// Carries a concrete object implementing IDockDialogContent, preserving its
    /// original Rc identity while making its native interface membership explicit.
    pub fn from_dialog_content<T: IDockDialogContent + Any>(value: Rc<T>) -> Self {
        Self {
            value: value.clone(),
            dialog_content: Some(value),
        }
    }
    /// Gets the original object handle.
    pub fn value(&self) -> BoxedValue {
        self.value.clone()
    }
    /// Queries the supplied source dialog-content interface without reflection.
    pub fn as_dialog_content(&self) -> Option<Rc<dyn IDockDialogContent>> {
        self.dialog_content.clone()
    }
}
impl From<BoxedValue> for DialogContent {
    fn from(value: BoxedValue) -> Self {
        Self::new(value)
    }
}
