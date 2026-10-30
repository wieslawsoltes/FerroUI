//! Source routable controls built on the actual reactive docking models.
mod routable_document;
pub use routable_document::RoutableDocument;
mod routable_tool;
pub use routable_tool::RoutableTool;
mod routable_root_dock;
pub use routable_root_dock::RoutableRootDock;
pub(crate) fn class_name<T>() -> String {
    std::any::type_name::<T>()
        .split('<')
        .next()
        .unwrap()
        .rsplit("::")
        .next()
        .unwrap()
        .to_owned()
}

mod routable_document_base;
pub use routable_document_base::RoutableDocumentBase;

mod routable_tool_base;
pub use routable_tool_base::RoutableToolBase;
