//! Camera-only presentation through the normal composition texture presenter.
#![forbid(unsafe_code)]
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::Stretch,
    reactive::{IDisposable, Subscription},
    rendering::composition::{ElementComposition, ICompositionGpuInterop, RenderContextState},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{ControlImpl, Panel, PanelImpl, TextBlock};
use ferroui_media::{MediaEvent, MediaSubscription};
use ferroui_media_camera::*;
pub use ferroui_media_controls::{TextureFrameImporter, TexturePresenter};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
/// Recreates a camera provider/importer for the restored compositor. Capture
/// the original shared frame budget when the native producer needs it.
pub type CameraProviderFactory =
    dyn Fn(
        &dyn ICompositionGpuInterop,
    ) -> Result<(Rc<dyn CameraProvider>, Rc<dyn TextureFrameImporter>), CameraError>;

#[repr(C)]
pub struct CameraPreview {
    base: Panel,
    presenter: RefCell<Option<Ref<TexturePresenter>>>,
    placeholder: RefCell<Option<Ref<TextBlock>>>,
    owned: RefCell<Option<CameraSession>>,
    shared: RefCell<Option<WeakCameraSession>>,
    capture: RefCell<Option<CameraCapture>>,
    timer: RefCell<Option<Subscription>>,
    generation: Cell<u64>,
    errors: MediaEvent<CameraError>,
    error_subscription: RefCell<Option<MediaSubscription>>,
    frame_subscription: RefCell<Option<MediaSubscription>>,
    provider_factory: RefCell<Option<Rc<CameraProviderFactory>>>,
    context_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    requested_options: RefCell<Option<CameraStartOptions>>,
    device_suspended: Cell<bool>,
}
ferro_class!(CameraPreview:Panel);
ferro_class_info!(CameraPreview {
    new: CameraPreview::new
});
ferro_impl_classes!(CameraPreview:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {impl CameraPreview {
    pub fn stretch_property()->StyledProperty<Stretch> {FerroProperty::register::<Self,_>("Stretch",Stretch::Uniform)}
}}
impl FerroObjectImpl for CameraPreview {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let presenter = TexturePresenter::new();
        let placeholder = TextBlock::new();
        placeholder.set_text(Some("Camera stopped"));
        let weak = this.to_ref().downgrade();
        this.error_subscription
            .replace(Some(presenter.errors().subscribe(move |error| {
                if let Some(view) = weak.upgrade() {
                    let kind = if error.kind == ferroui_media::MediaErrorKind::DeviceLost {
                        view.suspend_device();
                        CameraErrorKind::DeviceLost
                    } else {
                        view.stop();
                        CameraErrorKind::NativeFailure
                    };
                    view.errors.emit(&CameraError::new(kind, error.to_string()));
                }
            })));
        let weak = this.to_ref().downgrade();
        this.frame_subscription
            .replace(Some(presenter.frame_presented().subscribe(move |_| {
                if let Some(view) = weak.upgrade() {
                    if let Some(text) = view.placeholder.borrow().as_ref() {
                        text.set_is_visible(false);
                    }
                }
            })));
        this.children().add(&presenter);
        this.children().add(&placeholder);
        this.presenter.replace(Some(presenter));
        this.placeholder.replace(Some(placeholder));
    }
    fn on_property_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, args);
        if args.property() == Self::stretch_property().as_property() {
            if let Some(presenter) = this.presenter.borrow().as_ref() {
                presenter.set_stretch(this.stretch());
            }
        }
    }
}
impl VisualImpl for CameraPreview {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.subscribe_context();
        this.start_polling();
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        if this.owned.borrow().is_some() || this.capture.borrow().is_some() {
            // An owned session (or its pending start) ends with the view.
            this.stop();
        } else if let Some(timer) = this.timer.borrow_mut().take() {
            // A caller-owned session stays borrowed: the view only stops polling it (the
            // presenter releases its own leases when it detaches) and shows it again when
            // it is attached again.
            timer.dispose();
        }
        if let Some(subscription) = this.context_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}
impl CameraPreview {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            presenter: RefCell::new(None),
            placeholder: RefCell::new(None),
            owned: RefCell::new(None),
            shared: RefCell::new(None),
            capture: RefCell::new(None),
            timer: RefCell::new(None),
            generation: Cell::new(0),
            errors: MediaEvent::default(),
            error_subscription: RefCell::new(None),
            frame_subscription: RefCell::new(None),
            provider_factory: RefCell::new(None),
            context_subscription: RefCell::new(None),
            requested_options: RefCell::new(None),
            device_suspended: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }
    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value);
    }
    pub fn errors(&self) -> MediaEvent<CameraError> {
        self.errors.clone()
    }
    pub fn session(&self) -> Option<CameraSession> {
        self.owned.borrow().clone().or_else(|| {
            self.shared
                .borrow()
                .as_ref()
                .and_then(WeakCameraSession::upgrade)
        })
    }
    /// Borrows a caller-owned session. Detach releases this view's leases only.
    pub fn set_session(
        &self,
        session: Option<&CameraSession>,
        importer: Option<Rc<dyn TextureFrameImporter>>,
    ) {
        self.stop();
        self.shared.replace(session.map(CameraSession::downgrade));
        self.attach_source(importer);
        self.start_polling();
    }
    /// Explicitly creates an owned session. Detach cancels preparation and
    /// stops this session; construction/attachment never starts a camera.
    pub fn start(
        &self,
        capture: CameraCapture,
        options: CameraStartOptions,
        importer: Rc<dyn TextureFrameImporter>,
    ) -> CameraFuture<()> {
        self.stop();
        let generation = self.generation.get();
        self.requested_options.replace(Some(options.clone()));
        let pending = capture.start(options);
        self.capture.replace(Some(capture));
        let weak = self.to_ref().downgrade();
        Box::pin(async move {
            let session = pending.await?;
            let Some(view) = weak.upgrade().filter(|v| v.generation.get() == generation) else {
                session.stop();
                return Err(CameraError::new(
                    CameraErrorKind::Cancelled,
                    "Camera preview detached",
                ));
            };
            view.owned.replace(Some(session));
            view.attach_source(Some(importer));
            view.start_polling();
            Ok(())
        })
    }
    fn attach_source(&self, importer: Option<Rc<dyn TextureFrameImporter>>) {
        if let Some(presenter) = self.presenter.borrow().as_ref() {
            presenter.set_frame_source(self.session().map(|v| v.frames()), importer);
        }
    }
    pub fn stop(&self) {
        self.generation.set(
            self.generation
                .get()
                .checked_add(1)
                .expect("camera preview generation exhausted"),
        );
        self.device_suspended.set(false);
        self.requested_options.borrow_mut().take();
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
        if let Some(capture) = self.capture.borrow_mut().take() {
            capture.cancel_pending_start();
        }
        if let Some(session) = self.owned.borrow_mut().take() {
            session.stop();
        }
        self.shared.borrow_mut().take();
        if let Some(presenter) = self.presenter.borrow().as_ref() {
            presenter.set_frame_source(None, None);
        }
        if let Some(text) = self.placeholder.borrow().as_ref() {
            text.set_is_visible(true);
        }
    }
    /// Enables automatic restart only for this control's explicitly started,
    /// active session. Caller-supplied sessions are recovered by their owner.
    pub fn set_provider_factory(&self, factory: Option<Rc<CameraProviderFactory>>) {
        if let Some(subscription) = self.context_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        *self.provider_factory.borrow_mut() = factory;
        self.subscribe_context();
    }
    fn subscribe_context(&self) {
        if self.context_subscription.borrow().is_some()
            || self.provider_factory.borrow().is_none()
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
        self.suspend_device_inner(false);
    }
    fn suspend_device_inner(&self, stopped_by_device_loss: bool) {
        if self.device_suspended.get() {
            return;
        }
        let capture = self.capture.borrow().clone();
        let options = self.requested_options.borrow().clone();
        let was_active = self
            .owned
            .borrow()
            .as_ref()
            .is_some_and(|s| stopped_by_device_loss || !s.is_stopped());
        // Pending starts are cancelled without silently retrying consent.
        self.stop();
        if was_active && self.provider_factory.borrow().is_some() {
            self.capture.replace(capture);
            self.requested_options.replace(options);
            self.device_suspended.set(true);
        }
    }
    fn recover_device(&self) {
        let Some(factory) = self.provider_factory.borrow().clone() else {
            return;
        };
        let Some(capture) = self.capture.borrow().clone() else {
            return;
        };
        let Some(options) = self.requested_options.borrow().clone() else {
            return;
        };
        let Some(visual) = ElementComposition::get_element_visual(&self.to_ref()) else {
            return;
        };
        let compositor = visual.compositor().clone();
        let generation = self.generation.get();
        let weak = self.to_ref().downgrade();
        self.device_suspended.set(false);
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(view) = weak
                .upgrade()
                .filter(|v| v.generation.get() == generation && v.is_attached_to_visual_tree())
            else {
                return;
            };
            let result = match interop {
                Some(interop) if !interop.is_lost() => factory(&*interop),
                _ => Err(CameraError::new(
                    CameraErrorKind::DeviceLost,
                    "No restored camera GPU context",
                )),
            };
            let (provider, importer) = match result {
                Ok(value) => value,
                Err(error) => {
                    view.device_suspended.set(true);
                    view.errors.emit(&error);
                    return;
                }
            };
            if view.generation.get() != generation {
                return;
            }
            capture.rebind_provider(provider);
            let pending = view.start(capture, options, importer);
            let active_generation = view.generation.get();
            drop(view);
            if let Err(error) = pending.await {
                if error.kind != CameraErrorKind::Cancelled {
                    if let Some(view) = weak
                        .upgrade()
                        .filter(|v| v.generation.get() == active_generation)
                    {
                        view.errors.emit(&error);
                    }
                }
            }
        });
    }
    fn start_polling(&self) {
        if !self.is_attached_to_visual_tree()
            || self.timer.borrow().is_some()
            || self.session().is_none()
        {
            return;
        }
        let weak = self.to_ref().downgrade();
        let timer = DispatcherTimer::run(
            move || {
                let Some(view) = weak.upgrade() else {
                    return false;
                };
                let Some(session) = view.session().filter(|v| !v.is_stopped()) else {
                    view.stop();
                    return false;
                };
                if let Err(error) = session.poll() {
                    if error.kind == CameraErrorKind::DeviceLost {
                        // CameraSession.poll has already retired the failed native session.
                        view.suspend_device_inner(true);
                    } else {
                        view.stop();
                    }
                    view.errors.emit(&error);
                    return false;
                }
                let presenter = view.presenter.borrow().clone();
                if let Some(presenter) = presenter {
                    presenter.present_latest();
                }
                true
            },
            Duration::from_millis(16),
            DispatcherPriority::NORMAL,
        );
        self.timer.replace(Some(timer));
    }
}
impl Drop for CameraPreview {
    fn drop(&mut self) {
        self.stop();
        if let Some(subscription) = self.context_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
pub fn register_types() {
    ferroui_media_controls::register_types();
    ferroui_base::TypeInfo::register_namespaces(&[(
        "ferroui_media_camera_controls",
        "FerroUI.Media.Camera.Controls",
    )]);
    ferroui_base::TypeInfo::register(CameraPreview::TYPE);
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use ferroui_controls::testing::{TestServices, UnitTestApplication};
    use ferroui_media::frames::{FrameBudget, FrameLease};
    use std::{
        pin::Pin,
        task::{Context, Poll, Waker},
    };
    struct Native(Rc<Cell<u32>>);
    impl NativeCameraSession for Native {
        fn settings(&self) -> CameraSettings {
            CameraSettings {
                device: CameraDevice {
                    id: "fixture".into(),
                    label: None,
                    facing: CameraFacing::Unknown,
                    formats: vec![],
                },
                format: CameraFormat {
                    width: 16,
                    height: 16,
                    frames_per_second: 30.,
                },
                native_still: false,
                torch: false,
                zoom: None,
            }
        }
        fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
            Ok(None)
        }
        fn photo(&self) -> CameraFuture<CameraPhoto> {
            Box::pin(async { Err(CameraError::new(CameraErrorKind::Unsupported, "No photo")) })
        }
        fn set_control(&self, _: CameraControl) -> Result<(), CameraError> {
            Ok(())
        }
        fn stop(&self) {
            self.0.set(self.0.get() + 1);
        }
    }
    struct Provider(Rc<Cell<u32>>);
    impl CameraProvider for Provider {
        fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn permission(&self) -> CameraPermission {
            CameraPermission::Granted
        }
        fn start(
            &self,
            _: CameraStartOptions,
            _: u64,
            _: u64,
            _: FrameBudget,
        ) -> CameraFuture<Rc<dyn NativeCameraSession>> {
            let native: Rc<dyn NativeCameraSession> = Rc::new(Native(self.0.clone()));
            Box::pin(async { Ok(native) })
        }
    }
    struct Importer;
    impl TextureFrameImporter for Importer {
        fn import(
            &self,
            _: &dyn ICompositionGpuInterop,
            _: &FrameLease,
        ) -> Result<ferroui_media_controls::ImportedTextureFrame, ferroui_media::MediaError>
        {
            unreachable!("no frame fixture")
        }
    }
    fn ready<T>(mut future: Pin<Box<dyn std::future::Future<Output = T>>>) -> T {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("expected ready"),
        }
    }
    #[test]
    fn owned_camera_suspension_preserves_restart_request_until_explicit_stop() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let stops = Rc::new(Cell::new(0));
        let capture = CameraCapture::new(
            Rc::new(Provider(stops.clone())),
            FrameBudget::new(Default::default()),
        );
        let view = CameraPreview::new();
        view.set_provider_factory(Some(Rc::new(|_| {
            Err(CameraError::new(
                CameraErrorKind::Unsupported,
                "No GPU fixture",
            ))
        })));
        ready(view.start(capture, CameraStartOptions::default(), Rc::new(Importer))).unwrap();
        let old = view.session().unwrap();
        view.suspend_device();
        assert!(old.is_stopped());
        assert_eq!(stops.get(), 1);
        assert!(view.device_suspended.get());
        assert!(view.requested_options.borrow().is_some());
        assert!(view.session().is_none());
        view.stop();
        assert!(!view.device_suspended.get());
        assert!(view.requested_options.borrow().is_none());
        assert!(view.capture.borrow().is_none());
    }
    #[test]
    fn borrowed_camera_is_released_without_stopping_callers_session() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let stops = Rc::new(Cell::new(0));
        let capture = CameraCapture::new(
            Rc::new(Provider(stops.clone())),
            FrameBudget::new(Default::default()),
        );
        let session = ready(capture.start(Default::default())).unwrap();
        let view = CameraPreview::new();
        view.set_session(Some(&session), Some(Rc::new(Importer)));
        view.suspend_device();
        assert!(!session.is_stopped());
        assert_eq!(stops.get(), 0);
        assert!(view.session().is_none());
        session.stop();
        assert_eq!(stops.get(), 1);
    }
    #[test]
    fn a_borrowed_session_is_shown_again_after_the_view_is_reattached() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let stops = Rc::new(Cell::new(0));
        let capture = CameraCapture::new(
            Rc::new(Provider(stops.clone())),
            FrameBudget::new(Default::default()),
        );
        let session = ready(capture.start(Default::default())).unwrap();
        let view = CameraPreview::new();
        view.set_session(Some(&session), Some(Rc::new(Importer)));
        let window = ferroui_controls::Window::new();
        window.set_content(Some(ferroui_controls::Control::boxed(view.clone())));
        window.show();
        window.set_content(None);
        // Detached: the caller's session is neither stopped nor forgotten.
        assert!(!session.is_stopped());
        assert!(view.session().is_some());
        assert!(view.timer.borrow().is_none());
        window.set_content(Some(ferroui_controls::Control::boxed(view.clone())));
        assert!(view.session().is_some());
        assert!(view.timer.borrow().is_some(), "polling resumes when attached again");
        assert_eq!(stops.get(), 0);
        session.stop();
    }
}
