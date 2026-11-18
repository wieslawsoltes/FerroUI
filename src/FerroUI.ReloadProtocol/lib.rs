//! Source revision identities and optional development acknowledgements for supervised apps.

#[cfg(feature = "editor-events")]
mod events;
#[cfg(feature = "editor-events")]
pub use events::{Language, Status, enabled, mcp_endpoint, ready, source};

/// The FNV-1a revision shared by compiler metadata, reload receipts, and the editor.
pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}
