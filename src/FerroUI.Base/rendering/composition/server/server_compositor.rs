use super::{
    CompositorPools, IServerCompositionLight, IServerObject, IServerRenderResource, ReadbackData, ReadbackIndices,
    ServerCompositionTarget, ServerCompositionVisual, ServerCompositorAnimations, ServerObjectId, VisualReadback,
};
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{IBitmapImpl, IPlatformGraphics, IRenderTarget, LtrbRect};
use crate::rendering::composition::transport::{
    BatchMarker, BatchObject, BatchStreamData, BatchStreamReader, CommittedBatch, CompositionBatch,
};
use crate::rendering::composition::CompositionOptions;
use crate::rendering::PlatformRenderInterfaceContextManager;
use crate::reactive::IDisposable;
use crate::threading::Dispatcher;
use crate::utilities::ReleaseExcess;
use crate::{Matrix, PixelSize, Size, Vector};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The source of the compositor clock: time elapsed since an arbitrary
/// origin. It is read from both sides of the transport; like the server
/// compositor it is confined to the compositor's thread for now, because
/// the dispatcher's clock is.
pub type CompositorClock = Rc<dyn Fn() -> Duration>;

/// The queue of committed batches: the one point where data passes from
/// the UI-thread compositor to the server compositor.
///
/// The UI thread enqueues and rents buffers, the render thread dequeues and
/// returns them: both under a lock.
#[derive(Default)]
pub struct BatchQueue {
    queue: Mutex<QueuedBatches>,
    /// Emptied stream buffers, returned by the server for reuse.
    data_pool: Mutex<Vec<BatchStreamData>>,
}

#[derive(Default)]
struct QueuedBatches {
    batches: VecDeque<CommittedBatch>,
    /// The last tick of the render thread found no batch and drew nothing,
    /// and no batch has been enqueued since: see
    /// [`BatchQueue::note_idle_tick`].
    idle_tick: bool,
}

impl BatchQueue {
    /// Adds a batch. Returns whether the batch is late for the display
    /// frame: the last tick of the render thread came before it, found
    /// nothing to apply and drew nothing. One batch at most is answered
    /// `true` for one such tick.
    pub(crate) fn enqueue(&self, batch: CommittedBatch) -> bool {
        let mut queue = self.queue.lock().unwrap();
        queue.batches.push_back(batch);
        std::mem::take(&mut queue.idle_tick)
    }

    fn dequeue(&self) -> Option<CommittedBatch> {
        let mut queue = self.queue.lock().unwrap();
        let batch = queue.batches.pop_front();
        if batch.is_some() {
            queue.idle_tick = false;
        }
        batch
    }

    /// The render thread ends a tick in which it applied no batch and drew
    /// no frame. A batch that is enqueued before the next tick has missed
    /// the tick of its display frame for no reason but its timing; the
    /// compositor asks the render loop for a tick out of turn for it
    /// (desktop-performance.md, section 9.33), which draws the one frame
    /// this display frame is still without.
    ///
    /// Nothing is noted while a batch waits in the queue: it came during the
    /// tick, and is applied by the next one.
    fn note_idle_tick(&self) {
        let mut queue = self.queue.lock().unwrap();
        if queue.batches.is_empty() {
            queue.idle_tick = true;
        }
    }

    /// Takes a stream buffer for a new batch.
    pub(crate) fn rent_data(&self) -> BatchStreamData {
        self.data_pool.lock().unwrap().pop().unwrap_or_default()
    }

    /// Completes the batches nothing will apply any more, without applying
    /// them: the compositor is gone and its server compositor is released,
    /// or about to be. Whoever waits for such a batch, or holds back its
    /// next commit until the batch is processed (the media context does, for
    /// every compositor at once), is answered; what the batches carry (their
    /// jobs among it) is dropped.
    ///
    /// Not from upstream, where a compositor and its server live as long as
    /// the process.
    pub(crate) fn complete_all(&self) {
        loop {
            // Outside the lock of the queue: a continuation of the batch
            // runs here.
            let Some(batch) = self.dequeue() else { break };
            batch.batch.notify_processed();
            batch.batch.notify_rendered();
        }
    }

    fn return_data(&self, mut data: BatchStreamData) {
        data.reset();
        let mut pool = self.data_pool.lock().unwrap();
        if pool.len() < 16 {
            pool.push(data);
        }
    }
}

/// The queue is what the two threads share.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<BatchQueue>();
};

const COMMIT_GRACE_TICKS: i32 = 10;

/// A job in the queues of the server compositor: a job of a batch, or one
/// bound to the server object it was sent for. It never leaves the render
/// thread, unlike the [`ServerJob`] of a batch.
type RenderThreadJob = Box<dyn FnOnce(&ServerCompositor)>;

/// What the readback pass writes for one visual.
pub(crate) enum ReadbackQueueEntry {
    /// The visual, which the pass reads the values of.
    Visual(Rc<ServerCompositionVisual>),
    /// The slots of a visual and the values the update walk left it with.
    Snapshot(Arc<VisualReadback>, ReadbackData),
}

/// Server-side counterpart of the compositor: owns the server objects,
/// applies the batches committed by the UI thread and renders.
///
/// Confined to the render thread. UI-thread code interacts with it only by
/// enqueueing batches (and by reading the clock and the readback indices,
/// which are thread-safe values).
pub struct ServerCompositor {
    this: Weak<ServerCompositor>,
    batches: Arc<BatchQueue>,
    objects: RefCell<Vec<Option<Rc<dyn IServerObject>>>>,
    received_job_queue: RefCell<VecDeque<RenderThreadJob>>,
    disposed_in_batch: RefCell<Vec<ServerObjectId>>,
    received_post_target_job_queue: RefCell<VecDeque<RenderThreadJob>>,
    last_batch_id: Cell<i64>,
    clock: CompositorClock,
    server_now: Cell<Duration>,
    ui_thread_is_inside_render: Cell<bool>,
    /// Set by [`stop_rendering`](Self::stop_rendering).
    rendering_stopped: Cell<bool>,
    render_interface: Rc<PlatformRenderInterfaceContextManager>,
    options: CompositionOptions,
    readback: Arc<ReadbackIndices>,
    ticks_since_last_commit: Cell<i32>,
    /// Whether the batch pass of the current tick applied a batch.
    applied_batches_in_tick: Cell<bool>,
    to_notify_processed: RefCell<Vec<Arc<CompositionBatch>>>,
    to_notify_rendered: RefCell<Vec<Arc<CompositionBatch>>>,
    /// The batches of the tick before, whose frame a layer held back at the
    /// end of that tick: see
    /// [`notify_batches_rendered`](Self::notify_batches_rendered).
    held_rendered: RefCell<Vec<Arc<CompositionBatch>>>,
    render_resources_invalidation_queue: RefCell<VecDeque<Rc<dyn IServerRenderResource>>>,
    render_resources_invalidation_set: RefCell<HashSet<*const ()>>,
    // TODO: parallel processing maybe
    visual_own_properties_recompute_pass: RefCell<VecDeque<Rc<ServerCompositionVisual>>>,
    /// What the next readback pass writes, a visual at most once.
    visual_readback_update_pass_queue: RefCell<Vec<ReadbackQueueEntry>>,
    /// The number of the next readback pass, which the visuals in its queue
    /// are stamped with. Starts at one: zero is a visual in no queue.
    visual_readback_pass: Cell<u64>,
    adorner_update_queue: RefCell<VecDeque<Rc<ServerCompositionVisual>>>,
    /// The visual surfaces of the compositor, for their live updates.
    visual_surfaces: RefCell<Vec<Weak<super::ServerCompositionVisualSurface>>>,
    // Synchronous UI-thread frames and render-thread frames share this graph
    // under its lock. Lights must follow the graph, rather than a thread.
    lights: RefCell<Vec<Weak<dyn IServerCompositionLight>>>,
    active_targets: RefCell<Vec<Rc<ServerCompositionTarget>>>,
    pools: CompositorPools,
    animations: ServerCompositorAnimations,
    render_interface_feature_cache: RefCell<Option<Rc<HashMap<TypeId, Rc<dyn Any>>>>>,
    /// The text renderer of the debug overlays. It is made from the default
    /// typeface, which belongs to the thread of the font manager: see
    /// [`diagnostic_text_renderer`](Self::diagnostic_text_renderer).
    diagnostic_text_renderer: RefCell<Option<Rc<super::DiagnosticTextRenderer>>>,
    _context_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    context_change_handler: RefCell<Option<Arc<dyn Fn(super::super::RenderContextState) + Send + Sync>>>,
}

impl ServerCompositor {
    pub(super) fn register_light(&self, light: Weak<dyn IServerCompositionLight>) {
        let mut lights = self.lights.borrow_mut();
        lights.retain(|light| light.strong_count() > 0);
        lights.push(light);
    }

    /// Whether a light was registered and may still be alive: a render walk
    /// asks once, and looks for the lights of a visual only then
    /// (desktop-performance.md, section 9.24). Lights are registered while
    /// batches are applied, never during a walk.
    pub(super) fn has_registered_lights(&self) -> bool {
        self.lights.borrow().iter().any(|light| light.strong_count() > 0)
    }

    pub(super) fn live_lights(&self) -> Vec<Rc<dyn IServerCompositionLight>> {
        self.lights.borrow().iter().filter_map(Weak::upgrade).filter(|light| !light.is_disposed()).collect()
    }

    /// Creates the server compositor on the thread that renders. The queue of
    /// batches and the readback are what it shares with its compositor, which
    /// creates them.
    pub(crate) fn new(
        platform_graphics: Option<Arc<dyn IPlatformGraphics>>,
        options: CompositionOptions,
        batches: Arc<BatchQueue>,
        readback: Arc<ReadbackIndices>,
        clock: CompositorClock,
    ) -> Rc<ServerCompositor> {
        let compositor = Rc::new_cyclic(|this| ServerCompositor {
            this: this.clone(),
            batches,
            objects: RefCell::new(Vec::new()),
            received_job_queue: RefCell::new(VecDeque::new()),
            disposed_in_batch: RefCell::new(Vec::new()),
            received_post_target_job_queue: RefCell::new(VecDeque::new()),
            last_batch_id: Cell::new(0),
            clock,
            server_now: Cell::new(Duration::ZERO),
            ui_thread_is_inside_render: Cell::new(false),
            rendering_stopped: Cell::new(false),
            render_interface: PlatformRenderInterfaceContextManager::new(platform_graphics),
            options,
            readback,
            ticks_since_last_commit: Cell::new(0),
            applied_batches_in_tick: Cell::new(false),
            to_notify_processed: RefCell::new(Vec::new()),
            to_notify_rendered: RefCell::new(Vec::new()),
            held_rendered: RefCell::new(Vec::new()),
            render_resources_invalidation_queue: RefCell::new(VecDeque::new()),
            render_resources_invalidation_set: RefCell::new(HashSet::new()),
            visual_own_properties_recompute_pass: RefCell::new(VecDeque::new()),
            visual_readback_update_pass_queue: RefCell::new(Vec::new()),
            visual_readback_pass: Cell::new(1),
            adorner_update_queue: RefCell::new(VecDeque::new()),
            visual_surfaces: RefCell::new(Vec::new()),
            lights: RefCell::new(Vec::new()),
            active_targets: RefCell::new(Vec::new()),
            pools: CompositorPools::default(),
            animations: ServerCompositorAnimations::new(),
            render_interface_feature_cache: RefCell::new(None),
            diagnostic_text_renderer: RefCell::new(None),
            _context_subscriptions: RefCell::new(Vec::new()),
            context_change_handler: RefCell::new(None),
        });
        let weak = Rc::downgrade(&compositor);
        let disposed = compositor.render_interface.context_disposed(move || {
            if let Some(compositor) = weak.upgrade() {
                compositor.rt_on_context_disposed();
            }
        });
        let weak = Rc::downgrade(&compositor);
        let created = compositor.render_interface.context_created(move |context| {
            if let Some(compositor) = weak.upgrade() {
                compositor.rt_on_context_created(&**context);
            }
        });
        *compositor._context_subscriptions.borrow_mut() = vec![disposed.detach(), created.detach()];
        compositor
    }

    /// The object pools of the tree walks.
    pub fn pools(&self) -> &CompositorPools {
        &self.pools
    }

    /// The clock items and the animated objects of the compositor.
    pub fn animations(&self) -> &ServerCompositorAnimations {
        &self.animations
    }

    /// The sequence id of the batch applied last.
    pub fn last_batch_id(&self) -> i64 {
        self.last_batch_id.get()
    }

    /// The time elapsed on the compositor clock.
    pub fn clock_elapsed(&self) -> Duration {
        (self.clock)()
    }

    /// The compositor clock itself.
    pub fn clock(&self) -> &CompositorClock {
        &self.clock
    }

    /// The compositor time of the frame being rendered.
    pub fn server_now(&self) -> Duration {
        self.server_now.get()
    }

    pub fn render_interface(&self) -> &Rc<PlatformRenderInterfaceContextManager> {
        &self.render_interface
    }

    /// The text renderer of the debug overlays, shared by the targets.
    ///
    /// It can only be created on a thread that has the font manager (the UI
    /// thread; upstream's services are global and the first overlay creates
    /// it wherever it is drawn). The compositor asks for it under the lock
    /// when debug overlays are switched on; a frame of the render thread
    /// uses what is there and draws no overlay text until then.
    pub fn diagnostic_text_renderer(&self) -> Option<Rc<super::DiagnosticTextRenderer>> {
        if self.diagnostic_text_renderer.borrow().is_none() {
            // Absent in a unit test without a platform, and on a thread
            // without services.
            {
                use crate::LocatorExtensions as _;
                crate::FerroLocator::current().get_service::<dyn crate::platform::IFontManagerImpl>()?;
            }
            let renderer = Rc::new(super::DiagnosticTextRenderer::create_default());
            // The labels of the FPS counter, shaped here on the UI thread: the counter is
            // created by the first frame of the render thread, which cannot create glyph runs
            // where it has no services (the render thread of the browser).
            for label in super::FpsCounter::LABELS {
                renderer.shape_label(label);
            }
            *self.diagnostic_text_renderer.borrow_mut() = Some(renderer);
        }
        self.diagnostic_text_renderer.borrow().clone()
    }

    pub fn options(&self) -> &CompositionOptions {
        &self.options
    }

    /// The indices of the values the UI thread reads back from the server.
    pub fn readback(&self) -> &Arc<ReadbackIndices> {
        &self.readback
    }

    pub(crate) fn update_server_time(&self) {
        self.server_now.set(self.clock_elapsed());
    }

    // --- object table -------------------------------------------------------

    /// The server object with the given id, if it is alive.
    pub fn get_object(&self, id: ServerObjectId) -> Option<Rc<dyn IServerObject>> {
        self.objects.borrow().get(id.index()).and_then(Clone::clone)
    }

    /// The server object with the given id as its concrete type.
    pub fn get<T: IServerObject>(&self, id: ServerObjectId) -> Option<Rc<T>> {
        self.get_object(id).and_then(|object| object.into_any_rc().downcast::<T>().ok())
    }

    /// The server object with the given id as an animatable object.
    pub fn get_animated_object(&self, id: ServerObjectId) -> Option<Rc<dyn super::IAnimatedServerObject>> {
        self.get_object(id).and_then(|object| object.as_animated())
    }

    /// The number of live server objects.
    pub fn object_count(&self) -> usize {
        self.objects.borrow().iter().filter(|o| o.is_some()).count()
    }

    fn insert_object(&self, id: ServerObjectId, object: Rc<dyn IServerObject>) {
        let mut objects = self.objects.borrow_mut();
        if objects.len() <= id.index() {
            objects.resize_with(id.index() + 1, || None);
        }
        debug_assert!(objects[id.index()].is_none(), "a server object id was reused while still alive");
        objects[id.index()] = Some(object);
    }

    fn remove_object(&self, id: ServerObjectId) -> Option<Rc<dyn IServerObject>> {
        self.objects.borrow_mut().get_mut(id.index()).and_then(Option::take)
    }

    // --- batches ------------------------------------------------------------

    fn apply_pending_batches(&self) {
        let mut had_batches = false;
        while let Some(mut batch) = self.batches.dequeue() {
            // The batch belongs to this frame from here on: the thread of
            // the compositor may start on the next one.
            batch.batch.notify_accepted();
            {
                let mut stream = BatchStreamReader::new(&mut batch.changes);
                while !stream.is_object_eof() {
                    match stream.read_object() {
                        BatchObject::Marker(BatchMarker::CreateStart) => self.read_create_jobs(&mut stream),
                        BatchObject::Marker(BatchMarker::RenderThreadJobsStart) => self.read_server_jobs(
                            &mut stream,
                            &self.received_job_queue,
                            BatchMarker::RenderThreadJobsEnd,
                        ),
                        BatchObject::Marker(BatchMarker::RenderThreadPostTargetJobsStart) => self.read_server_jobs(
                            &mut stream,
                            &self.received_post_target_job_queue,
                            BatchMarker::RenderThreadPostTargetJobsEnd,
                        ),
                        BatchObject::Marker(BatchMarker::RenderThreadDisposeStart) => {
                            self.read_dispose_jobs(&mut stream)
                        }
                        BatchObject::ServerObject(id) => {
                            let Some(target) = self.get_object(id) else {
                                panic!("a batch refers to server object {id:?}, which does not exist");
                            };
                            target.deserialize_changes(&mut stream, batch.committed_at);
                            if cfg!(debug_assertions) {
                                // Each object's changes are followed by a
                                // marker in debug builds, which catches a
                                // serializer and deserializer that disagree.
                                match stream.read_object() {
                                    BatchObject::Marker(BatchMarker::ObjectEnd) => {}
                                    _ => panic!("server object {id:?} failed to deserialize properly on the object stream"),
                                }
                                if stream.read::<u64>() != OBJECT_END_MAGIC {
                                    panic!("server object {id:?} failed to deserialize properly on the data stream");
                                }
                            }
                        }
                        _ => panic!("unexpected item at the top level of a batch object stream"),
                    }
                }
            }
            // The objects the batch released leave the table once the whole
            // batch has been read: its jobs, which follow the dispose list,
            // resolve them.
            for id in std::mem::take(&mut *self.disposed_in_batch.borrow_mut()) {
                self.remove_object(id);
            }
            self.batches.return_data(std::mem::take(&mut batch.changes));
            self.last_batch_id.set(batch.batch.sequence_id());
            self.to_notify_processed.borrow_mut().push(batch.batch);
            had_batches = true;
        }

        self.applied_batches_in_tick.set(had_batches);
        if had_batches {
            self.ticks_since_last_commit.set(0);
        } else if self.ticks_since_last_commit.get() < i32::MAX {
            self.ticks_since_last_commit.set(self.ticks_since_last_commit.get() + 1);
        }
    }

    fn read_create_jobs(&self, reader: &mut BatchStreamReader<'_>) {
        let Some(this) = self.this.upgrade() else { return };
        let mut count = reader.read::<i32>();
        while count > 0 {
            let Some(id) = reader.read_server_object() else {
                panic!("a batch creates a server object without an id");
            };
            let BatchObject::Create(factory) = reader.read_object() else {
                panic!("a batch creates a server object without a factory");
            };
            self.insert_object(id, factory(&this, id));
            count -= 1;
        }
    }

    fn read_server_jobs(&self, reader: &mut BatchStreamReader<'_>, queue: &RefCell<VecDeque<RenderThreadJob>>, end: BatchMarker) {
        loop {
            match reader.read_object() {
                BatchObject::Marker(marker) if marker == end => break,
                BatchObject::Job(job) => queue.borrow_mut().push_back(job),
                BatchObject::ServerObject(id) => {
                    let target = self.get_object(id);
                    match reader.read_object() {
                        BatchObject::ObjectJob(job) => {
                            queue.borrow_mut().push_back(Box::new(move |compositor| job(compositor, target)))
                        }
                        _ => panic!("a server object in the job list of a batch is not followed by its job"),
                    }
                }
                _ => panic!("unexpected item in the job list of a batch"),
            }
        }
    }

    fn read_dispose_jobs(&self, reader: &mut BatchStreamReader<'_>) {
        let mut count = reader.read::<i32>();
        while count > 0 {
            let id = reader.read_server_object();
            let dispose = reader.read::<bool>();
            let release = reader.read::<bool>();
            if let Some(id) = id {
                if let Some(object) = self.get_object(id) {
                    if dispose {
                        object.dispose();
                    }
                    if release {
                        self.disposed_in_batch.borrow_mut().push(id);
                    }
                }
            }
            count -= 1;
        }
    }

    fn execute_server_jobs(&self, queue: &RefCell<VecDeque<RenderThreadJob>>) {
        loop {
            let Some(job) = queue.borrow_mut().pop_front() else { break };
            job(self);
        }
    }

    fn notify_batches_processed(&self) {
        let processed = std::mem::take(&mut *self.to_notify_processed.borrow_mut());
        for batch in &processed {
            batch.notify_processed();
        }
        self.to_notify_rendered.borrow_mut().extend(processed);
    }

    /// Ends a tick: the batches whose frame is on its way to the screen are
    /// reported as rendered.
    ///
    /// These are the batches the tick applied, unless a layer holds the
    /// frame of the tick back
    /// ([`IDrawingContextLayerImpl::has_pending_frame`](crate::platform::IDrawingContextLayerImpl::has_pending_frame)):
    /// what the tick presented is then the frame before, and the batches
    /// wait for the next tick, which presents theirs (the compositor asks
    /// for that tick itself). They wait for one tick, never longer: a target
    /// that cannot present in that tick could not have presented upstream
    /// either, where the batches are reported regardless.
    ///
    /// Not from upstream, where every tick presents what it drew
    /// (desktop-performance.md, section 9.33).
    fn notify_batches_rendered(&self) {
        let held = std::mem::take(&mut *self.held_rendered.borrow_mut());
        for batch in &held {
            batch.notify_rendered();
        }
        let rendered = std::mem::take(&mut *self.to_notify_rendered.borrow_mut());
        if !rendered.is_empty() && self.has_pending_frames() {
            *self.held_rendered.borrow_mut() = rendered;
            return;
        }
        for batch in &rendered {
            batch.notify_rendered();
        }
    }

    /// Whether a layer of an active target holds a frame back. Never while
    /// rendering is stopped: no tick would present the frame.
    pub(crate) fn has_pending_frames(&self) -> bool {
        if self.rendering_stopped.get() {
            return false;
        }
        // At the end of a tick that unwinds the list may still be borrowed.
        let Ok(targets) = self.active_targets.try_borrow() else { return false };
        targets.iter().any(|target| target.has_pending_frame())
    }

    // --- passes -------------------------------------------------------------

    fn apply_enqueued_render_resource_changes_pass(&self) {
        loop {
            let Some(resource) = self.render_resources_invalidation_queue.borrow_mut().pop_front() else { break };
            // Removed before the call so a resource invalidated again while
            // observers react is queued again, like the dequeue upstream.
            resource.queued_invalidate();
        }
        self.render_resources_invalidation_set.borrow_mut().clear();
    }

    /// Queues a render resource whose observers must be told that it
    /// changed, once per frame.
    pub fn enqueue_render_resource_for_invalidation(&self, resource: Rc<dyn IServerRenderResource>) {
        let key = Rc::as_ptr(&resource) as *const ();
        if self.render_resources_invalidation_set.borrow_mut().insert(key) {
            self.render_resources_invalidation_queue.borrow_mut().push_back(resource);
        }
    }

    fn visual_own_properties_update_pass(&self) {
        loop {
            let Some(visual) = self.visual_own_properties_recompute_pass.borrow_mut().pop_front() else { break };
            visual.recompute_own_properties();
        }
        // The first frame of a large tree queues every visual of it.
        self.visual_own_properties_recompute_pass.borrow_mut().release_excess();
    }

    pub fn enqueue_visual_for_own_properties_update_pass(&self, visual: Rc<ServerCompositionVisual>) {
        self.visual_own_properties_recompute_pass.borrow_mut().push_back(visual);
    }

    pub(crate) fn visual_readback_update_pass(&self) {
        if self.visual_readback_update_pass_queue.borrow().is_empty() {
            return;
        }

        // The queue is taken whole and the next pass starts before the
        // entries are written: a visual enqueued from here on is in the queue
        // of the next pass.
        let mut entries = std::mem::take(&mut *self.visual_readback_update_pass_queue.borrow_mut());
        self.visual_readback_pass.set(self.visual_readback_pass.get() + 1);

        // visual.HitTest is waiting for this lock to be released, so we need to be quick
        // this is why we have a queue in the first place
        let scope = self.readback.begin_write();
        let read = self.readback.read_revision();
        let write = self.readback.write_revision();
        // The readbacks are listed as they are written (not from upstream), so that an index of
        // the children of a visual reads the children this pass wrote and no others
        // (desktop-performance.md, section 9.49).
        let mut written = scope.written(entries.len());
        for entry in entries.drain(..) {
            let key = match entry {
                ReadbackQueueEntry::Visual(visual) => {
                    visual.update_readback(write, read);
                    visual.readback_key()
                }
                ReadbackQueueEntry::Snapshot(readback, data) => {
                    readback.update_readback(write, read, data);
                    readback.key()
                }
            };
            if let Some(written) = written.as_mut() {
                written.push(key);
            }
        }
        drop(written);
        drop(scope);

        // The storage of the queue is kept for the next pass, up to what an
        // emptied buffer keeps: the pass of the first frame of a large tree
        // writes every visual of it.
        let mut queue = self.visual_readback_update_pass_queue.borrow_mut();
        if queue.is_empty() {
            entries.release_excess();
            *queue = entries;
        }
    }

    /// How many visuals the next readback pass writes.
    #[cfg(test)]
    pub(crate) fn readback_queue_len(&self) -> usize {
        self.visual_readback_update_pass_queue.borrow().len()
    }

    /// The number of the readback pass the queue is collected for.
    pub(crate) fn readback_pass(&self) -> u64 {
        self.visual_readback_pass.get()
    }

    /// Adds an entry to the queue of the readback pass and returns where it
    /// is.
    pub(crate) fn push_readback_entry(&self, entry: ReadbackQueueEntry) -> usize {
        let mut queue = self.visual_readback_update_pass_queue.borrow_mut();
        queue.push(entry);
        queue.len() - 1
    }

    /// Replaces the entry at `index` of the queue of the readback pass.
    pub(crate) fn set_readback_entry(&self, index: usize, entry: ReadbackQueueEntry) {
        let previous = match self.visual_readback_update_pass_queue.borrow_mut().get_mut(index) {
            Some(slot) => std::mem::replace(slot, entry),
            None => return,
        };
        // A visual is released outside the borrow of the queue.
        drop(previous);
    }

    /// Makes the entry at `index` of the queue of the readback pass a copy
    /// of the values `data` for the slots `readback`.
    pub(crate) fn set_readback_snapshot(&self, index: usize, readback: &Arc<VisualReadback>, data: ReadbackData) {
        let mut queue = self.visual_readback_update_pass_queue.borrow_mut();
        let previous = match queue.get_mut(index) {
            Some(ReadbackQueueEntry::Snapshot(_, values)) => {
                *values = data;
                return;
            }
            Some(slot) => std::mem::replace(slot, ReadbackQueueEntry::Snapshot(readback.clone(), data)),
            None => return,
        };
        drop(queue);
        drop(previous);
    }

    pub fn enqueue_adorner_update(&self, visual: Rc<ServerCompositionVisual>) {
        self.adorner_update_queue.borrow_mut().push_back(visual);
    }

    fn adorner_update_pass(&self) {
        loop {
            let Some(adorner) = self.adorner_update_queue.borrow_mut().pop_front() else { break };
            adorner.update_adorner();
        }
    }

    /// Registers a visual surface for its live updates (not from upstream).
    pub fn register_visual_surface(&self, surface: Weak<super::ServerCompositionVisualSurface>) {
        self.visual_surfaces.borrow_mut().push(surface);
    }

    /// Invalidates the visual surfaces whose source changed, and runs the
    /// passes again for what that invalidated (not from upstream).
    fn visual_surface_update_pass(&self) {
        if self.visual_surfaces.borrow().is_empty() {
            return;
        }
        if super::ServerCompositionVisualSurface::update_live_surfaces(self, &self.visual_surfaces) {
            self.apply_enqueued_render_resource_changes_pass();
            self.visual_own_properties_update_pass();
        }
    }

    // --- targets ------------------------------------------------------------

    pub fn add_composition_target(&self, target: &Rc<ServerCompositionTarget>) {
        self.active_targets.borrow_mut().push(target.clone());
    }

    pub fn remove_composition_target(&self, target: &Rc<ServerCompositionTarget>) {
        let mut targets = self.active_targets.borrow_mut();
        if let Some(index) = targets.iter().position(|t| Rc::ptr_eq(t, target)) {
            targets.remove(index);
        }
    }

    pub fn create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        let current = self.render_interface.ensure_current();
        let target = self.render_interface.create_render_target(surfaces);
        current.dispose();
        target
    }

    pub fn is_ready_to_create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> bool {
        self.render_interface.is_ready_to_create_render_target(surfaces)
    }

    // --- user APIs ----------------------------------------------------------

    pub(crate) fn set_context_change_handler(&self, handler: Arc<dyn Fn(super::super::RenderContextState) + Send + Sync>) {
        *self.context_change_handler.borrow_mut() = Some(handler);
    }

    fn notify_context_changed(&self, state: super::super::RenderContextState) {
        let handler = self.context_change_handler.borrow().clone();
        if let Some(handler) = handler { handler(state); }
    }

    fn rt_on_context_created(&self, context: &dyn crate::platform::IPlatformRenderInterfaceContext) {
        *self.render_interface_feature_cache.borrow_mut() = Some(Rc::new(context.public_features()));
        self.notify_context_changed(super::super::RenderContextState::Created);
    }

    fn rt_on_context_disposed(&self) {
        *self.render_interface_feature_cache.borrow_mut() = None;
        self.notify_context_changed(super::super::RenderContextState::Lost);
    }

    /// The public features of the render interface, if a backend context
    /// exists (callable from any place that can reach the compositor).
    pub fn at_try_get_cached_render_interface_features(&self) -> Option<Rc<HashMap<TypeId, Rc<dyn Any>>>> {
        self.render_interface_feature_cache.borrow().clone()
    }

    /// The public features of the render interface; creates the backend
    /// context when needed.
    pub fn rt_get_render_interface_features(&self) -> Rc<HashMap<TypeId, Rc<dyn Any>>> {
        if let Some(features) = self.render_interface_feature_cache.borrow().clone() {
            return features;
        }
        let features = Rc::new(self.render_interface.value().public_features());
        *self.render_interface_feature_cache.borrow_mut() = Some(features.clone());
        features
    }

    /// Renders a visual (and, with `render_children`, its subtree) into a
    /// new bitmap.
    pub fn create_composition_visual_snapshot(
        &self,
        visual: &Rc<ServerCompositionVisual>,
        scaling: f64,
        render_children: bool,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        let current = self.render_interface.ensure_current();
        let size = visual.size();
        let pixel_size = PixelSize::from_size(Size::new(size.x, size.y), scaling);
        let scale_transform = Matrix::create_scale(scaling, scaling);

        let target = self.render_interface.value().create_offscreen_render_target(
            pixel_size,
            Vector::new(scaling, scaling),
            true,
        );
        {
            let mut canvas = target.create_drawing_context();
            canvas.set_transform(scale_transform);
            visual.render(&mut *canvas, LtrbRect::INFINITE, None, render_children, false, false);
            canvas.dispose();
        }

        let result: std::sync::Arc<crate::platform::SharedBitmapImpl> = match target.as_layer_with_render_context_affinity() {
            Some(affined) if affined.has_render_context_affinity() => {
                let snapshot = affined.create_non_affined_snapshot();
                target.dispose();
                snapshot
            }
            // The original returns the target itself; a layer does not
            // leave the render thread here, so its contents do.
            _ => {
                let snapshot = target.create_shared_snapshot();
                target.dispose();
                snapshot
            }
        };
        current.dispose();
        result
    }

    pub fn reset_all_gpu_resources(&self) {
        let targets = self.active_targets.borrow().clone();
        for target in targets {
            target.reset_render_target();
        }
        self.render_interface.reset();
    }

    /// Releases what the compositor draws with, while its graphics context
    /// is current: the layer and the render target of every active target,
    /// then the backend context. The context that was current before is
    /// restored. Does nothing when no backend context was ever created, and
    /// nothing for a graphics context that is lost (nothing can be released
    /// in it).
    ///
    /// For the two places where these objects go for good: before the graph
    /// of a compositor is dropped, and when the surface the compositor draws
    /// to is closed. An object of a graphics interface with a current
    /// context (OpenGL) is deleted in whatever context is current when it is
    /// dropped: with none current the call fails, and with the context of
    /// another compositor current it deletes an object of that one. Once the
    /// backend context is released here, what is dropped later touches no
    /// graphics context.
    ///
    /// Unlike [`reset_all_gpu_resources`](Self::reset_all_gpu_resources),
    /// nothing is asked for that may have to be created: it also works for
    /// graphics that are not ready, and after the compositor has taken its
    /// platform handles back.
    ///
    /// Not from upstream, where these objects are finalized by the garbage
    /// collector, on the thread of the context.
    pub fn release_gpu_resources(&self) {
        if self.render_interface.existing_backend_context().is_none() {
            return;
        }
        let gpu_context = self.render_interface.gpu_context();
        if gpu_context.as_ref().is_some_and(|context| context.is_lost()) {
            return;
        }
        let current = gpu_context.map(|context| context.ensure_current());
        let targets = self.active_targets.borrow().clone();
        for target in targets {
            target.release_render_target();
        }
        self.render_interface.reset();
        if let Some(current) = current {
            current.dispose();
        }
    }

    /// Ends the rendering of the compositor for good: from here on a frame
    /// applies the batches that arrive (so that whoever waits for one is
    /// answered), drops the jobs they carry, renders nothing and does not
    /// ask for another tick.
    ///
    /// For a compositor whose surface is closed while the compositor lives
    /// on: its graphics would be "not ready" for ever, and a compositor that
    /// is not ready asks for the next tick each time. Upstream's browser
    /// backend takes the server compositor out of the render loop at that
    /// point; here the compositor stays in the loop, because the thread of
    /// the compositor still waits for the batch that disposes its target.
    pub fn stop_rendering(&self) {
        self.rendering_stopped.set(true);
    }

    /// Whether [`stop_rendering`](Self::stop_rendering) was called.
    pub fn is_rendering_stopped(&self) -> bool {
        self.rendering_stopped.get()
    }

    pub fn invalidate_all_composition_targets(&self) {
        let targets = self.active_targets.borrow().clone();
        for target in targets {
            target.request_full_redraw();
        }
    }

    // --- rendering ----------------------------------------------------------

    /// Whether the caller may access the objects of the compositor: whether
    /// the compositor is rendering a frame. Upstream the frame is rendered
    /// under the compositor lock by the thread that holds it.
    pub fn check_access(&self) -> bool {
        self.ui_thread_is_inside_render.get()
    }

    /// Panics when the caller may not access the objects of the compositor
    /// (see [`check_access`](Self::check_access)).
    pub fn verify_access(&self) {
        if !self.check_access() {
            panic!("This object can be only accessed under compositor lock");
        }
    }

    /// [`verify_access`](Self::verify_access) of a compositor that may be
    /// gone: no frame of a compositor that is gone is being rendered.
    /// Returns the compositor.
    pub fn verify_access_of(compositor: Option<&Rc<ServerCompositor>>) -> Rc<ServerCompositor> {
        match compositor {
            Some(compositor) => {
                compositor.verify_access();
                compositor.clone()
            }
            None => panic!("This object can be only accessed under compositor lock"),
        }
    }

    /// Runs one frame: applies the pending batches, runs the global passes
    /// and the queued jobs. Returns whether another tick is needed even if
    /// nothing else wakes the render loop up.
    ///
    /// Panics on reentrancy.
    pub fn render(&self) -> bool {
        crate::rendering::native_render_scope::run(|| self.render_in_native_scope())
    }

    fn render_in_native_scope(&self) -> bool {
        if self.ui_thread_is_inside_render.replace(true) {
            panic!("Reentrancy is not supported");
        }
        let _reset = ResetFlag(&self.ui_thread_is_inside_render);

        // While the UI thread renders, nothing may pump its dispatcher from
        // inside the frame. A frame of the render thread leaves the
        // dispatcher alone.
        let dispatcher = Dispatcher::ui_thread();
        let processing_disabled = dispatcher.check_access().then(|| dispatcher.disable_processing());
        let _notify = NotifyRendered(self);
        // Bound after `_notify`, so processing is enabled again (when the
        // value drops, also when the frame unwinds) before the rendered
        // notification runs.
        let _enable = processing_disabled;
        self.render_core()
    }

    fn execute_global_passes(&self) -> Duration {
        let compositor_global_passes_started = self.clock_elapsed();
        self.apply_pending_batches();
        self.notify_batches_processed();
        self.animations.process();
        self.apply_enqueued_render_resource_changes_pass();
        self.visual_own_properties_update_pass();
        // A light whose coordinate space moved in this pass draws what it
        // lights again (winui-materials.md): the content it invalidates is
        // recomputed in a second pass.
        if super::invalidate_lights_with_moved_spaces(self) {
            self.visual_own_properties_update_pass();
        }
        // Adorners need to be updated after own properties recompute pass,
        // because they may depend on ancestor's transform chain to be consistent
        self.adorner_update_pass();
        // The visual surfaces follow their sources once the sources have
        // their new state (the WinUI port).
        self.visual_surface_update_pass();
        self.clock_elapsed().saturating_sub(compositor_global_passes_started)
    }

    fn render_core(&self) -> bool {
        self.update_server_time();
        let compositor_global_passes_elapsed = self.execute_global_passes();

        if self.rendering_stopped.get() {
            // The jobs are dropped outside the borrow of their queue.
            let jobs = std::mem::take(&mut *self.received_job_queue.borrow_mut());
            let post_target_jobs = std::mem::take(&mut *self.received_post_target_job_queue.borrow_mut());
            drop((jobs, post_target_jobs));
            return false;
        }

        if !self.render_interface.is_ready() {
            return true;
        }
        self.render_interface.ensure_valid_backend_context();
        self.execute_server_jobs(&self.received_job_queue);

        let targets = self.active_targets.borrow().clone();
        let mut drew_frame = false;
        for target in &targets {
            target.update(compositor_global_passes_elapsed);
            target.render();
            drew_frame |= target.drew_frame();
        }
        if !drew_frame && !self.applied_batches_in_tick.get() {
            self.batches.note_idle_tick();
        }

        self.visual_readback_update_pass();

        self.execute_server_jobs(&self.received_post_target_job_queue);

        // Completion is independent of whether a visual still requests
        // presentation. Retire submitted reads even after the last view detaches.
        let pending_gpu_work = self.render_interface.poll_pending_gpu_work();

        // Request a tick if we have active animations or if there are recent batches
        if pending_gpu_work || self.animations.need_next_tick() || self.ticks_since_last_commit.get() < COMMIT_GRACE_TICKS {
            return true;
        }

        // A frame a layer holds back is presented by the next tick.
        if self.has_pending_frames() {
            return true;
        }

        // Request a tick if we had unready targets in the last tick, to check if they are ready next time
        // But skip targets that are waiting for a render loop wakeup from the platform
        let targets = self.active_targets.borrow().clone();
        for target in &targets {
            if target.is_waiting_for_ready_render_target() && !target.is_waiting_for_render_loop_wakeup() {
                return true;
            }
        }

        // Otherwise there is no need to waste CPU cycles, tell the timer to pause
        false
    }
}

/// The value written to the value stream after each object's changes in
/// debug builds.
pub(crate) const OBJECT_END_MAGIC: u64 = 0x4F42_4A45_4E44_2121;

struct ResetFlag<'a>(&'a Cell<bool>);

impl Drop for ResetFlag<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl Drop for ServerCompositor {
    /// No tick follows: the batches that waited for one are answered.
    fn drop(&mut self) {
        for batch in std::mem::take(self.held_rendered.get_mut()) {
            batch.notify_rendered();
        }
    }
}

struct NotifyRendered<'a>(&'a ServerCompositor);

impl Drop for NotifyRendered<'_> {
    fn drop(&mut self) {
        self.0.notify_batches_rendered();
    }
}
