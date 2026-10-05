use crate::media::{IBrush, IImmutableBrush};
use std::rc::Rc;

/// Represents a mutable brush which can return an immutable clone of itself.
pub trait IMutableBrush: IBrush {
    /// Creates an immutable clone of the brush.
    fn to_immutable(&self) -> Rc<dyn IImmutableBrush>;
}
