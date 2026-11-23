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
    /// The memos and effects that read the node. A long list also holds what left it since it
    /// was last swept (`stale`): see [`Runtime::leave`].
    subscribers: Vec<NodeId>,
    /// `subscribers` holds entries of nodes that follow the node no more.
    stale: bool,
    /// Some of those nodes may be alive (they stopped reading the node): only a look at what
    /// each subscriber follows tells them from the others.
    stale_followers: bool,
    /// The signals and memos the node read when it last ran; while it runs, those it has read
    /// in this run.
    sources: Vec<NodeId>,
    /// While the node runs: what it read in the run before. The first `kept` of them it has read
    /// again (they are in `sources` too); the rest it still follows, until the run ends without
    /// reading them. Empty between runs.
    previous: Vec<NodeId>,
    kept: usize,
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
            stale: false,
            stale_followers: false,
            sources: Vec::new(),
            previous: Vec::new(),
            kept: 0,
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
            Self::own(&mut nodes, owner, id);
        }
        id
    }

    /// Adds `id` to what `owner` owns. A long list keeps the nodes that were disposed one by
    /// one (see [`dispose`](Self::dispose)); they are taken out here before the list would grow,
    /// so the list is never longer than twice what is alive in it and what it had room for.
    #[inline]
    fn own(nodes: &mut [Node], owner: NodeId, id: NodeId) {
        let Some(node) = Self::node_mut(nodes, owner) else { return };
        if node.owned.len() == node.owned.capacity() && node.owned.len() > Self::SHORT_LIST {
            Self::sweep_owned(nodes, owner);
            let Some(node) = Self::node_mut(nodes, owner) else { return };
            node.owned.push(id);
            return;
        }
        node.owned.push(id);
    }

    /// Takes the disposed nodes out of what `owner` owns.
    #[cold]
    #[inline(never)]
    fn sweep_owned(nodes: &mut [Node], owner: NodeId) {
        let Some(node) = Self::node_mut(nodes, owner) else { return };
        let mut owned = std::mem::take(&mut node.owned);
        owned.retain(|owned| Self::node_mut(nodes, *owned).is_some());
        if let Some(node) = Self::node_mut(nodes, owner) {
            node.owned = owned;
        }
    }

    /// A list up to this long is searched for the entry that leaves it; a longer one keeps the
    /// entry until it is swept.
    const SHORT_LIST: usize = 16;

    /// `subscriber` follows `source` no more: it stopped reading it (`alive`), or it is being
    /// disposed.
    ///
    /// A list of a few subscribers is searched and the entry taken out. A signal every element of
    /// a screen reads has thousands, and a search for each one that leaves costs the size of the
    /// screen for each element that goes: a screen that is disposed, or a list whose rows leave,
    /// cost the square of its size. The entry of a long list stays where it is and the list is
    /// marked: [`swept`](Self::sweep) before the source notifies, before a subscriber is added to
    /// a full list, and before a subscriber that may have an entry in it follows it again. A
    /// disposed node is told from the others by its generation, a node that stopped reading by
    /// what it follows, so nothing is recorded for an entry.
    #[inline]
    fn leave(nodes: &mut [Node], source: NodeId, subscriber: NodeId, alive: bool) {
        let Some(node) = Self::node_mut(nodes, source) else { return };
        if node.subscribers.len() <= Self::SHORT_LIST {
            node.subscribers.retain(|known| *known != subscriber);
        } else {
            node.stale = true;
            node.stale_followers |= alive;
        }
    }

    /// Takes out of the subscribers of `source` the entries of what left it ([`leave`](Self::leave)),
    /// and every entry of `adding`, a node that is about to be added: an entry it has is one it
    /// left, whatever it follows by now.
    #[cold]
    #[inline(never)]
    fn sweep(nodes: &mut [Node], source: NodeId, adding: Option<NodeId>) {
        let Some(node) = Self::node_mut(nodes, source) else { return };
        let followers = std::mem::take(&mut node.stale_followers);
        node.stale = false;
        let mut subscribers = std::mem::take(&mut node.subscribers);
        subscribers.retain(|subscriber| {
            let Some(node) = nodes.get(subscriber.index as usize) else { return false };
            if !node.alive || node.generation != subscriber.generation || Some(*subscriber) == adding {
                return false;
            }
            // While a node runs, what it has not read again yet is followed still.
            let unread = node.previous.get(node.kept..);
            !followers || node.sources.contains(&source) || unread.is_some_and(|unread| unread.contains(&source))
        });
        if let Some(node) = Self::node_mut(nodes, source) {
            node.subscribers = subscribers;
        }
    }

    /// Adds `subscriber`, which did not follow `source` until now, to the subscribers of
    /// `source`.
    #[inline]
    fn subscribe(nodes: &mut [Node], source: NodeId, subscriber: NodeId) {
        let Some(node) = Self::node_mut(nodes, source) else { return };
        // An entry of the subscriber itself may be among what left, and a full list is swept
        // before it grows.
        if node.stale && (node.stale_followers || node.subscribers.len() == node.subscribers.capacity()) {
            Self::sweep(nodes, source, Some(subscriber));
            let Some(node) = Self::node_mut(nodes, source) else { return };
            node.subscribers.push(subscriber);
            return;
        }
        node.subscribers.push(subscriber);
    }

    /// [`subscribers_of`](Self::subscribers_of) as a call: marking an effect dirty, which is
    /// done for every subscriber of a write, does not carry the code of a memo.
    #[inline(never)]
    fn subscribers_of_memo(nodes: &mut [Node], id: NodeId) -> Vec<NodeId> {
        Self::subscribers_of(nodes, id)
    }

    /// The subscribers of `id` to notify: without what left it.
    #[inline]
    fn subscribers_of(nodes: &mut [Node], id: NodeId) -> Vec<NodeId> {
        let Some(node) = Self::node_mut(nodes, id) else { return Vec::new() };
        if node.stale {
            Self::sweep(nodes, id, None);
            return Self::node_mut(nodes, id).map(|node| node.subscribers.clone()).unwrap_or_default();
        }
        node.subscribers.clone()
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
    ///
    /// A node is among the subscribers of exactly what it has as a source (in `sources`, or in
    /// what is left of `previous` while it runs), so whether it follows `source` already is a
    /// question to its own few sources and never to the subscribers of `source`, which for a
    /// signal every element of a screen reads are thousands. A run that reads what the run
    /// before read, in the same order, touches no list of subscribers at all.
    fn track(&self, source: NodeId) {
        let Some(observer) = self.observer.get() else { return };
        let mut nodes = self.nodes.borrow_mut();
        if Self::node_mut(&mut nodes, source).is_none() {
            return;
        }
        let Some(node) = Self::node_mut(&mut nodes, observer) else { return };
        if node.sources.contains(&source) {
            return;
        }
        // Read in the run before: the node follows it already. The next of them in the order of
        // that run is the usual case; any other is moved there.
        let followed = match node.previous[node.kept..].iter().position(|previous| *previous == source) {
            Some(offset) => {
                let kept = node.kept;
                node.previous.swap(kept, kept + offset);
                node.kept += 1;
                true
            }
            None => false,
        };
        node.sources.push(source);
        if !followed {
            Self::subscribe(&mut nodes, source, observer);
        }
    }

    /// The beginning of a run of `id`: what it read before becomes what it may read again.
    fn begin_tracking(&self, id: NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        let Some(node) = Self::node_mut(&mut nodes, id) else { return };
        // What the run before left in `previous` is nothing, unless it did not end (a panic
        // unwound through it): then it is what that run had not read again, which is followed
        // still. The two lists change places, so a run fills the list of two runs ago and no
        // run of an effect that reads what it read before asks for memory.
        let kept = std::mem::take(&mut node.kept);
        let read = kept.min(node.previous.len());
        node.previous.drain(..read);
        std::mem::swap(&mut node.previous, &mut node.sources);
        let Node { previous, sources, .. } = node;
        previous.append(sources);
    }

    /// The end of a run of `id`: what the run did not read again is followed no more.
    fn end_tracking(&self, id: NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        let Some(node) = Self::node_mut(&mut nodes, id) else { return };
        let kept = std::mem::take(&mut node.kept);
        let mut unread = std::mem::take(&mut node.previous);
        if kept == unread.len() {
            // Every source read again: the list is kept for the next run, emptied.
            unread.clear();
            node.previous = unread;
            return;
        }
        for source in unread.drain(kept..) {
            Self::leave(&mut nodes, source, id, true);
        }
        unread.clear();
        if let Some(node) = Self::node_mut(&mut nodes, id) {
            node.previous = unread;
        }
    }

    /// Marks what reads `id` as dirty and runs the effects that are, unless
    /// a batch or a run is in progress.
    fn notify(&self, id: NodeId) {
        let subscribers = Self::subscribers_of(&mut self.nodes.borrow_mut(), id);
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
                Kind::Memo(_) => (false, Self::subscribers_of_memo(&mut nodes, id)),
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
        self.begin_tracking(id);
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
        self.end_tracking(id);
        changed
    }

    fn clear_sources(&self, id: NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        let sources = match Self::node_mut(&mut nodes, id) {
            Some(node) => {
                // A node disposed while it runs also follows what it has not read again yet.
                let kept = std::mem::take(&mut node.kept);
                let mut sources = std::mem::take(&mut node.sources);
                sources.extend(std::mem::take(&mut node.previous).into_iter().skip(kept));
                sources
            }
            None => return,
        };
        for source in sources {
            Self::leave(&mut nodes, source, id, false);
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
            // An owner that lives on forgets the node. An owner of many (a list of rows) keeps
            // the entry, which is of no node once this one is disposed, until the list would
            // grow (`own`): rows that leave one by one do not each cost the number of rows.
            if let Some(owner) = owner.and_then(|owner| Self::node_mut(&mut nodes, owner)) {
                if owner.owned.len() <= Self::SHORT_LIST {
                    owner.owned.retain(|owned| *owned != id);
                }
            }
            for subscriber in subscribers {
                if let Some(subscriber) = Self::node_mut(&mut nodes, subscriber) {
                    subscriber.sources.retain(|source| *source != id);
                    // A subscriber that is running may have the node among what it read before.
                    if let Some(index) = subscriber.previous.iter().position(|source| *source == id) {
                        subscriber.previous.remove(index);
                        if index < subscriber.kept {
                            subscriber.kept -= 1;
                        }
                    }
                }
            }
        }
        // The value is dropped outside the borrow: dropping it may run code.
        drop(value);
        self.free.borrow_mut().push(id.index);
    }
}

/// How many memos and effects follow `signal` (tests).
#[cfg(test)]
pub(crate) fn subscriber_count<T>(signal: &Signal<T>) -> usize {
    with_runtime(|runtime| {
        let mut nodes = runtime.nodes.borrow_mut();
        Runtime::subscribers_of(&mut nodes, signal.id).len()
    })
}

/// How many nodes of the runtime of the thread are alive: its signals, memos, effects and owners
/// (tests).
#[cfg(test)]
pub(crate) fn live_nodes() -> usize {
    with_runtime(|runtime| runtime.nodes.borrow().iter().filter(|node| node.alive).count())
}

/// The size of a node of the runtime, which every signal, memo, effect and owner is (tests).
#[cfg(test)]
pub(crate) fn node_size() -> usize {
    std::mem::size_of::<Node>()
}

/// How many entries the list of subscribers of `signal` holds, with those of what left it and
/// was not taken out yet (tests).
#[cfg(test)]
pub(crate) fn subscriber_entries<T>(signal: &Signal<T>) -> usize {
    with_runtime(|runtime| {
        let mut nodes = runtime.nodes.borrow_mut();
        Runtime::node_mut(&mut nodes, signal.id).map_or(0, |node| node.subscribers.len())
    })
}

/// How many entries the list of what `owner` owns holds, with those of nodes that were disposed
/// and not taken out yet (tests).
#[cfg(test)]
pub(crate) fn owned_entries(owner: &Owner) -> usize {
    with_runtime(|runtime| {
        let mut nodes = runtime.nodes.borrow_mut();
        Runtime::node_mut(&mut nodes, owner.id).map_or(0, |node| node.owned.len())
    })
}

/// Where the two lists of sources of each memo and effect that follows `signal` are in memory:
/// the list of what it read in its last run, and the list its next run fills (tests).
#[cfg(test)]
pub(crate) fn tracking_lists<T>(signal: &Signal<T>) -> Vec<(usize, usize)> {
    with_runtime(|runtime| {
        let mut nodes = runtime.nodes.borrow_mut();
        let subscribers = Runtime::subscribers_of(&mut nodes, signal.id);
        subscribers
            .into_iter()
            .filter_map(|subscriber| {
                let node = Runtime::node_mut(&mut nodes, subscriber)?;
                Some((node.sources.as_ptr() as usize, node.previous.as_ptr() as usize))
            })
            .collect()
    })
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
