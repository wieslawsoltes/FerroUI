//! `ReactiveRecord` (`ReactiveUI.Shared/ReactiveObject/ReactiveRecord.cs`).

use crate::ReactiveObject;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// The part of a reactive record (`ReactiveRecord`): a [`ReactiveObject`] for a type with value
/// equality. The notification state is not data: two parts are always equal, hash to nothing and
/// print nothing, so a record that derives `PartialEq`, `Eq`, `Hash` and `Debug` compares, hashes
/// and prints its own members only, as upstream's record does.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): a clone of the part is a new part without handlers or
/// state, the counterpart of the copy of a record (`with`), which gets its own state upstream.
///
/// ```
/// use ferroui_reactiveui::{reactive_object, ReactiveRecord};
///
/// #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
/// struct Point {
///     reactive: ReactiveRecord,
///     x: i32,
/// }
/// reactive_object!(Point);
///
/// let first = Point { x: 1, ..Point::default() };
/// let _ = first.reactive.suppress_change_notifications();
/// assert_eq!(first, Point { x: 1, ..Point::default() });
/// assert_eq!(format!("{first:?}"), "Point { reactive: ReactiveRecord, x: 1 }");
/// ```
#[derive(Default)]
pub struct ReactiveRecord {
    inner: ReactiveObject,
}

impl ReactiveRecord {
    /// The part of a reactive record without handlers or state.
    pub fn new() -> Self {
        Self { inner: ReactiveObject::new() }
    }
}

impl Deref for ReactiveRecord {
    type Target = ReactiveObject;

    fn deref(&self) -> &ReactiveObject {
        &self.inner
    }
}

impl AsRef<ReactiveObject> for ReactiveRecord {
    fn as_ref(&self) -> &ReactiveObject {
        &self.inner
    }
}

impl Clone for ReactiveRecord {
    fn clone(&self) -> Self {
        Self::new()
    }
}

/// The parts of two records are equal: the state is not data (`Equals(ReactiveRecord?)`).
impl PartialEq for ReactiveRecord {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for ReactiveRecord {}

/// The part adds nothing to the hash of a record (`GetHashCode`).
impl Hash for ReactiveRecord {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

/// The part prints no members (`PrintMembers`).
impl fmt::Debug for ReactiveRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReactiveRecord")
    }
}
