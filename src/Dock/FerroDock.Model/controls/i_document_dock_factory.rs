// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Original CLR document callback, sharing the locator Func identity and nullable
/// reference result. CLR nullable annotations do not define a different delegate type.
pub type DocumentFactory = crate::core::DockableFactory;

/// The `IDocumentDockFactory` docking contract.
pub trait IDocumentDockFactory {
    /// Gets or sets factory method used to create new documents.
    fn document_factory(&self) -> Option<DocumentFactory>;
    /// Sets `DocumentFactory`.
    fn set_document_factory(&self, value: Option<DocumentFactory>);
}

impl PartialEq for dyn IDocumentDockFactory {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
