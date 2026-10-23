mod i_active_element;
mod insertion_context;
mod snippet;
mod snippet_anchor_element;
mod snippet_element;
mod snippet_event_args;
mod snippet_session;

pub use i_active_element::{IActiveElement, IReplaceableActiveElement};
pub use insertion_context::{SnippetInsertionContext, SnippetInsertionOptions};
pub use snippet::{PreparedSnippet, Snippet, SnippetElement, SnippetError};
pub use snippet_anchor_element::AnchorElement;
pub use snippet_element::{ISnippetElement, SnippetElementRef, SnippetTextTransform};
pub use snippet_event_args::{DeactivateReason, SnippetEventArgs};
pub use snippet_session::{SnippetEditValidator, SnippetSession, WeakSnippetSession};

#[cfg(test)]
mod snippet_extension_tests;
