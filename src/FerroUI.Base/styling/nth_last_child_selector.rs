use super::{NthChildSelector, Selector};

/// The `:nth-last-child()` pseudo-class matches elements based on their
/// position among a group of siblings, counting from the end.
///
/// Element indices are 1-based.
pub(crate) struct NthLastChildSelector;

impl NthLastChildSelector {
    /// Creates an instance of the nth-last-child selector.
    ///
    /// `step` is the position step and `offset` the initial index offset,
    /// counting from the end.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(previous: Option<Selector>, step: i32, offset: i32) -> NthChildSelector {
        NthChildSelector::with_direction(previous, step, offset, true)
    }
}
