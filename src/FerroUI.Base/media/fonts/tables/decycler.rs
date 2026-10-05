use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

/// Errors that can occur during graph traversal with cycle detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DecyclerError {
    /// A cycle was detected in the graph.
    CycleDetected,
    /// The maximum depth limit was exceeded.
    DepthLimitExceeded,
}

/// The error reported when a decycler error occurs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecyclerException {
    error: DecyclerError,
    message: String,
}

impl DecyclerException {
    pub fn new(error: DecyclerError, message: impl Into<String>) -> Self {
        Self { error, message: message.into() }
    }

    pub fn error(&self) -> DecyclerError {
        self.error
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DecyclerException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DecyclerException {}

/// A guard that tracks entry into a node and ensures proper cleanup.
///
/// The node is exited when the guard is dropped or [`dispose`](CycleGuard::dispose)d,
/// whichever comes first.
pub struct CycleGuard<'a, T: Copy + Eq + Hash> {
    decycler: &'a Decycler<T>,
    id: T,
    exited: bool,
}

impl<'a, T: Copy + Eq + Hash> CycleGuard<'a, T> {
    fn new(decycler: &'a Decycler<T>, id: T) -> Self {
        Self { decycler, id, exited: false }
    }

    /// Exits the guard, removing the node ID from the visited set.
    pub fn dispose(&mut self) {
        if !self.exited {
            self.decycler.exit(self.id);
            self.exited = true;
        }
    }
}

/// A copy carries its own exited flag, as copies of the reference
/// implementation's guard struct do.
impl<T: Copy + Eq + Hash> Clone for CycleGuard<'_, T> {
    fn clone(&self) -> Self {
        Self { decycler: self.decycler, id: self.id, exited: self.exited }
    }
}

impl<T: Copy + Eq + Hash> Drop for CycleGuard<'_, T> {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// Tracks visited nodes to detect cycles in a graph (composite glyphs, paint
/// graphs, etc.). Uses a depth limit to prevent stack overflow during
/// recursive traversal.
///
/// The intended usage pattern is "one instance per traversal": rent an
/// instance from a pool at the start of a walk and return it when done.
///
/// [`enter`](Decycler::enter) reports cycle / depth-limit failures as errors.
/// Callers that traverse user-supplied data (e.g. font tables) should treat
/// the failure as "stop traversing this subgraph".
pub struct Decycler<T: Copy + Eq + Hash> {
    visited: RefCell<HashSet<T>>,
    max_depth: i32,
    current_depth: Cell<i32>,
}

impl<T: Copy + Eq + Hash> Decycler<T> {
    /// Creates a new decycler with the specified maximum depth.
    ///
    /// Panics when `max_depth` is less than 1.
    pub fn new(max_depth: i32) -> Self {
        assert!(max_depth >= 1, "maxDepth must be at least 1. (Parameter 'maxDepth')");

        Self { visited: RefCell::new(HashSet::new()), max_depth, current_depth: Cell::new(0) }
    }

    /// Attempts to enter a node with the given ID.
    /// Returns a guard that exits the node when dropped.
    pub fn enter(&self, id: T) -> Result<CycleGuard<'_, T>, DecyclerException> {
        if self.current_depth.get() >= self.max_depth {
            return Err(DecyclerException::new(
                DecyclerError::DepthLimitExceeded,
                format!("Graph depth limit of {} exceeded", self.max_depth),
            ));
        }

        // `insert` returns false if the item was already in the set, which indicates a cycle.
        if !self.visited.borrow_mut().insert(id) {
            return Err(DecyclerException::new(DecyclerError::CycleDetected, "Cycle detected in graph"));
        }

        self.current_depth.set(self.current_depth.get() + 1);

        Ok(CycleGuard::new(self, id))
    }

    /// Exits a node, removing it from the visited set.
    /// Called automatically by the guard.
    pub(crate) fn exit(&self, id: T) {
        // A copied guard can double-exit; only give depth budget back for an id
        // that was actually in the visited set.
        if self.visited.borrow_mut().remove(&id) {
            self.current_depth.set(self.current_depth.get() - 1);
        }
    }

    /// Returns the current traversal depth.
    pub fn current_depth(&self) -> i32 {
        self.current_depth.get()
    }

    /// Returns the maximum allowed traversal depth.
    pub fn max_depth(&self) -> i32 {
        self.max_depth
    }

    /// Resets the decycler to its initial state, clearing all visited nodes.
    pub fn reset(&self) {
        self.visited.borrow_mut().clear();
        self.current_depth.set(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter_error<T: Copy + Eq + Hash>(decycler: &Decycler<T>, id: T) -> DecyclerException {
        match decycler.enter(id) {
            Ok(_) => panic!("expected the enter to fail"),
            Err(error) => error,
        }
    }

    #[test]
    #[should_panic(expected = "maxDepth must be at least 1")]
    fn constructor_throws_when_max_depth_is_zero() {
        let _ = Decycler::<i32>::new(0);
    }

    #[test]
    #[should_panic(expected = "maxDepth must be at least 1")]
    fn constructor_throws_when_max_depth_is_minus_one() {
        let _ = Decycler::<i32>::new(-1);
    }

    #[test]
    #[should_panic(expected = "maxDepth must be at least 1")]
    fn constructor_throws_when_max_depth_is_min_value() {
        let _ = Decycler::<i32>::new(i32::MIN);
    }

    #[test]
    fn constructor_accepts_max_depth_of_one() {
        let decycler = Decycler::<i32>::new(1);

        assert_eq!(decycler.current_depth(), 0);
        assert_eq!(decycler.max_depth(), 1);
    }

    #[test]
    fn enter_increments_current_depth() {
        let decycler = Decycler::<i32>::new(4);
        let _guard = decycler.enter(1).unwrap();

        assert_eq!(decycler.current_depth(), 1);
    }

    #[test]
    fn disposing_guard_restores_current_depth() {
        let decycler = Decycler::<i32>::new(4);

        {
            let _guard = decycler.enter(1).unwrap();
            assert_eq!(decycler.current_depth(), 1);
        }

        assert_eq!(decycler.current_depth(), 0);
    }

    #[test]
    fn nested_enters_stack_depth_and_unwind_in_reverse() {
        let decycler = Decycler::<i32>::new(4);

        {
            let _a = decycler.enter(1).unwrap();
            assert_eq!(decycler.current_depth(), 1);

            {
                let _b = decycler.enter(2).unwrap();
                assert_eq!(decycler.current_depth(), 2);

                {
                    let _c = decycler.enter(3).unwrap();
                    assert_eq!(decycler.current_depth(), 3);
                }

                assert_eq!(decycler.current_depth(), 2);
            }

            assert_eq!(decycler.current_depth(), 1);
        }

        assert_eq!(decycler.current_depth(), 0);
    }

    #[test]
    fn re_entering_visited_id_throws_cycle_detected() {
        let decycler = Decycler::<i32>::new(4);
        let _outer = decycler.enter(1).unwrap();

        let error = enter_error(&decycler, 1);

        assert_eq!(error.error(), DecyclerError::CycleDetected);
    }

    #[test]
    fn same_id_can_be_re_entered_after_exit() {
        let decycler = Decycler::<i32>::new(4);

        {
            let _guard = decycler.enter(1).unwrap();
        }

        // The id is no longer in the visited set; re-entering must succeed.
        let _guard = decycler.enter(1).unwrap();

        assert_eq!(decycler.current_depth(), 1);
    }

    #[test]
    fn enter_beyond_max_depth_throws_depth_limit_exceeded() {
        let decycler = Decycler::<i32>::new(2);
        let _a = decycler.enter(1).unwrap();
        let _b = decycler.enter(2).unwrap();

        let error = enter_error(&decycler, 3);

        assert_eq!(error.error(), DecyclerError::DepthLimitExceeded);
        assert_eq!(error.message(), "Graph depth limit of 2 exceeded");
    }

    #[test]
    fn depth_limit_check_runs_before_cycle_check() {
        // When both conditions could apply (depth is exhausted AND the id is
        // already visited), the depth check fires first.
        let decycler = Decycler::<i32>::new(1);
        let _guard = decycler.enter(1).unwrap();

        let error = enter_error(&decycler, 1);

        assert_eq!(error.error(), DecyclerError::DepthLimitExceeded);
    }

    #[test]
    fn failed_enter_does_not_mutate_state() {
        let decycler = Decycler::<i32>::new(1);
        let _guard = decycler.enter(1).unwrap();

        assert!(decycler.enter(2).is_err());

        // The failed enter must not have incremented depth or registered the id.
        assert_eq!(decycler.current_depth(), 1);
    }

    #[test]
    fn reset_clears_visited_and_depth() {
        let decycler = Decycler::<i32>::new(4);
        let guard = decycler.enter(1).unwrap();

        // Skip the disposal: simulate an abandoned traversal that needs to be
        // cleaned up by reset (the validator path on the pool).
        std::mem::forget(guard);

        decycler.reset();

        assert_eq!(decycler.current_depth(), 0);

        // The previously visited id must be enterable again after reset.
        let _fresh = decycler.enter(1).unwrap();

        assert_eq!(decycler.current_depth(), 1);
    }

    #[test]
    fn reset_is_safe_to_call_on_empty_decycler() {
        let decycler = Decycler::<i32>::new(4);

        decycler.reset();
        decycler.reset();

        assert_eq!(decycler.current_depth(), 0);
    }

    #[test]
    fn guard_dispose_is_idempotent() {
        let decycler = Decycler::<i32>::new(4);
        let mut guard = decycler.enter(1).unwrap();

        guard.dispose();
        guard.dispose(); // second call must not double-decrement.
        drop(guard); // nor must the drop.

        assert_eq!(decycler.current_depth(), 0);

        // Depth must not be negative; entering a new node still works.
        let _next = decycler.enter(2).unwrap();

        assert_eq!(decycler.current_depth(), 1);
    }

    #[test]
    fn copied_guard_cannot_double_exit() {
        // Each copy carries its own exited flag, so the idempotence of dispose on one copy
        // doesn't protect against the other copy exiting again. Exit only returns depth
        // budget for ids actually in the visited set.
        let decycler = Decycler::<i32>::new(4);
        let mut guard = decycler.enter(1).unwrap();
        let mut copy = guard.clone();

        guard.dispose();
        copy.dispose(); // second exit for id 1, via an un-exited copy.

        assert_eq!(decycler.current_depth(), 0);
    }

    #[test]
    fn max_depth_property_reflects_constructor_argument() {
        let decycler = Decycler::<i32>::new(17);

        assert_eq!(decycler.max_depth(), 17);
    }

    #[test]
    fn decycler_exception_carries_error_code_and_message() {
        let error = DecyclerException::new(DecyclerError::CycleDetected, "boom");

        assert_eq!(error.error(), DecyclerError::CycleDetected);
        assert_eq!(error.message(), "boom");
        assert_eq!(error.to_string(), "boom");
    }

    #[test]
    fn works_with_other_struct_types() {
        let decycler = Decycler::<u16>::new(4);

        {
            let _guard = decycler.enter(1).unwrap();
            assert!(decycler.enter(1u16).is_err());
        }

        assert_eq!(decycler.current_depth(), 0);
    }
}
