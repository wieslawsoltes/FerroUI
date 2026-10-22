//! Renderer-independent editor services. Public document offsets count UTF-16
//! code units; logical line and column numbers are one-based.
//!
//! Documents are shared handles and snapshots are immutable. Editing services
//! operate on the same document/change contract used by the UI control.

pub mod completion;
pub mod document;
pub mod editing;
pub mod folding;
pub mod highlighting;
pub mod search;
pub mod snippets;
mod text_view_position;
pub mod utils;

pub use document::{TextDocument, TextLocation};
pub use text_view_position::TextViewPosition;
