use super::server::{BatchQueue, CompositorClock, ServerCompositor, ServerObjectId};
use super::transport::{
    BatchMarker, BatchObject, BatchStreamWriter, CommittedBatch, CompositionBatch, ServerJob, ServerObjectFactory,
};
use super::{CompositionOptions, ICompositorSerializable};
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
    batches: Rc<BatchQueue>,
    clock: CompositorClock,
    next_commit: RefCell<Option<Arc<CompositionBatch>>>,
    object_serialization_queue: RefCell<VecDeque<Rc<dyn ICompositorSerializable>>>,
    object_serialization_hash_set: RefCell<HashSet<*const ()>>,
    invoke_before_commit_write: RefCell<VecDeque<Box<dyn FnOnce()>>>,
    invoke_before_commit_read: RefCell<VecDeque<Box<dyn FnOnce()>>>,
    dispose_on_next_batch: RefCell<Vec<ServerObjectId>>,
    pending_creations: RefCell<Vec<(ServerObjectId, ServerObjectFactory)>>,
    next_server_object_id: Cell<u32>,
    free_server_object_ids: RefCell<Vec<u32>>,
    pending_batch: Arc<Mutex<Option<Arc<CompositionBatch>>>>,
    pending_server_compositor_jobs: RefCell<Vec<ServerJob>>,
    pending_server_compositor_post_target_jobs: RefCell<Vec<ServerJob>>,
    scheduler: Weak<dyn ICompositorScheduler>,
    dispatcher: Arc<Dispatcher>,
    after_commit: HandlerList<dyn Fn()>,
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
        let batches = Rc::new(BatchQueue::default());
        let server = ServerCompositor::new(gpu, options, batches.clone(), clock.clone());
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
        });
        COMPOSITORS.with(|compositors| {
            let mut compositors = compositors.borrow_mut();
            compositors.retain(|_, c| c.strong_count() > 0);
            compositors.insert(key, Rc::downgrade(&compositor));
        });
        render_loop.add(loop_task);
        compositor
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

    pub(super) fn this_handle(&self) -> Rc<Compositor> {
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
        let disposed: Vec<ServerObjectId>;
        {
            let mut writer = BatchStreamWriter::new(&mut changes);

            // Objects serialize references to server objects, so the objects
            // created since the last commit come first.
            let mut objects_written = false;
            loop {
                self.write_pending_creations(&mut writer);
                let Some(object) = self.object_serialization_queue.borrow_mut().pop_front() else { break };
                if let Some(server_object) = object.try_get_server(self) {
                    writer.write_server_object(Some(server_object));
                    object.serialize_changes(self, &mut writer);
                    if cfg!(debug_assertions) {
                        writer.write(super::server::OBJECT_END_MAGIC);
                        writer.write_marker(BatchMarker::ObjectEnd);
                    }
                    objects_written = true;
                }
            }
            let _ = objects_written;
            self.object_serialization_hash_set.borrow_mut().clear();

            disposed = std::mem::take(&mut *self.dispose_on_next_batch.borrow_mut());
            if !disposed.is_empty() {
                writer.write_marker(BatchMarker::RenderThreadDisposeStart);
                writer.write(disposed.len() as i32);
                for id in &disposed {
                    writer.write_server_object(Some(*id));
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
        self.free_server_object_ids.borrow_mut().extend(disposed.iter().map(|id| id.0));

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
        list: &RefCell<Vec<ServerJob>>,
        start_marker: BatchMarker,
        end_marker: BatchMarker,
    ) {
        let jobs = std::mem::take(&mut *list.borrow_mut());
        if !jobs.is_empty() {
            writer.write_marker(start_marker);
            for job in jobs {
                writer.write_object(BatchObject::Job(job));
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
        self.dispose_on_next_batch.borrow_mut().retain(|id| *id != server);
        let batch = CompositionBatch::new();
        let mut changes = self.batches.rent_data();
        {
            let mut writer = BatchStreamWriter::new(&mut changes);
            writer.write_marker(BatchMarker::RenderThreadDisposeStart);
            writer.write(1i32);
            writer.write_server_object(Some(server));
        }
        self.free_server_object_ids.borrow_mut().push(server.0);
        self.enqueue_batch(CommittedBatch { batch: batch.clone(), changes, committed_at: self.clock_elapsed() });
        batch
    }

    /// Allocates the id of a new server object and queues its creation for
    /// the next batch. `factory` runs on the render thread.
    pub fn create_server_object(
        &self,
        factory: impl FnOnce(&Rc<ServerCompositor>, ServerObjectId) -> Rc<dyn IServerObject> + 'static,
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

    /// Disposes a server object with the next batch.
    pub fn dispose_on_next_batch(&self, obj: ServerObjectId) {
        let added = {
            let mut list = self.dispose_on_next_batch.borrow_mut();
            if list.contains(&obj) {
                false
            } else {
                list.push(obj);
                true
            }
        };
        if added {
            self.request_commit_async();
        }
    }

    /// Disposes a server object with a later batch, without asking for a
    /// commit: used when a UI-thread object is dropped, which must not
    /// schedule work by itself.
    pub(crate) fn dispose_with_a_later_batch(&self, obj: ServerObjectId) {
        if let Ok(mut list) = self.dispose_on_next_batch.try_borrow_mut() {
            if !list.contains(&obj) {
                list.push(obj);
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
    pub fn post_server_job(&self, job: impl FnOnce(&ServerCompositor) + 'static, post_target: bool) {
        self.dispatcher.verify_access();
        let list = if post_target {
            &self.pending_server_compositor_post_target_jobs
        } else {
            &self.pending_server_compositor_jobs
        };
        list.borrow_mut().push(Box::new(job));
        self.request_commit_async();
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
