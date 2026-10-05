//! Server-side counterpart of `CompositionCacheMode`.
//!
//! The only cache mode is the bitmap cache, so the members of the abstract
//! class (the visuals attached to the mode) are implemented on
//! [`ServerCompositionBitmapCache`](super::ServerCompositionBitmapCache).

pub use super::server_composition_bitmap_cache::ServerCompositionBitmapCache as ServerCompositionCacheMode;
