use super::BatchStreamData;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

static NEXT_SEQUENCE_ID: AtomicI64 = AtomicI64::new(1);

type Continuation = Box<dyn FnOnce() + Send>;

/// A one-shot completion signal that can be observed from any thread
/// (the role of the `Task` exposed by a batch upstream).
#[derive(Default)]
pub struct BatchCompletion {
    state: Mutex<CompletionState>,
    completed: Condvar,
}

#[derive(Default)]
struct CompletionState {
    completed: bool,
    continuations: Vec<Continuation>,
}

impl BatchCompletion {
    /// Whether the signal has been raised.
    pub fn is_completed(&self) -> bool {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).completed
    }

    /// Runs `continuation` when the signal is raised: synchronously on the
    /// raising thread, or right now if it has been raised already
    /// (`ContinueWith(.., ExecuteSynchronously)`).
    pub fn on_completed(&self, continuation: impl FnOnce() + Send + 'static) {
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if !state.completed {
                state.continuations.push(Box::new(continuation));
                return;
            }
        }
        continuation();
    }

    /// Blocks until the completion is set (`Task.Wait`): how the UI thread
    /// waits for the render thread at the synchronous points.
    pub fn wait(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        while !state.completed {
            state = self.completed.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }

    pub(crate) fn try_set(&self) {
        let continuations = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.completed {
                return;
            }
            state.completed = true;
            std::mem::take(&mut state.continuations)
        };
        self.completed.notify_all();
        for continuation in continuations {
            continuation();
        }
    }
}

/// Represents a group of composition changes that are applied by the server
/// compositor atomically. This is the handle the UI thread keeps; it can be
/// shared with any thread.
pub struct CompositionBatch {
    sequence_id: i64,
    accepted: BatchCompletion,
    processed: BatchCompletion,
    rendered: BatchCompletion,
}

impl CompositionBatch {
    pub(crate) fn new() -> Arc<CompositionBatch> {
        Arc::new(CompositionBatch {
            sequence_id: NEXT_SEQUENCE_ID.fetch_add(1, Ordering::SeqCst) + 1,
            accepted: BatchCompletion::default(),
            processed: BatchCompletion::default(),
            rendered: BatchCompletion::default(),
        })
    }

    /// The process-wide sequence number of the batch.
    pub fn sequence_id(&self) -> i64 {
        self.sequence_id
    }

    /// Indicates that the render thread took the batch from the queue and is
    /// about to apply it: from here on the batch belongs to the frame the
    /// render thread is starting, and a batch committed now is applied by
    /// the frame after that one. Raised before [`processed`](Self::processed),
    /// by the same thread; a batch that is completed without being applied
    /// raises it too.
    ///
    /// Not from upstream. It is what the media context holds the next
    /// commit back for: the thread of the compositor prepares its next
    /// batch while the render thread applies and draws this one, instead of
    /// starting once the batch is applied (desktop-performance.md, section
    /// 9.33). Nothing of the batch can be observed yet: who needs its
    /// effects waits for `processed` or [`rendered`](Self::rendered).
    ///
    /// Continuations run synchronously on the render thread, so they should
    /// be as short as possible.
    pub fn accepted(&self) -> &BatchCompletion {
        &self.accepted
    }

    /// Indicates that the batch got deserialized on the render thread and
    /// will soon be rendered. It's generally a good time to start producing
    /// the next one.
    ///
    /// To allow timing-sensitive code to receive the notification in time,
    /// continuations run synchronously on the render thread, so they should
    /// be as short as possible.
    pub fn processed(&self) -> &BatchCompletion {
        &self.processed
    }

    /// Indicates that the batch got rendered on the render thread.
    ///
    /// To allow timing-sensitive code to receive the notification in time,
    /// continuations run synchronously on the render thread, so they should
    /// be as short as possible.
    pub fn rendered(&self) -> &BatchCompletion {
        &self.rendered
    }

    pub(crate) fn notify_accepted(&self) {
        self.accepted.try_set();
    }

    pub(crate) fn notify_processed(&self) {
        // A batch is never processed without having been accepted.
        self.accepted.try_set();
        self.processed.try_set();
    }

    pub(crate) fn notify_rendered(&self) {
        self.rendered.try_set();
    }
}

/// A committed batch on its way to the server compositor: the serialized
/// changes together with the handle the UI thread observes.
///
/// This is the one value that crosses from the UI-thread compositor to the
/// server compositor.
pub struct CommittedBatch {
    pub(crate) batch: Arc<CompositionBatch>,
    pub(crate) changes: BatchStreamData,
    pub(crate) committed_at: Duration,
}

/// A batch crosses from the UI thread to the render thread: everything in
/// it is `Send`, and stays so.
const _: fn() = || {
    fn sent<T: Send>() {}
    sent::<CommittedBatch>();
};
