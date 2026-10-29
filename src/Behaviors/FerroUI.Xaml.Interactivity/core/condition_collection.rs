//! Port of `Core/ConditionCollection.cs`.

use crate::Condition;
use ferroui_base::collections::FerroList;
use ferroui_base::Ref;
use std::ops::Deref;

/// Represents a collection of [`Condition`] objects.
///
/// It is a reference object: clones refer to the same collection and compare by identity.
#[derive(Clone, PartialEq)]
pub struct ConditionCollection(FerroList<Ref<Condition>>);

impl ConditionCollection {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<Condition>> {
        &self.0
    }
}

impl Default for ConditionCollection {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for ConditionCollection {
    type Target = FerroList<Ref<Condition>>;

    fn deref(&self) -> &FerroList<Ref<Condition>> {
        &self.0
    }
}

impl std::fmt::Debug for ConditionCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConditionCollection({})", self.0.count())
    }
}
