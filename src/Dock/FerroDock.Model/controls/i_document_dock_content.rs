// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use crate::BoxedValue;
use std::rc::Rc;

/// The `IDocumentDockContent` docking contract.
pub trait IDocumentDockContent: IDock {
    /// Gets or sets document template.
    fn document_template(&self) -> Option<Rc<dyn IDocumentTemplate>>;
    /// Sets `DocumentTemplate`.
    fn set_document_template(&self, value: Option<Rc<dyn IDocumentTemplate>>);
    /// Create new document from template.
    fn create_document_from_template(&self) -> Option<BoxedValue>;
}

impl PartialEq for dyn IDocumentDockContent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
