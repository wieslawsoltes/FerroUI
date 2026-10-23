//! Native FerroUI prototype host. Presentation uses a disposable preview of the
//! authored document, activation-scoped Dispatcher timers, and the public
//! FerroUI animation clock. Transition scenes use normal FerroUI drawing.

use crate::canvas::{DesignCanvas, Tool};
use ferrodesign_core::{
    Document, DocumentError, Editor, NodeId, Point, PrototypeAction, PrototypeFrame, PrototypePlayer,
    PrototypeScene, PrototypeTimer, Transition, TransitionKind, Trigger, Viewport, MAX_DOCUMENT_DEPTH,
};
use ferroui_base::animation::{Clock, IClock, PlayState, TimeSpan};
use ferroui_base::reactive::{AnonymousObserver, IDisposable, IObservable};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::Ref;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq)]
pub enum PrototypeRuntimeEvent {
    Started(NodeId),
    Stopped,
    Changed,
    /// A validated request awaiting a host implementation, not a completed
    /// browser launch. Use take_pending_url after accepting/handling it.
    OpenUrl(String),
    /// A content-scroll action has been processed in the disposable preview.
    ScrollTo(NodeId),
}

struct PresentationBefore {
    frame: Option<PrototypeFrame>,
    current: Option<NodeId>,
    viewport: Viewport,
}

struct RunningTransition {
    clock: Rc<Clock>,
    subscription: Rc<dyn IDisposable>,
    from_viewport: Viewport,
    to_viewport: Viewport,
    final_viewport: Viewport,
}

struct AuthoredSnapshot {
    document: Document,
    selection: Vec<NodeId>,
    active_page: NodeId,
    viewport: Viewport,
    tool: Tool,
}

struct ScheduledTimer {
    subscription: Rc<dyn IDisposable>,
}

pub struct PrototypeRuntime {
    editor: Rc<RefCell<Editor>>,
    canvas: Ref<DesignCanvas>,
    player: RefCell<PrototypePlayer>,
    authored: RefCell<Option<AuthoredSnapshot>>,
    timers: RefCell<BTreeMap<PrototypeTimer, ScheduledTimer>>,
    animation: RefCell<Option<RunningTransition>>,
    animation_generation: Cell<u64>,
    hover_viewport: Cell<Option<Viewport>>,
    generation: Cell<u64>,
    callback: RefCell<Option<Rc<dyn Fn(PrototypeRuntimeEvent)>>>,
}

impl PrototypeRuntime {
    pub fn new(editor: Rc<RefCell<Editor>>, canvas: Ref<DesignCanvas>) -> Rc<Self> {
        Rc::new(Self {
            editor,
            canvas,
            player: RefCell::new(PrototypePlayer::default()),
            authored: RefCell::new(None),
            timers: RefCell::new(BTreeMap::new()),
            animation: RefCell::new(None),
            animation_generation: Cell::new(0),
            hover_viewport: Cell::new(None),
            generation: Cell::new(0),
            callback: RefCell::new(None),
        })
    }

    pub fn set_callback(&self, callback: impl Fn(PrototypeRuntimeEvent) + 'static) {
        *self.callback.borrow_mut() = Some(Rc::new(callback));
    }

    pub fn is_active(&self) -> bool {
        self.authored.borrow().is_some()
    }

    pub fn current(&self) -> Option<NodeId> {
        self.player.borrow().current
    }

    pub fn is_animating(&self) -> bool {
        self.animation.borrow().is_some()
    }

    pub fn animation_progress(&self) -> Option<f64> {
        self.canvas.prototype_scene.borrow().as_ref().map(|scene| scene.progress)
    }

    /// Bounded inspection for the browser validation bridge, outside the
    /// rendering loop. It exposes sampled scene geometry, not authored edits.
    pub fn animation_state(&self) -> Option<serde_json::Value> {
        self.canvas.prototype_scene.borrow().as_ref().map(|scene| {
            let layers: Vec<_> = scene.document.nodes.values().take(64).map(|node| serde_json::json!({
                "name": node.name,
                "x": node.x, "y": node.y, "width": node.width, "height": node.height,
                "opacity": node.opacity,
                "fill": node.fills.first().map(|fill| fill.paint.representative_color()),
            })).collect();
            serde_json::json!({
                "progress": scene.progress,
                "kind": scene.transition.kind,
                "animatedNodes": scene.animated_nodes(),
                "nodeCount": scene.document.nodes.len(),
                "layers": layers,
            })
        })
    }

    /// Exports and autosaves always read authored state, including during a
    /// preview that changed bound fills, text, visibility or variable overrides.
    pub fn document_json(&self) -> Result<String, DocumentError> {
        if let Some(authored) = self.authored.borrow().as_ref() {
            authored.document.to_json()
        } else {
            self.editor.borrow().to_json()
        }
    }

    /// Compact local snapshots use the same authored-state boundary as portable
    /// exports. Borrow only for serialization, then release before host storage.
    pub fn autosave_json(&self) -> Result<String, DocumentError> {
        let result = if let Some(authored) = self.authored.borrow().as_ref() {
            serde_json::to_string(&authored.document)
        } else {
            serde_json::to_string(&self.editor.borrow().document)
        };
        result.map_err(|error| DocumentError::Json(error.to_string()))
    }

    pub fn start(self: &Rc<Self>, root: NodeId) -> bool {
        self.canvas.finish_pen();
        self.canvas.cancel_active_edit();
        self.canvas.reset_presentation_input();
        self.hover_viewport.set(None);
        let document = self
            .authored
            .borrow()
            .as_ref()
            .map(|authored| authored.document.clone())
            .unwrap_or_else(|| self.editor.borrow().document.clone());
        if !self.player.borrow_mut().start_at(&document, root) {
            return false;
        }
        self.cancel_timers();
        self.cancel_animation(false);
        self.generation.set(self.generation.get().wrapping_add(1));
        {
            let mut editor = self.editor.borrow_mut();
            if self.authored.borrow().is_none() {
                // Finish an existing pointer transaction before capturing its
                // final authored state; preview actions never enter history.
                editor.commit_transaction();
                *self.authored.borrow_mut() = Some(AuthoredSnapshot {
                    document: editor.document.clone(),
                    selection: editor.selection.clone(),
                    active_page: editor.active_page,
                    viewport: self.canvas.viewport.get(),
                    tool: self.canvas.tool.get(),
                });
            }
            editor.document = document;
            for node in editor.document.nodes.values_mut() {
                if node.prototype_overflow != ferrodesign_core::PrototypeOverflow::None {
                    node.clip_content = true;
                }
            }
            if let Some(page) = editor.document.page_of(root) {
                editor.active_page = page;
            }
            editor.refresh();
        }
        self.canvas.leave_vector_edit();
        self.canvas.presentation.set(Some(root));
        self.canvas.overlays.borrow_mut().clear();
        self.canvas.reset_fit();
        self.canvas.focus();
        self.synchronize_timers();
        self.emit(PrototypeRuntimeEvent::Started(root));
        true
    }

    pub fn stop(&self) -> bool {
        let Some(authored) = self.authored.borrow_mut().take() else {
            return false;
        };
        self.generation.set(self.generation.get().wrapping_add(1));
        self.canvas.reset_presentation_input();
        self.hover_viewport.set(None);
        self.cancel_timers();
        self.cancel_animation(false);
        self.player.borrow_mut().stop();
        {
            let mut editor = self.editor.borrow_mut();
            editor.document = authored.document;
            editor.selection = authored.selection;
            editor.active_page = authored.active_page;
            editor.refresh();
        }
        self.canvas.presentation.set(None);
        self.canvas.overlays.borrow_mut().clear();
        self.canvas.viewport.set(authored.viewport);
        self.canvas.tool.set(authored.tool);
        self.canvas.invalidate_visual();
        self.emit(PrototypeRuntimeEvent::Stopped);
        true
    }

    pub fn back(self: &Rc<Self>) -> bool {
        if !self.is_active() {
            return false;
        }
        let previous = self.before_action();
        let changed = self
            .player
            .borrow_mut()
            .back(&self.editor.borrow().document);
        if changed {
            self.after_action(previous);
        }
        changed
    }

    /// Bubble pointer events through the active root's ancestry. The topmost
    /// overlay is modal; locked design layers still respond in presentation.
    pub fn dispatch(self: &Rc<Self>, node: NodeId, trigger: Trigger) -> bool {
        if !self.is_active() || matches!(trigger, Trigger::Key | Trigger::AfterDelay) {
            return false;
        }
        let releasing = trigger == Trigger::Release && self.player.borrow().has_transient(Trigger::WhilePress);
        if !releasing && !self.has_pointer_rule(node, trigger) {
            return false;
        }
        let had_hover = self.player.borrow().has_transient(Trigger::WhileHover);
        let previous = self.before_action();
        let hover_viewport = previous.viewport;
        let changed = {
            let mut editor = self.editor.borrow_mut();
            let mut player = self.player.borrow_mut();
            let restored = releasing && player.end_transient(&mut editor.document, Trigger::WhilePress);
            let root = player.interaction_root();
            let mut source = Some(node);
            let mut changed = restored;
            for _ in 0..MAX_DOCUMENT_DEPTH {
                let Some(id) = source else {
                    break;
                };
                if player.trigger(&mut editor.document, id, trigger) {
                    changed = true;
                    break;
                }
                if Some(id) == root {
                    break;
                }
                source = editor.document.nodes.get(&id).and_then(|node| node.parent);
            }
            changed
        };
        if changed {
            if !had_hover && self.player.borrow().has_transient(Trigger::WhileHover) {
                self.hover_viewport.set(Some(hover_viewport));
            } else if !self.player.borrow().has_transient(Trigger::WhileHover) {
                self.hover_viewport.set(None);
            }
            self.after_action(previous);
        }
        changed
    }

    fn has_pointer_rule(&self, node: NodeId, trigger: Trigger) -> bool {
        let editor = self.editor.borrow();
        let Some(root) = self.player.borrow().interaction_root() else { return false };
        if node != root && !editor.document.is_ancestor(root, node) { return false; }
        let mut current = Some(node);
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(id) = current else { break };
            let Some(node) = editor.document.nodes.get(&id) else { break };
            if node.interactions.iter().any(|rule| rule.trigger == trigger
                || (trigger == Trigger::Hover && rule.trigger == Trigger::WhileHover)
                || (trigger == Trigger::Press && rule.trigger == Trigger::WhilePress))
            {
                return true;
            }
            if id == root { break; }
            current = node.parent;
        }
        false
    }

    /// Screen coordinates are tested against the viewport captured on hover
    /// entry, so an animated navigation cannot move its own exit boundary.
    pub fn pointer_moved(self: &Rc<Self>, screen: Point) {
        let Some(viewport) = self.hover_viewport.get() else { return };
        if !self.player.borrow().has_transient(Trigger::WhileHover) {
            self.hover_viewport.set(None);
            return;
        }
        if !self.player.borrow().transient_contains(Trigger::WhileHover, viewport.screen_to_world(screen)) {
            self.pointer_exited();
        }
    }

    pub fn pointer_exited(self: &Rc<Self>) {
        if !self.is_active() || !self.player.borrow().has_transient(Trigger::WhileHover) { return; }
        let before = self.before_action();
        let changed = self.player.borrow_mut().end_transient(&mut self.editor.borrow_mut().document, Trigger::WhileHover);
        self.hover_viewport.set(None);
        if changed { self.after_action(before); }
    }

    /// Native wheel movement changes content offsets immediately. Hit testing
    /// is refreshed in the canonical preview and the presentation camera stays
    /// fixed; no document history or authored state is touched.
    pub fn scroll(self: &Rc<Self>, node: NodeId, delta: Point) -> bool {
        if !self.is_active() { return false; }
        self.cancel_animation(true);
        let changed = {
            let mut editor = self.editor.borrow_mut();
            let mut player = self.player.borrow_mut();
            let before = player.scroll.offsets().clone();
            let changed = player.scroll_by(&mut editor.document, node, delta);
            if changed {
                let mut dirty = BTreeSet::new();
                for (&container, &offset) in player.scroll.offsets() {
                    if before.get(&container).copied().unwrap_or_default() != offset {
                        dirty.insert(container);
                        if let Some(frame) = editor.document.nodes.get(&container) {
                            dirty.extend(frame.children.iter().copied());
                        }
                    }
                }
                editor.refresh_preview_nodes(&dirty.into_iter().collect::<Vec<_>>());
            }
            changed
        };
        if changed { self.canvas.invalidate_visual(); }
        changed
    }

    pub fn scroll_offsets(&self) -> BTreeMap<NodeId, Point> {
        self.player.borrow().scroll.offsets().clone()
    }

    /// Resolve key chords from topmost descendants toward the presentation
    /// root. Only a matching authored key rule can consume the key event.
    pub fn key(self: &Rc<Self>, key: &str) -> bool {
        if !self.is_active() {
            return false;
        }
        let previous = self.before_action();
        let changed = {
            let mut editor = self.editor.borrow_mut();
            let mut player = self.player.borrow_mut();
            let Some(root) = player.interaction_root() else {
                return false;
            };
            editor
                .document
                .descendants(root)
                .into_iter()
                .rev()
                .any(|node| player.trigger_key(&mut editor.document, node, key))
        };
        if changed {
            self.after_action(previous);
        }
        changed
    }

    pub fn take_pending_url(&self) -> Option<String> {
        self.player.borrow_mut().pending_url.take()
    }

    pub fn take_scroll_target(&self) -> Option<NodeId> {
        self.player.borrow_mut().scroll_target.take()
    }

    fn before_action(&self) -> PresentationBefore {
        let current = self.current();
        let frame = if let Some(scene) = self.canvas.prototype_scene.borrow().as_ref() {
            // A delay can interrupt a transition. Continue from the actual
            // displayed scene instead of jumping to the old action's endpoint.
            Some(PrototypeFrame {
                document: scene.document.clone(),
                root: scene.root,
                origin: Point::default(),
                presentation: None,
                overlays: Vec::new(),
            })
        } else {
            current.and_then(|root| {
                PrototypeFrame::capture(
                    &self.editor.borrow().document,
                    root,
                    &self.player.borrow().overlays,
                )
            })
        };
        PresentationBefore { frame, current, viewport: self.canvas.viewport.get() }
    }

    fn after_action(self: &Rc<Self>, previous: PresentationBefore) {
        let (current, overlays, action, mut transition, url, scroll) = {
            let player = self.player.borrow();
            (
                player.current,
                player.overlays.clone(),
                player.last_action.clone(),
                player.last_transition.clone(),
                player.pending_url.clone(),
                player.scroll_target,
            )
        };
        {
            let mut editor = self.editor.borrow_mut();
            if let Some(page) = current.and_then(|id| editor.document.page_of(id)) {
                editor.active_page = page;
            }
            editor.refresh();
        }
        if !self.player.borrow().has_transients() && matches!(
            action,
            Some(
                PrototypeAction::NavigateTo { .. }
                    | PrototypeAction::OpenOverlay { .. }
                    | PrototypeAction::CloseOverlay
                    | PrototypeAction::Back
            )
        ) {
            self.canvas.reset_presentation_input();
        }
        self.canvas.presentation.set(current);
        *self.canvas.overlays.borrow_mut() = overlays;
        let navigated = current != previous.current;
        if matches!(action, Some(PrototypeAction::ScrollTo { .. }))
            && transition.kind != TransitionKind::Instant
        {
            // Scrolling interpolates content positions; slide/dissolve would
            // move or fade the viewport itself instead of scrolling content.
            transition.kind = TransitionKind::SmartAnimate;
        }
        if matches!(action, Some(PrototypeAction::OpenUrl { .. })) {
            self.canvas.invalidate_visual();
        } else if !self.start_transition(previous, transition, navigated) {
            self.cancel_animation(true);
            if navigated {
                self.canvas.reset_fit();
            } else {
                self.canvas.invalidate_visual();
            }
        }
        self.synchronize_timers();
        self.emit(PrototypeRuntimeEvent::Changed);
        if !self.is_active() {
            return;
        }
        match action {
            Some(PrototypeAction::OpenUrl { .. }) => {
                if let Some(url) = url {
                    self.emit(PrototypeRuntimeEvent::OpenUrl(url));
                }
            }
            Some(PrototypeAction::ScrollTo { .. }) => {
                if let Some(target) = scroll {
                    self.emit(PrototypeRuntimeEvent::ScrollTo(target));
                }
            }
            _ => {}
        }
    }

    fn start_transition(
        self: &Rc<Self>,
        before: PresentationBefore,
        transition: Transition,
        fit_destination: bool,
    ) -> bool {
        if transition.kind == TransitionKind::Instant || transition.duration_ms == 0 {
            return false;
        }
        let Some(from) = before.frame else { return false };
        let Some(to) = self.current().and_then(|root| {
            PrototypeFrame::capture(
                &self.editor.borrow().document,
                root,
                &self.player.borrow().overlays,
            )
        }) else { return false };
        if from.document.nodes == to.document.nodes {
            return false;
        }
        let mut from_viewport = before.viewport;
        from_viewport.pan = from_viewport.pan + from.origin * from_viewport.zoom;
        let mut to_viewport = from_viewport;
        if fit_destination {
            to_viewport.fit(to.bounds(), 65.0);
        }
        let mut final_viewport = to_viewport;
        final_viewport.pan = final_viewport.pan - to.origin * final_viewport.zoom;
        let scene = PrototypeScene::new(from, to, transition);
        self.cancel_animation(false);
        self.canvas.viewport.set(from_viewport);
        *self.canvas.prototype_scene.borrow_mut() = Some(scene);
        let generation = self.animation_generation.get();
        let clock = Clock::new();
        let weak = Rc::downgrade(self);
        let subscription = clock.subscribe(Rc::new(AnonymousObserver::new(move |time: TimeSpan| {
            if let Some(runtime) = weak.upgrade() {
                if runtime.animation_generation.get() == generation && runtime.is_active() {
                    runtime.tick_animation(time.total_milliseconds());
                }
            }
        })));
        *self.animation.borrow_mut() = Some(RunningTransition {
            clock, subscription, from_viewport, to_viewport, final_viewport,
        });
        self.canvas.invalidate_visual();
        true
    }

    fn tick_animation(&self, elapsed_ms: f64) {
        let (finished, progress) = {
            let mut scene = self.canvas.prototype_scene.borrow_mut();
            let Some(scene) = scene.as_mut() else { return };
            let finished = scene.sample(elapsed_ms);
            (finished, scene.eased_progress())
        };
        if let Some(animation) = self.animation.borrow().as_ref() {
            let mut viewport = animation.from_viewport;
            viewport.pan = animation.from_viewport.pan.lerp(animation.to_viewport.pan, progress);
            viewport.zoom = (animation.from_viewport.zoom
                + (animation.to_viewport.zoom - animation.from_viewport.zoom) * progress)
                .clamp(0.02, 64.0);
            self.canvas.viewport.set(viewport);
        }
        if finished {
            self.cancel_animation(true);
            self.emit(PrototypeRuntimeEvent::Changed);
        }
        self.canvas.invalidate_visual();
    }

    fn cancel_animation(&self, settle: bool) {
        self.animation_generation.set(self.animation_generation.get().wrapping_add(1));
        let animation = self.animation.borrow_mut().take();
        if let Some(animation) = animation {
            animation.subscription.dispose();
            animation.clock.set_play_state(PlayState::Stop);
            if settle {
                self.canvas.viewport.set(animation.final_viewport);
            }
        }
        self.canvas.prototype_scene.borrow_mut().take();
    }

    fn synchronize_timers(self: &Rc<Self>) {
        let wanted = self
            .player
            .borrow_mut()
            .pending_timers(&self.editor.borrow().document);
        let keys: BTreeSet<_> = wanted.iter().copied().collect();
        let obsolete: Vec<_> = self
            .timers
            .borrow()
            .keys()
            .filter(|timer| !keys.contains(timer))
            .copied()
            .collect();
        for timer in obsolete {
            if let Some(timer) = self.timers.borrow_mut().remove(&timer) {
                timer.subscription.dispose();
            }
        }
        for timer in wanted {
            if !self.timers.borrow().contains_key(&timer) {
                self.schedule_timer(timer, Instant::now());
            }
        }
    }

    fn schedule_timer(self: &Rc<Self>, timer: PrototypeTimer, start: Instant) {
        let elapsed = start.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let remaining = (timer.delay_ms as u64).saturating_sub(elapsed);
        // DispatcherTimer periods are signed 32-bit milliseconds. Long delays
        // are split into cancellable chunks without firing the action early.
        // Zero delay always yields at least one UI turn, avoiding recursion.
        let interval = Duration::from_millis(remaining.clamp(1, i32::MAX as u64));
        let generation = self.generation.get();
        let weak = Rc::downgrade(self);
        let subscription = DispatcherTimer::run_once(
            move || {
                let Some(runtime) = weak.upgrade() else {
                    return;
                };
                if runtime.generation.get() != generation || !runtime.is_active() {
                    return;
                }
                runtime.timers.borrow_mut().remove(&timer);
                let elapsed = start.elapsed().as_millis().min(u64::MAX as u128) as u64;
                if elapsed < timer.delay_ms as u64 {
                    // A changed activation can invalidate a long-delay chunk.
                    let pending = runtime
                        .player
                        .borrow_mut()
                        .pending_timers(&runtime.editor.borrow().document);
                    if pending.contains(&timer) {
                        runtime.schedule_timer(timer, start);
                    }
                    return;
                }
                let previous = runtime.before_action();
                let changed = {
                    let mut editor = runtime.editor.borrow_mut();
                    runtime
                        .player
                        .borrow_mut()
                        .fire_timer(&mut editor.document, timer, elapsed)
                };
                if changed {
                    runtime.after_action(previous);
                } else {
                    runtime.synchronize_timers();
                }
            },
            interval,
            DispatcherPriority::BACKGROUND,
        );
        self.timers
            .borrow_mut()
            .insert(timer, ScheduledTimer { subscription });
    }

    fn cancel_timers(&self) {
        let timers = std::mem::take(&mut *self.timers.borrow_mut());
        for (_, timer) in timers {
            timer.subscription.dispose();
        }
    }

    fn emit(&self, event: PrototypeRuntimeEvent) {
        let callback = self.callback.borrow().clone();
        if let Some(callback) = callback {
            callback(event);
        }
    }
}

impl Drop for PrototypeRuntime {
    fn drop(&mut self) {
        self.cancel_animation(false);
        for (_, timer) in std::mem::take(self.timers.get_mut()) {
            timer.subscription.dispose();
        }
        if let Some(authored) = self.authored.get_mut().take() {
            if let Ok(mut editor) = self.editor.try_borrow_mut() {
                editor.document = authored.document;
                editor.selection = authored.selection;
                editor.active_page = authored.active_page;
                editor.refresh();
            }
        }
    }
}
