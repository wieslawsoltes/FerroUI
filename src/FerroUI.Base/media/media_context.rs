use super::media_context_clock::MediaContextClock;
use crate::animation::{IGlobalClock, TimeSpan};
use crate::platform::IPlatformRenderInterface;
use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::composition::transport::CompositionBatch;
use crate::rendering::composition::{Compositor, ICompositorScheduler};
use crate::rendering::IRenderer;
use crate::threading::{Dispatcher, DispatcherOperation, DispatcherOptions, DispatcherPriority, DispatcherTimer};
use crate::{FerroLocator, LocatorExtensions};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

type RenderCallback = Rc<dyn Fn()>;

struct TopLevelInfo {
    renderer: Rc<dyn IRenderer>,
}

/// Coordinates the work that has to happen once per frame on the UI thread:
/// the animation clock, layout passes and other callbacks that must run
/// right before rendering, and the commit of the compositors.
pub struct MediaContext {
    this: Weak<MediaContext>,
    next_render_op: RefCell<Option<DispatcherOperation<()>>>,
    input_marker_op: RefCell<Option<DispatcherOperation<()>>>,
    input_marker_added_at: Cell<Duration>,
    is_rendering: Cell<bool>,
    animations_are_waiting_for_composition: Cell<bool>,
    max_seconds_without_input: f64,
    requested_commits: RefCell<Vec<Rc<Compositor>>>,
    pending_composition_batches: RefCell<Vec<(Weak<Compositor>, Arc<CompositionBatch>)>>,
    dispatcher: Arc<Dispatcher>,
    invoke_on_render_callbacks: RefCell<Option<Vec<RenderCallback>>>,
    invoke_on_render_callback_list_pool: RefCell<Vec<Vec<RenderCallback>>>,
    animations_timer: Rc<DispatcherTimer>,
    top_levels: RefCell<HashMap<usize, TopLevelInfo>>,
    clock: Rc<MediaContextClock>,
    time_origin: i64,
}

fn same_compositor(a: &Compositor, b: &Compositor) -> bool {
    std::ptr::eq(a, b)
}

impl MediaContext {
    fn new(dispatcher: Arc<Dispatcher>, input_starvation_timeout: Duration) -> Rc<MediaContext> {
        // Since this timer is used to drive animations that didn't
        // contribute to the previous frame at all, a 16ms interval is safe
        // until the animation system reports the next expected frame.
        let animations_timer = DispatcherTimer::with_priority(DispatcherPriority::RENDER);
        animations_timer.set_interval(Duration::from_millis(16));
        let time_origin = dispatcher.platform_impl().now();
        let context = Rc::new_cyclic(|this: &Weak<MediaContext>| MediaContext {
            this: this.clone(),
            next_render_op: RefCell::new(None),
            input_marker_op: RefCell::new(None),
            input_marker_added_at: Cell::new(Duration::ZERO),
            is_rendering: Cell::new(false),
            animations_are_waiting_for_composition: Cell::new(false),
            max_seconds_without_input: input_starvation_timeout.as_secs_f64(),
            requested_commits: RefCell::new(Vec::new()),
            pending_composition_batches: RefCell::new(Vec::new()),
            dispatcher,
            invoke_on_render_callbacks: RefCell::new(None),
            invoke_on_render_callback_list_pool: RefCell::new(Vec::new()),
            animations_timer,
            top_levels: RefCell::new(HashMap::new()),
            clock: MediaContextClock::new(this.clone()),
            time_origin,
        });
        let weak = Rc::downgrade(&context);
        // The subscription lives as long as the timer, which the context
        // owns.
        let _ = context.animations_timer.tick(move |timer| {
            timer.stop();
            if let Some(context) = weak.upgrade() {
                context.schedule_render(false);
            }
        });
        context
    }

    /// The media context of the application.
    ///
    /// It is registered in the service locator and created on first use, so
    /// a locator scope (a unit test) gets a fresh one.
    pub fn instance() -> Rc<MediaContext> {
        if let Some(context) = FerroLocator::current().get_service::<MediaContext>() {
            return context;
        }
        let options = FerroLocator::current().get_service::<DispatcherOptions>().map(|o| (*o).clone()).unwrap_or_default();
        // The context belongs to the thread that first asks for it, which is
        // the UI thread in an application.
        let context = MediaContext::new(Dispatcher::current_dispatcher(), options.input_starvation_timeout);
        FerroLocator::current_mutable().bind::<MediaContext>().to_constant(context.clone());
        context
    }

    /// The context as the scheduler compositors request commits from.
    pub fn scheduler(&self) -> Rc<dyn ICompositorScheduler> {
        self.this.upgrade().expect("the media context is alive while it is used")
    }

    /// The time elapsed since the context was created, on the dispatcher's
    /// clock.
    fn elapsed(&self) -> Duration {
        Duration::from_millis((self.dispatcher.platform_impl().now() - self.time_origin).max(0) as u64)
    }

    /// Whether a render pass has been requested and not yet run.
    pub fn is_render_scheduled(&self) -> bool {
        self.next_render_op.borrow().is_some()
    }

    pub(super) fn verify_access(&self) {
        self.dispatcher.verify_access();
    }

    pub(super) fn schedule_render(&self, now: bool) {
        // Already scheduled, nothing to do.
        if let Some(next_render_op) = self.next_render_op.borrow().as_ref() {
            if now {
                next_render_op.set_priority(DispatcherPriority::RENDER);
            }
            return;
        }

        // Sometimes the animation, layout and render passes take more than
        // a frame to complete, which can cause a "freeze"-like state where
        // the UI is being updated but input is never processed. An
        // operation with input priority is injected to check whether input
        // has been processed lately; if it has not, the next render is
        // scheduled to happen after all pending input.
        let mut priority = DispatcherPriority::RENDER;

        if self.input_marker_op.borrow().is_none() {
            let weak = self.this.clone();
            let op = self.dispatcher.invoke_async_local_with_priority(
                move || {
                    // Clear the marker so we know that input priority has
                    // been processed.
                    if let Some(context) = weak.upgrade() {
                        *context.input_marker_op.borrow_mut() = None;
                    }
                },
                DispatcherPriority::INPUT,
            );
            *self.input_marker_op.borrow_mut() = Some(op);
            self.input_marker_added_at.set(self.elapsed());
        } else if !now
            && (self.elapsed().saturating_sub(self.input_marker_added_at.get())).as_secs_f64()
                > self.max_seconds_without_input
        {
            priority = DispatcherPriority::INPUT;
        }

        let weak = self.this.clone();
        let render_op = self.dispatcher.invoke_async_local_with_priority(
            move || {
                if let Some(context) = weak.upgrade() {
                    context.render();
                }
            },
            priority,
        );
        *self.next_render_op.borrow_mut() = Some(render_op);
    }

    /// Runs a render pass: pulses the animation clock, fires the queued
    /// pre-render callbacks and commits the compositors.
    pub fn render(&self) {
        struct Finally<'a>(&'a MediaContext);
        impl Drop for Finally<'_> {
            fn drop(&mut self) {
                *self.0.next_render_op.borrow_mut() = None;
                self.0.is_rendering.set(false);
            }
        }

        self.is_rendering.set(true);
        let _finally = Finally(self);
        self.render_core();
    }

    fn render_core(&self) {
        let now = self.elapsed();
        if !self.animations_are_waiting_for_composition.get() {
            self.clock.pulse(TimeSpan::from(now));
        }

        // Since new animations could be started during the layout and can
        // affect layout/render, several iterations are done when it happens.
        for _ in 0..10 {
            self.fire_invoke_on_render_callbacks();

            if self.clock.has_new_subscriptions() {
                self.clock.pulse_new_subscriptions();
                continue;
            }

            break;
        }

        if !self.requested_commits.borrow().is_empty() || self.clock.has_subscriptions() {
            self.animations_are_waiting_for_composition.set(self.commit_compositors_with_throttling());
            if !self.animations_are_waiting_for_composition.get() && self.clock.has_subscriptions() {
                self.animations_timer.start();
            }
        }
    }

    /// Whether a top-level is registered (for unit tests).
    pub fn is_top_level_active(&self, key: usize) -> bool {
        self.top_levels.borrow().contains_key(&key)
    }

    /// Registers a top-level: starts its renderer and schedules a render.
    /// `key` identifies the top-level (the address of its object).
    pub fn add_top_level(&self, key: usize, renderer: Rc<dyn IRenderer>) {
        if self.top_levels.borrow().contains_key(&key) {
            return;
        }
        self.top_levels.borrow_mut().insert(key, TopLevelInfo { renderer: renderer.clone() });
        renderer.start();
        self.schedule_render(true);
    }

    /// Unregisters a top-level and stops its renderer.
    pub fn remove_top_level(&self, key: usize) {
        let info = self.top_levels.borrow_mut().remove(&key);
        if let Some(info) = info {
            info.renderer.stop();
        }
    }

    fn fire_invoke_on_render_callbacks(&self) {
        let mut callback_loop_count = 0;
        let pending = |this: &MediaContext| this.invoke_on_render_callbacks.borrow().as_ref().map_or(0, Vec::len);
        let mut count = pending(self);

        // This outer loop is to re-run layout in case the app causes a
        // layout to get enqueued in response to a callback. In this case
        // layout is re-run before render is allowed.
        loop {
            while count > 0 {
                callback_loop_count += 1;
                if callback_loop_count > 153 {
                    panic!("Infinite layout loop detected");
                }

                let mut callbacks =
                    self.invoke_on_render_callbacks.borrow_mut().take().expect("there are pending callbacks");

                for callback in callbacks.iter().take(count) {
                    callback();
                }

                callbacks.clear();
                self.invoke_on_render_callback_list_pool.borrow_mut().push(callbacks);

                count = pending(self);
            }

            count = pending(self);
            if count == 0 {
                break;
            }
        }
    }

    /// Queues a callback to run before the next render.
    pub fn begin_invoke_on_render(&self, callback: Rc<dyn Fn()>) {
        {
            let mut callbacks = self.invoke_on_render_callbacks.borrow_mut();
            callbacks
                .get_or_insert_with(|| self.invoke_on_render_callback_list_pool.borrow_mut().pop().unwrap_or_default())
                .push(callback);
        }

        if !self.is_rendering.get() {
            self.schedule_render(true);
        }
    }

    // --- clock --------------------------------------------------------------

    /// The global animation clock: what the application registers as the
    /// `dyn IGlobalClock` service, so that every animation without an
    /// explicit clock is pulsed by the render passes of this context.
    pub fn clock(&self) -> Rc<dyn IGlobalClock> {
        self.clock.clone()
    }

    /// The global animation clock, as its concrete type.
    pub fn media_context_clock(&self) -> &Rc<MediaContextClock> {
        &self.clock
    }

    /// Runs `action` with the frame time on the next pulse of the animation
    /// clock.
    pub fn request_animation_frame(&self, action: impl FnOnce(TimeSpan) + 'static) {
        self.clock.request_animation_frame(action);
    }

    // --- compositors --------------------------------------------------------

    fn commit_compositor(&self, compositor: &Rc<Compositor>) -> Arc<CompositionBatch> {
        // The compositor is allowed to schedule the next batch during the
        // commit if an update was requested while it was doing an update
        // (e.g. a visual invalidated from its own render), so the commit
        // request flag is cleared before committing and can be set again.
        self.requested_commits.borrow_mut().retain(|c| !same_compositor(c, compositor));
        let commit = compositor.commit();
        {
            let mut pending = self.pending_composition_batches.borrow_mut();
            pending.retain(|(c, _)| c.upgrade().is_some_and(|c| !same_compositor(&c, compositor)));
            pending.push((Rc::downgrade(compositor), commit.clone()));
        }
        let dispatcher = self.dispatcher.clone();
        let sequence_id = commit.sequence_id();
        commit.processed().on_completed(move || {
            dispatcher.post(
                move || {
                    // Looked up again on the UI thread: the context is
                    // thread-affine.
                    if let Some(context) = FerroLocator::current().get_service::<MediaContext>() {
                        context.composition_batch_finished(sequence_id);
                    }
                },
                DispatcherPriority::SEND,
            );
        });
        commit
    }

    fn composition_batch_finished(&self, sequence_id: i64) {
        // Only the last committed batch of a compositor is tracked, since a
        // new one is sometimes sent without waiting for the previous one.
        self.pending_composition_batches.borrow_mut().retain(|(_, batch)| batch.sequence_id() != sequence_id);
        if self.pending_composition_batches.borrow().is_empty() {
            self.animations_are_waiting_for_composition.set(false);

            // Schedule a new render pass if there are requested commits or
            // active animations.
            if !self.requested_commits.borrow().is_empty() || self.clock.has_subscriptions() {
                self.schedule_render(false);
            }
        }
    }

    /// Returns whether a commit is on its way, i.e. the animations have to
    /// wait for composition.
    fn commit_compositors_with_throttling(&self) -> bool {
        self.dispatcher.verify_access();
        // Check if we are still waiting for previous composition batches.
        if !self.pending_composition_batches.borrow().is_empty() {
            // The previous commit isn't handled yet.
            return true;
        }

        if self.requested_commits.borrow().is_empty() {
            // Nothing to do, and there are no pending commits.
            return false;
        }

        let requested = self.requested_commits.borrow().clone();
        for compositor in &requested {
            self.commit_compositor(compositor);
        }

        true
    }

    /// Commits a compositor and waits until the server has applied (and,
    /// with `wait_full_render`, rendered) the batch.
    pub(crate) fn sync_commit(&self, compositor: &Rc<Compositor>, wait_full_render: bool) {
        // Unit tests assume that they can call any API without setting up
        // platforms.
        if FerroLocator::current().get_service::<dyn IPlatformRenderInterface>().is_none() {
            return;
        }
        let batch = self.commit_compositor(compositor);
        self.sync_wait_compositor_batch(compositor, &batch, wait_full_render);
    }

    fn sync_wait_compositor_batch(&self, compositor: &Rc<Compositor>, _batch: &Arc<CompositionBatch>, _wait_full_render: bool) {
        // The server compositor runs on this thread (see the threading
        // notes of the composition module): rendering it here is what
        // applies the batch, in every render loop configuration.
        compositor.server().render();
    }

    /// Commits the compositor of a composition target right away and
    /// renders it.
    pub fn immediate_render_requested(&self, compositor: &Rc<Compositor>) {
        self.sync_commit(compositor, true);
    }

    /// Disposes the server side of a composition target outside of the
    /// normal commit cycle and waits for it.
    pub fn sync_dispose_composition_target(&self, compositor: &Rc<Compositor>, server_target: ServerObjectId) {
        let oob_batch = compositor.oob_dispose(server_target);
        self.sync_wait_compositor_batch(compositor, &oob_batch, false);
    }
}

impl ICompositorScheduler for MediaContext {
    fn commit_requested(&self, compositor: &Rc<Compositor>) {
        {
            let mut requested = self.requested_commits.borrow_mut();
            if requested.iter().any(|c| same_compositor(c, compositor)) {
                return;
            }
            requested.push(compositor.clone());
        }

        self.schedule_render(false);
    }
}
