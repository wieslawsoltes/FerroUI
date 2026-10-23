//! Completion of groups of animations: WinUI's `CompositionScopedBatch` and
//! `CompositionCommitBatch`, with the `Compositor` factories
//! `CreateScopedBatch` and `GetCommitBatch` (and `CreatePropertySet`).
//!
//! # How completion is tracked
//!
//! A batch is a counter shared between the threads (`BatchShared`, `Send +
//! Sync`): the number of animations of the batch that are still running, and
//! whether the batch is ended. Every key frame animation that is started while
//! batches are open takes a [`BatchCompletionGuard`] of each of them (gathered
//! in an [`AnimationCompletion`]); the guard travels with the factory of the
//! animation instance to the server and is released there when the animation
//! finishes, or when the instance is dropped (stopped, replaced by another
//! animation of the same property, its target disposed). Releasing is
//! dropping, so it happens once.
//!
//! A batch completes when it is ended and none of its animations is running.
//! The thread that completes it (the render thread, or this one when the last
//! guard is dropped here) cannot touch the UI-thread objects: it posts the
//! completion to the dispatcher of the compositor, which looks the handlers of
//! the batch up in a registry of the thread by the id of the batch and raises
//! `Completed`. As in WinUI, `Completed` is raised asynchronously, also for a
//! batch that is ended with no animations, so a handler subscribed right after
//! `End` is called.
//!
//! The registry holds the handler list of a batch from `End` (for a commit
//! batch: from `GetCommitBatch`) until completion: the application does not
//! have to keep the batch alive to get the event, as in WinUI.

use super::{AsCompositionObject, CompositionObject, CompositionPropertySet, Compositor};
use crate::reactive::{Disposable, IDisposable};
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::utilities::HandlerList;
use bitflags::bitflags;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

bitflags! {
    /// Batch types for [`CompositionCommitBatch`] and
    /// [`CompositionScopedBatch`] (WinUI `CompositionBatchTypes`, a flags
    /// enumeration).
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct CompositionBatchTypes: u32 {
        /// The batch contains animations.
        const ANIMATION = 1;
        /// The batch contains effects.
        const EFFECT = 2;
        /// The batch contains an infinite animation.
        const INFINITE_ANIMATION = 4;
        /// The batch contains all animations.
        const ALL_ANIMATIONS = 5;
    }
}

impl CompositionBatchTypes {
    /// None.
    pub const NONE: CompositionBatchTypes = CompositionBatchTypes::empty();
}

/// The arguments of the `Completed` event of a batch (WinUI
/// `CompositionBatchCompletedEventArgs`, which has no members of its own).
#[derive(Clone, Copy, Debug)]
pub struct CompositionBatchCompletedEventArgs {
    _private: (),
}

/// The handlers of the `Completed` event of a batch.
type CompletedHandlers = HandlerList<dyn Fn(&CompositionBatchCompletedEventArgs)>;

static NEXT_BATCH_ID: AtomicU64 = AtomicU64::new(1);

crate::ferro_thread_local! {
    /// The handlers of the batches of this thread that are ended and not
    /// completed yet, by batch id: where a completion posted from another
    /// thread finds them.
    static PENDING_COMPLETIONS: RefCell<HashMap<u64, Rc<CompletedHandlers>>> = RefCell::new(HashMap::new());
}

/// Raises `Completed` of the batch `id`: runs on the thread of the batch,
/// posted by `BatchShared::check_completed`.
fn raise_completed(id: u64) {
    let handlers = PENDING_COMPLETIONS
        .try_with(|pending| pending.try_borrow_mut().ok().and_then(|mut pending| pending.remove(&id)))
        .ok()
        .flatten();
    let Some(handlers) = handlers else { return };
    let args = CompositionBatchCompletedEventArgs { _private: () };
    for (_, handler) in handlers.snapshot().iter() {
        handler(&args);
    }
}

fn register_pending_completion(id: u64, handlers: Rc<CompletedHandlers>) {
    PENDING_COMPLETIONS.with(|pending| {
        pending.borrow_mut().insert(id, handlers);
    });
}

/// Subscribes `handler` to `handlers`; disposing the result unsubscribes.
fn subscribe(
    handlers: &Rc<CompletedHandlers>,
    handler: impl Fn(&CompositionBatchCompletedEventArgs) + 'static,
) -> Rc<dyn IDisposable> {
    let token = handlers.add(Rc::new(handler));
    let weak = Rc::downgrade(handlers);
    Disposable::create(move || {
        if let Some(handlers) = weak.upgrade() {
            handlers.remove(token);
        }
    })
}

struct BatchState {
    /// The batch types whose animations the completion waits for.
    types: CompositionBatchTypes,
    /// Running animations of the batch that end by themselves.
    finite: usize,
    /// Running infinite animations of the batch.
    infinite: usize,
    ended: bool,
    completed: bool,
    /// Whether a UI-thread batch object waits for the completion: the batch
    /// of a frame nobody asked for completes silently.
    observed: bool,
}

/// The state of a batch that the threads share: what the guards of its
/// animations count down.
pub(crate) struct BatchShared {
    id: u64,
    dispatcher: Arc<Dispatcher>,
    state: Mutex<BatchState>,
}

impl BatchShared {
    fn new(dispatcher: Arc<Dispatcher>, types: CompositionBatchTypes, observed: bool) -> Arc<BatchShared> {
        Arc::new(BatchShared {
            id: NEXT_BATCH_ID.fetch_add(1, Ordering::Relaxed),
            dispatcher,
            state: Mutex::new(BatchState {
                types,
                finite: 0,
                infinite: 0,
                ended: false,
                completed: false,
                observed,
            }),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, BatchState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn acquire(self: &Arc<Self>, infinite: bool) -> BatchCompletionGuard {
        {
            let mut state = self.lock();
            if infinite {
                state.infinite += 1;
            } else {
                state.finite += 1;
            }
        }
        BatchCompletionGuard { shared: self.clone(), infinite }
    }

    fn release(&self, infinite: bool) {
        let mut state = self.lock();
        if infinite {
            state.infinite = state.infinite.saturating_sub(1);
        } else {
            state.finite = state.finite.saturating_sub(1);
        }
        self.check_completed(state);
    }

    fn end(&self) {
        let mut state = self.lock();
        state.ended = true;
        self.check_completed(state);
    }

    fn is_ended(&self) -> bool {
        self.lock().ended
    }

    /// Marks the batch as observed by a UI-thread object waiting for the
    /// given types.
    fn observe(&self, types: CompositionBatchTypes) {
        let mut state = self.lock();
        state.observed = true;
        state.types |= types;
    }

    /// Completes the batch when it is ended and none of the animations it
    /// waits for is running, posting `Completed` to the thread of the batch.
    fn check_completed(&self, mut state: std::sync::MutexGuard<'_, BatchState>) {
        if state.completed || !state.ended {
            return;
        }
        let mut running = 0;
        if state.types.contains(CompositionBatchTypes::ANIMATION) {
            running += state.finite;
        }
        if state.types.contains(CompositionBatchTypes::INFINITE_ANIMATION) {
            running += state.infinite;
        }
        if running != 0 {
            return;
        }
        state.completed = true;
        let observed = state.observed;
        drop(state);
        if observed {
            let id = self.id;
            self.dispatcher.post(move || raise_completed(id), DispatcherPriority::NORMAL);
        }
    }
}

/// The share of one batch in a running animation: dropping it tells the
/// batch that the animation is over.
pub struct BatchCompletionGuard {
    shared: Arc<BatchShared>,
    infinite: bool,
}

impl Drop for BatchCompletionGuard {
    fn drop(&mut self) {
        self.shared.release(self.infinite);
    }
}

/// The guards of the batches a running animation belongs to. It is `Send`:
/// it is created on the UI thread with the animation and moved to the
/// server instance of the animation, which releases it when the animation
/// is over.
#[derive(Default)]
pub struct AnimationCompletion {
    guards: Vec<BatchCompletionGuard>,
}

impl AnimationCompletion {
    /// The completion of an animation that belongs to no batch.
    pub fn none() -> AnimationCompletion {
        AnimationCompletion::default()
    }

    /// Whether the animation belongs to no batch (any more).
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty()
    }

    /// Tells the batches that the animation is over. Releasing again does
    /// nothing.
    pub fn release(&mut self) {
        self.guards.clear();
    }
}

/// The batches of a compositor: the scoped batches that are open and the
/// batch of the frame that is not committed yet.
#[derive(Default)]
pub(crate) struct CompositorBatches {
    scoped: RefCell<Vec<Weak<CompositionScopedBatch>>>,
    /// The batch of the current frame: created with the first animation of
    /// the frame (or by `GetCommitBatch`) and ended by the commit.
    frame: RefCell<Option<Arc<BatchShared>>>,
    /// The UI-thread object of the batch of the current frame, once asked
    /// for.
    commit_batch: RefCell<Option<Rc<CompositionCommitBatch>>>,
    /// Whether a batch has ever been asked for: until then no completion
    /// is waited for, and disposing an object need not release any.
    used: Cell<bool>,
}

impl CompositorBatches {
    pub(crate) fn new() -> CompositorBatches {
        CompositorBatches::default()
    }

    fn frame(&self, dispatcher: &Arc<Dispatcher>) -> Arc<BatchShared> {
        self.frame
            .borrow_mut()
            .get_or_insert_with(|| BatchShared::new(dispatcher.clone(), CompositionBatchTypes::NONE, false))
            .clone()
    }

    /// Whether a scoped or commit batch has ever been asked for.
    pub(crate) fn used(&self) -> bool {
        self.used.get()
    }

    /// Ends the batch of the frame that is being committed; the next
    /// animation or `GetCommitBatch` starts the batch of the next frame.
    pub(crate) fn on_commit(&self) {
        let frame = self.frame.borrow_mut().take();
        let commit_batch = self.commit_batch.borrow_mut().take();
        if let Some(frame) = frame {
            frame.end();
        }
        drop(commit_batch);
    }
}

/// An explicitly created group of active animations or effects (WinUI
/// `CompositionScopedBatch`).
///
/// Every key frame animation started while the batch is active (created and
/// neither suspended nor ended) belongs to it, also the animations of a
/// group and implicit animations. Once [`end`](Self::end) has been called
/// and all its animations are over, the batch raises
/// [`completed`](Self::completed) on the thread of its compositor.
///
/// Several batches may be active at once; an animation belongs to each of
/// them.
pub struct CompositionScopedBatch {
    object: CompositionObject,
    shared: Arc<BatchShared>,
    is_suspended: Cell<bool>,
    is_ended: Cell<bool>,
    completed: Rc<CompletedHandlers>,
}

impl CompositionScopedBatch {
    /// The associated compositor.
    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// Whether the batch is open for animations to be aggregated: false
    /// once it is suspended or ended.
    pub fn is_active(&self) -> bool {
        !self.is_suspended.get() && !self.is_ended.get()
    }

    /// Whether the batch has been closed and can no longer accept changes.
    pub fn is_ended(&self) -> bool {
        self.is_ended.get()
    }

    /// Closes the batch. Once closed it cannot be suspended or resumed
    /// again. `Completed` is raised once all the animations of the batch are
    /// over, right away (asynchronously) when there are none. Ending a batch
    /// again does nothing.
    pub fn end(&self) {
        if self.is_ended.get() {
            return;
        }
        self.is_ended.set(true);
        let this: *const CompositionScopedBatch = self;
        self.compositor()
            .animation_batches()
            .scoped
            .borrow_mut()
            .retain(|batch| batch.strong_count() > 0 && !std::ptr::eq(batch.as_ptr(), this));
        register_pending_completion(self.shared.id, self.completed.clone());
        self.shared.end();
    }

    /// Suspends aggregating animations in the batch until
    /// [`resume`](Self::resume). Panics when the batch is ended (WinUI
    /// throws).
    pub fn suspend(&self) {
        if self.is_ended.get() {
            panic!("The batch has been ended and cannot be suspended");
        }
        self.is_suspended.set(true);
    }

    /// Resumes aggregating animations in the batch. Panics when the batch
    /// is ended (WinUI throws).
    pub fn resume(&self) {
        if self.is_ended.get() {
            panic!("The batch has been ended and cannot be resumed");
        }
        self.is_suspended.set(false);
    }

    /// Raised once all animations of the ended batch have completed.
    /// Disposing the returned handle unsubscribes.
    // Deviation (winui-composition.md): the handler gets the arguments only,
    // not the sender, as the other events of FerroUI; the batch need not be
    // kept alive by the application to raise the event.
    pub fn completed(&self, handler: impl Fn(&CompositionBatchCompletedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        subscribe(&self.completed, handler)
    }
}

impl AsCompositionObject for CompositionScopedBatch {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionScopedBatch"
    }
}

/// The group of the animations started in the current frame (WinUI
/// `CompositionCommitBatch`): it is ended when the compositor commits the
/// frame and raises [`completed`](Self::completed) once the animations
/// started in the frame are over.
// Deviation (winui-composition.md): there is one commit batch per frame;
// asking for it again with other types returns the same batch, which then
// waits for the union of the types.
pub struct CompositionCommitBatch {
    object: CompositionObject,
    shared: Arc<BatchShared>,
    completed: Rc<CompletedHandlers>,
}

impl CompositionCommitBatch {
    /// The associated compositor.
    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// Whether the batch can still aggregate animations: until the frame is
    /// committed.
    pub fn is_active(&self) -> bool {
        !self.shared.is_ended()
    }

    /// Whether the frame of the batch has been committed.
    pub fn is_ended(&self) -> bool {
        self.shared.is_ended()
    }

    /// Raised once all animations of the batch have completed. Disposing
    /// the returned handle unsubscribes.
    pub fn completed(&self, handler: impl Fn(&CompositionBatchCompletedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        subscribe(&self.completed, handler)
    }
}

impl AsCompositionObject for CompositionCommitBatch {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionCommitBatch"
    }
}

impl Compositor {
    /// Creates a scoped batch that aggregates the animations of the given
    /// types started from now on (WinUI `CreateScopedBatch`).
    pub fn create_scoped_batch(&self, batch_type: CompositionBatchTypes) -> Rc<CompositionScopedBatch> {
        let batch = Rc::new(CompositionScopedBatch {
            object: CompositionObject::new(&self.this_handle(), None),
            shared: BatchShared::new(self.dispatcher().clone(), batch_type, true),
            is_suspended: Cell::new(false),
            is_ended: Cell::new(false),
            completed: Rc::new(HandlerList::new()),
        });
        self.animation_batches().used.set(true);
        let mut scoped = self.animation_batches().scoped.borrow_mut();
        scoped.retain(|batch| batch.strong_count() > 0);
        scoped.push(Rc::downgrade(&batch));
        batch
    }

    /// The batch of the animations started in the current frame, which the
    /// next commit ends (WinUI `GetCommitBatch`). The animations started in
    /// the frame before the call belong to it too. A commit is requested, so
    /// the batch ends even if nothing else changes.
    pub fn get_commit_batch(&self, batch_type: CompositionBatchTypes) -> Rc<CompositionCommitBatch> {
        let batches = self.animation_batches();
        batches.used.set(true);
        let existing = batches.commit_batch.borrow().clone();
        if let Some(existing) = existing {
            existing.shared.observe(batch_type);
            return existing;
        }
        let shared = batches.frame(self.dispatcher());
        shared.observe(batch_type);
        let batch = Rc::new(CompositionCommitBatch {
            object: CompositionObject::new(&self.this_handle(), None),
            shared: shared.clone(),
            completed: Rc::new(HandlerList::new()),
        });
        register_pending_completion(shared.id, batch.completed.clone());
        *batches.commit_batch.borrow_mut() = Some(batch.clone());
        self.request_commit_async();
        batch
    }

    /// Creates a property set (WinUI `CreatePropertySet`).
    pub fn create_property_set(&self) -> Rc<CompositionPropertySet> {
        CompositionPropertySet::new(&self.this_handle())
    }

    /// The completion guards of an animation started now: one for each
    /// active scoped batch and one for the batch of the current frame.
    /// `infinite` is set for an animation that does not end by itself.
    ///
    /// Key frame animations take them when their instance is created.
    // Deviation (winui-composition.md): expression animations belong to no
    // batch, also not to a batch of infinite animations.
    pub(crate) fn acquire_animation_completion(&self, infinite: bool) -> AnimationCompletion {
        let batches = self.animation_batches();
        let mut guards = Vec::new();
        let scoped: Vec<Rc<CompositionScopedBatch>> =
            batches.scoped.borrow().iter().filter_map(Weak::upgrade).collect();
        for batch in scoped {
            if batch.is_active() {
                guards.push(batch.shared.acquire(infinite));
            }
        }
        guards.push(batches.frame(self.dispatcher()).acquire(infinite));
        AnimationCompletion { guards }
    }
}
