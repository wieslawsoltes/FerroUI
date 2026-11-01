use crate::{TextureFrameImporter, TexturePresenter};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{Brushes, Stretch},
    reactive::IDisposable,
    rendering::composition::{ElementComposition, ICompositionGpuInterop, RenderContextState},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    utilities::ThreadBound,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs, WeakRef,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl, TopLevel};
use ferroui_media::{
    frames::*, MediaError, MediaErrorKind, MediaEvent, MediaSubscription, RenderSchedule,
    TextureRenderer, TextureRendererHost,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

// Producer callbacks only signal this object. UI ownership is cleared explicitly
// on the UI thread before subscriptions are disposed; a producer finishing its
// callback on another thread therefore cannot leak a live UI weak reference.
struct RendererWake {
    dispatcher: Arc<Dispatcher>,
    owner: ThreadBound<RefCell<Option<WeakRef<RendererHost>>>>,
    enabled: AtomicBool,
    running: AtomicBool,
    queued: AtomicBool,
    serial: AtomicU64,
}
impl RendererWake {
    fn signal(self: &Arc<Self>) {
        self.serial.fetch_add(1, Ordering::AcqRel);
        if !self.enabled.load(Ordering::Acquire)
            || self.running.load(Ordering::Acquire)
            || self.queued.swap(true, Ordering::AcqRel)
        {
            return;
        }
        let wake = self.clone();
        self.dispatcher.post(
            move || {
                wake.queued.store(false, Ordering::Release);
                let owner = wake.owner.get().borrow().clone();
                if let Some(view) = owner.and_then(|weak| weak.upgrade()) {
                    view.start();
                }
            },
            DispatcherPriority::RENDER,
        );
    }
    fn cancel(&self) {
        self.enabled.store(false, Ordering::Release);
        self.running.store(false, Ordering::Release);
        self.owner.get().borrow_mut().take();
    }
}

/// Creates client-owned rendering resources on the active context. The new
/// source identity must be used for the renderer's output. Capture the original
/// shared budget; do not discard old-generation retirement accounting.
pub type TextureRendererFactory =
    dyn Fn(
        &dyn ICompositionGpuInterop,
        u64,
    ) -> Result<(Rc<dyn TextureRenderer>, Rc<dyn TextureFrameImporter>), MediaError>;

/// A normal composition control which schedules a client-owned renderer. Input
/// mailboxes borrow already-acquired frames; this control never opens a source.
#[repr(C)]
pub struct RendererHost {
    base: Panel,
    presenter: RefCell<Option<Ref<TexturePresenter>>>,
    host: RefCell<Option<TextureRendererHost>>,
    inputs: RefCell<Vec<Arc<FrameMailbox>>>,
    last_inputs: RefCell<Option<Vec<FrameIdentity>>>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    wake: RefCell<Option<Arc<RendererWake>>>,
    source_subscriptions: RefCell<Vec<FrameMailboxSubscription>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    scaling_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    errors: MediaEvent<MediaError>,
    presenter_errors: RefCell<Option<MediaSubscription>>,
    failed: Cell<bool>,
    generation: Cell<u64>,
    renderer_factory: RefCell<Option<Rc<TextureRendererFactory>>>,
    context_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    device_suspended: Cell<bool>,
    device_epoch: Cell<u64>,
    renderer_enabled: Cell<bool>,
}
ferro_class!(RendererHost:Panel);
ferro_class_info!(RendererHost {
    new: RendererHost::new
});
ferro_impl_classes!(RendererHost:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,PanelImpl);
impl ControlImpl for RendererHost {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        ferroui_controls::automation::peers::ControlAutomationPeer::new(this).upcast()
    }
}
ferro_properties! {impl RendererHost {
    pub fn schedule_property()->StyledProperty<RenderSchedule>{FerroProperty::register::<Self,_>("Schedule",RenderSchedule::OnInvalidation)}
}}
impl FerroObjectImpl for RendererHost {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let presenter = TexturePresenter::new();
        presenter.set_stretch(Stretch::Fill);
        let weak = this.to_ref().downgrade();
        this.presenter_errors
            .replace(Some(presenter.errors().subscribe(move |error| {
                if let Some(view) = weak.upgrade() {
                    view.failed.set(true);
                    if let Some(host) = view.host.borrow().as_ref() {
                        host.set_active(false);
                    }
                    view.update_wakeup();
                    view.errors.emit(error);
                }
            })));
        this.children().add(&presenter);
        this.presenter.replace(Some(presenter));
        this.set_clip_to_bounds(true);
        this.set_background(Some(Brushes::transparent()));
        this.set_focusable(true);
        let weak = this.to_ref().downgrade();
        this.visibility_subscription
            .replace(Some(this.is_effectively_visible_changed(move || {
                if let Some(view) = weak.upgrade() {
                    view.update_wakeup();
                    if view.is_effectively_visible() {
                        view.start();
                    } else {
                        view.tick();
                    }
                }
            })));
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::schedule_property().as_property() {
            if let Some(host) = this.host.borrow().as_ref() {
                host.set_schedule(this.schedule());
            }
            this.start();
        } else if args.property() == Visual::bounds_property().as_property() {
            this.invalidate_renderer();
        }
    }
}
impl VisualImpl for RendererHost {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.subscribe_context();
        if this.device_suspended.get() {
            this.recover_device();
        } else {
            this.start();
        }
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        if this.renderer_factory.borrow().is_some() {
            this.suspend_device();
        } else {
            this.stop();
        }
        if let Some(subscription) = this.context_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}
impl Drop for RendererHost {
    fn drop(&mut self) {
        self.stop();
        if let Some(subscription) = self.context_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
impl RendererHost {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            presenter: RefCell::new(None),
            host: RefCell::new(None),
            inputs: RefCell::new(Vec::new()),
            last_inputs: RefCell::new(None),
            timer: RefCell::new(None),
            wake: RefCell::new(None),
            source_subscriptions: RefCell::new(Vec::new()),
            visibility_subscription: RefCell::new(None),
            scaling_subscription: RefCell::new(None),
            errors: MediaEvent::default(),
            presenter_errors: RefCell::new(None),
            failed: Cell::new(false),
            generation: Cell::new(0),
            renderer_factory: RefCell::new(None),
            context_subscription: RefCell::new(None),
            device_suspended: Cell::new(false),
            device_epoch: Cell::new(0),
            renderer_enabled: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn schedule(&self) -> RenderSchedule {
        self.get_value(Self::schedule_property())
    }
    pub fn set_schedule(&self, value: RenderSchedule) {
        self.set_value(Self::schedule_property(), value);
    }
    pub fn errors(&self) -> MediaEvent<MediaError> {
        self.errors.clone()
    }
    /// The renderer's output identity must use source and the viewport's
    /// generation. The framework does not own its scenes or rendering library.
    pub fn set_renderer(
        &self,
        renderer: Option<Rc<dyn TextureRenderer>>,
        source: u64,
        importer: Option<Rc<dyn TextureFrameImporter>>,
    ) {
        self.device_epoch.set(
            self.device_epoch
                .get()
                .checked_add(1)
                .expect("renderer device epoch exhausted"),
        );
        self.device_suspended.set(false);
        self.renderer_enabled.set(renderer.is_some());
        self.stop();
        let host = renderer.map(|renderer| TextureRendererHost::new(renderer, source));
        if let Some(host) = host.as_ref() {
            host.set_schedule(self.schedule());
        }
        if let Some(view) = self.presenter.borrow().as_ref() {
            view.set_frame_source(host.as_ref().map(|h| h.output()), importer);
        }
        self.host.replace(host);
        self.failed.set(false);
        self.last_inputs.borrow_mut().take();
        if self.is_attached_to_visual_tree() {
            self.start();
        }
    }
    /// Enables recreation after context loss. Scene and pipeline recreation
    /// remain client code; source inputs and scheduling remain on this host.
    pub fn set_renderer_factory(&self, factory: Option<Rc<TextureRendererFactory>>) {
        if let Some(subscription) = self.context_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.device_epoch.set(
            self.device_epoch
                .get()
                .checked_add(1)
                .expect("renderer device epoch exhausted"),
        );
        *self.renderer_factory.borrow_mut() = factory;
        self.subscribe_context();
    }
    fn subscribe_context(&self) {
        if self.context_subscription.borrow().is_some()
            || self.renderer_factory.borrow().is_none()
            || !self.is_attached_to_visual_tree()
        {
            return;
        }
        let Some(visual) = ElementComposition::get_element_visual(&self.to_ref()) else {
            return;
        };
        let weak = self.to_ref().downgrade();
        *self.context_subscription.borrow_mut() =
            Some(visual.compositor().render_context_changed(move |state| {
                if let Some(view) = weak.upgrade() {
                    match state {
                        RenderContextState::Lost => view.suspend_device(),
                        RenderContextState::Created if view.device_suspended.get() => {
                            view.recover_device()
                        }
                        _ => {}
                    }
                }
            }));
    }
    fn suspend_device(&self) {
        if !self.renderer_enabled.get() || self.device_suspended.replace(true) {
            return;
        }
        self.device_epoch.set(
            self.device_epoch
                .get()
                .checked_add(1)
                .expect("renderer device epoch exhausted"),
        );
        self.stop();
        self.host.borrow_mut().take();
        self.failed.set(true);
        if let Some(view) = self.presenter.borrow().clone() {
            view.set_frame_source(None, None);
        }
    }
    fn recover_device(&self) {
        let Some(factory) = self.renderer_factory.borrow().clone() else {
            return;
        };
        let Some(visual) = ElementComposition::get_element_visual(&self.to_ref()) else {
            return;
        };
        let compositor = visual.compositor().clone();
        let epoch = self.device_epoch.get();
        let weak = self.to_ref().downgrade();
        self.device_suspended.set(false);
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(view) = weak
                .upgrade()
                .filter(|v| v.device_epoch.get() == epoch && v.is_attached_to_visual_tree())
            else {
                return;
            };
            let source = ferroui_media::allocate_source_id();
            let result = match interop {
                Some(interop) if !interop.is_lost() => factory(&*interop, source),
                _ => Err(MediaError::new(
                    MediaErrorKind::DeviceLost,
                    "No restored renderer GPU context",
                )),
            };
            if view.device_epoch.get() != epoch {
                return;
            }
            match result {
                Ok((renderer, importer)) => {
                    view.set_renderer(Some(renderer), source, Some(importer))
                }
                Err(error) => {
                    view.device_suspended.set(true);
                    view.errors.emit(&error);
                }
            }
        });
    }
    pub fn output(&self) -> Option<Arc<FrameMailbox>> {
        self.host.borrow().as_ref().map(|h| h.output())
    }
    pub fn set_inputs(&self, inputs: Vec<Arc<FrameMailbox>>) {
        self.stop();
        self.inputs.replace(inputs);
        if let Some(host) = self.host.borrow().as_ref() {
            host.clear_output();
        }
        self.last_inputs.borrow_mut().take();
        self.invalidate_renderer();
    }
    pub fn invalidate_renderer(&self) {
        self.failed.set(false);
        if let Some(host) = self.host.borrow().as_ref() {
            host.invalidate();
        }
        self.start();
    }
    /// Map local pointer coordinates after normal control transforms. The
    /// caller subscribes to ordinary routed input and owns hit testing.
    pub fn viewport_point(&self, point: Point) -> Option<Point> {
        let bounds = self.bounds();
        self.host
            .borrow()
            .as_ref()?
            .viewport_point(point.x, point.y, bounds.width, bounds.height)
            .map(|(x, y)| Point::new(x, y))
    }
    fn stop(&self) {
        if let Some(wake) = self.wake.borrow_mut().take() {
            wake.cancel();
        }
        self.source_subscriptions.borrow_mut().clear();
        if let Some(subscription) = self.scaling_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.generation.set(
            self.generation
                .get()
                .checked_add(1)
                .expect("renderer control generation exhausted"),
        );
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
        if let Some(host) = self.host.borrow().as_ref() {
            host.set_active(false);
        }
        self.last_inputs.borrow_mut().take();
    }
    fn start(&self) {
        if self.host.borrow().is_none() || !self.is_attached_to_visual_tree() {
            return;
        }
        if self.wake.borrow().is_none() {
            let wake = Arc::new(RendererWake {
                dispatcher: Dispatcher::ui_thread(),
                owner: ThreadBound::new(RefCell::new(Some(self.to_ref().downgrade()))),
                enabled: AtomicBool::new(true),
                running: AtomicBool::new(false),
                queued: AtomicBool::new(false),
                serial: AtomicU64::new(0),
            });
            let subscriptions = self
                .inputs
                .borrow()
                .iter()
                .map(|source| {
                    let wake = wake.clone();
                    source.subscribe(move || wake.signal())
                })
                .collect();
            self.source_subscriptions.replace(subscriptions);
            self.wake.replace(Some(wake));
            if let Some(top) = TopLevel::get_top_level(Some(self)) {
                let weak = self.to_ref().downgrade();
                self.scaling_subscription
                    .replace(Some(top.scaling_changed(move || {
                        if let Some(view) = weak.upgrade() {
                            view.invalidate_renderer();
                        }
                    })));
            }
        }
        self.update_wakeup();
        if self.failed.get() || !self.is_effectively_visible() || self.timer.borrow().is_some() {
            return;
        }
        let wake = self.wake.borrow().as_ref().unwrap().clone();
        wake.running.store(true, Ordering::Release);
        let weak = self.to_ref().downgrade();
        self.timer.replace(Some(DispatcherTimer::run(
            move || {
                let Some(view) = weak.upgrade() else {
                    return false;
                };
                let before = wake.serial.load(Ordering::Acquire);
                view.tick();
                let keep = view.wants_timer();
                if !keep {
                    view.timer.borrow_mut().take();
                    wake.running.store(false, Ordering::Release);
                    // A producer racing the final tick either observes stopped
                    // or changes this serial, so its publication cannot be lost.
                    if wake.serial.load(Ordering::Acquire) != before {
                        wake.signal();
                    }
                }
                keep
            },
            Duration::from_millis(16),
            DispatcherPriority::RENDER,
        )));
        self.tick();
    }
    fn update_wakeup(&self) {
        let enabled = self.is_attached_to_visual_tree()
            && self.is_effectively_visible()
            && !self.failed.get();
        if let Some(wake) = self.wake.borrow().as_ref() {
            wake.enabled.store(enabled, Ordering::Release);
        }
        if !enabled {
            if let Some(timer) = self.timer.borrow_mut().take() {
                timer.dispose();
            }
            if let Some(wake) = self.wake.borrow().as_ref() {
                wake.running.store(false, Ordering::Release);
            }
        }
    }
    fn wants_timer(&self) -> bool {
        !self.failed.get()
            && self.is_attached_to_visual_tree()
            && self.is_effectively_visible()
            && self
                .inputs
                .borrow()
                .iter()
                .all(|source| source.latest().is_some())
            && self.host.borrow().as_ref().is_some_and(|host| {
                self.schedule() == RenderSchedule::Continuous || host.has_pending_work()
            })
    }
    fn tick(&self) {
        if self.failed.get() {
            return;
        }
        let Some(host) = self.host.borrow().clone() else {
            return;
        };
        if !self.is_attached_to_visual_tree() || !self.is_effectively_visible() {
            host.set_active(false);
            if let Some(view) = self.presenter.borrow().as_ref() {
                view.present_latest();
            }
            return;
        }
        let bounds = self.bounds();
        let scale = TopLevel::get_top_level(Some(self))
            .map(|t| t.render_scaling())
            .unwrap_or(1.);
        if let Err(error) = host.resize(bounds.width, bounds.height, scale, 8192) {
            self.failed.set(true);
            host.set_active(false);
            self.update_wakeup();
            self.errors.emit(&error);
            return;
        }
        // set_active(true) invalidates; only call when transitioning from hidden.
        host.activate_if_needed();
        let inputs: Option<Vec<_>> = self.inputs.borrow().iter().map(|s| s.latest()).collect();
        let Some(inputs) = inputs else {
            if self.last_inputs.borrow_mut().take().is_some() {
                host.clear_output();
            }
            if let Some(view) = self.presenter.borrow().as_ref() {
                view.present_latest();
            }
            return;
        };
        let ids = inputs
            .iter()
            .map(|v| v.metadata().identity)
            .collect::<Vec<_>>();
        if self.last_inputs.borrow().as_ref() != Some(&ids) {
            host.invalidate();
            self.last_inputs.replace(Some(ids));
        }
        if let Some(pending) = host.render(inputs) {
            let weak = self.to_ref().downgrade();
            let generation = self.generation.get();
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let result = pending.await;
                if let Some(view) = weak.upgrade().filter(|v| v.generation.get() == generation) {
                    if let Err(error) = result {
                        if error.kind != MediaErrorKind::Cancelled {
                            view.failed.set(true);
                            if let Some(host) = view.host.borrow().as_ref() {
                                host.set_active(false);
                            }
                            view.update_wakeup();
                            view.errors.emit(&error);
                        }
                    }
                    if let Some(presenter) = view.presenter.borrow().as_ref() {
                        presenter.present_latest();
                    }
                }
                // Input replacement, resize or invalidation may have arrived
                // while this render was busy, including an older control epoch.
                if let Some(view) = weak.upgrade() {
                    view.start();
                }
            });
        }
        if let Some(view) = self.presenter.borrow().as_ref() {
            view.present_latest();
        }
    }
}

#[cfg(test)]
mod scheduling_tests {
    use super::*;
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Control, Window,
    };
    struct Renderer {
        calls: Cell<u64>,
        budget: FrameBudget,
    }
    fn metadata(source: u64, generation: u64, sequence: u64, w: u32, h: u32) -> FrameMetadata {
        let bytes = u64::from(w) * u64::from(h) * 4;
        FrameMetadata {
            identity: FrameIdentity {
                source,
                generation,
                sequence,
            },
            width: w,
            height: h,
            visible: FrameRegion {
                x: 0,
                y: 0,
                width: w,
                height: h,
            },
            format: FrameFormat::Rgba8,
            color: FrameColor::Srgb,
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: true,
            timestamp: Duration::ZERO,
            device: Some(DeviceIdentity::Context(1)),
            device_generation: 1,
            planes: vec![FramePlane {
                offset: 0,
                stride: u64::from(w) * 4,
                length: bytes,
            }],
            retained_bytes: bytes,
        }
    }
    impl TextureRenderer for Renderer {
        fn render(
            &self,
            viewport: ferroui_media::RendererViewport,
            _: Vec<FrameLease>,
            completion: GpuCompletion,
        ) -> ferroui_media::MediaFuture<FrameLease> {
            self.calls.set(self.calls.get() + 1);
            let frame = self
                .budget
                .lease(
                    metadata(
                        9,
                        viewport.generation,
                        self.calls.get(),
                        viewport.width,
                        viewport.height,
                    ),
                    (),
                )
                .unwrap();
            completion.retain(frame.clone()).unwrap();
            completion.complete(); // This mock records no GPU work.
            Box::pin(async move { Ok(frame) })
        }
    }
    fn settle() {
        for _ in 0..6 {
            Dispatcher::ui_thread().run_jobs(None);
            for timer in Dispatcher::timers_for_unit_tests() {
                Dispatcher::force_fire_timer_for_unit_tests(&timer);
            }
        }
        Dispatcher::ui_thread().run_jobs(None);
    }
    #[test]
    fn static_host_sleeps_and_worker_frame_changes_wake_the_owning_dispatcher() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let source = Arc::new(FrameMailbox::new(1, 1));
        let input = FrameBudget::new(Default::default());
        source
            .publish(input.lease(metadata(1, 1, 1, 2, 2), ()).unwrap())
            .unwrap();
        let renderer = Rc::new(Renderer {
            calls: Cell::new(0),
            budget: FrameBudget::new(Default::default()),
        });
        let view = RendererHost::new();
        view.set_width(4.);
        view.set_height(4.);
        view.set_renderer(Some(renderer.clone()), 9, None);
        view.set_inputs(vec![source.clone()]);
        let window = Window::new();
        window.set_content(Some(Control::boxed(view.clone())));
        window.show();
        settle();
        assert_eq!(renderer.calls.get(), 1);
        assert!(
            view.timer.borrow().is_none(),
            "static output must stop its polling timer"
        );
        let producer = source.clone();
        let budget = input.clone();
        std::thread::spawn(move || {
            producer
                .publish(budget.lease(metadata(1, 1, 2, 2, 2), ()).unwrap())
                .unwrap()
        })
        .join()
        .unwrap();
        settle();
        assert_eq!(renderer.calls.get(), 2);
        assert!(view.timer.borrow().is_none());
        source.clear();
        settle();
        assert!(view.output().unwrap().latest().is_none());
        assert!(view.timer.borrow().is_none());
        window.close();
        settle();
        source
            .publish(input.lease(metadata(1, 1, 3, 2, 2), ()).unwrap())
            .unwrap();
        settle();
        assert_eq!(
            renderer.calls.get(),
            2,
            "detached subscriptions must not restart the host"
        );
        source.clear();
    }
    #[test]
    fn ancestor_visibility_stops_continuous_work_and_reshow_uses_latest_input() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let source = Arc::new(FrameMailbox::new(1, 1));
        let input = FrameBudget::new(Default::default());
        source
            .publish(input.lease(metadata(1, 1, 1, 2, 2), ()).unwrap())
            .unwrap();
        let renderer = Rc::new(Renderer {
            calls: Cell::new(0),
            budget: FrameBudget::new(Default::default()),
        });
        let view = RendererHost::new();
        view.set_width(4.);
        view.set_height(4.);
        view.set_schedule(RenderSchedule::Continuous);
        view.set_renderer(Some(renderer.clone()), 9, None);
        view.set_inputs(vec![source.clone()]);
        let parent = Panel::new();
        parent.children().add(&view);
        let window = Window::new();
        window.set_content(Some(Control::boxed(parent.clone())));
        window.show();
        settle();
        assert!(renderer.calls.get() > 1);
        assert!(view.timer.borrow().is_some());
        parent.set_is_visible(false);
        settle();
        let paused = renderer.calls.get();
        assert!(view.timer.borrow().is_none());
        assert!(view.output().unwrap().latest().is_none());
        source
            .publish(input.lease(metadata(1, 1, 2, 2, 2), ()).unwrap())
            .unwrap();
        settle();
        assert_eq!(renderer.calls.get(), paused);
        view.set_schedule(RenderSchedule::OnInvalidation);
        parent.set_is_visible(true);
        settle();
        assert_eq!(renderer.calls.get(), paused + 1);
        assert!(view.timer.borrow().is_none());
        window.close();
        settle();
    }
    #[test]
    fn context_suspension_retires_output_without_discarding_inputs_or_retained_bytes() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let budget = FrameBudget::new(Default::default());
        let renderer = Rc::new(Renderer {
            calls: Cell::new(0),
            budget: budget.clone(),
        });
        let input = Arc::new(FrameMailbox::new(1, 1));
        let view = RendererHost::new();
        view.set_renderer(Some(renderer), 9, None);
        view.set_inputs(vec![input.clone()]);
        let output = view.output().unwrap();
        let generation = view.host.borrow().as_ref().unwrap().viewport().generation;
        output
            .publish(budget.lease(metadata(9, generation, 1, 2, 2), ()).unwrap())
            .unwrap();
        let retained = output.latest().unwrap();
        view.suspend_device();
        assert!(view.output().is_none());
        assert!(output.latest().is_none());
        assert!(!retained.is_current());
        assert!(Arc::ptr_eq(&view.inputs.borrow()[0], &input));
        assert_eq!(budget.usage().frames, 1);
        assert_eq!(budget.usage().bytes, 16);
        drop(retained);
        assert_eq!(budget.usage().frames, 0);
        view.set_renderer(None, 9, None);
        assert!(!view.renderer_enabled.get());
        assert!(!view.device_suspended.get());
    }
}
