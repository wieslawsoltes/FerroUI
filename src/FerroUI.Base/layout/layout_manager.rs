use super::layout_queue::LayoutQueue;
use super::{
    BringIntoViewRequest, EffectiveViewportChangedEventArgs, IBringIntoViewLayoutManager, ILayoutManager, ILayoutRoot,
    Layoutable,
};
use crate::animation::TimeSpan;
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::MediaContext;
use crate::rendering::LayoutPassTiming;
use crate::threading::Dispatcher;
use crate::utilities::HandlerList;
use crate::{Matrix, Rect, Ref, Size, Visual};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

const MAX_PASSES: i32 = 10;

struct EffectiveViewportChangedListener {
    listener: Ref<Layoutable>,
    viewport: Cell<Option<Rect>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ArrangeResult {
    Arranged,
    NotVisible,
    AncestorMeasureInvalid,
}

/// Manages measuring and arranging of controls.
pub struct LayoutManager {
    this: Weak<LayoutManager>,
    owner: Weak<dyn ILayoutRoot>,
    to_measure: RefCell<LayoutQueue<Ref<Layoutable>>>,
    to_arrange: RefCell<LayoutQueue<Ref<Layoutable>>>,
    to_arrange_after_measure: RefCell<Vec<Ref<Layoutable>>>,
    effective_viewport_changed_listeners: RefCell<Vec<Rc<EffectiveViewportChangedListener>>>,
    bring_into_view_requests: RefCell<Option<Vec<Rc<dyn BringIntoViewRequest>>>>,
    disposed: Cell<bool>,
    queued: Cell<bool>,
    running: Cell<bool>,
    processing_bring_into_view_requests: Cell<bool>,
    total_pass_count: Cell<i32>,
    layout_updated: HandlerList<dyn Fn()>,
    layout_pass_timed: RefCell<Option<Rc<dyn Fn(LayoutPassTiming)>>>,
}

impl LayoutManager {
    /// Creates a layout manager for a layout root.
    pub fn new(owner: Weak<dyn ILayoutRoot>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            owner,
            to_measure: RefCell::new(LayoutQueue::new(|v| !v.is_measure_valid())),
            to_arrange: RefCell::new(LayoutQueue::new(|v| !v.is_arrange_valid())),
            to_arrange_after_measure: RefCell::new(Vec::new()),
            effective_viewport_changed_listeners: RefCell::new(Vec::new()),
            bring_into_view_requests: RefCell::new(None),
            disposed: Cell::new(false),
            queued: Cell::new(false),
            running: Cell::new(false),
            processing_bring_into_view_requests: Cell::new(false),
            total_pass_count: Cell::new(0),
            layout_updated: HandlerList::new(),
            layout_pass_timed: RefCell::new(None),
        })
    }

    /// Whether a layout pass is currently executing.
    pub fn is_in_layout_pass(&self) -> bool {
        self.running.get()
    }

    /// The number of layout passes executed so far.
    pub fn total_pass_count(&self) -> i32 {
        self.total_pass_count.get()
    }

    /// The callback that receives the timing of each layout pass (C#
    /// `LayoutPassTimed`, internal), or `None`.
    pub fn layout_pass_timed(&self) -> Option<Rc<dyn Fn(LayoutPassTiming)>> {
        self.layout_pass_timed.borrow().clone()
    }

    /// Sets the callback that receives the timing of each layout pass.
    pub fn set_layout_pass_timed(&self, value: Option<Rc<dyn Fn(LayoutPassTiming)>>) {
        *self.layout_pass_timed.borrow_mut() = value;
    }

    /// The number of controls queued for measure. For tests.
    #[cfg(any(test, feature = "testing"))]
    pub fn to_measure_count(&self) -> usize {
        self.to_measure.borrow().count()
    }

    /// The number of controls queued for arrange. For tests.
    #[cfg(any(test, feature = "testing"))]
    pub fn to_arrange_count(&self) -> usize {
        self.to_arrange.borrow().count()
    }

    fn is_owned_root(&self, control: &Layoutable) -> bool {
        match (control.get_layout_root(), self.owner.upgrade()) {
            (Some(root), Some(owner)) => std::ptr::addr_eq(Rc::as_ptr(&root), Rc::as_ptr(&owner)),
            _ => false,
        }
    }

    fn is_root_visual(control: &Layoutable) -> bool {
        control.get_layout_root().is_some_and(|root| {
            let root_visual = root.root_visual();
            std::ptr::eq(&*root_visual as *const Layoutable, control as *const Layoutable)
        })
    }

    /// C# `Stopwatch.GetTimestamp()`: the clock of the dispatcher of the
    /// thread, in milliseconds.
    fn get_timestamp() -> i64 {
        Dispatcher::current_dispatcher().now()
    }

    fn execute_queued_layout_pass(&self) {
        if !self.queued.get() {
            return;
        }
        self.execute_layout_pass();
    }

    /// Attempts to execute each pending bring-into-view request once.
    ///
    /// Returns true if at least one request was executed; false if there was
    /// nothing to do or no request could make progress.
    fn process_bring_into_view_requests(&self) -> bool {
        if self.processing_bring_into_view_requests.get()
            || !self.bring_into_view_requests.borrow().as_ref().is_some_and(|requests| !requests.is_empty())
        {
            return false;
        }

        self.processing_bring_into_view_requests.set(true);
        let _guard = RunningGuard(&self.processing_bring_into_view_requests);

        let is_at = |requests: &Option<Vec<Rc<dyn BringIntoViewRequest>>>, i: usize, request: &Rc<dyn BringIntoViewRequest>| {
            requests
                .as_ref()
                .and_then(|requests| requests.get(i))
                .is_some_and(|current| std::ptr::addr_eq(Rc::as_ptr(current), Rc::as_ptr(request)))
        };

        let mut executed_any = false;
        let mut i = 0;

        loop {
            let request = match self.bring_into_view_requests.borrow().as_ref().and_then(|requests| requests.get(i)) {
                Some(request) => request.clone(),
                None => break,
            };

            // The target has been detached, abort.
            if !self.is_owned_root(&request.target()) {
                let mut requests = self.bring_into_view_requests.borrow_mut();
                if is_at(&requests, i, &request) {
                    if let Some(requests) = requests.as_mut() {
                        requests.remove(i);
                    }
                }
                continue;
            }

            let executed = request.try_execute();
            executed_any |= executed;

            // Executing the request may have replaced it with a new one for
            // the same target.
            let mut requests = self.bring_into_view_requests.borrow_mut();
            if executed && is_at(&requests, i, &request) {
                if let Some(requests) = requests.as_mut() {
                    requests.remove(i);
                }
            } else {
                i += 1;
            }
        }

        executed_any
    }

    fn inner_layout_pass(&self) {
        for _ in 0..MAX_PASSES {
            self.execute_measure_pass();
            self.execute_arrange_pass();

            if self.to_measure.borrow().count() == 0 {
                break;
            }
        }
    }

    fn execute_measure_pass(&self) {
        while self.to_measure.borrow().count() > 0 {
            let control = self.to_measure.borrow_mut().dequeue();

            if !control.is_measure_valid() {
                Self::measure(&control);
            }
            self.to_arrange.borrow_mut().enqueue(control);
        }
    }

    fn execute_arrange_pass(&self) {
        while self.to_arrange.borrow().count() > 0 {
            let control = self.to_arrange.borrow_mut().dequeue();

            if !control.is_arrange_valid() && Self::arrange(&control) == ArrangeResult::AncestorMeasureInvalid {
                self.to_arrange_after_measure.borrow_mut().push(control);
            }
        }

        let pending = std::mem::take(&mut *self.to_arrange_after_measure.borrow_mut());
        for control in pending {
            self.invalidate_arrange(&control);
        }
    }

    fn measure(control: &Layoutable) -> bool {
        if !control.is_visible() || !control.is_attached_to_visual_tree() {
            return false;
        }

        // Controls closest to the visual root need to be arranged first. We
        // don't try to store ordered invalidation lists, instead we traverse
        // the tree upwards, measuring the controls closest to the root first.
        // This has been shown by benchmarks to be the fastest and most
        // memory-efficient algorithm.
        if let Some(parent) = control.visual_parent() {
            if let Some(parent) = parent.downcast_ref::<Layoutable>() {
                if !Self::measure(parent) {
                    return false;
                }
            }
        }

        // If the control being measured has a valid measure here then its
        // measure was handled by an ancestor and can be ignored. The measure
        // may have also caused the control to be removed.
        if !control.is_measure_valid() {
            if Self::is_root_visual(control) {
                control.measure(Size::INFINITY);
            } else if let Some(previous) = control.previous_measure() {
                control.measure(previous);
            }
        }

        true
    }

    fn arrange(control: &Layoutable) -> ArrangeResult {
        if !control.is_visible() || !control.is_attached_to_visual_tree() {
            return ArrangeResult::NotVisible;
        }

        if let Some(parent) = control.visual_parent() {
            if let Some(parent) = parent.downcast_ref::<Layoutable>() {
                let parent_result = Self::arrange(parent);
                if parent_result != ArrangeResult::Arranged {
                    return parent_result;
                }
            }
        }

        if !control.is_measure_valid() {
            return ArrangeResult::AncestorMeasureInvalid;
        }

        if !control.is_arrange_valid() {
            if Self::is_root_visual(control) {
                control.arrange(Rect::from_size(control.desired_size()));
            } else if let Some(previous) = control.previous_arrange() {
                control.arrange(previous);
            }
        }

        ArrangeResult::Arranged
    }

    fn queue_layout_pass(&self) {
        if !self.queued.get() && !self.running.get() {
            self.queued.set(true);
            let this = self.this.clone();
            MediaContext::instance().begin_invoke_on_render(Rc::new(move || {
                if let Some(this) = this.upgrade() {
                    this.execute_queued_layout_pass();
                }
            }));
        }
    }

    fn raise_effective_viewport_changed(&self) -> bool {
        let start_count = self.to_measure.borrow().count() + self.to_arrange.borrow().count();

        let listeners: Vec<_> = self.effective_viewport_changed_listeners.borrow().clone();
        for l in listeners {
            if !l.listener.is_attached_to_visual_tree() {
                continue;
            }
            let viewport = Self::calculate_effective_viewport(&l.listener);
            if Some(viewport) != l.viewport.get() {
                l.listener.raise_effective_viewport_changed(&EffectiveViewportChangedEventArgs::new(viewport));
                l.viewport.set(Some(viewport));
            }
        }

        start_count != self.to_measure.borrow().count() + self.to_arrange.borrow().count()
    }

    fn calculate_effective_viewport(control: &Visual) -> Rect {
        let mut viewport = Rect::new(0.0, 0.0, f64::INFINITY, f64::INFINITY);
        Self::calculate_effective_viewport_core(control, control, &mut viewport);
        viewport
    }

    fn calculate_effective_viewport_core(target: &Visual, control: &Visual, viewport: &mut Rect) {
        // Recurse until the top level control.
        match control.visual_parent() {
            Some(parent) => Self::calculate_effective_viewport_core(target, &parent, viewport),
            None => *viewport = Rect::from_size(control.bounds().size()),
        }

        let is_target = std::ptr::eq(control, target);

        // Apply the control clip bounds if it's not the target control. We
        // don't apply it to the target control because it may itself be
        // clipped to bounds and if so the viewport we calculate would be of no
        // use.
        if !is_target && control.clip_to_bounds() {
            *viewport = control.bounds().intersect(*viewport);
        }

        // Translate the viewport into this control's coordinate space.
        *viewport = viewport.translate((-control.bounds().position()).into());

        if !is_target {
            if let Some(transform) = control.render_transform() {
                match transform.value().try_invert() {
                    Some(inverted_matrix) => {
                        let origin = control.render_transform_origin().to_pixels(control.bounds().size());
                        let offset = Matrix::create_translation(origin.x, origin.y);
                        *viewport = viewport.transform_to_aabb(-offset * inverted_matrix * offset);
                    }
                    None => *viewport = Rect::default(),
                }
            }
        }
    }
}

impl ILayoutManager for LayoutManager {
    fn add_layout_updated(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.layout_updated.add(handler)
    }

    fn remove_layout_updated(&self, token: u64) {
        self.layout_updated.remove(token);
    }

    fn invalidate_measure(&self, control: &Layoutable) {
        Dispatcher::ui_thread().verify_access();

        if self.disposed.get() {
            return;
        }
        if !control.is_attached_to_visual_tree() {
            debug_assert!(
                false,
                "LayoutManager.InvalidateMeasure called on a control that is detached from the visual tree."
            );
            return;
        }
        assert!(self.is_owned_root(control), "Attempt to call InvalidateMeasure on wrong LayoutManager.");

        self.to_measure.borrow_mut().enqueue(control.to_ref());
        self.queue_layout_pass();
    }

    fn invalidate_arrange(&self, control: &Layoutable) {
        Dispatcher::ui_thread().verify_access();

        if self.disposed.get() {
            return;
        }
        if !control.is_attached_to_visual_tree() {
            debug_assert!(
                false,
                "LayoutManager.InvalidateArrange called on a control that is detached from the visual tree."
            );
            return;
        }
        assert!(self.is_owned_root(control), "Attempt to call InvalidateArrange on wrong LayoutManager.");

        self.to_arrange.borrow_mut().enqueue(control.to_ref());
        self.queue_layout_pass();
    }

    fn execute_layout_pass(&self) {
        Dispatcher::ui_thread().verify_access();

        if self.disposed.get() {
            return;
        }

        if !self.running.get() {
            const TIMING_LOG_LEVEL: LogEventLevel = LogEventLevel::Information;
            let capture_timing =
                self.layout_pass_timed.borrow().is_some() || Logger::is_enabled(TIMING_LOG_LEVEL, LogArea::LAYOUT);
            let mut starting_timestamp = 0;

            if capture_timing {
                if let Some(logger) = Logger::try_get(TIMING_LOG_LEVEL, LogArea::LAYOUT) {
                    logger.log_with_values(
                        Some(self as &dyn Any),
                        "Started layout pass. To measure: {Measure} To arrange: {Arrange}",
                        &[&self.to_measure.borrow().count(), &self.to_arrange.borrow().count()],
                    );
                }

                starting_timestamp = Self::get_timestamp();
            }

            self.to_measure.borrow_mut().begin_loop(MAX_PASSES);
            self.to_arrange.borrow_mut().begin_loop(MAX_PASSES);

            self.running.set(true);
            {
                let _guard = RunningGuard(&self.running);
                self.total_pass_count.set(self.total_pass_count.get() + 1);

                for _ in 0..MAX_PASSES {
                    self.inner_layout_pass();
                    if self.raise_effective_viewport_changed() {
                        continue;
                    }

                    // The layout is now stable: bring-into-view requests can
                    // execute against final bounds. Executing them typically
                    // changes scroll offsets, which invalidates layout again.
                    if !self.process_bring_into_view_requests()
                        || (self.to_measure.borrow().count() == 0 && self.to_arrange.borrow().count() == 0)
                    {
                        break;
                    }
                }
            }

            self.to_measure.borrow_mut().end_loop();
            self.to_arrange.borrow_mut().end_loop();

            if capture_timing {
                let elapsed =
                    Duration::from_millis(Self::get_timestamp().saturating_sub(starting_timestamp).max(0) as u64);
                let layout_pass_timed = self.layout_pass_timed.borrow().clone();
                if let Some(layout_pass_timed) = layout_pass_timed {
                    layout_pass_timed(LayoutPassTiming::new(self.total_pass_count.get(), elapsed));
                }

                if let Some(logger) = Logger::try_get(TIMING_LOG_LEVEL, LogArea::LAYOUT) {
                    logger.log_with_values(
                        Some(self as &dyn Any),
                        "Layout pass finished in {Time}",
                        &[&TimeSpan::from(elapsed)],
                    );
                }
            }
        } else if self.processing_bring_into_view_requests.get() {
            // A layout pass forced while executing a bring-into-view request
            // is part of the enclosing pass: run inner passes inline, and let
            // the enclosing pass raise the layout updated event once all
            // requests have been processed.
            for _ in 0..MAX_PASSES {
                self.inner_layout_pass();

                if !self.raise_effective_viewport_changed() {
                    break;
                }
            }

            return;
        }

        self.queued.set(false);
        if !self.layout_updated.is_empty() {
            for (_, handler) in self.layout_updated.snapshot().iter() {
                handler();
            }
        }
    }

    fn execute_initial_layout_pass(&self) {
        if self.disposed.get() {
            return;
        }

        let Some(owner) = self.owner.upgrade() else { return };
        let root = owner.root_visual();

        self.running.set(true);
        {
            let _guard = RunningGuard(&self.running);
            Self::measure(&root);
            Self::arrange(&root);
        }

        // Running the initial layout pass may have caused some control to be
        // invalidated so run a full layout pass now (this is usually due to
        // scrollbars: it's not known whether they will need to be shown until
        // the layout pass has run and if the first guess was incorrect the
        // layout will need to be updated).
        self.execute_layout_pass();
    }

    fn register_effective_viewport_listener(&self, control: &Layoutable) {
        self.effective_viewport_changed_listeners
            .borrow_mut()
            .push(Rc::new(EffectiveViewportChangedListener { listener: control.to_ref(), viewport: Cell::new(None) }));
    }

    fn unregister_effective_viewport_listener(&self, control: &Layoutable) {
        self.effective_viewport_changed_listeners
            .borrow_mut()
            .retain(|l| !std::ptr::eq(&*l.listener as *const Layoutable, control as *const Layoutable));
    }

    fn dispose(&self) {
        self.disposed.set(true);
        self.to_measure.borrow_mut().dispose();
        self.to_arrange.borrow_mut().dispose();
        *self.bring_into_view_requests.borrow_mut() = None;
    }

    fn as_bring_into_view_layout_manager(&self) -> Option<&dyn IBringIntoViewLayoutManager> {
        Some(self)
    }
}

impl IBringIntoViewLayoutManager for LayoutManager {
    fn is_in_layout_pass(&self) -> bool {
        LayoutManager::is_in_layout_pass(self)
    }

    fn enqueue_bring_into_view(&self, request: Rc<dyn BringIntoViewRequest>) {
        Dispatcher::ui_thread().verify_access();

        if self.disposed.get() {
            return;
        }

        // The targets are read outside of the borrow of the queue.
        let target = request.target();
        let pending: Vec<_> = self.bring_into_view_requests.borrow().clone().unwrap_or_default();
        let existing = pending.iter().position(|existing| existing.target() == target);

        {
            let mut requests = self.bring_into_view_requests.borrow_mut();
            let requests = requests.get_or_insert_with(Vec::new);

            match existing.and_then(|i| requests.get_mut(i)) {
                Some(existing) => *existing = request,
                None => requests.push(request),
            }
        }

        // The request will usually be consumed by the already pending layout
        // pass that will lay out its target, but make sure a pass is scheduled
        // in case there is none.
        self.queue_layout_pass();
    }
}

struct RunningGuard<'a>(&'a Cell<bool>);

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
