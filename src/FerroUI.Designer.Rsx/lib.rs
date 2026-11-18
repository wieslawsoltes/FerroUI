//! Portable XAML/RSX translation for the visual designer. Parsed Rust is data,
//! and the mapping is tied to the source snapshot used to construct it.

mod blocks;
mod catalog;
mod language;
mod literal;
mod placement;
mod project;
mod registered;
mod source_edits;
mod tree;
mod xaml;

pub use language::{RsxLanguageDocument, analyze_rsx};
pub use project::{RsxNode, RsxProjection, project_rust};
pub use registered::RsxCatalog;
pub use source_edits::{RsxEvent, event_setter};
pub use xaml::{apply_rsx, xaml_to_rsx};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod registered_tests;
