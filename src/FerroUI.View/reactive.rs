//! Fine-grained reactivity: signals, memos, effects and the owners that
//! dispose them.
//!
//! A [`Signal`] holds a value. An [`effect`] runs a closure, records the
//! signals and memos it reads, and runs it again when one of them changes; a
//! [`Memo`] is a value computed the same way and read like a signal. This is
//! how a property of a control follows state: the element sets the property
//! from an effect (`text: move || format!(..)`), so a change updates exactly
//! that property, with no tree to compare.
//!
//! Handles are `Copy`: they are indices into a runtime of the thread, so a
//! `move` closure takes a signal without a clone. Every signal, memo and
//! effect belongs to the [`Owner`] that was current when it was made (an
//! effect is the owner of what is made while it runs), and is disposed with
//! it: a mounted element tree has an owner that ends with it, and the owner
//! of an effect disposes what the effect made before it runs again. Using a
//! handle after its owner was disposed panics.
//!
//! Changes propagate when the outermost write ends: a write marks the
//! effects and memos that read the value as dirty, and the dirty effects run
//! (reading a dirty memo computes it again first). [`batch`] defers the runs
//! until the end of a group of writes.

use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;

/// The identity of a node of the runtime: its slot and the generation of the
/// slot, so that a handle of a disposed node is never taken for the node that
/// reused its slot.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct NodeId {
    index: u32,
    generation: u32,
}

impl NodeId {
    fn as_u64(self) -> u64 {
        (u64::from(self.generation) << 32) | u64::from(self.index)
    }
}

enum Kind {
    /// A value that is written from outside.
    Signal,
    /// A computed value: the closure computes it into its cell and says
    /// whether it changed.
    Memo(Rc<dyn Fn() -> bool>),
    /// A closure that runs again when what it read changes.
    Effect(Rc<dyn Fn()>),
    /// Owns what is made while it is current, and nothing else.
    Owner,
}

struct Node {
    generation: u32,
    alive: bool,
    kind: Kind,
    /// `Rc<RefCell<T>>` of a signal, `Rc<RefCell<Option<T>>>` of a memo.
    value: Option<Rc<dyn Any>>,
    /// The memos and effects that read the node.
    subscribers: Vec<NodeId>,
    /// The signals and memos the node read when it last ran.
    sources: Vec<NodeId>,
    dirty: bool,
    owner: Option<NodeId>,
    owned: Vec<NodeId>,
    cleanups: Vec<Box<dyn FnOnce()>>,
    /// The values provided to what the node owns (see [`provide_context`]).
    contexts: Vec<(TypeId, Rc<dyn Any>)>,
}

#[derive(Default)]
struct Runtime {
    nodes: RefCell<Vec<Node>>,
    free: RefCell<Vec<u32>>,
    /// The memo or effect that is running: what is read is its source.
    observer: Cell<Option<NodeId>>,
    /// What owns what is made now.
    owner: Cell<Option<NodeId>>,
    queue: RefCell<VecDeque<NodeId>>,
    batch_depth: Cell<u32>,
    flushing: Cell<bool>,
    /// Counts the propagations of changes: what one write (or batch) runs.
    propagation: Cell<u64>,
}

ferroui_base::ferro_thread_local! {
    static RUNTIME: Runtime = Runtime::default();
}

fn with_runtime<R>(f: impl FnOnce(&Runtime) -> R) -> R {
    RUNTIME.with(f)
}

impl Runtime {
    fn create(&self, kind: Kind, value: Option<Rc<dyn Any>>) -> NodeId {
        let owner = self.owner.get();
        let node = Node {
            generation: 0,
            alive: true,
            kind,
            value,
            subscribers: Vec::new(),
            sources: Vec::new(),
            dirty: false,
            owner,
            owned: Vec::new(),
            cleanups: Vec::new(),
            contexts: Vec::new(),
        };
        let mut nodes = self.nodes.borrow_mut();
        let id = match self.free.borrow_mut().pop() {
            Some(index) => {
                let slot = &mut nodes[index as usize];
                let generation = slot.generation + 1;
                *slot = Node { generation, ..node };
                NodeId { index, generation }
            }
            None => {
                nodes.push(node);
                NodeId { index: (nodes.len() - 1) as u32, generation: 0 }
            }
        };
        if let Some(owner) = owner {
            if let Some(owner) = Self::node_mut(&mut nodes, owner) {
                owner.owned.push(id);
            }
        }
        id
    }

    fn node_mut(nodes: &mut [Node], id: NodeId) -> Option<&mut Node> {
        nodes.get_mut(id.index as usize).filter(|node| node.alive && node.generation == id.generation)
    }

    fn is_alive(&self, id: NodeId) -> bool {
        let nodes = self.nodes.borrow();
        nodes.get(id.index as usize).is_some_and(|node| node.alive && node.generation == id.generation)
    }

    fn value(&self, id: NodeId) -> Rc<dyn Any> {
        let nodes = self.nodes.borrow();
        match nodes.get(id.index as usize) {
            Some(node) if node.alive && node.generation == id.generation => {
                node.value.clone().expect("the node holds a value")
            }
            _ => panic!("a signal or memo was used after its owner was disposed"),
        }
    }

    /// Records that the running memo or effect read `source`.
    fn track(&self, source: NodeId) {
        let Some(observer) = self.observer.get() else { return };
        let mut nodes = self.nodes.borrow_mut();
        let Some(node) = Self::node_mut(&mut nodes, source) else { return };
        if !node.subscribers.contains(&observer) {
            node.subscribers.push(observer);
        }
        if let Some(observer) = Self::node_mut(&mut nodes, observer) {
            if !observer.sources.contains(&source) {
                observer.sources.push(source);
            }
        }
    }

    /// Marks what reads `id` as dirty and runs the effects that are, unless
    /// a batch or a run is in progress.
    fn notify(&self, id: NodeId) {
        let subscribers = {
            let mut nodes = self.nodes.borrow_mut();
            Self::node_mut(&mut nodes, id).map(|node| node.subscribers.clone()).unwrap_or_default()
        };
        for subscriber in subscribers {
            self.mark_dirty(subscriber);
        }
        self.flush();
    }

    fn mark_dirty(&self, id: NodeId) {
        let (is_effect, subscribers) = {
            let mut nodes = self.nodes.borrow_mut();
            let Some(node) = Self::node_mut(&mut nodes, id) else { return };
            if node.dirty {
                return;
            }
            node.dirty = true;
            match node.kind {
                Kind::Effect(_) => (true, Vec::new()),
                Kind::Memo(_) => (false, node.subscribers.clone()),
                _ => (false, Vec::new()),
            }
        };
        if is_effect {
            self.queue.borrow_mut().push_back(id);
        }
        for subscriber in subscribers {
            self.mark_dirty(subscriber);
        }
    }

    fn flush(&self) {
        if self.batch_depth.get() > 0 || self.flushing.get() {
            return;
        }
        self.flushing.set(true);
        self.propagation.set(self.propagation.get() + 1);
        loop {
            let next = self.queue.borrow_mut().pop_front();
            let Some(id) = next else { break };
            let dirty = {
                let mut nodes = self.nodes.borrow_mut();
                Self::node_mut(&mut nodes, id).is_some_and(|node| node.dirty)
            };
            if dirty {
                self.run(id);
            }
        }
        self.flushing.set(false);
    }

    /// Runs a memo or an effect: what it made and read before is let go, and
    /// it is the observer and the owner while it runs.
    fn run(&self, id: NodeId) -> bool {
        self.dispose_owned(id);
        self.clear_sources(id);
        let kind = {
            let mut nodes = self.nodes.borrow_mut();
            let Some(node) = Self::node_mut(&mut nodes, id) else { return false };
            node.dirty = false;
            match &node.kind {
                Kind::Memo(compute) => Some(Ok(compute.clone())),
                Kind::Effect(run) => Some(Err(run.clone())),
                _ => None,
            }
        };
        let previous = (self.observer.replace(Some(id)), self.owner.replace(Some(id)));
        let changed = match kind {
            Some(Ok(compute)) => compute(),
            Some(Err(run)) => {
                run();
                true
            }
            None => false,
        };
        self.observer.set(previous.0);
        self.owner.set(previous.1);
        changed
    }

    fn clear_sources(&self, id: NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        let sources = match Self::node_mut(&mut nodes, id) {
            Some(node) => std::mem::take(&mut node.sources),
            None => return,
        };
        for source in sources {
            if let Some(source) = Self::node_mut(&mut nodes, source) {
                source.subscribers.retain(|subscriber| *subscriber != id);
            }
        }
    }

    fn dispose_owned(&self, id: NodeId) {
        let (owned, cleanups) = {
            let mut nodes = self.nodes.borrow_mut();
            match Self::node_mut(&mut nodes, id) {
                Some(node) => (std::mem::take(&mut node.owned), std::mem::take(&mut node.cleanups)),
                None => return,
            }
        };
        for cleanup in cleanups.into_iter().rev() {
            cleanup();
        }
        for child in owned.into_iter().rev() {
            self.dispose(child);
        }
    }

    fn dispose(&self, id: NodeId) {
        if !self.is_alive(id) {
            return;
        }
        self.dispose_owned(id);
        self.clear_sources(id);
        let (subscribers, value, owner) = {
            let mut nodes = self.nodes.borrow_mut();
            let Some(node) = Self::node_mut(&mut nodes, id) else { return };
            node.alive = false;
            node.kind = Kind::Owner;
            node.contexts.clear();
            (std::mem::take(&mut node.subscribers), node.value.take(), node.owner.take())
        };
        {
            let mut nodes = self.nodes.borrow_mut();
            // An owner that lives on forgets the node.
            if let Some(owner) = owner.and_then(|owner| Self::node_mut(&mut nodes, owner)) {
                owner.owned.retain(|owned| *owned != id);
            }
            for subscriber in subscribers {
                if let Some(subscriber) = Self::node_mut(&mut nodes, subscriber) {
                    subscriber.sources.retain(|source| *source != id);
                }
            }
        }
        // The value is dropped outside the borrow: dropping it may run code.
        drop(value);
        self.free.borrow_mut().push(id.index);
    }
}

/// A value that the reactive closures reading it follow. `Copy`: a `move`
/// closure takes it without a clone.
pub struct Signal<T: 'static> {
    id: NodeId,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Signal<T> {}

impl<T: 'static> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signal({:?})", self.id)
    }
}

/// Creates a signal that holds `value`, owned by the current owner.
pub fn signal<T: 'static>(value: T) -> Signal<T> {
    let id = with_runtime(|runtime| runtime.create(Kind::Signal, Some(Rc::new(RefCell::new(value)))));
    Signal { id, _marker: PhantomData }
}

impl<T: 'static> Signal<T> {
    fn cell(&self) -> Rc<RefCell<T>> {
        with_runtime(|runtime| runtime.value(self.id))
            .downcast::<RefCell<T>>()
            .unwrap_or_else(|_| panic!("a signal holds a value of its type"))
    }

    /// Calls `f` with the value, and records the read when a memo or an
    /// effect is running.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        with_runtime(|runtime| runtime.track(self.id));
        self.with_untracked(f)
    }

    /// Calls `f` with the value without recording the read.
    pub fn with_untracked<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let cell = self.cell();
        let value = cell.borrow();
        f(&value)
    }

    /// A copy of the value; the read is recorded.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    /// A copy of the value; the read is not recorded.
    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        self.with_untracked(T::clone)
    }

    /// Replaces the value, and runs what reads it.
    pub fn set(&self, value: T) {
        *self.cell().borrow_mut() = value;
        with_runtime(|runtime| runtime.notify(self.id));
    }

    /// Replaces the value when it differs from the one held; a value that is
    /// equal changes nothing and runs nothing.
    pub fn set_if_changed(&self, value: T)
    where
        T: PartialEq,
    {
        let changed = self.with_untracked(|current| *current != value);
        if changed {
            self.set(value);
        }
    }

    /// Changes the value in place, and runs what reads it.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.cell().borrow_mut());
        with_runtime(|runtime| runtime.notify(self.id));
    }

    /// Takes the value, leaving the default, and runs what reads it.
    pub fn take(&self) -> T
    where
        T: Default,
    {
        let value = std::mem::take(&mut *self.cell().borrow_mut());
        with_runtime(|runtime| runtime.notify(self.id));
        value
    }

    /// The signal as a two-way value of a property: the property follows the
    /// signal, and a change of the property (the user typed, moved a slider)
    /// is written into the signal.
    pub fn two_way(self) -> TwoWay<T> {
        TwoWay(self)
    }

    /// Whether the owner of the signal is still alive.
    pub fn is_alive(&self) -> bool {
        with_runtime(|runtime| runtime.is_alive(self.id))
    }

    /// The identity of the signal: what values that follow it are compared
    /// by.
    pub(crate) fn source_id(&self) -> u64 {
        self.id.as_u64()
    }
}

/// A signal used as a two-way value of a property (see [`Signal::two_way`]).
pub struct TwoWay<T: 'static>(pub(crate) Signal<T>);

/// A value computed from signals and memos, computed again when one of them
/// changes and read like a signal. `Copy`.
pub struct Memo<T: 'static> {
    id: NodeId,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Memo<T> {}

/// Creates a memo of `compute`, owned by the current owner. It is computed
/// when it is first read; `compute` gets the previous value.
pub fn memo<T: PartialEq + 'static>(compute: impl Fn(Option<&T>) -> T + 'static) -> Memo<T> {
    let cell: Rc<RefCell<Option<T>>> = Rc::new(RefCell::new(None));
    let target = cell.clone();
    let recompute = move || {
        let next = {
            let previous = target.borrow();
            compute(previous.as_ref())
        };
        let changed = target.borrow().as_ref() != Some(&next);
        if changed {
            *target.borrow_mut() = Some(next);
        }
        changed
    };
    let id = with_runtime(|runtime| {
        let id = runtime.create(Kind::Memo(Rc::new(recompute)), Some(cell));
        // Dirty until it is first read.
        let mut nodes = runtime.nodes.borrow_mut();
        if let Some(node) = Runtime::node_mut(&mut nodes, id) {
            node.dirty = true;
        }
        id
    });
    Memo { id, _marker: PhantomData }
}

impl<T: 'static> Memo<T> {
    /// Calls `f` with the value, computing it first when it is dirty; the read
    /// is recorded.
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let dirty = with_runtime(|runtime| {
            runtime.track(self.id);
            let mut nodes = runtime.nodes.borrow_mut();
            Runtime::node_mut(&mut nodes, self.id).is_some_and(|node| node.dirty)
        });
        if dirty {
            with_runtime(|runtime| runtime.run(self.id));
        }
        let cell = with_runtime(|runtime| runtime.value(self.id))
            .downcast::<RefCell<Option<T>>>()
            .unwrap_or_else(|_| panic!("a memo holds a value of its type"));
        let value = cell.borrow();
        f(value.as_ref().expect("a memo that was read is computed"))
    }

    /// A copy of the value; the read is recorded.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    /// A copy of the value; the read is not recorded.
    pub fn get_untracked(&self) -> T
    where
        T: Clone,
    {
        untrack(|| self.get())
    }

    /// The identity of the memo: what values that follow it are compared by.
    pub(crate) fn source_id(&self) -> u64 {
        self.id.as_u64()
    }
}

/// Runs `f` now and again whenever a signal or memo it read changes, until
/// its owner is disposed. What `f` makes while it runs is owned by the effect
/// and disposed before it runs again.
pub fn effect(f: impl Fn() + 'static) {
    with_runtime(|runtime| {
        let id = runtime.create(Kind::Effect(Rc::new(f)), None);
        runtime.run(id);
    });
}

/// Runs `f` once the current owner (or the running effect) is disposed or
/// runs again.
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    with_runtime(|runtime| {
        let Some(owner) = runtime.owner.get() else { return };
        let mut nodes = runtime.nodes.borrow_mut();
        if let Some(node) = Runtime::node_mut(&mut nodes, owner) {
            node.cleanups.push(Box::new(f));
        }
    });
}

/// The number of the propagation of changes in progress (or the last one):
/// what the diagnostics count the runs of a component in.
pub(crate) fn propagation() -> u64 {
    with_runtime(|runtime| runtime.propagation.get())
}

/// The state of the runtime around a run that may panic: the observer, the
/// owner, the depth of batches and whether changes are propagating. A panic
/// that unwinds through `Owner::with`, `untrack`, `batch` or a nested run
/// leaves them as they were inside; an error boundary puts them back.
pub(crate) struct Scope {
    observer: Option<NodeId>,
    owner: Option<NodeId>,
    batch_depth: u32,
    flushing: bool,
}

impl Scope {
    /// The state of the runtime now.
    pub(crate) fn save() -> Self {
        with_runtime(|runtime| Scope {
            observer: runtime.observer.get(),
            owner: runtime.owner.get(),
            batch_depth: runtime.batch_depth.get(),
            flushing: runtime.flushing.get(),
        })
    }

    /// Puts the runtime back into the saved state.
    pub(crate) fn restore(self) {
        with_runtime(|runtime| {
            runtime.observer.set(self.observer);
            runtime.owner.set(self.owner);
            runtime.batch_depth.set(self.batch_depth);
            runtime.flushing.set(self.flushing);
        });
    }
}

/// Runs `f` without recording what it reads.
pub fn untrack<R>(f: impl FnOnce() -> R) -> R {
    let previous = with_runtime(|runtime| runtime.observer.replace(None));
    let result = f();
    with_runtime(|runtime| runtime.observer.set(previous));
    result
}

/// Runs `f`, and runs what its writes changed once, when it returns.
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    with_runtime(|runtime| runtime.batch_depth.set(runtime.batch_depth.get() + 1));
    let result = f();
    with_runtime(|runtime| {
        runtime.batch_depth.set(runtime.batch_depth.get() - 1);
        runtime.flush();
    });
    result
}

/// What owns signals, memos and effects: they are disposed with it. A
/// mounted element tree has one; each item of a dynamic list has its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Owner {
    id: NodeId,
}

impl Owner {
    /// A new owner, owned by the current one (if any): disposed with it.
    pub fn new() -> Self {
        Owner { id: with_runtime(|runtime| runtime.create(Kind::Owner, None)) }
    }

    /// A new owner that no other owner disposes: the root of a tree.
    pub fn new_root() -> Self {
        let previous = with_runtime(|runtime| runtime.owner.replace(None));
        let owner = Self::new();
        with_runtime(|runtime| runtime.owner.set(previous));
        owner
    }

    /// The owner of what is made now, if any.
    pub fn current() -> Option<Self> {
        with_runtime(|runtime| runtime.owner.get()).map(|id| Owner { id })
    }

    /// Runs `f` with this owner current (and no observer: `f` itself is not
    /// reactive).
    pub fn with<R>(&self, f: impl FnOnce() -> R) -> R {
        let previous = with_runtime(|runtime| (runtime.owner.replace(Some(self.id)), runtime.observer.replace(None)));
        let result = f();
        with_runtime(|runtime| {
            runtime.owner.set(previous.0);
            runtime.observer.set(previous.1);
        });
        result
    }

    /// Disposes the owner and everything it owns; cleanups run.
    pub fn dispose(self) {
        with_runtime(|runtime| runtime.dispose(self.id));
    }

    /// Whether the owner was not disposed.
    pub fn is_alive(&self) -> bool {
        with_runtime(|runtime| runtime.is_alive(self.id))
    }
}

impl Owner {
    /// Provides `value` to what this owner owns, at any depth: [`use_context`]
    /// finds it from an owner below. A value of the same type provided again
    /// replaces it.
    pub fn provide_context<T: Clone + 'static>(&self, value: T) {
        with_runtime(|runtime| {
            let mut nodes = runtime.nodes.borrow_mut();
            if let Some(node) = Runtime::node_mut(&mut nodes, self.id) {
                let type_id = TypeId::of::<T>();
                node.contexts.retain(|(id, _)| *id != type_id);
                node.contexts.push((type_id, Rc::new(value)));
            }
        });
    }

    /// The value of type `T` provided to this owner or to the nearest owner
    /// above it.
    pub fn context<T: Clone + 'static>(&self) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let value = with_runtime(|runtime| {
            let nodes = runtime.nodes.borrow();
            let mut current = Some(self.id);
            while let Some(id) = current {
                let node = nodes.get(id.index as usize).filter(|node| node.alive && node.generation == id.generation)?;
                if let Some((_, value)) = node.contexts.iter().find(|(id, _)| *id == type_id) {
                    return Some(value.clone());
                }
                current = node.owner;
            }
            None
        })?;
        value.downcast_ref::<T>().cloned()
    }
}

/// Provides `value` to what the current owner owns (see
/// [`Owner::provide_context`]). In the render of a component that runs again
/// it is provided to the component, so that its children find it. Without an
/// owner it does nothing.
pub fn provide_context<T: Clone + 'static>(value: T) {
    if let Some(owner) = crate::hooks::render_owner().or_else(Owner::current) {
        owner.provide_context(value);
    }
}

/// The value of type `T` provided by [`provide_context`] to the current
/// owner or an owner above it.
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    Owner::current().and_then(|owner| owner.context::<T>())
}

impl Default for Owner {
    fn default() -> Self {
        Self::new()
    }
}
