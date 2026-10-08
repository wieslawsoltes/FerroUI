use super::server::{BatchQueue, CompositorClock, ServerCompositionVisual, ServerCompositor, ServerObjectId};
use super::transport::{
    BatchMarker, BatchObject, BatchStreamWriter, CommittedBatch, CompositionBatch, ServerJob, ServerObjectFactory, ServerObjectJob,
};
use super::{CompositionOptions, CompositionVisual, ICompositorSerializable};
use crate::animation::easings::{IEasing, SplineEasing};
use crate::animation::KeySpline;
use crate::media::imaging::Bitmap;
use crate::platform::IPlatformGraphics;
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::server::IServerObject;
use crate::rendering::{IRenderLoop, IRenderLoopTask};
use crate::threading::{Dispatcher, DispatcherPriority};
use crate::utilities::HandlerList;
use crate::{FerroLocator, LocatorExtensions};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Receives the request to commit a compositor's pending changes; the media
/// context implements it and schedules the commit with the next frame.
pub trait ICompositorScheduler {
    fn commit_requested(&self, compositor: &Rc<Compositor>);
}

static NEXT_COMPOSITOR_KEY: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// The compositors of this thread by key: lets `Send` callbacks that
    /// come back to the thread find their compositor.
    static COMPOSITORS: RefCell<HashMap<u64, Weak<Compositor>>> = RefCell::new(HashMap::new());
}

fn find_compositor(key: u64) -> Option<Rc<Compositor>> {
    COMPOSITORS.with(|compositors| compositors.borrow().get(&key).and_then(Weak::upgrade))
}

/// The compositor: manages the composition objects of the UI thread and
/// commits their changes to the server compositor in batches.
pub struct Compositor {
    this: Weak<Compositor>,
    key: u64,
    render_loop: Arc<dyn IRenderLoop>,
    loop_task: Arc<dyn IRenderLoopTask>,
    use_ui_thread_for_synchronous_commits: bool,
    server: Rc<ServerCompositor>,
    /// The readback of the server: what the two threads share besides the
    /// queue of batches.
    readback: Arc<super::server::ReadbackIndices>,
    batches: Arc<BatchQueue>,
    clock: CompositorClock,
    next_commit: RefCell<Option<Arc<CompositionBatch>>>,
    object_serialization_queue: RefCell<VecDeque<Rc<dyn ICompositorSerializable>>>,
    object_serialization_hash_set: RefCell<HashSet<*const ()>>,
    invoke_before_commit_write: RefCell<VecDeque<Box<dyn FnOnce()>>>,
    invoke_before_commit_read: RefCell<VecDeque<Box<dyn FnOnce()>>>,
    dispose_on_next_batch: RefCell<Vec<PendingDisposal>>,
    pending_creations: RefCell<Vec<(ServerObjectId, ServerObjectFactory)>>,
    next_server_object_id: Cell<u32>,
    free_server_object_ids: RefCell<Vec<u32>>,
    pending_batch: Arc<Mutex<Option<Arc<CompositionBatch>>>>,
    pending_server_compositor_jobs: RefCell<Vec<PendingServerJob>>,
    pending_server_compositor_post_target_jobs: RefCell<Vec<PendingServerJob>>,
    scheduler: Weak<dyn ICompositorScheduler>,
    dispatcher: Arc<Dispatcher>,
    after_commit: HandlerList<dyn Fn()>,
    default_easing: Rc<dyn IEasing>,
}

/// The render loop task of a compositor. The loop may tick on any thread;
/// the server compositor is confined to the thread of its compositor, so a
/// tick from elsewhere is marshalled there.
struct ServerCompositorLoopTask {
    key: u64,
    dispatcher: Arc<Dispatcher>,
    marshalled: Arc<AtomicBool>,
    last_result: Arc<AtomicBool>,
}

impl IRenderLoopTask for ServerCompositorLoopTask {
    fn render(&self) -> bool {
        if let Some(compositor) = find_compositor(self.key) {
            let result = compositor.server.render();
            self.last_result.store(result, Ordering::SeqCst);
            return result;
        }
        if !self.marshalled.swap(true, Ordering::SeqCst) {
            let key = self.key;
            let marshalled = self.marshalled.clone();
            let last_result = self.last_result.clone();
            self.dispatcher.post(
                move || {
                    marshalled.store(false, Ordering::SeqCst);
                    if let Some(compositor) = find_compositor(key) {
                        last_result.store(compositor.server.render(), Ordering::SeqCst);
                    }
                },
                DispatcherPriority::RENDER,
            );
        }
        self.last_result.load(Ordering::SeqCst)
    }
}

impl Compositor {
    /// Creates a compositor that renders on the render loop registered in
    /// the service locator and commits through the media context.
    pub fn new(gpu: Option<Rc<dyn IPlatformGraphics>>, use_ui_thread_for_synchronous_commits: bool) -> Rc<Compositor> {
        let render_loop = FerroLocator::current().get_required_service::<Arc<dyn IRenderLoop>>();
        let scheduler: Rc<dyn ICompositorScheduler> = crate::media::MediaContext::instance().scheduler();
        Self::with_scheduler(
            (*render_loop).clone(),
            gpu,
            use_ui_thread_for_synchronous_commits,
            &scheduler,
            Dispatcher::ui_thread(),
            None,
            None,
        )
    }

    /// Creates a compositor with every collaborator given explicitly.
    ///
    /// `options` defaults to the options registered in the service locator;
    /// `clock` defaults to the dispatcher's clock.
    pub fn with_scheduler(
        render_loop: Arc<dyn IRenderLoop>,
        gpu: Option<Rc<dyn IPlatformGraphics>>,
        use_ui_thread_for_synchronous_commits: bool,
        scheduler: &Rc<dyn ICompositorScheduler>,
        dispatcher: Arc<Dispatcher>,
        options: Option<CompositionOptions>,
        clock: Option<CompositorClock>,
    ) -> Rc<Compositor> {
        let options = options
            .or_else(|| FerroLocator::current().get_service::<CompositionOptions>().map(|o| *o))
            .unwrap_or_default();
        let clock = clock.unwrap_or_else(|| {
            let platform_impl = dispatcher.platform_impl();
            let origin = platform_impl.now();
            Rc::new(move || Duration::from_millis((platform_impl.now() - origin).max(0) as u64))
        });
        let batches = Arc::new(BatchQueue::default());
        let readback = Arc::new(super::server::ReadbackIndices::new());
        let server = ServerCompositor::new(gpu, options, batches.clone(), readback.clone(), clock.clone());
        let key = NEXT_COMPOSITOR_KEY.fetch_add(1, Ordering::SeqCst);
        let loop_task: Arc<dyn IRenderLoopTask> = Arc::new(ServerCompositorLoopTask {
            key,
            dispatcher: dispatcher.clone(),
            marshalled: Arc::new(AtomicBool::new(false)),
            last_result: Arc::new(AtomicBool::new(true)),
        });
        let compositor = Rc::new_cyclic(|this| Compositor {
            this: this.clone(),
            key,
            render_loop: render_loop.clone(),
            loop_task: loop_task.clone(),
            use_ui_thread_for_synchronous_commits,
            readback,
            server,
            batches,
            clock,
            next_commit: RefCell::new(None),
            object_serialization_queue: RefCell::new(VecDeque::new()),
            object_serialization_hash_set: RefCell::new(HashSet::new()),
            invoke_before_commit_write: RefCell::new(VecDeque::new()),
            invoke_before_commit_read: RefCell::new(VecDeque::new()),
            dispose_on_next_batch: RefCell::new(Vec::new()),
            pending_creations: RefCell::new(Vec::new()),
            next_server_object_id: Cell::new(0),
            free_server_object_ids: RefCell::new(Vec::new()),
            pending_batch: Arc::new(Mutex::new(None)),
            pending_server_compositor_jobs: RefCell::new(Vec::new()),
            pending_server_compositor_post_target_jobs: RefCell::new(Vec::new()),
            scheduler: Rc::downgrade(scheduler),
            dispatcher,
            after_commit: HandlerList::new(),
            default_easing: Rc::new(SplineEasing::with_key_spline(KeySpline::with_points(0.25, 0.1, 0.25, 1.0))),
        });
        COMPOSITORS.with(|compositors| {
            let mut compositors = compositors.borrow_mut();
            compositors.retain(|_, c| c.strong_count() > 0);
            compositors.insert(key, Rc::downgrade(&compositor));
        });
        render_loop.add(loop_task);
        compositor
    }

    /// The easing of the key frames that are inserted without one.
    pub(crate) fn default_easing(&self) -> Rc<dyn IEasing> {
        self.default_easing.clone()
    }

    /// The render loop the server compositor is driven by.
    pub fn render_loop(&self) -> &Arc<dyn IRenderLoop> {
        &self.render_loop
    }

    pub fn use_ui_thread_for_synchronous_commits(&self) -> bool {
        self.use_ui_thread_for_synchronous_commits
    }

    /// The server compositor. It belongs to the render thread: UI-thread
    /// code may only use it to drive a frame when the render loop runs on
    /// the UI thread, and in tests.
    pub fn server(&self) -> &Rc<ServerCompositor> {
        &self.server
    }

    /// The readback indices the server writes and this side reads.
    pub fn readback(&self) -> &Arc<super::server::ReadbackIndices> {
        &self.readback
    }

    /// The dispatcher of the thread the compositor belongs to.
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    /// The time elapsed on the compositor clock.
    pub fn clock_elapsed(&self) -> Duration {
        (self.clock)()
    }

    /// Subscribes to the notification raised after each commit.
    pub fn after_commit(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.after_commit.add(Rc::new(handler));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.after_commit.remove(token);
            }
        })
    }

    pub(crate) fn this_handle(&self) -> Rc<Compositor> {
        self.this()
    }

    fn this(&self) -> Rc<Compositor> {
        self.this.upgrade().expect("the compositor is alive while it is used")
    }

    fn trigger_commit_requested(&self) {
        if let Some(scheduler) = self.scheduler.upgrade() {
            scheduler.commit_requested(&self.this());
        }
    }

    /// Requests the pending changes to be committed and returns the batch
    /// they will be part of.
    pub fn request_composition_batch_commit_async(&self) -> Arc<CompositionBatch> {
        self.dispatcher.verify_access();
        if let Some(next_commit) = self.next_commit.borrow().clone() {
            return next_commit;
        }
        let next_commit = CompositionBatch::new();
        *self.next_commit.borrow_mut() = Some(next_commit.clone());
        let pending = self.pending_batch.lock().unwrap_or_else(|e| e.into_inner()).clone();
        match pending {
            Some(pending) => {
                // The previous batch has not been processed yet: ask for
                // the commit once it has been.
                let key = self.key;
                let dispatcher = self.dispatcher.clone();
                pending.processed().on_completed(move || {
                    dispatcher.post(
                        move || {
                            if let Some(compositor) = find_compositor(key) {
                                compositor.trigger_commit_requested();
                            }
                        },
                        DispatcherPriority::SEND,
                    );
                });
            }
            None => self.trigger_commit_requested(),
        }
        next_commit
    }

    /// Requests the pending changes to be committed.
    pub fn request_commit_async(&self) -> Arc<CompositionBatch> {
        self.request_composition_batch_commit_async()
    }

    /// Serializes the pending changes into a batch and hands it to the
    /// server compositor.
    pub fn commit(&self) -> Arc<CompositionBatch> {
        let result = self.commit_core();
        if !self.invoke_before_commit_write.borrow().is_empty() {
            self.request_commit_async();
        }
        if !self.after_commit.is_empty() {
            for (_, handler) in self.after_commit.snapshot().iter() {
                handler();
            }
        }
        result
    }

    fn commit_core(&self) -> Arc<CompositionBatch> {
        self.dispatcher.verify_access();

        let commit = self.next_commit.borrow_mut().get_or_insert_with(CompositionBatch::new).clone();

        std::mem::swap(
            &mut *self.invoke_before_commit_read.borrow_mut(),
            &mut *self.invoke_before_commit_write.borrow_mut(),
        );
        loop {
            let Some(action) = self.invoke_before_commit_read.borrow_mut().pop_front() else { break };
            action();
        }

        let mut changes = self.batches.rent_data();
        let disposed: Vec<PendingDisposal>;
        {
            let mut writer = BatchStreamWriter::new(&mut changes);

            // Objects serialize references to server objects, so the objects
            // created since the last commit come first.
            let mut objects_written = false;
            // The set of queued objects knows an object by its address, so
            // an object stays alive until the set is cleared: an object
            // created while another one is serialized can then never get the
            // address of one that was serialized (and released) before it,
            // and be taken for queued.
            let mut serialized: Vec<Rc<dyn ICompositorSerializable>> = Vec::new();
            loop {
                self.write_pending_creations(&mut writer);
                let Some(object) = self.object_serialization_queue.borrow_mut().pop_front() else { break };
                if let Some(server_object) = object.try_get_server(self) {
                    // Server objects the object creates for its changes
                    // must exist before the changes are read.
                    object.prepare_serialization(self);
                    self.write_pending_creations(&mut writer);
                    writer.write_server_object(Some(server_object));
                    object.serialize_changes(self, &mut writer);
                    if cfg!(debug_assertions) {
                        writer.write(super::server::OBJECT_END_MAGIC);
                        writer.write_marker(BatchMarker::ObjectEnd);
                    }
                    objects_written = true;
                }
                serialized.push(object);
            }
            let _ = objects_written;
            self.object_serialization_hash_set.borrow_mut().clear();
            drop(serialized);

            disposed = std::mem::take(&mut *self.dispose_on_next_batch.borrow_mut());
            if !disposed.is_empty() {
                writer.write_marker(BatchMarker::RenderThreadDisposeStart);
                writer.write(disposed.len() as i32);
                for entry in &disposed {
                    writer.write_server_object(Some(entry.id));
                    writer.write(entry.dispose);
                    writer.write(entry.release);
                }
            }

            Self::serialize_server_jobs(
                &mut writer,
                &self.pending_server_compositor_jobs,
                BatchMarker::RenderThreadJobsStart,
                BatchMarker::RenderThreadJobsEnd,
            );
            Self::serialize_server_jobs(
                &mut writer,
                &self.pending_server_compositor_post_target_jobs,
                BatchMarker::RenderThreadPostTargetJobsStart,
                BatchMarker::RenderThreadPostTargetJobsEnd,
            );
        }
        // Ids disposed by this batch can be reused by objects created for a
        // later one.
        self.free_server_object_ids
            .borrow_mut()
            .extend(disposed.iter().filter(|entry| entry.release).map(|entry| entry.id.0));

        self.enqueue_batch(CommittedBatch { batch: commit.clone(), changes, committed_at: self.clock_elapsed() });

        *self.pending_batch.lock().unwrap_or_else(|e| e.into_inner()) = Some(commit.clone());
        let pending_batch = self.pending_batch.clone();
        let sequence_id = commit.sequence_id();
        commit.processed().on_completed(move || {
            let mut pending = pending_batch.lock().unwrap_or_else(|e| e.into_inner());
            if pending.as_ref().is_some_and(|p| p.sequence_id() == sequence_id) {
                *pending = None;
            }
        });
        *self.next_commit.borrow_mut() = None;

        commit
    }

    fn write_pending_creations(&self, writer: &mut BatchStreamWriter<'_>) {
        let creations = std::mem::take(&mut *self.pending_creations.borrow_mut());
        if creations.is_empty() {
            return;
        }
        writer.write_marker(BatchMarker::CreateStart);
        writer.write(creations.len() as i32);
        for (id, factory) in creations {
            writer.write_server_object(Some(id));
            writer.write_object(BatchObject::Create(factory));
        }
    }

    fn serialize_server_jobs(
        writer: &mut BatchStreamWriter<'_>,
        list: &RefCell<Vec<PendingServerJob>>,
        start_marker: BatchMarker,
        end_marker: BatchMarker,
    ) {
        let jobs = std::mem::take(&mut *list.borrow_mut());
        if !jobs.is_empty() {
            writer.write_marker(start_marker);
            for job in jobs {
                match job {
                    PendingServerJob::Job(job) => writer.write_object(BatchObject::Job(job)),
                    PendingServerJob::ObjectJob(target, job) => {
                        writer.write_server_object(Some(target));
                        writer.write_object(BatchObject::ObjectJob(job));
                    }
                }
            }
            writer.write_marker(end_marker);
        }
    }

    fn enqueue_batch(&self, batch: CommittedBatch) {
        self.batches.enqueue(batch);
        self.render_loop.wakeup();
    }

    /// Disposes a server object in a batch of its own, outside of the
    /// regular commit.
    pub(crate) fn oob_dispose(&self, server: ServerObjectId) -> Arc<CompositionBatch> {
        self.dispose_on_next_batch.borrow_mut().retain(|entry| entry.id != server);
        let batch = CompositionBatch::new();
        let mut changes = self.batches.rent_data();
        {
            let mut writer = BatchStreamWriter::new(&mut changes);
            writer.write_marker(BatchMarker::RenderThreadDisposeStart);
            writer.write(1i32);
            writer.write_server_object(Some(server));
            writer.write(true);
            writer.write(true);
        }
        self.free_server_object_ids.borrow_mut().push(server.0);
        self.enqueue_batch(CommittedBatch { batch: batch.clone(), changes, committed_at: self.clock_elapsed() });
        batch
    }

    /// Allocates the id of a new server object and queues its creation for
    /// the next batch. `factory` runs on the render thread.
    pub fn create_server_object(
        &self,
        factory: impl FnOnce(&Rc<ServerCompositor>, ServerObjectId) -> Rc<dyn IServerObject> + Send + 'static,
    ) -> ServerObjectId {
        self.dispatcher.verify_access();
        let id = match self.free_server_object_ids.borrow_mut().pop() {
            Some(id) => ServerObjectId(id),
            None => {
                let id = self.next_server_object_id.get();
                self.next_server_object_id.set(id + 1);
                ServerObjectId(id)
            }
        };
        self.pending_creations.borrow_mut().push((id, Box::new(factory)));
        self.request_commit_async();
        id
    }

    /// Queues an object whose changes must be written to the next batch.
    pub fn register_for_serialization(&self, composition_object: Rc<dyn ICompositorSerializable>) {
        self.dispatcher.verify_access();
        let key = composition_object.serialization_key();
        if self.object_serialization_hash_set.borrow_mut().insert(key) {
            self.object_serialization_queue.borrow_mut().push_back(composition_object);
        }
        self.request_commit_async();
    }

    /// Disposes a server object with the next batch, and releases it: the
    /// server compositor drops it and its id can be reused.
    pub fn dispose_on_next_batch(&self, obj: ServerObjectId) {
        if self.queue_disposal(obj, true, true) {
            self.request_commit_async();
        }
    }

    /// Disposes a server object with the next batch but keeps it, under its
    /// id, until [`release_with_a_later_batch`](Self::release_with_a_later_batch):
    /// the disposal of a composition object, whose server object stays
    /// reachable by the jobs and animations that refer to it while the
    /// UI-thread object is alive, as upstream the UI-thread object holds its
    /// server object.
    pub(crate) fn dispose_and_keep_on_next_batch(&self, obj: ServerObjectId) {
        if self.queue_disposal(obj, true, false) {
            self.request_commit_async();
        }
    }

    /// Disposes a server object with a later batch, without asking for a
    /// commit: used when a UI-thread object is dropped, which must not
    /// schedule work by itself.
    pub(crate) fn dispose_with_a_later_batch(&self, obj: ServerObjectId) {
        self.queue_disposal(obj, true, true);
    }

    /// Releases a server object that was already disposed with a later
    /// batch, without asking for a commit: used when a disposed UI-thread
    /// object is dropped.
    pub(crate) fn release_with_a_later_batch(&self, obj: ServerObjectId) {
        self.queue_disposal(obj, false, true);
    }

    /// Adds `obj` to the disposal list of the next batch, or merges the
    /// request into its entry. Returns whether the list changed.
    fn queue_disposal(&self, obj: ServerObjectId, dispose: bool, release: bool) -> bool {
        let Ok(mut list) = self.dispose_on_next_batch.try_borrow_mut() else { return false };
        match list.iter_mut().find(|entry| entry.id == obj) {
            Some(entry) => {
                let changed = (dispose && !entry.dispose) || (release && !entry.release);
                entry.dispose |= dispose;
                entry.release |= release;
                changed
            }
            None => {
                list.push(PendingDisposal { id: obj, dispose, release });
                true
            }
        }
    }

    /// Enqueues a callback to be run before the next commit.
    pub fn request_composition_update(&self, action: impl FnOnce() + 'static) {
        self.dispatcher.verify_access();
        self.invoke_before_commit_write.borrow_mut().push_back(Box::new(action));
        self.request_commit_async();
    }

    /// Posts a job that runs on the render thread with the next batch:
    /// before the composition targets are rendered, or after them when
    /// `post_target` is set.
    pub fn post_server_job(&self, job: impl FnOnce(&ServerCompositor) + Send + 'static, post_target: bool) {
        self.dispatcher.verify_access();
        let list = if post_target {
            &self.pending_server_compositor_post_target_jobs
        } else {
            &self.pending_server_compositor_jobs
        };
        list.borrow_mut().push(PendingServerJob::Job(Box::new(job)));
        self.request_commit_async();
    }

    /// Posts a job that runs on the render thread with the next batch and
    /// receives the server object `target` (`None` when it does not
    /// exist). Upstream a job holds the server object itself; this job
    /// names it, and the object is resolved when the batch is read, so the
    /// job reaches an object that the same batch disposes.
    pub fn post_server_object_job(
        &self,
        target: ServerObjectId,
        job: impl FnOnce(&ServerCompositor, Option<Rc<dyn IServerObject>>) + Send + 'static,
        post_target: bool,
    ) {
        self.dispatcher.verify_access();
        let list = if post_target {
            &self.pending_server_compositor_post_target_jobs
        } else {
            &self.pending_server_compositor_jobs
        };
        list.borrow_mut().push(PendingServerJob::ObjectJob(target, Box::new(job)));
        self.request_commit_async();
    }

    /// [`invoke_server_job_async`](Self::invoke_server_job_async) with a job
    /// that receives the server object `target`; see
    /// [`post_server_object_job`](Self::post_server_object_job).
    pub fn invoke_server_object_job_async<T: Send + 'static>(
        &self,
        target: ServerObjectId,
        job: impl FnOnce(&ServerCompositor, Option<Rc<dyn IServerObject>>) -> Result<T, ServerJobError>
            + Send
            + 'static,
        post_target: bool,
    ) -> ServerJobTask<T> {
        let task = ServerJobTask::new();
        let completion = task.clone();
        self.post_server_object_job(
            target,
            move |compositor, object| match job(compositor, object) {
                Ok(result) => completion.set_result(result),
                Err(error) => completion.set_exception(error),
            },
            post_target,
        );
        task
    }

    /// Runs `job` on the render thread with the next batch and returns a
    /// task that completes with its result (`InvokeServerJobAsync`). A job
    /// that fails faults the task with its error.
    pub fn invoke_server_job_async<T: Send + 'static>(
        &self,
        job: impl FnOnce(&ServerCompositor) -> Result<T, ServerJobError> + Send + 'static,
        post_target: bool,
    ) -> ServerJobTask<T> {
        let task = ServerJobTask::new();
        let completion = task.clone();
        self.post_server_job(
            move |compositor| match job(compositor) {
                Ok(result) => completion.set_result(result),
                Err(error) => completion.set_exception(error),
            },
            post_target,
        );
        task
    }

    /// Renders a visual and its children into a new bitmap, on the render
    /// thread with the next batch, after the composition targets are
    /// rendered (`CreateCompositionVisualSnapshot`).
    ///
    /// The task is faulted with a [`CompositionVisualSnapshotError`] when
    /// the visual belongs to another compositor or is not attached to a
    /// composition target (`InvalidOperationException` upstream, which the
    /// asynchronous method also reports through its task).
    pub fn create_composition_visual_snapshot(&self, visual: &CompositionVisual, scaling: f64) -> ServerJobTask<Bitmap> {
        if !Rc::ptr_eq(visual.compositor(), &self.this()) {
            return ServerJobTask::from_exception(Arc::new(CompositionVisualSnapshotError(
                "the visual belongs to another compositor",
            )));
        }
        if visual.root().is_none() {
            return ServerJobTask::from_exception(Arc::new(CompositionVisualSnapshotError(
                "the visual is not attached to a composition target",
            )));
        }
        // The job names the server visual by id, as the other jobs of an object do: the id
        // stays bound to the server visual while the visual is alive.
        let server = visual.server();
        self.invoke_server_job_async(
            move |compositor| match compositor.get::<ServerCompositionVisual>(server) {
                Some(visual) => Ok(Bitmap::from_impl(compositor.create_composition_visual_snapshot(&visual, scaling, true))),
                None => {
                    let error: ServerJobError =
                        Arc::new(CompositionVisualSnapshotError("the server visual no longer exists"));
                    Err(error)
                }
            },
            true,
        )
    }

    /// The interop with GPU objects created outside of the framework, when
    /// the render interface supports it (`TryGetCompositionGpuInterop`).
    ///
    /// Upstream the answer is awaited from the render thread; the server
    /// compositor runs on this thread, so it is given directly.
    pub fn try_get_composition_gpu_interop(&self) -> Option<Rc<dyn super::ICompositionGpuInterop>> {
        let feature = self.try_get_render_interface_feature(std::any::TypeId::of::<
            dyn crate::platform::IExternalObjectsRenderInterfaceContextFeature,
        >())?;
        let external_objects = feature
            .downcast_ref::<Rc<dyn crate::platform::IExternalObjectsRenderInterfaceContextFeature>>()?
            .clone();
        Some(super::CompositionInterop::new(&self.this(), external_objects))
    }

    /// Attempts to query for a feature from the platform render interface.
    ///
    /// Upstream the answer comes from a job on the render thread when it is
    /// not cached; the server compositor runs on this thread, so it is
    /// asked directly.
    pub fn try_get_render_interface_feature(&self, feature_type: std::any::TypeId) -> Option<Rc<dyn std::any::Any>> {
        self.dispatcher.verify_access();
        if let Some(features) = self.server.at_try_get_cached_render_interface_features() {
            return features.get(&feature_type).cloned();
        }
        if !self.server.render_interface().is_ready() {
            return None;
        }
        self.server.rt_get_render_interface_features().get(&feature_type).cloned()
    }

    /// Whether an object is queued for serialization (for unit tests).
    pub fn unit_test_is_registered_for_serialization(&self, serializable: &dyn ICompositorSerializable) -> bool {
        self.object_serialization_hash_set.borrow().contains(&serializable.serialization_key())
    }

    /// The compositor registered in the service locator, if any.
    pub fn try_get_default_compositor() -> Option<Rc<Compositor>> {
        FerroLocator::current().get_service::<Compositor>()
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        self.render_loop.remove(&self.loop_task);
        let key = self.key;
        // The registry may already be gone when the thread is exiting.
        let _ = COMPOSITORS.try_with(|compositors| {
            if let Ok(mut compositors) = compositors.try_borrow_mut() {
                compositors.remove(&key);
            }
        });
    }
}

/// A server object listed for disposal by the next batch: the server
/// object is disposed when `dispose` is set, and dropped by the server
/// compositor, its id becoming free, when `release` is set.
struct PendingDisposal {
    id: ServerObjectId,
    dispose: bool,
    release: bool,
}

/// The error of a snapshot that cannot be taken (`InvalidOperationException`):
/// see [`Compositor::create_composition_visual_snapshot`].
#[derive(Clone, Debug)]
pub struct CompositionVisualSnapshotError(&'static str);

impl std::fmt::Display for CompositionVisualSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for CompositionVisualSnapshotError {}

/// A job waiting for the next batch.
enum PendingServerJob {
    Job(ServerJob),
    ObjectJob(ServerObjectId, ServerObjectJob),
}

/// The error of a job run on the render thread: it crosses to the thread
/// that asked for the job.
pub type ServerJobError = Arc<dyn std::error::Error + Send + Sync>;

/// The outcome of a job run on the render thread: what
/// [`Compositor::invoke_server_job_async`] returns (a task upstream).
///
/// The handle is shared by its clones; the job completes it once, from the
/// render thread. Continuations belong to the thread that created the task
/// and run there: right away when the job completes on that thread,
/// otherwise from its dispatcher.
pub struct ServerJobTask<T> {
    shared: Arc<ServerJobTaskShared<T>>,
}

struct ServerJobTaskShared<T> {
    outcome: std::sync::Mutex<ServerJobOutcome<T>>,
    continuations: crate::utilities::ThreadBound<RefCell<Vec<Box<dyn FnOnce()>>>>,
    thread: std::thread::ThreadId,
}

enum ServerJobOutcome<T> {
    Running,
    RanToCompletion(Option<T>),
    Faulted(ServerJobError),
}

impl<T> Clone for ServerJobTask<T> {
    fn clone(&self) -> Self {
        Self { shared: self.shared.clone() }
    }
}

impl<T: Send + 'static> ServerJobTask<T> {
    fn with_outcome(outcome: ServerJobOutcome<T>) -> Self {
        Self {
            shared: Arc::new(ServerJobTaskShared {
                outcome: std::sync::Mutex::new(outcome),
                continuations: crate::utilities::ThreadBound::new(RefCell::new(Vec::new())),
                thread: std::thread::current().id(),
            }),
        }
    }

    pub(crate) fn new() -> Self {
        Self::with_outcome(ServerJobOutcome::Running)
    }

    /// A task that has completed with `result`.
    pub fn from_result(result: T) -> Self {
        Self::with_outcome(ServerJobOutcome::RanToCompletion(Some(result)))
    }

    /// A task that has failed with `error`.
    pub fn from_exception(error: ServerJobError) -> Self {
        Self::with_outcome(ServerJobOutcome::Faulted(error))
    }

    fn run_continuations(shared: &ServerJobTaskShared<T>) {
        let continuations = std::mem::take(&mut *shared.continuations.get().borrow_mut());
        for continuation in continuations {
            continuation();
        }
    }

    fn complete(&self, outcome: ServerJobOutcome<T>) {
        {
            let mut current = self.shared.outcome.lock().unwrap();
            if !matches!(*current, ServerJobOutcome::Running) {
                return;
            }
            *current = outcome;
        }

        if self.shared.continuations.is_on_thread() {
            Self::run_continuations(&self.shared);
        } else if let Some(dispatcher) = Dispatcher::from_thread(self.shared.thread) {
            // Completed by the render thread: the continuations run on the
            // thread of the task.
            let shared = self.shared.clone();
            dispatcher.post(move || Self::run_continuations(&shared), DispatcherPriority::SEND);
        }
    }

    pub(crate) fn set_result(&self, result: T) {
        self.complete(ServerJobOutcome::RanToCompletion(Some(result)))
    }

    pub(crate) fn set_exception(&self, error: ServerJobError) {
        self.complete(ServerJobOutcome::Faulted(error))
    }

    /// Whether the job has run.
    pub fn is_completed(&self) -> bool {
        !matches!(*self.shared.outcome.lock().unwrap(), ServerJobOutcome::Running)
    }

    /// Whether the job has run without an error.
    pub fn is_completed_successfully(&self) -> bool {
        matches!(*self.shared.outcome.lock().unwrap(), ServerJobOutcome::RanToCompletion(_))
    }

    /// Whether the job failed.
    pub fn is_faulted(&self) -> bool {
        matches!(*self.shared.outcome.lock().unwrap(), ServerJobOutcome::Faulted(_))
    }

    /// The error of a failed job.
    pub fn exception(&self) -> Option<ServerJobError> {
        match &*self.shared.outcome.lock().unwrap() {
            ServerJobOutcome::Faulted(error) => Some(error.clone()),
            _ => None,
        }
    }

    /// Takes the result of a job that ran to completion: `None` while it
    /// runs, and once taken; the error of a failed job.
    pub fn take_result(&self) -> Option<Result<T, ServerJobError>> {
        match &mut *self.shared.outcome.lock().unwrap() {
            ServerJobOutcome::Running => None,
            ServerJobOutcome::RanToCompletion(result) => result.take().map(Ok),
            ServerJobOutcome::Faulted(error) => Some(Err(error.clone())),
        }
    }

    /// Runs `continuation` once the job has run; right away if it has.
    /// Called on the thread that created the task.
    pub fn on_completed(&self, continuation: impl FnOnce() + 'static) {
        {
            // The outcome is read under its lock, so a job completing on the
            // render thread either sees the continuation or has completed.
            let outcome = self.shared.outcome.lock().unwrap();
            if matches!(*outcome, ServerJobOutcome::Running) {
                self.shared.continuations.get().borrow_mut().push(Box::new(continuation));
                return;
            }
        }
        continuation();
    }
}
