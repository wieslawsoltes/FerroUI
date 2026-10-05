use crate::media::IBrush;

/// Represents an immutable brush which can be safely used with various
/// threading contexts.
pub trait IImmutableBrush: IBrush {}
