use crate::{Ref, Visual};

/// Interface for controls that are at the root of a hosted visual tree,
/// such as popups.
///
/// An input element that is such a root returns itself from the
/// `as_hosted_visual_tree_root` virtual member of `InputElement`.
pub trait IHostedVisualTreeRoot {
    /// The visual tree host.
    fn host(&self) -> Option<Ref<Visual>>;
}
