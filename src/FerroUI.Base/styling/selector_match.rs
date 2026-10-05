use super::activators::{AndActivatorBuilder, IStyleActivator};
use std::fmt;
use std::rc::Rc;

/// Describes how a [`SelectorMatch`] matches a control and its type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelectorMatchResult {
    /// The selector never matches this type.
    NeverThisType,
    /// The selector never matches this instance, but can match this type.
    NeverThisInstance,
    /// The selector matches this instance based on the activator.
    Sometimes,
    /// The selector always matches this instance, but doesn't always match
    /// this type.
    AlwaysThisInstance,
    /// The selector always matches this type.
    AlwaysThisType,
}

/// Holds the result of a selector match.
///
/// A selector match describes whether and how a selector matches a control,
/// and in addition whether the selector can ever match a control of the same
/// type.
#[derive(Clone)]
pub struct SelectorMatch {
    result: SelectorMatchResult,
    activator: Option<Rc<dyn IStyleActivator>>,
}

impl SelectorMatch {
    /// A selector match with the result of [`SelectorMatchResult::NeverThisType`].
    pub const NEVER_THIS_TYPE: SelectorMatch = SelectorMatch::new(SelectorMatchResult::NeverThisType);

    /// A selector match with the result of [`SelectorMatchResult::NeverThisInstance`].
    pub const NEVER_THIS_INSTANCE: SelectorMatch = SelectorMatch::new(SelectorMatchResult::NeverThisInstance);

    /// A selector match with the result of [`SelectorMatchResult::AlwaysThisType`].
    pub const ALWAYS_THIS_TYPE: SelectorMatch = SelectorMatch::new(SelectorMatchResult::AlwaysThisType);

    /// A selector match with the result of [`SelectorMatchResult::AlwaysThisInstance`].
    pub const ALWAYS_THIS_INSTANCE: SelectorMatch = SelectorMatch::new(SelectorMatchResult::AlwaysThisInstance);

    /// Creates a match with a [`SelectorMatchResult::Sometimes`] result,
    /// whose state is tracked by `activator`.
    pub fn sometimes(activator: Rc<dyn IStyleActivator>) -> Self {
        Self { result: SelectorMatchResult::Sometimes, activator: Some(activator) }
    }

    /// Creates a match with the specified result and no activator.
    pub const fn new(result: SelectorMatchResult) -> Self {
        Self { result, activator: None }
    }

    /// Whether the match was positive.
    ///
    /// The match is positive if the result is `Sometimes`,
    /// `AlwaysThisInstance` or `AlwaysThisType`.
    #[inline]
    pub fn is_match(&self) -> bool {
        self.result >= SelectorMatchResult::Sometimes
    }

    /// The result of the selector match.
    #[inline]
    pub fn result(&self) -> SelectorMatchResult {
        self.result
    }

    /// An activator which tracks the selector match, in the case of
    /// selectors that can change over time.
    #[inline]
    pub fn activator(&self) -> Option<&Rc<dyn IStyleActivator>> {
        self.activator.as_ref()
    }

    pub(crate) fn into_activator(self) -> Option<Rc<dyn IStyleActivator>> {
        self.activator
    }

    /// Logical-AND the selector match with another.
    pub fn and(self, other: SelectorMatch) -> SelectorMatch {
        let result = self.result.min(other.result);

        if result == SelectorMatchResult::Sometimes {
            let mut activators = AndActivatorBuilder::new();
            activators.add(self.activator);
            activators.add(other.activator);
            SelectorMatch::sometimes(activators.get())
        } else {
            SelectorMatch::new(result)
        }
    }
}

impl fmt::Debug for SelectorMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.result, f)
    }
}

impl fmt::Display for SelectorMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.result, f)
    }
}
