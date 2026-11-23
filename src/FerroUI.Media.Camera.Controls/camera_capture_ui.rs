//! Explicit capture UI over CameraPreview; it never acquires a camera on load.
//! Encoded results stay owned in memory, independently of preview GPU storage.
use crate::{CameraPreview, CameraProviderFactory};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    rendering::composition::{frames::FrameBudget, ElementComposition},
    threading::Dispatcher,
    FerroObjectImpl, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    Button, ControlImpl, TextBlock,
};
use ferroui_media::{MediaEvent, MediaSubscription};
use ferroui_media_camera::{
    CameraCapture, CameraError, CameraErrorKind, CameraPhoto, CameraRecording, CameraStartOptions,
    CameraVideo,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Debug)]
pub enum CameraCaptureResult {
    Photo(CameraPhoto),
    Video(CameraVideo),
}

#[repr(C)]
pub struct CameraCaptureUI {
    base: TemplatedControl,
    preview: RefCell<Option<Ref<CameraPreview>>>,
    start: RefCell<Option<Ref<Button>>>,
    photo: RefCell<Option<Ref<Button>>>,
    record: RefCell<Option<Ref<Button>>>,
    finish: RefCell<Option<Ref<Button>>>,
    cancel: RefCell<Option<Ref<Button>>>,
    status: RefCell<Option<Ref<TextBlock>>>,
    factory: RefCell<Option<Rc<CameraProviderFactory>>>,
    budget: RefCell<FrameBudget>,
    options: RefCell<CameraStartOptions>,
    generation: Cell<u64>,
    template_generation: Cell<u64>,
    busy: Cell<bool>,
    recording: RefCell<Option<CameraRecording>>,
    completed: MediaEvent<CameraCaptureResult>,
    errors: MediaEvent<CameraError>,
    preview_errors: RefCell<Option<MediaSubscription>>,
}
ferro_class!(CameraCaptureUI:TemplatedControl);
ferro_class_info!(CameraCaptureUI {
    new: CameraCaptureUI::new
});
ferro_impl_classes!(CameraCaptureUI:StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
impl FerroObjectImpl for CameraCaptureUI {}
impl VisualImpl for CameraCaptureUI {
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.cancel();
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}
impl TemplatedControlImpl for CameraCaptureUI {
    fn on_template_changed(this: &Self, args: &FerroPropertyChangedEventArgs<'_>) {
        this.cancel();
        this.template_generation
            .set(this.template_generation.get().wrapping_add(1));
        this.preview_errors.borrow_mut().take();
        this.preview.borrow_mut().take();
        for part in [
            &this.start,
            &this.photo,
            &this.record,
            &this.finish,
            &this.cancel,
        ] {
            part.borrow_mut().take();
        }
        this.status.borrow_mut().take();
        Self::parent_on_template_changed(this, args);
    }
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, args);
        this.cancel();
        let generation = this.template_generation.get().wrapping_add(1);
        this.template_generation.set(generation);
        let scope = args.name_scope();
        *this.preview.borrow_mut() = scope.find_as::<CameraPreview>("CapturePreview");
        this.preview_errors.borrow_mut().take();
        if let Some(preview) = this.preview.borrow().clone() {
            let weak = this.to_ref().downgrade();
            *this.preview_errors.borrow_mut() = Some(preview.errors().subscribe(move |error| {
                if let Some(view) = weak.upgrade() {
                    view.cancel();
                    view.report(error);
                }
            }));
        }
        for (slot, name) in [
            (&this.start, "CaptureStart"),
            (&this.photo, "CapturePhoto"),
            (&this.record, "CaptureRecord"),
            (&this.finish, "CaptureFinish"),
            (&this.cancel, "CaptureCancel"),
        ] {
            let part = scope.find_as::<Button>(name);
            *slot.borrow_mut() = part.clone();
            if let Some(part) = part {
                let weak = this.to_ref().downgrade();
                part.click(move |_, _| {
                    if let Some(view) = weak
                        .upgrade()
                        .filter(|v| v.template_generation.get() == generation)
                    {
                        match name {
                            "CaptureStart" => view.start_preview(),
                            "CapturePhoto" => view.take_photo(),
                            "CaptureRecord" => view.start_recording(),
                            "CaptureFinish" => view.finish_recording(),
                            _ => view.cancel(),
                        }
                    }
                });
            }
        }
        *this.status.borrow_mut() = scope.find_as::<TextBlock>("CaptureStatus");
        this.message(if this.factory.borrow().is_some() {
            "Camera stopped"
        } else {
            "Camera is unavailable in this host"
        });
        this.refresh();
    }
}
impl CameraCaptureUI {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            preview: RefCell::new(None),
            start: RefCell::new(None),
            photo: RefCell::new(None),
            record: RefCell::new(None),
            finish: RefCell::new(None),
            cancel: RefCell::new(None),
            status: RefCell::new(None),
            factory: RefCell::new(None),
            budget: RefCell::new(FrameBudget::new(Default::default())),
            options: RefCell::new(Default::default()),
            generation: Cell::new(0),
            template_generation: Cell::new(0),
            busy: Cell::new(false),
            recording: RefCell::new(None),
            completed: MediaEvent::default(),
            errors: MediaEvent::default(),
            preview_errors: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Configures resources only. Starting still requires an explicit user action.
    pub fn set_provider_factory(
        &self,
        factory: Option<Rc<CameraProviderFactory>>,
        budget: FrameBudget,
    ) {
        self.cancel();
        *self.factory.borrow_mut() = factory;
        *self.budget.borrow_mut() = budget;
        self.refresh();
    }
    pub fn set_start_options(&self, options: CameraStartOptions) {
        *self.options.borrow_mut() = options;
    }
    pub fn completed(&self) -> MediaEvent<CameraCaptureResult> {
        self.completed.clone()
    }
    pub fn errors(&self) -> MediaEvent<CameraError> {
        self.errors.clone()
    }
    pub fn preview(&self) -> Option<Ref<CameraPreview>> {
        self.preview.borrow().clone()
    }
    pub fn is_busy(&self) -> bool {
        self.busy.get()
    }
    fn message(&self, text: &str) {
        if let Some(status) = self.status.borrow().clone() {
            status.set_text(Some(text));
        }
    }
    fn report(&self, error: &CameraError) {
        self.message(&error.message);
        self.errors.emit(error);
    }
    fn refresh(&self) {
        let session = self
            .preview()
            .and_then(|p| p.session())
            .filter(|s| !s.is_stopped());
        let idle = !self.busy.get();
        let recording = self.recording.borrow().is_some();
        for (button, enabled) in [
            (
                &self.start,
                idle && session.is_none() && self.factory.borrow().is_some(),
            ),
            (&self.photo, idle && session.is_some() && !recording),
            (
                &self.record,
                idle && !recording
                    && session
                        .as_ref()
                        .is_some_and(|s| s.supports_video_recording()),
            ),
            (&self.finish, idle && recording),
            (&self.cancel, session.is_some() || !idle),
        ] {
            if let Some(button) = button.borrow().clone() {
                button.set_is_enabled(enabled);
            }
        }
        if let Some(button) = self.record.borrow().clone() {
            ferroui_controls::ToolTip::set_tip(
                &button,
                Some(Rc::new(
                    if session
                        .as_ref()
                        .is_some_and(|s| s.supports_video_recording())
                    {
                        "Record silent video"
                    } else {
                        "This camera provider does not support video recording"
                    }
                    .to_owned(),
                )),
            );
        }
    }
    pub fn start_preview(&self) {
        if self.busy.replace(true) {
            return;
        }
        let Some(preview) = self.preview() else {
            self.busy.set(false);
            return;
        };
        if preview.session().is_some() {
            self.busy.set(false);
            return;
        }
        let Some(factory) = self.factory.borrow().clone() else {
            self.busy.set(false);
            return;
        };
        let Some(visual) = ElementComposition::get_element_visual(&self.to_ref()) else {
            self.busy.set(false);
            self.report(&CameraError::new(
                CameraErrorKind::Unsupported,
                "Capture UI is not attached to a compositor",
            ));
            self.refresh();
            return;
        };
        let compositor = visual.compositor().clone();
        let generation = self.generation.get();
        let budget = self.budget.borrow().clone();
        let options = self.options.borrow().clone();
        let weak = self.to_ref().downgrade();
        self.message("Starting camera");
        self.refresh();
        Dispatcher::ui_thread().spawn_local(async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(view) = weak
                .upgrade()
                .filter(|v| v.generation.get() == generation && v.is_attached_to_visual_tree())
            else {
                return;
            };
            let result = match interop {
                Some(interop) => match factory(interop.as_ref()) {
                    Ok((provider, importer)) => {
                        preview
                            .start(CameraCapture::new(provider, budget), options, importer)
                            .await
                    }
                    Err(error) => Err(error),
                },
                None => Err(CameraError::new(
                    CameraErrorKind::Unsupported,
                    "No compatible composition GPU interop",
                )),
            };
            if view.generation.get() != generation {
                return;
            }
            view.busy.set(false);
            match result {
                Ok(())
                    if preview
                        .session()
                        .is_some_and(|session| session.supports_video_recording()) =>
                {
                    view.message("Camera ready")
                }
                Ok(()) => {
                    view.message("Camera ready. Video recording is unavailable from this provider.")
                }
                Err(error) => view.report(&error),
            }
            view.refresh();
        });
    }
    pub fn take_photo(&self) {
        if self.busy.get() || self.recording.borrow().is_some() {
            return;
        }
        let Some(session) = self.preview().and_then(|p| p.session()) else {
            return;
        };
        self.busy.set(true);
        self.message("Taking photo");
        self.refresh();
        let generation = self.generation.get();
        let pending = session.photo();
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().spawn_local(async move {
            let result = pending.await;
            if let Some(view) = weak.upgrade().filter(|v| v.generation.get() == generation) {
                view.busy.set(false);
                view.refresh();
                match result {
                    Ok(photo) => {
                        view.message("Photo captured");
                        view.completed.emit(&CameraCaptureResult::Photo(photo));
                    }
                    Err(error) => view.report(&error),
                }
            }
        });
    }
    pub fn start_recording(&self) {
        if self.busy.get() || self.recording.borrow().is_some() {
            return;
        }
        let Some(session) = self.preview().and_then(|p| p.session()) else {
            return;
        };
        self.busy.set(true);
        self.refresh();
        let generation = self.generation.get();
        let pending = session.start_recording();
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().spawn_local(async move {
            let result = pending.await;
            if let Some(view) = weak.upgrade().filter(|v| v.generation.get() == generation) {
                view.busy.set(false);
                match result {
                    Ok(recording) => {
                        *view.recording.borrow_mut() = Some(recording);
                        view.message("Recording silent video");
                    }
                    Err(error) => view.report(&error),
                }
                view.refresh();
            }
        });
    }
    pub fn finish_recording(&self) {
        if self.busy.get() {
            return;
        }
        let Some(recording) = self.recording.borrow_mut().take() else {
            return;
        };
        self.busy.set(true);
        self.message("Finishing video");
        self.refresh();
        let generation = self.generation.get();
        let pending = recording.finish();
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().spawn_local(async move {
            let result = pending.await;
            if let Some(view) = weak.upgrade().filter(|v| v.generation.get() == generation) {
                view.busy.set(false);
                view.refresh();
                match result {
                    Ok(video) => {
                        view.message("Video captured");
                        view.completed.emit(&CameraCaptureResult::Video(video));
                    }
                    Err(error) => view.report(&error),
                }
            }
        });
    }
    pub fn cancel(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        self.recording.borrow_mut().take();
        if let Some(preview) = self.preview() {
            preview.stop();
        }
        self.busy.set(false);
        self.message("Camera stopped");
        self.refresh();
    }
}

impl ControlImpl for CameraCaptureUI {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        crate::automation::CameraCaptureUIAutomationPeer::new(this).upcast()
    }
}
