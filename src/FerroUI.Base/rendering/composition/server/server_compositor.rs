use super::{
    CompositorPools, IServerObject, IServerRenderResource, ReadbackIndices, ServerCompositionTarget,
    ServerCompositionVisual, ServerCompositorAnimations, ServerObjectId,
};
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{IBitmapImpl, IPlatformGraphics, IRenderTarget, LtrbRect};
use crate::rendering::composition::transport::{
    BatchMarker, BatchObject, BatchStreamData, BatchStreamReader, CommittedBatch, CompositionBatch, ServerJob,
};
use crate::rendering::composition::CompositionOptions;
use crate::rendering::PlatformRenderInterfaceContextManager;
use crate::reactive::IDisposable;
use crate::threading::Dispatcher;
use crate::{Matrix, PixelSize, Size, Vector};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// The source of the compositor clock: time elapsed since an arbitrary
/// origin. It is read from both sides of the transport; like the server
/// compositor it is confined to the compositor's thread for now, because
/// the dispatcher's clock is.
pub type CompositorClock = Rc<dyn Fn() -> Duration>;

/// The queue of committed batches: the one point where data passes from
/// the UI-thread compositor to the server compositor.
///
/// Both sides run on one thread today (see the threading notes in
/// `rendering/composition/mod.rs`), so the queue is a `RefCell`. Moving the
/// server to its own thread means making [`CommittedBatch`] `Send` and
/// turning this into a mutex-protected queue; nothing else is shared.
#[derive(Default)]
pub struct BatchQueue {
    batches: RefCell<VecDeque<CommittedBatch>>,
    /// Emptied stream buffers, returned by the server for reuse.
    data_pool: RefCell<Vec<BatchStreamData>>,
}

impl BatchQueue {
    pub(crate) fn enqueue(&self, batch: CommittedBatch) {
        self.batches.borrow_mut().push_back(batch);
    }

    fn dequeue(&self) -> Option<CommittedBatch> {
        self.batches.borrow_mut().pop_front()
    }

    /// Takes a stream buffer for a new batch.
    pub(crate) fn rent_data(&self) -> BatchStreamData {
        self.data_pool.borrow_mut().pop().unwrap_or_default()
    }

    fn return_data(&self, mut data: BatchStreamData) {
        data.reset();
        let mut pool = self.data_pool.borrow_mut();
        if pool.len() < 16 {
            pool.push(data);
        }
    }
}

const COMMIT_GRACE_TICKS: i32 = 10;

/// Server-side counterpart of the compositor: owns the server objects,
/// applies the batches committed by the UI thread and renders.
///

/// A job in the queues of the server compositor: a job of a batch, or one
/// bound to the server object it was sent for. It never leaves the render
/// thread, unlike the [`ServerJob`] of a batch.
type RenderThreadJob = Box<dyn FnOnce(&ServerCompositor)>;

/// Confined to the render thread. UI-thread code interacts with it only by
/// enqueueing batches (and by reading the clock and the readback indices,
/// which are thread-safe values).
pub struct ServerCompositor {
    this: Weak<ServerCompositor>,
    batches: Rc<BatchQueue>,
    objects: RefCell<Vec<Option<Rc<dyn IServerObject>>>>,
    received_job_queue: RefCell<VecDeque<RenderThreadJob>>,
    disposed_in_batch: RefCell<Vec<ServerObjectId>>,
    received_post_target_job_queue: RefCell<VecDeque<RenderThreadJob>>,
    last_batch_id: Cell<i64>,
    clock: CompositorClock,
    server_now: Cell<Duration>,
    ui_thread_is_inside_render: Cell<bool>,
    render_interface: Rc<PlatformRenderInterfaceContextManager>,
    options: CompositionOptions,
    readback: Arc<ReadbackIndices>,
    ticks_since_last_commit: Cell<i32>,
    to_notify_processed: RefCell<Vec<Arc<CompositionBatch>>>,
    to_notify_rendered: RefCell<Vec<Arc<CompositionBatch>>>,
    render_resources_invalidation_queue: RefCell<VecDeque<Rc<dyn IServerRenderResource>>>,
    render_resources_invalidation_set: RefCell<HashSet<*const ()>>,
    // TODO: parallel processing maybe
    visual_own_properties_recompute_pass: RefCell<VecDeque<Rc<ServerCompositionVisual>>>,
    visual_readback_update_pass_queue: RefCell<VecDeque<Rc<ServerCompositionVisual>>>,
    adorner_update_queue: RefCell<VecDeque<Rc<ServerCompositionVisual>>>,
    active_targets: RefCell<Vec<Rc<ServerCompositionTarget>>>,
    pools: CompositorPools,
    animations: ServerCompositorAnimations,
    render_interface_feature_cache: RefCell<Option<Rc<HashMap<TypeId, Rc<dyn Any>>>>>,
    _context_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl ServerCompositor {
    pub(crate) fn new(
        platform_graphics: Option<Rc<dyn IPlatformGraphics>>,
        options: CompositionOptions,
        batches: Rc<BatchQueue>,
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
            render_interface: PlatformRenderInterfaceContextManager::new(platform_graphics),
            options,
            readback: Arc::new(ReadbackIndices::new()),
            ticks_since_last_commit: Cell::new(0),
            to_notify_processed: RefCell::new(Vec::new()),
            to_notify_rendered: RefCell::new(Vec::new()),
            render_resources_invalidation_queue: RefCell::new(VecDeque::new()),
            render_resources_invalidation_set: RefCell::new(HashSet::new()),
            visual_own_properties_recompute_pass: RefCell::new(VecDeque::new()),
            visual_readback_update_pass_queue: RefCell::new(VecDeque::new()),
            adorner_update_queue: RefCell::new(VecDeque::new()),
            active_targets: RefCell::new(Vec::new()),
            pools: CompositorPools::default(),
            animations: ServerCompositorAnimations::new(),
            render_interface_feature_cache: RefCell::new(None),
            _context_subscriptions: RefCell::new(Vec::new()),
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
        *compositor._context_subscriptions.borrow_mut() = vec![disposed, created];
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

    fn notify_batches_rendered(&self) {
        let rendered = std::mem::take(&mut *self.to_notify_rendered.borrow_mut());
        for batch in &rendered {
            batch.notify_rendered();
        }
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
    }

    pub fn enqueue_visual_for_own_properties_update_pass(&self, visual: Rc<ServerCompositionVisual>) {
        self.visual_own_properties_recompute_pass.borrow_mut().push_back(visual);
    }

    fn visual_readback_update_pass(&self) {
        if self.visual_readback_update_pass_queue.borrow().is_empty() {
            return;
        }

        // visual.HitTest is waiting for this lock to be released, so we need to be quick
        // this is why we have a queue in the first place
        let scope = self.readback.begin_write();
        let read = self.readback.read_revision();
        let write = self.readback.write_revision();
        loop {
            let Some(visual) = self.visual_readback_update_pass_queue.borrow_mut().pop_front() else { break };
            visual.update_readback(write, read);
        }
        drop(scope);
    }

    pub fn enqueue_visual_for_readback_update_pass(&self, visual: Rc<ServerCompositionVisual>) {
        self.visual_readback_update_pass_queue.borrow_mut().push_back(visual);
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

    pub fn create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        let current = self.render_interface.ensure_current();
        let target = self.render_interface.create_render_target(surfaces);
        current.dispose();
        target
    }

    pub fn is_ready_to_create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> bool {
        self.render_interface.is_ready_to_create_render_target(surfaces)
    }

    // --- user APIs ----------------------------------------------------------

    fn rt_on_context_created(&self, context: &dyn crate::platform::IPlatformRenderInterfaceContext) {
        *self.render_interface_feature_cache.borrow_mut() = Some(Rc::new(context.public_features()));
    }

    fn rt_on_context_disposed(&self) {
        *self.render_interface_feature_cache.borrow_mut() = None;
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
        if self.ui_thread_is_inside_render.replace(true) {
            panic!("Reentrancy is not supported");
        }
        let _reset = ResetFlag(&self.ui_thread_is_inside_render);

        // While the server runs on a dispatcher thread, nothing may pump the
        // dispatcher from inside a frame.
        let dispatcher = Dispatcher::current_dispatcher();
        let processing_disabled = dispatcher.disable_processing();
        let _notify = NotifyRendered(self);
        // Declared after `_notify`, so processing is enabled again before
        // the rendered notification runs.
        let _enable = EnableProcessing(processing_disabled);
        self.render_core()
    }

    fn execute_global_passes(&self) -> Duration {
        let compositor_global_passes_started = self.clock_elapsed();
        self.apply_pending_batches();
        self.notify_batches_processed();
        self.animations.process();
        self.apply_enqueued_render_resource_changes_pass();
        self.visual_own_properties_update_pass();
        // Adorners need to be updated after own properties recompute pass,
        // because they may depend on ancestor's transform chain to be consistent
        self.adorner_update_pass();
        self.clock_elapsed().saturating_sub(compositor_global_passes_started)
    }

    fn render_core(&self) -> bool {
        self.update_server_time();
        let compositor_global_passes_elapsed = self.execute_global_passes();

        if !self.render_interface.is_ready() {
            return true;
        }
        self.render_interface.ensure_valid_backend_context();
        self.execute_server_jobs(&self.received_job_queue);

        let targets = self.active_targets.borrow().clone();
        for target in &targets {
            target.update(compositor_global_passes_elapsed);
            target.render();
        }

        self.visual_readback_update_pass();

        self.execute_server_jobs(&self.received_post_target_job_queue);

        // Request a tick if we have active animations or if there are recent batches
        if self.animations.need_next_tick() || self.ticks_since_last_commit.get() < COMMIT_GRACE_TICKS {
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

struct NotifyRendered<'a>(&'a ServerCompositor);

impl Drop for NotifyRendered<'_> {
    fn drop(&mut self) {
        self.0.notify_batches_rendered();
    }
}

/// Re-enables dispatcher processing when the frame ends, also when it
/// unwinds: the handle only re-enables on `dispose`.
struct EnableProcessing(crate::threading::DispatcherProcessingDisabled);

impl Drop for EnableProcessing {
    fn drop(&mut self) {
        crate::reactive::IDisposable::dispose(&self.0);
    }
}
