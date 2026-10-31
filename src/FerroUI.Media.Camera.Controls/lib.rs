//! Camera-only presentation through the normal composition texture presenter.
#![forbid(unsafe_code)]
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::Stretch,
    reactive::IDisposable,
    threading::{DispatcherPriority, DispatcherTimer},
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
#[repr(C)]
pub struct CameraPreview {
    base: Panel,
    presenter: RefCell<Option<Ref<TexturePresenter>>>,
    placeholder: RefCell<Option<Ref<TextBlock>>>,
    owned: RefCell<Option<CameraSession>>,
    shared: RefCell<Option<WeakCameraSession>>,
    capture: RefCell<Option<CameraCapture>>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
    generation: Cell<u64>,
    errors: MediaEvent<CameraError>,
    error_subscription: RefCell<Option<MediaSubscription>>,
    frame_subscription: RefCell<Option<MediaSubscription>>,
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
                    view.stop();
                    view.errors.emit(&CameraError::new(
                        CameraErrorKind::NativeFailure,
                        error.to_string(),
                    ));
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
        this.start_polling();
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.stop();
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
        self.generation.set(self.generation.get().wrapping_add(1));
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
                    view.stop();
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
