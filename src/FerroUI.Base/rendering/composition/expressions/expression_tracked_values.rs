//! The set of objects an expression reads from.

use std::collections::HashSet;

use super::expression_evaluation_context::IExpressionObject;

/// The identity of an expression object (see [`IExpressionObject::key`]).
///
/// The reference implementation keeps references to the tracked objects and
/// compares them by reference; this port keeps their identity keys instead,
/// so that the collection neither borrows nor owns server objects. Whoever
/// enumerates the collection resolves the keys back to objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExpressionObjectKey(pub usize);

/// The distinct objects an expression reads from, in the order they were
/// first added.
#[derive(Clone, Debug, Default)]
pub struct ExpressionTrackedObjects {
    list: Vec<ExpressionObjectKey>,
    hash_set: HashSet<ExpressionObjectKey>,
}

impl ExpressionTrackedObjects {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tracks an object. The member name is not recorded.
    pub fn add(&mut self, obj: &dyn IExpressionObject, member: &str) {
        self.add_key(obj.key(), member);
    }

    /// Tracks an object given by its identity key.
    pub fn add_key(&mut self, obj: ExpressionObjectKey, _member: &str) {
        if self.hash_set.insert(obj) {
            self.list.push(obj);
        }
    }

    /// Removes every tracked object.
    pub fn clear(&mut self) {
        self.list.clear();
        self.hash_set.clear();
    }

    /// Enumerates the tracked objects in the order they were first added.
    pub fn iter(&self) -> std::slice::Iter<'_, ExpressionObjectKey> {
        self.list.iter()
    }
}

impl<'a> IntoIterator for &'a ExpressionTrackedObjects {
    type Item = &'a ExpressionObjectKey;
    type IntoIter = std::slice::Iter<'a, ExpressionObjectKey>;

    fn into_iter(self) -> Self::IntoIter {
        self.list.iter()
    }
}

/// A pool of [`ExpressionTrackedObjects`] collections (the nested `Pool` type of the
/// reference implementation).
#[derive(Debug, Default)]
pub struct ExpressionTrackedObjectsPool {
    stack: Vec<ExpressionTrackedObjects>,
}

impl ExpressionTrackedObjectsPool {
    /// Creates an empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes a collection out of the pool, or creates one. A pooled
    /// collection is handed out as it was returned (it is not cleared).
    pub fn get(&mut self) -> ExpressionTrackedObjects {
        self.stack.pop().unwrap_or_default()
    }

    /// Puts a collection back. The pool keeps only the most recently
    /// returned collection.
    pub fn return_(&mut self, obj: ExpressionTrackedObjects) {
        self.stack.clear();
        self.stack.push(obj);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::composition::expressions::ExpressionVariant;

    struct Obj(#[allow(dead_code)] u8);

    impl IExpressionObject for Obj {
        fn get_property(&self, _name: &str) -> ExpressionVariant {
            ExpressionVariant::Invalid
        }
    }

    struct Keyed(usize);

    impl IExpressionObject for Keyed {
        fn get_property(&self, _name: &str) -> ExpressionVariant {
            ExpressionVariant::Invalid
        }

        fn key(&self) -> ExpressionObjectKey {
            ExpressionObjectKey(self.0)
        }
    }

    #[test]
    fn tracks_each_object_once_in_insertion_order() {
        let (a, b) = (Obj(1), Obj(2));
        let mut tracked = ExpressionTrackedObjects::new();
        tracked.add(&b, "Offset");
        tracked.add(&a, "Offset");
        tracked.add(&b, "Size");
        tracked.add(&a as &dyn IExpressionObject, "Opacity");

        let keys: Vec<_> = tracked.iter().copied().collect();
        assert_eq!(keys, vec![b.key(), a.key()]);
        assert_ne!(a.key(), b.key());
        assert_eq!((&tracked).into_iter().count(), 2);

        tracked.clear();
        assert_eq!(tracked.iter().count(), 0);
        tracked.add(&a, "Offset");
        assert_eq!(tracked.iter().count(), 1);
    }

    #[test]
    fn uses_the_key_of_the_object() {
        let mut tracked = ExpressionTrackedObjects::new();
        tracked.add(&Keyed(7), "A");
        tracked.add(&Keyed(7), "B");
        tracked.add(&Keyed(9), "A");
        tracked.add_key(ExpressionObjectKey(9), "C");
        let keys: Vec<_> = tracked.iter().map(|k| k.0).collect();
        assert_eq!(keys, vec![7, 9]);
    }

    #[test]
    fn pool_keeps_only_the_last_returned_collection() {
        let mut pool = ExpressionTrackedObjectsPool::new();
        assert_eq!(pool.get().iter().count(), 0);

        let mut first = ExpressionTrackedObjects::new();
        first.add_key(ExpressionObjectKey(1), "A");
        let mut second = ExpressionTrackedObjects::new();
        second.add_key(ExpressionObjectKey(2), "A");
        second.add_key(ExpressionObjectKey(3), "A");
        pool.return_(first);
        pool.return_(second);

        // The collection comes back as it was returned.
        assert_eq!(pool.get().iter().count(), 2);
        assert_eq!(pool.get().iter().count(), 0);
    }
}
