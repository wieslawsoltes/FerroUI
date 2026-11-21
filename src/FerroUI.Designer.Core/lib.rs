//! Portable, source-preserving XAML editing for FerroUI designers.
//!
//! [`Document`] is the transaction boundary shared by source editors, visual tools,
//! collaboration adapters and automation. It retains authored text when it is
//! temporarily invalid; [`Document::ast`] and [`Document::valid_source`] remain the
//! last validated snapshot. Visual edits refuse to overwrite an invalid buffer.
//!
//! The syntax model preserves comments, CDATA, namespace declarations, attribute
//! quotes and all unrelated bytes. Node identifiers survive edits and history;
//! named elements are reconciled when source is replaced. No rendering platform,
//! dispatcher or runtime loader is required.
mod ast;
pub mod design_fixtures;
mod document;
mod history;
mod markup;
mod source_anchor;

pub use ast::{Ast, Attribute, Node, NodeId, NodeKind, SourceSpan, XmlText};
pub use document::{
    ChangeSet, DesignerError, Diagnostic, Document, Edit, SourceUpdate, TextPatch, Transaction,
};
pub use history::SourceHistory;
pub use markup::{MarkupArgument, MarkupExtension, MarkupValue};
pub use source_anchor::{SourceAnchor, SourceSegment};

#[cfg(test)]
mod preview_snapshot_tests;
#[cfg(test)]
mod tests;
