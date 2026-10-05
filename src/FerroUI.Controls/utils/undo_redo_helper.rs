use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// The object whose state an [`UndoRedoHelper`] records and restores.
pub trait IUndoRedoHost<TState> {
    fn undo_redo_state(&self) -> TState;

    fn set_undo_redo_state(&self, value: TState);

    fn on_undo_stack_changed(&self);

    fn on_redo_stack_changed(&self);
}

/// The position of the helper in its list of states.
enum CurrentNode<TState> {
    /// No state has been recorded.
    None,
    /// The state at this index of the list.
    Index(usize),
    /// A state that was dropped from the list by the limit right after it
    /// was recorded (a limit below one): it is still the current state, but
    /// it has no neighbours and the list no longer holds it.
    Detached(TState),
}

/// Records snapshots of the state of a host and moves between them.
///
/// The host owns its helper, so the helper only holds a weak handle to the
/// host. No borrow of the helper is held while the host is called: the host
/// may call back into the helper from every member of [`IUndoRedoHost`].
pub struct UndoRedoHelper<TState> {
    host: RefCell<Option<Weak<dyn IUndoRedoHost<TState>>>>,
    states: RefCell<VecDeque<TState>>,
    current_node: RefCell<CurrentNode<TState>>,
    limit: Cell<i32>,
}

impl<TState: Clone + PartialEq + 'static> UndoRedoHelper<TState> {
    pub const DEFAULT_UNDO_LIMIT: i32 = 10;

    pub fn new(host: Weak<dyn IUndoRedoHost<TState>>) -> Self {
        let helper = Self::unattached();
        helper.set_host(host);
        helper
    }

    /// Creates a helper whose host is set later with [`set_host`]
    /// (Self::set_host): the field initialisers of a class run before its
    /// handle exists.
    pub fn unattached() -> Self {
        Self {
            host: RefCell::new(None),
            states: RefCell::new(VecDeque::new()),
            current_node: RefCell::new(CurrentNode::None),
            limit: Cell::new(Self::DEFAULT_UNDO_LIMIT),
        }
    }

    /// Sets the host of a helper created with [`unattached`](Self::unattached).
    pub fn set_host(&self, host: Weak<dyn IUndoRedoHost<TState>>) {
        *self.host.borrow_mut() = Some(host);
    }

    fn host(&self) -> Option<Rc<dyn IUndoRedoHost<TState>>> {
        self.host.borrow().as_ref().and_then(Weak::upgrade)
    }

    /// Maximum number of states this helper can store for undo/redo.
    /// If -1, no limit is imposed.
    pub fn limit(&self) -> i32 {
        self.limit.get()
    }

    pub fn set_limit(&self, value: i32) {
        self.limit.set(value)
    }

    pub fn can_undo(&self) -> bool {
        matches!(*self.current_node.borrow(), CurrentNode::Index(index) if index > 0)
    }

    pub fn can_redo(&self) -> bool {
        matches!(*self.current_node.borrow(), CurrentNode::Index(index) if index + 1 < self.states.borrow().len())
    }

    pub fn undo(&self) {
        let Some(host) = self.host() else { return };

        let state = match &mut *self.current_node.borrow_mut() {
            CurrentNode::Index(index) if *index > 0 => {
                *index -= 1;
                self.states.borrow()[*index].clone()
            }
            _ => return,
        };

        host.set_undo_redo_state(state);
        host.on_undo_stack_changed();
        host.on_redo_stack_changed();
    }

    pub fn is_last_state(&self) -> bool {
        match &*self.current_node.borrow() {
            CurrentNode::None => false,
            CurrentNode::Index(index) => index + 1 == self.states.borrow().len(),
            CurrentNode::Detached(_) => true,
        }
    }

    /// The current state when it is the last one (C# `TryGetLastState(out state)`).
    pub fn try_get_last_state(&self) -> Option<TState> {
        if !self.is_last_state() {
            return None;
        }

        match &*self.current_node.borrow() {
            CurrentNode::None => None,
            CurrentNode::Index(index) => self.states.borrow().get(*index).cloned(),
            CurrentNode::Detached(state) => Some(state.clone()),
        }
    }

    pub fn has_state(&self) -> bool {
        !matches!(*self.current_node.borrow(), CurrentNode::None)
    }

    /// Replaces the last recorded state (C# `UpdateLastState(TState)`).
    pub fn update_last_state_with(&self, state: TState) {
        if let Some(last) = self.states.borrow_mut().back_mut() {
            *last = state;
        }
    }

    /// Replaces the last recorded state with the state of the host
    /// (C# `UpdateLastState()`).
    pub fn update_last_state(&self) {
        let Some(host) = self.host() else { return };

        let state = host.undo_redo_state();
        self.update_last_state_with(state);
    }

    pub fn discard_redo(&self) {
        if let CurrentNode::Index(index) = &*self.current_node.borrow() {
            self.states.borrow_mut().truncate(index + 1);
        }

        if let Some(host) = self.host() {
            host.on_redo_stack_changed();
        }
    }

    pub fn redo(&self) {
        let Some(host) = self.host() else { return };

        let state = match &mut *self.current_node.borrow_mut() {
            CurrentNode::Index(index) if *index + 1 < self.states.borrow().len() => {
                *index += 1;
                self.states.borrow()[*index].clone()
            }
            _ => return,
        };

        host.set_undo_redo_state(state);
        host.on_redo_stack_changed();
        host.on_undo_stack_changed();
    }

    pub fn snapshot(&self) {
        let Some(host) = self.host() else { return };

        let current = host.undo_redo_state();

        let is_current = match &*self.current_node.borrow() {
            CurrentNode::None => false,
            CurrentNode::Index(index) => self.states.borrow()[*index] == current,
            CurrentNode::Detached(state) => *state == current,
        };

        if !is_current {
            if self.can_redo() {
                self.discard_redo();
            }

            {
                let mut states = self.states.borrow_mut();
                let mut current_node = self.current_node.borrow_mut();

                states.push_back(current);
                *current_node = CurrentNode::Index(states.len() - 1);

                let limit = self.limit.get();

                if limit != -1 && states.len() as i64 > limit as i64 {
                    let first = states.pop_front();

                    *current_node = match (states.len(), first) {
                        // The state just recorded was the only one.
                        (0, Some(first)) => CurrentNode::Detached(first),
                        (length, _) => CurrentNode::Index(length - 1),
                    };
                }
            }

            host.on_undo_stack_changed();
            host.on_redo_stack_changed();
        }
    }

    pub fn clear(&self) {
        self.states.borrow_mut().clear();
        *self.current_node.borrow_mut() = CurrentNode::None;

        if let Some(host) = self.host() {
            host.on_undo_stack_changed();
            host.on_redo_stack_changed();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IUndoRedoHost, UndoRedoHelper};
    use std::cell::{Cell, RefCell};
    use std::rc::{Rc, Weak};

    struct Host {
        state: RefCell<String>,
        undo_changes: Cell<i32>,
        redo_changes: Cell<i32>,
        helper: UndoRedoHelper<String>,
        // What the helper reported from inside the notifications.
        can_undo: Cell<bool>,
        can_redo: Cell<bool>,
    }

    impl IUndoRedoHost<String> for Host {
        fn undo_redo_state(&self) -> String {
            self.state.borrow().clone()
        }

        fn set_undo_redo_state(&self, value: String) {
            *self.state.borrow_mut() = value;
        }

        fn on_undo_stack_changed(&self) {
            self.undo_changes.set(self.undo_changes.get() + 1);
            self.can_undo.set(self.helper.can_undo());
        }

        fn on_redo_stack_changed(&self) {
            self.redo_changes.set(self.redo_changes.get() + 1);
            self.can_redo.set(self.helper.can_redo());
        }
    }

    fn host() -> Rc<Host> {
        let host = Rc::new(Host {
            state: RefCell::new(String::new()),
            undo_changes: Cell::new(0),
            redo_changes: Cell::new(0),
            helper: UndoRedoHelper::unattached(),
            can_undo: Cell::new(false),
            can_redo: Cell::new(false),
        });
        let weak: Weak<dyn IUndoRedoHost<String>> = Rc::<Host>::downgrade(&host);
        host.helper.set_host(weak);
        host
    }

    fn snapshot(host: &Host, state: &str) {
        *host.state.borrow_mut() = state.to_owned();
        host.helper.snapshot();
    }

    #[test]
    fn snapshot_records_only_changed_states() {
        let host = host();

        assert!(!host.helper.has_state());
        assert_eq!(host.helper.limit(), UndoRedoHelper::<String>::DEFAULT_UNDO_LIMIT);

        snapshot(&host, "a");
        snapshot(&host, "a");

        assert!(host.helper.has_state());
        assert!(host.helper.is_last_state());
        assert!(!host.helper.can_undo());
        assert_eq!(host.undo_changes.get(), 1);
        assert_eq!(host.redo_changes.get(), 1);

        snapshot(&host, "b");

        assert!(host.helper.can_undo());
        assert!(host.can_undo.get());
        assert!(!host.helper.can_redo());
    }

    #[test]
    fn undo_and_redo_move_between_states() {
        let host = host();
        snapshot(&host, "a");
        snapshot(&host, "b");
        snapshot(&host, "c");

        host.helper.undo();
        assert_eq!(*host.state.borrow(), "b");
        assert!(host.can_redo.get());
        assert!(!host.helper.is_last_state());
        assert_eq!(host.helper.try_get_last_state(), None);

        host.helper.undo();
        assert_eq!(*host.state.borrow(), "a");
        assert!(!host.helper.can_undo());

        // Nothing before the first state: no change and no notification.
        let notifications = host.undo_changes.get();
        host.helper.undo();
        assert_eq!(*host.state.borrow(), "a");
        assert_eq!(host.undo_changes.get(), notifications);

        host.helper.redo();
        host.helper.redo();
        assert_eq!(*host.state.borrow(), "c");
        assert!(!host.helper.can_redo());
        assert_eq!(host.helper.try_get_last_state().as_deref(), Some("c"));

        let notifications = host.redo_changes.get();
        host.helper.redo();
        assert_eq!(host.redo_changes.get(), notifications);
    }

    #[test]
    fn snapshot_after_undo_discards_redo() {
        let host = host();
        snapshot(&host, "a");
        snapshot(&host, "b");
        snapshot(&host, "c");

        host.helper.undo();
        host.helper.undo();
        snapshot(&host, "d");

        assert!(!host.helper.can_redo());
        host.helper.undo();
        assert_eq!(*host.state.borrow(), "a");
        host.helper.redo();
        assert_eq!(*host.state.borrow(), "d");
    }

    #[test]
    fn discard_redo_always_notifies() {
        let host = host();

        host.helper.discard_redo();
        assert_eq!(host.redo_changes.get(), 1);

        snapshot(&host, "a");
        snapshot(&host, "b");
        host.helper.undo();
        host.helper.discard_redo();

        assert!(!host.helper.can_redo());
        assert!(host.helper.is_last_state());
    }

    #[test]
    fn limit_drops_the_oldest_state() {
        let host = host();
        host.helper.set_limit(2);

        snapshot(&host, "a");
        snapshot(&host, "b");
        snapshot(&host, "c");

        host.helper.undo();
        assert_eq!(*host.state.borrow(), "b");
        assert!(!host.helper.can_undo());

        // No limit.
        host.helper.set_limit(-1);
        host.helper.redo();
        for state in ["d", "e", "f", "g", "h", "i", "j", "k", "l", "m"] {
            snapshot(&host, state);
        }
        for _ in 0..11 {
            host.helper.undo();
        }
        assert_eq!(*host.state.borrow(), "b");
    }

    #[test]
    fn limit_of_zero_keeps_a_current_state_without_neighbours() {
        let host = host();
        host.helper.set_limit(0);

        snapshot(&host, "a");

        assert!(host.helper.has_state());
        assert!(host.helper.is_last_state());
        assert!(!host.helper.can_undo());
        assert!(!host.helper.can_redo());
        assert_eq!(host.helper.try_get_last_state().as_deref(), Some("a"));

        // The same state is not recorded again.
        let notifications = host.undo_changes.get();
        host.helper.snapshot();
        assert_eq!(host.undo_changes.get(), notifications);

        // The list is empty: there is no last state to update.
        host.helper.update_last_state_with("z".to_owned());
        assert_eq!(host.helper.try_get_last_state().as_deref(), Some("a"));
    }

    #[test]
    fn update_last_state_replaces_the_last_state() {
        let host = host();
        snapshot(&host, "a");
        snapshot(&host, "b");

        *host.state.borrow_mut() = "c".to_owned();
        host.helper.update_last_state();
        assert_eq!(host.helper.try_get_last_state().as_deref(), Some("c"));

        host.helper.update_last_state_with("d".to_owned());
        host.helper.undo();
        host.helper.redo();
        assert_eq!(*host.state.borrow(), "d");
    }

    #[test]
    fn clear_removes_every_state() {
        let host = host();
        snapshot(&host, "a");
        snapshot(&host, "b");

        host.helper.clear();

        assert!(!host.helper.has_state());
        assert!(!host.helper.can_undo());
        assert!(!host.helper.is_last_state());
        assert_eq!(host.helper.try_get_last_state(), None);
    }

    #[test]
    fn helper_does_not_keep_its_host_alive() {
        let host = host();
        let weak = Rc::downgrade(&host);

        drop(host);

        assert!(weak.upgrade().is_none());
    }
}
