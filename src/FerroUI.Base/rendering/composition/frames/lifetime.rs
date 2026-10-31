use super::{FrameError, FrameMetadata};
use std::{
    any::Any,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

/// Limits cover live, old-generation and GPU-retiring allocations together.
#[derive(Clone, Copy, Debug)]
pub struct FrameBudgetLimits {
    pub frames: u64,
    pub bytes: u64,
    pub max_dimension: u32,
    /// Counts independent GPU submissions, including repeated reads of one frame.
    pub gpu_reads: u64,
}
impl Default for FrameBudgetLimits {
    fn default() -> Self {
        Self {
            frames: 3,
            bytes: 128 * 1024 * 1024,
            max_dimension: 8192,
            gpu_reads: 6,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBudgetUsage {
    pub frames: u64,
    pub bytes: u64,
}
struct BudgetInner {
    limits: FrameBudgetLimits,
    usage: Mutex<FrameBudgetUsage>,
    counters: super::FrameTransferCounters,
    gpu_reads: AtomicU64,
    quarantined_reads: AtomicU64,
}
#[derive(Clone)]
pub struct FrameBudget(Arc<BudgetInner>);
impl FrameBudget {
    pub fn new(limits: FrameBudgetLimits) -> Self {
        Self(Arc::new(BudgetInner {
            limits,
            usage: Mutex::new(FrameBudgetUsage::default()),
            counters: super::FrameTransferCounters::default(),
            gpu_reads: AtomicU64::new(0),
            quarantined_reads: AtomicU64::new(0),
        }))
    }
    pub fn usage(&self) -> FrameBudgetUsage {
        *self.0.usage.lock().unwrap()
    }
    pub fn counters(&self) -> super::FrameTransferCounters {
        self.0.counters.clone()
    }
    pub fn gpu_reads(&self) -> u64 {
        self.0.gpu_reads.load(Ordering::Acquire)
    }
    pub fn quarantined_reads(&self) -> u64 {
        self.0.quarantined_reads.load(Ordering::Acquire)
    }
    pub fn limits(&self) -> FrameBudgetLimits {
        self.0.limits
    }
    /// Admits an owned immutable native allocation. The resource's Drop must
    /// release native ownership, never mutate/recycle it behind live leases.
    pub fn lease<T: Any + Send + Sync>(
        &self,
        metadata: FrameMetadata,
        resource: T,
    ) -> Result<FrameLease, FrameError> {
        Ok(self.reserve(metadata)?.lease(resource))
    }
    /// Reserves capacity before allocating/submitting GPU storage. An unused
    /// reservation returns its capacity automatically; saturation allocates no
    /// native resource and never waits for another consumer.
    pub fn reserve(&self, metadata: FrameMetadata) -> Result<FrameReservation, FrameError> {
        metadata.validate(self.0.limits.max_dimension)?;
        let mut usage = self.0.usage.lock().unwrap();
        let bytes = usage
            .bytes
            .checked_add(metadata.retained_bytes)
            .ok_or(FrameError::BudgetExceeded)?;
        let frames = usage
            .frames
            .checked_add(1)
            .ok_or(FrameError::BudgetExceeded)?;
        if bytes > self.0.limits.bytes || frames > self.0.limits.frames {
            return Err(FrameError::BudgetExceeded);
        }
        *usage = FrameBudgetUsage { bytes, frames };
        drop(usage);
        let retained_bytes = metadata.retained_bytes;
        Ok(FrameReservation {
            metadata,
            retirement: FrameRetirement(Arc::new(RetirementInner {
                budget: self.clone(),
                retained_bytes,
            })),
        })
    }
}
/// Admission held before native allocation. Keep its retirement token when
/// native destruction must be queued to an owning context/thread.
pub struct FrameReservation {
    metadata: FrameMetadata,
    retirement: FrameRetirement,
}
impl FrameReservation {
    pub fn retirement(&self) -> FrameRetirement {
        self.retirement.clone()
    }
    pub fn lease<T: Any + Send + Sync>(self, resource: T) -> FrameLease {
        FrameLease(
            Arc::new(FrameInner {
                metadata: self.metadata,
                acquired_at: std::time::Instant::now(),
                resource: Some(Box::new(resource)),
                retirement: self.retirement,
            }),
            None,
        )
    }
}
/// Holds allocation accounting through asynchronous native destruction. Drop
/// only after storage has actually retired. Quarantine when native destruction
/// cannot be established, including a rejected owning-thread cleanup request.
#[derive(Clone)]
pub struct FrameRetirement(Arc<RetirementInner>);
impl FrameRetirement {
    pub fn quarantine(self) {
        std::mem::forget(self);
    }
}
struct RetirementInner {
    budget: FrameBudget,
    retained_bytes: u64,
}
impl Drop for RetirementInner {
    fn drop(&mut self) {
        let mut usage = self.budget.0.usage.lock().unwrap();
        usage.frames -= 1;
        usage.bytes -= self.retained_bytes;
    }
}
struct FrameInner {
    metadata: FrameMetadata,
    acquired_at: std::time::Instant,
    resource: Option<Box<dyn Any + Send + Sync>>,
    retirement: FrameRetirement,
}
impl FrameInner {
    fn budget(&self) -> &FrameBudget {
        &self.retirement.0.budget
    }
}
impl Drop for FrameInner {
    fn drop(&mut self) {
        // Destruction/queued retirement starts before accounting is released.
        drop(self.resource.take());
    }
}
/// Clones share native storage. A UI view, camera session and each submitted
/// GPU read hold independent leases; no clone duplicates pixel storage.
#[derive(Clone)]
pub struct FrameLease(Arc<FrameInner>, Option<PublicationValidity>);
#[derive(Clone)]
struct PublicationValidity {
    epoch: Arc<AtomicU64>,
    version: u64,
}
impl FrameLease {
    /// Time since framework acquisition, independent of media presentation
    /// timestamps, which may use a source clock or restart after seeking.
    pub fn acquisition_age(&self) -> std::time::Duration {
        self.0.acquired_at.elapsed()
    }
    /// Source replacement/clear invalidates queued presentation while native
    /// storage remains leased until all already-submitted GPU reads finish.
    pub fn is_current(&self) -> bool {
        self.1
            .as_ref()
            .is_none_or(|validity| validity.epoch.load(Ordering::Acquire) == validity.version)
    }
    pub fn counters(&self) -> super::FrameTransferCounters {
        self.0.budget().0.counters.clone()
    }
    pub fn metadata(&self) -> &FrameMetadata {
        &self.0.metadata
    }
    pub fn resource<T: Any>(&self) -> Option<&T> {
        self.0.resource.as_ref()?.downcast_ref()
    }
    pub fn same_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuCompletionStatus {
    Pending,
    Complete,
    Indeterminate,
}
struct CompletionState {
    status: GpuCompletionStatus,
    frames: Vec<FrameLease>,
    // A lost/dropped callback must not free submitted storage. The bounded
    // frame budget remains charged until completion or native device teardown.
    keep_alive: Option<Arc<CompletionInner>>,
    waiters: Vec<std::task::Waker>,
}
struct CompletionInner {
    state: Mutex<CompletionState>,
}
/// Native backends signal this only after actual GPU completion. Render-job
/// completion, submission and cancellation are not GPU-completion evidence.
#[derive(Clone)]
pub struct GpuCompletion(Arc<CompletionInner>);
impl Default for GpuCompletion {
    fn default() -> Self {
        Self::new()
    }
}
impl GpuCompletion {
    pub fn new() -> Self {
        Self(Arc::new(CompletionInner {
            state: Mutex::new(CompletionState {
                status: GpuCompletionStatus::Pending,
                frames: Vec::new(),
                keep_alive: None,
                waiters: Vec::new(),
            }),
        }))
    }
    pub fn status(&self) -> GpuCompletionStatus {
        self.0.state.lock().unwrap().status
    }
    /// Call before submitting a GPU read. All consumers register independently.
    pub fn retain(&self, frame: FrameLease) -> Result<(), FrameError> {
        let mut state = self.0.state.lock().unwrap();
        match state.status {
            GpuCompletionStatus::Pending => {}
            GpuCompletionStatus::Complete => return Err(FrameError::CompletionAlreadySignaled),
            GpuCompletionStatus::Indeterminate => {}
        }
        // Deduplicate one allocation used repeatedly in the same submission.
        if !state.frames.iter().any(|f| f.same_storage(&frame)) {
            let budget = &frame.0.budget().0;
            budget
                .gpu_reads
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |reads| {
                    (reads < budget.limits.gpu_reads).then(|| reads + 1)
                })
                .map_err(|_| FrameError::BudgetExceeded)?;
            if state.status == GpuCompletionStatus::Indeterminate {
                budget.quarantined_reads.fetch_add(1, Ordering::AcqRel);
            }
            state.frames.push(frame);
        }
        state.keep_alive = Some(self.0.clone());
        Ok(())
    }
    pub fn complete(&self) {
        let (frames, keep_alive, waiters, indeterminate) = {
            let mut state = self.0.state.lock().unwrap();
            let indeterminate = state.status == GpuCompletionStatus::Indeterminate;
            state.status = GpuCompletionStatus::Complete;
            (
                std::mem::take(&mut state.frames),
                state.keep_alive.take(),
                std::mem::take(&mut state.waiters),
                indeterminate,
            )
        };
        // Native destruction can reenter a backend: never release under lock.
        for frame in &frames {
            frame.0.budget().0.gpu_reads.fetch_sub(1, Ordering::AcqRel);
            if indeterminate {
                frame
                    .0
                    .budget()
                    .0
                    .quarantined_reads
                    .fetch_sub(1, Ordering::AcqRel);
            }
        }
        drop(frames);
        drop(keep_alive);
        for waker in waiters {
            waker.wake();
        }
    }
    pub fn mark_indeterminate(&self) {
        let waiters = {
            let mut state = self.0.state.lock().unwrap();
            if state.status == GpuCompletionStatus::Pending {
                state.status = GpuCompletionStatus::Indeterminate;
                for frame in &state.frames {
                    frame
                        .0
                        .budget()
                        .0
                        .quarantined_reads
                        .fetch_add(1, Ordering::AcqRel);
                }
            }
            std::mem::take(&mut state.waiters)
        };
        for waker in waiters {
            waker.wake();
        }
    }
    /// Yields to the host scheduler; never waits or polls a GPU fence on the
    /// UI thread. Native callbacks wake the future after completion/failure.
    pub fn wait(&self) -> impl std::future::Future<Output = Result<(), FrameError>> + 'static {
        let completion = self.clone();
        std::future::poll_fn(move |cx| {
            let mut state = completion.0.state.lock().unwrap();
            match state.status {
                GpuCompletionStatus::Complete => std::task::Poll::Ready(Ok(())),
                GpuCompletionStatus::Indeterminate => {
                    std::task::Poll::Ready(Err(FrameError::IndeterminateCompletion))
                }
                GpuCompletionStatus::Pending => {
                    if !state.waiters.iter().any(|w| w.will_wake(cx.waker())) {
                        state.waiters.push(cx.waker().clone());
                    }
                    std::task::Poll::Pending
                }
            }
        })
    }

    pub fn retained_frames(&self) -> usize {
        self.0.state.lock().unwrap().frames.len()
    }
}

/// Single-slot latest-frame publication shared by independent 2D/3D/analysis
/// consumers. Changing generations rejects old callbacks immediately.
pub struct FrameMailbox {
    source: u64,
    generation: AtomicU64,
    publication_epoch: Arc<AtomicU64>,
    latest: Mutex<Option<FrameLease>>,
}
impl Drop for FrameMailbox {
    fn drop(&mut self) {
        self.invalidate_publications();
        // Leases submitted to a GPU remain owned by their completion token.
        self.latest.get_mut().unwrap().take();
    }
}
impl FrameMailbox {
    pub fn new(source: u64, generation: u64) -> Self {
        Self {
            source,
            generation: AtomicU64::new(generation),
            publication_epoch: Arc::new(AtomicU64::new(1)),
            latest: Mutex::new(None),
        }
    }
    pub fn latest(&self) -> Option<FrameLease> {
        self.latest.lock().unwrap().clone()
    }
    /// Publishes producer-ready immutable storage. A GPU producer must await
    /// its write completion before calling this; render submission alone is
    /// insufficient. Presentation importers also validate their native ready
    /// token, and renderer-host output waits for GPU completion before publish.
    pub fn publish(&self, mut frame: FrameLease) -> Result<(), FrameError> {
        let mut latest = self.latest.lock().unwrap();
        let id = frame.metadata().identity;
        if !frame.is_current()
            || id.source != self.source
            || id.generation != self.generation.load(Ordering::Acquire)
            || latest
                .as_ref()
                .is_some_and(|f| id.sequence <= f.metadata().identity.sequence)
        {
            return Err(FrameError::StaleGeneration);
        }
        frame.1 = Some(PublicationValidity {
            epoch: self.publication_epoch.clone(),
            version: self.publication_epoch.load(Ordering::Acquire),
        });
        let previous = latest.replace(frame);
        drop(latest);
        drop(previous);
        Ok(())
    }
    pub fn replace_generation(&self, generation: u64) -> Result<(), FrameError> {
        let mut latest = self.latest.lock().unwrap();
        if generation <= self.generation.load(Ordering::Acquire) {
            return Err(FrameError::StaleGeneration);
        }
        self.generation.store(generation, Ordering::Release);
        self.invalidate_publications();
        let previous = latest.take();
        drop(latest);
        drop(previous);
        Ok(())
    }
    fn invalidate_publications(&self) {
        self.publication_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .expect("publication identity exhausted");
    }
    pub fn clear(&self) {
        let mut latest = self.latest.lock().unwrap();
        self.invalidate_publications();
        let previous = latest.take();
        drop(latest);
        drop(previous);
    }
}
