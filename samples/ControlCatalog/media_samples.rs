//! Media pages shared by catalog hosts. Native providers are installed by each
//! host; constructing or navigating to a page never starts playback/capture.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{imaging::Bitmap, immutable::ImmutableSolidColorBrush, Color, Stretch, TextWrapping},
    reactive::Subscription,
    rendering::composition::{frames::*, ElementComposition, ICompositionGpuInterop},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, Ref, StyledElementImpl, Thickness, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{
    automation::AutomationProperties, primitives::TemplatedControlImpl, Button, ContentPage,
    Control, ControlImpl, Image, Page, PageImpl, ScrollViewer, StackPanel, TextBlock, TextBox,
    WrapPanel,
};
use ferroui_media::{MediaError, MediaSubscription};
use ferroui_media_camera::{CameraCapture, CameraError, CameraFormat, CameraProvider};
use ferroui_media_camera_controls::{CameraPreview, CameraProviderFactory};
use ferroui_media_controls::TextureFrameImporter;
use ferroui_media_qr::{QrAnalysis, QrAnalysisGate, QrCandidate, QrError};
use ferroui_media_video::{MediaPlayerProvider, MediaSource};
use ferroui_media_video_controls::MediaPlayerElement;
use std::{
    cell::{Cell, RefCell},
    io::Cursor,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
    time::Duration,
};

pub type VideoProvider =
    dyn Fn(
        &dyn ICompositionGpuInterop,
        FrameBudget,
    ) -> Result<(Rc<dyn MediaPlayerProvider>, Rc<dyn TextureFrameImporter>), MediaError>;
pub type CameraProviderBinding =
    dyn Fn(
        &dyn ICompositionGpuInterop,
        FrameBudget,
    ) -> Result<(Rc<dyn CameraProvider>, Rc<dyn TextureFrameImporter>), CameraError>;
pub type QrRecognizer = dyn Fn(
    QrAnalysis,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Vec<QrCandidate>, QrError>>>,
>;
#[derive(Clone)]
pub struct MediaSampleProviders {
    pub video: Rc<VideoProvider>,
    pub camera: Rc<CameraProviderBinding>,
    pub qr: Option<Rc<QrRecognizer>>,
}
ferroui_base::ferro_thread_local! { static LAST_PAGE: RefCell<Weak<State>> = const { RefCell::new(Weak::new()) }; }
ferroui_base::ferro_thread_local! { static PROVIDERS: RefCell<Option<MediaSampleProviders>> = const { RefCell::new(None) }; }
/// Install on the host UI thread before opening pages. Missing providers are
/// displayed as unavailable; the offline QR fixture remains usable everywhere.
pub fn install(providers: MediaSampleProviders) {
    PROVIDERS.with(|p| *p.borrow_mut() = Some(providers));
}
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Video,
    Camera,
    Qr,
}
struct State {
    mode: Mode,
    providers: Option<MediaSampleProviders>,
    budget: FrameBudget,
    video: Ref<MediaPlayerElement>,
    camera: Ref<CameraPreview>,
    status: Ref<TextBlock>,
    photo: Ref<Image>,
    photo_pending: Cell<bool>,
    source: Ref<TextBox>,
    attached: Cell<bool>,
    epoch: Cell<u64>,
    clock: std::time::Instant,
    timer: RefCell<Option<Subscription>>,
    subscriptions: RefCell<Vec<MediaSubscription>>,
    gate: QrAnalysisGate,
    qr_source: Cell<Option<(u64, u64)>>,
}
impl State {
    fn status(&self, text: impl AsRef<str>) {
        self.status.set_text(Some(text.as_ref()));
    }
    fn stop(&self) {
        self.epoch.set(self.epoch.get() + 1);
        self.gate.stop();
        self.qr_source.set(None);
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
        self.camera.stop();
        self.video.set_source_uri(None);
        self.video.set_media_player(None, None);
    }
    fn active(&self, epoch: u64) -> bool {
        self.attached.get() && self.epoch.get() == epoch
    }
    fn load(self: &Rc<Self>, bundled: bool) {
        if !self.attached.get() {
            return;
        }
        self.stop();
        let Some(providers) = self.providers.clone() else {
            self.status("Video is unavailable in this host.");
            return;
        };
        if !bundled {
            let value = self.source.text().unwrap_or_default();
            if value.trim().is_empty() {
                self.status("Enter a file path or a supported media URL.");
                return;
            }
            self.video.set_source_uri(Some(value));
            self.status("Preparing media. Press Play when ready.");
            return;
        }
        let epoch = self.epoch.get();
        let weak = Rc::downgrade(self);
        let view = self.video.downgrade();
        let budget = self.budget.clone();
        self.status("Preparing bundled video. Playback starts with Play.");
        Dispatcher::ui_thread().spawn_local(async move {
            let result = async {
                let view = view.upgrade().ok_or_else(MediaError::cancelled)?;
                let visual = ElementComposition::get_element_visual(&view)
                    .ok_or_else(MediaError::cancelled)?;
                let interop = visual
                    .compositor()
                    .try_get_composition_gpu_interop()
                    .await
                    .ok_or_else(|| {
                        MediaError::new(
                            ferroui_media::MediaErrorKind::Unsupported,
                            "This renderer does not provide GPU media presentation.",
                        )
                    })?;
                if !weak.upgrade().is_some_and(|s| s.active(epoch)) {
                    return Err(MediaError::cancelled());
                }
                let (provider, importer) = (providers.video)(&*interop, budget)?;
                view.set_source(sample_source(), provider, importer).await
            }
            .await;
            if let Some(s) = weak.upgrade().filter(|s| s.active(epoch)) {
                match result {
                    Ok(()) => {
                        s.status("Bundled video ready. Use Play, seek, volume and mute below.")
                    }
                    Err(e) => s.status(e.to_string()),
                }
            }
        });
    }
    fn start_camera(self: &Rc<Self>, discover: bool) {
        if !self.attached.get() {
            return;
        }
        self.stop();
        let Some(providers) = self.providers.clone() else {
            self.status("Camera capture is unavailable in this host.");
            return;
        };
        let weak = Rc::downgrade(self);
        let epoch = self.epoch.get();
        self.status(if discover {
            "Discovering cameras; no hardware is acquired."
        } else {
            "Waiting for camera permission and capture."
        });
        Dispatcher::ui_thread().spawn_local(async move {
            let result = async {
                let s = weak
                    .upgrade()
                    .filter(|s| s.active(epoch))
                    .ok_or_else(cancelled)?;
                let visual =
                    ElementComposition::get_element_visual(&s.camera).ok_or_else(cancelled)?;
                let interop = visual
                    .compositor()
                    .try_get_composition_gpu_interop()
                    .await
                    .ok_or_else(|| {
                        CameraError::new(
                            ferroui_media_camera::CameraErrorKind::Unsupported,
                            "This renderer does not provide GPU camera presentation.",
                        )
                    })?;
                if !s.active(epoch) {
                    return Err(cancelled());
                }
                let (provider, importer) = (providers.camera)(&*interop, s.budget.clone())?;
                let capture = CameraCapture::new(provider, s.budget.clone());
                if discover {
                    let devices = capture.devices().await?;
                    if s.active(epoch) {
                        s.status(format!(
                            "{} camera(s); permission {:?}. {}",
                            devices.len(),
                            capture.permission(),
                            devices
                                .iter()
                                .map(|d| d.label.as_deref().unwrap_or("Unnamed camera"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                } else {
                    let mut options = ferroui_media_camera::CameraStartOptions::default();
                    options.selection.format = Some(CameraFormat {
                        width: 640,
                        height: 480,
                        frames_per_second: 30.,
                    });
                    s.camera.start(capture, options, importer).await?;
                    if !s.active(epoch) {
                        return Err(cancelled());
                    }
                    if let Some(session) = s.camera.session() {
                        let settings = session.settings();
                        s.status(format!(
                            "{} × {} at {} fps; {:?}",
                            settings.format.width,
                            settings.format.height,
                            settings.format.frames_per_second,
                            settings.device.label
                        ));
                    }
                    if s.mode == Mode::Qr {
                        s.scan();
                    }
                }
                Ok(())
            }
            .await;
            if let Err(error) = result {
                if let Some(s) = weak.upgrade().filter(|s| s.active(epoch)) {
                    s.status(error.to_string());
                }
            }
        });
    }
    fn scan(self: &Rc<Self>) {
        let Some(recognize) = self.providers.as_ref().and_then(|p| p.qr.clone()) else {
            return;
        };
        let weak = Rc::downgrade(self);
        let epoch = self.epoch.get();
        *self.timer.borrow_mut() = Some(DispatcherTimer::run(
            move || {
                let Some(s) = weak.upgrade().filter(|s| s.active(epoch)) else {
                    return false;
                };
                let Some(frame) = s.camera.session().and_then(|v| v.frames().latest()) else {
                    return true;
                };
                let id = frame.metadata().identity;
                if s.qr_source.get() != Some((id.source, id.generation)) {
                    s.gate.activate(id.source, id.generation);
                    s.qr_source.set(Some((id.source, id.generation)));
                }
                if let Ok(analysis) = s.gate.begin(frame, s.clock.elapsed()) {
                    let pending = recognize(analysis);
                    let weak = weak.clone();
                    Dispatcher::ui_thread().spawn_local(async move {
                        let result = pending.await;
                        if let Some(s) = weak.upgrade().filter(|s| s.active(epoch)) {
                            match result {
                                Ok(values) if !values.is_empty() => {
                                    s.status(format!("Last detected: {}", results_text(&values)))
                                }
                                Ok(_)
                                | Err(QrError::StaleFrame | QrError::Cancelled | QrError::Busy) => {
                                }
                                Err(e) => s.status(format!("QR analysis: {e:?}")),
                            }
                        }
                    });
                }
                true
            },
            Duration::from_millis(100),
            DispatcherPriority::BACKGROUND,
        ));
    }
    fn photo(self: &Rc<Self>) {
        let Some(session) = self.camera.session() else {
            self.status("Start the camera before taking a photo.");
            return;
        };
        if self.photo_pending.replace(true) {
            self.status("A photo capture is already pending.");
            return;
        }
        let pending = session.photo();
        let weak = Rc::downgrade(self);
        let epoch = self.epoch.get();
        Dispatcher::ui_thread().spawn_local(async move {
            let result = pending.await;
            if let Some(s) = weak.upgrade() {
                s.photo_pending.set(false);
            }
            if let Some(s) = weak.upgrade().filter(|s| s.active(epoch)) {
                match result {
                    Ok(photo) => {
                        match Bitmap::from_stream(&mut Cursor::new(photo.encoded.as_ref())) {
                            Ok(bitmap) => {
                                s.photo.set_source(Some(Rc::new(bitmap)));
                                s.photo.set_is_visible(true);
                                s.status(format!(
                                    "Photo {} × {}, {} bytes, {:?}",
                                    photo.width,
                                    photo.height,
                                    photo.encoded.len(),
                                    photo.origin
                                ));
                            }
                            Err(e) => s.status(e.to_string()),
                        }
                    }
                    Err(e) => s.status(e.to_string()),
                }
            }
        });
    }
    fn decode(self: &Rc<Self>) {
        let Some(slot) = ImageAnalysisSlot::acquire() else {
            self.status("QR image analysis is already running.");
            return;
        };
        self.stop();
        let gate = self.gate.clone();
        let weak = Rc::downgrade(self);
        let epoch = self.epoch.get();
        self.status("Decoding the bundled QR image…");
        let result = Arc::new(Mutex::new(None));
        let output = result.clone();
        #[cfg(not(target_arch = "wasm32"))]
        if std::thread::Builder::new()
            .name("Catalog QR image".into())
            .spawn(move || {
                let _slot = slot;
                *output.lock().unwrap() = Some(decode_fixture_with_gate(gate));
            })
            .is_err()
        {
            self.status("Could not start QR analysis.");
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _slot = slot;
            *output.lock().unwrap() = Some(decode_fixture_with_gate(gate));
        }
        DispatcherTimer::run(
            move || {
                let Some(s) = weak.upgrade().filter(|s| s.active(epoch)) else {
                    return false;
                };
                let Some(result) = result.lock().unwrap().take() else {
                    return true;
                };
                match result {
                    Ok(values) => s.status(format!(
                        "Decoded {} QR code(s): {}",
                        values.len(),
                        results_text(&values)
                    )),
                    Err(e) => s.status(format!("QR image: {e:?}")),
                }
                false
            },
            Duration::from_millis(25),
            DispatcherPriority::BACKGROUND,
        )
        .detach();
    }
}
fn cancelled() -> CameraError {
    CameraError::new(
        ferroui_media_camera::CameraErrorKind::Cancelled,
        "Camera page detached",
    )
}
fn sample_source() -> MediaSource {
    // Keep encoded video out of WebAssembly and out of catalog asset prefetch.
    // HTMLMediaElement requests this same-origin file only after Load is chosen.
    #[cfg(target_arch = "wasm32")]
    {
        MediaSource::Url("./media/sintel-sample.mp4".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        MediaSource::Resource {
            name: "sintel-sample.mp4".into(),
            content_type: "video/mp4".into(),
            encoded: Arc::from(include_bytes!("Media/sintel-sample.mp4").as_slice()),
        }
    }
}
fn results_text(values: &[QrCandidate]) -> String {
    values
        .iter()
        .map(|v| v.text.clone().unwrap_or_else(|| "Binary QR payload".into()))
        .collect::<Vec<_>>()
        .join(" · ")
}
#[cfg(test)]
fn decode_fixture() -> Result<Vec<QrCandidate>, QrError> {
    decode_fixture_with_gate(QrAnalysisGate::new(Default::default()))
}
static IMAGE_ANALYSIS_BUSY: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
struct ImageAnalysisSlot;
impl ImageAnalysisSlot {
    fn acquire() -> Option<Self> {
        IMAGE_ANALYSIS_BUSY
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .ok()
            .map(|_| Self)
    }
}
impl Drop for ImageAnalysisSlot {
    fn drop(&mut self) {
        IMAGE_ANALYSIS_BUSY.store(false, std::sync::atomic::Ordering::Release);
    }
}
fn decode_fixture_with_gate(gate: QrAnalysisGate) -> Result<Vec<QrCandidate>, QrError> {
    let image = image::load_from_memory(include_bytes!("Assets/Media/qr-multiple.png"))
        .map_err(|_| QrError::InvalidPlane)?
        .to_rgba8();
    let (width, height) = image.dimensions();
    let bytes = image.into_raw();
    let size = bytes.len() as u64;
    let id = ferroui_media::allocate_source_id();
    let budget = FrameBudget::new(Default::default());
    let frame = budget
        .lease(
            FrameMetadata {
                identity: FrameIdentity {
                    source: id,
                    generation: 1,
                    sequence: 1,
                },
                width,
                height,
                visible: FrameRegion {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                format: FrameFormat::Rgba8,
                color: FrameColor::Srgb,
                alpha: FrameAlpha::Opaque,
                rotation: FrameRotation::None,
                mirrored: false,
                top_left_origin: true,
                timestamp: Duration::ZERO,
                device: None,
                device_generation: 1,
                planes: vec![FramePlane {
                    offset: 0,
                    stride: width as u64 * 4,
                    length: size,
                }],
                retained_bytes: size,
            },
            bytes,
        )
        .map_err(|_| QrError::InvalidPlane)?;
    gate.activate(id, 1);
    let analysis = gate.begin(frame, Duration::ZERO)?;
    ferroui_media_qr::decode(
        &analysis,
        ferroui_media_qr::AnalysisPlane {
            data: analysis
                .frame
                .resource::<Vec<u8>>()
                .ok_or(QrError::InvalidPlane)?,
            width,
            height,
            row_stride: width as usize * 4,
            pixel_stride: 4,
        },
    )
}
#[repr(C)]
struct MediaSamplePage {
    base: ContentPage,
    state: Rc<State>,
}
ferro_class!(MediaSamplePage:ContentPage);
ferro_class_info!(MediaSamplePage {});
ferro_impl_classes!(MediaSamplePage:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,PageImpl);
impl VisualImpl for MediaSamplePage {
    fn on_attached_to_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, args);
        this.state.attached.set(true);
    }
    fn on_detached_from_visual_tree(this: &Self, args: &VisualTreeAttachmentEventArgs) {
        this.state.attached.set(false);
        this.state.stop();
        Self::parent_on_detached_from_visual_tree(this, args);
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.stop();
    }
}
pub fn video_page() -> Ref<Page> {
    page(Mode::Video)
}
pub fn camera_page() -> Ref<Page> {
    page(Mode::Camera)
}
pub fn qr_page() -> Ref<Page> {
    page(Mode::Qr)
}
fn page(mode: Mode) -> Ref<Page> {
    let providers = PROVIDERS.with(|p| p.borrow().clone());
    let state = Rc::new(State {
        mode,
        providers,
        budget: FrameBudget::new(Default::default()),
        video: MediaPlayerElement::new(),
        camera: CameraPreview::new(),
        status: TextBlock::new(),
        photo: Image::new(),
        photo_pending: Cell::new(false),
        source: TextBox::new(),
        attached: Cell::new(false),
        epoch: Cell::new(0),
        clock: std::time::Instant::now(),
        timer: RefCell::new(None),
        subscriptions: RefCell::new(Vec::new()),
        gate: QrAnalysisGate::new(Default::default()),
        qr_source: Cell::new(None),
    });
    LAST_PAGE.with(|p| *p.borrow_mut() = Rc::downgrade(&state));
    let page = instantiate(MediaSamplePage {
        base: ContentPage::construct(),
        state: state.clone(),
    });
    let header = match mode {
        Mode::Video => "Video",
        Mode::Camera => "Camera",
        Mode::Qr => "QR codes",
    };
    page.set_header(Some(Rc::new(header.to_owned())));
    let content = StackPanel::new();
    content.set_spacing(12.);
    content.set_margin(Thickness::uniform(20.));
    let description = TextBlock::new();
    description.set_text(Some(match mode {
        Mode::Video => "Play the bundled video or open a file/URL. Autoplay is off. Navigating away closes the player.",
        Mode::Camera => "Discover devices without acquiring a camera. Start requests consent; Stop or navigation releases capture. Photos stay in memory.",
        Mode::Qr => "Decode the bundled two-symbol image offline, or explicitly start live scanning where this host supports it. Results are displayed as text.",
    }));
    description.set_text_wrapping(TextWrapping::Wrap);
    state.status.set_text_wrapping(TextWrapping::Wrap);
    content.children().add(&description);
    content.children().add(&state.status);
    let actions = WrapPanel::new();
    actions.set_item_spacing(8.);
    actions.set_line_spacing(8.);
    let action = |label: &str, callback: fn(&Rc<State>)| {
        let button = Button::new();
        button.set_content(Some(Rc::new(label.to_owned())));
        AutomationProperties::set_name(&button, Some(label));
        let weak = Rc::downgrade(&state);
        button.click(move |_, _| {
            if let Some(state) = weak.upgrade() {
                callback(&state)
            }
        });
        actions.children().add(&button);
        button
    };
    if mode == Mode::Video {
        if let Some(providers) = state.providers.clone() {
            let budget = state.budget.clone();
            state
                .video
                .set_provider_factory(Some(Rc::new(move |interop| {
                    (providers.video)(interop, budget.clone())
                })));
        }
        state.video.set_are_transport_controls_enabled(true);
        state
            .video
            .set_background(Some(Rc::new(ImmutableSolidColorBrush::new(
                Color::from_rgb(0, 0, 0),
            ))));
        state.video.set_height(320.);
        let placeholder = TextBlock::new();
        placeholder.set_foreground(Some(Rc::new(ImmutableSolidColorBrush::new(
            Color::from_rgb(190, 190, 190),
        ))));
        placeholder.set_text(Some("Load a video to show its first frame."));
        state.video.set_placeholder(Some(placeholder.upcast()));
        state
            .source
            .set_placeholder_text(Some("Media file path or supported URL"));
        AutomationProperties::set_name(&state.source, Some("Media file path or URL"));
        content.children().add(&state.source);
        content.children().add(&actions);
        let credit = TextBlock::new();
        credit.set_text(Some("Sintel trailer excerpt · Durian Open Movie Team · © Blender Foundation | durian.blender.org · CC BY 3.0 (creativecommons.org/licenses/by/3.0). Trimmed and re-encoded for this sample."));
        credit.set_text_wrapping(TextWrapping::Wrap);
        content.children().add(&credit);
        action("Load bundled video", |s| s.load(true));
        action("Open file or URL", |s| s.load(false));
        action("Unload video", |s| {
            s.stop();
            s.status("Video closed.")
        });
        action("Toggle fit / fill", |s| {
            s.video
                .set_stretch(if s.video.stretch() == Stretch::Uniform {
                    Stretch::UniformToFill
                } else {
                    Stretch::Uniform
                })
        });
        let weak = Rc::downgrade(&state);
        state
            .subscriptions
            .borrow_mut()
            .push(state.video.errors().subscribe(move |e| {
                if let Some(s) = weak.upgrade() {
                    s.status(e.to_string())
                }
            }));
        content.children().add(&state.video);
        state.status("Choose media, then press Play.");
    } else {
        content.children().add(&actions);
        if let Some(providers) = state.providers.clone() {
            let budget = state.budget.clone();
            let factory: Rc<CameraProviderFactory> =
                Rc::new(move |interop| (providers.camera)(interop, budget.clone()));
            state.camera.set_provider_factory(Some(factory));
        }
        action("Discover cameras", |s| s.start_camera(true));
        let start = action(
            if mode == Mode::Qr {
                "Start scanning"
            } else {
                "Start camera"
            },
            |s| s.start_camera(false),
        );
        if mode == Mode::Qr
            && state
                .providers
                .as_ref()
                .and_then(|p| p.qr.as_ref())
                .is_none()
        {
            start.set_is_enabled(false);
        }
        action("Stop camera", |s| {
            s.stop();
            s.status("Camera stopped.")
        });
        if mode == Mode::Camera {
            action("Capture photo", |s| s.photo());
            state.photo.set_height(200.);
            state.photo.set_is_visible(false);
        }
        if mode == Mode::Qr {
            action("Decode bundled QR image", |s| s.decode());
            let image = Image::new();
            image.set_height(180.);
            if let Ok(bitmap) = Bitmap::from_stream(&mut Cursor::new(include_bytes!(
                "Assets/Media/qr-multiple.png"
            ))) {
                image.set_source(Some(Rc::new(bitmap)));
            }
            content.children().add(&image);
        }
        state.camera.set_height(300.);
        content.children().add(&state.camera);
        if mode == Mode::Camera {
            content.children().add(&state.photo);
        }
        let weak = Rc::downgrade(&state);
        state
            .subscriptions
            .borrow_mut()
            .push(state.camera.errors().subscribe(move |e| {
                if let Some(s) = weak.upgrade() {
                    s.status(e.to_string())
                }
            }));
        state.status(if mode == Mode::Qr {
            "QR decoding is opt-in. Live scanning requires a supported camera provider."
        } else {
            "Camera stopped. Discovery does not request permission."
        });
    }
    let scroll = ScrollViewer::new();
    scroll.set_content(Some(Control::boxed(content)));
    page.set_content(Some(Control::boxed(scroll)));
    page.upcast()
}
#[cfg(test)]
mod tests {
    #[test]
    fn bundled_qr_image_decodes_two_independent_symbols() {
        let values = super::decode_fixture().unwrap();
        let mut payloads: Vec<_> = values.iter().map(|v| v.text.as_deref().unwrap()).collect();
        payloads.sort();
        assert_eq!(
            payloads,
            ["ferroui-native-qr-left", "ferroui-native-qr-right"]
        );
    }
}

pub(crate) fn register_types() {
    ferroui_base::TypeInfo::register(MediaSamplePage::TYPE);
}

/// Development-only actions used by the desktop's explicitly requested media smoke.
#[doc(hidden)]
pub fn smoke_action(action: &str) -> bool {
    LAST_PAGE.with(|p| {
        let Some(s) = p.borrow().upgrade().filter(|s| s.attached.get()) else {
            return false;
        };
        match action {
            "load" => s.load(true),
            "discover" => s.start_camera(true),
            "start_camera" => s.start_camera(false),
            "decode" => s.decode(),
            "play" => {
                let Some(player) = s.video.media_player() else {
                    return false;
                };
                let mut options = player.options();
                options.muted = true;
                let _ = player.set_options(options);
                Dispatcher::ui_thread().spawn_local(async move {
                    let _ = player.play().await;
                });
            }
            _ => return false,
        }
        true
    })
}
#[doc(hidden)]
pub struct MediaSampleSnapshot {
    pub title: &'static str,
    pub attached: bool,
    pub status: String,
    pub budget: FrameBudget,
    pub playing: bool,
    pub paused: bool,
    pub frame: bool,
    pub camera_active: bool,
}
#[doc(hidden)]
pub fn smoke_snapshot() -> Option<MediaSampleSnapshot> {
    LAST_PAGE.with(|p| {
        p.borrow().upgrade().map(|s| {
            let player = s.video.media_player();
            let state = player.as_ref().map(|p| p.playback_session().state);
            MediaSampleSnapshot {
                title: match s.mode {
                    Mode::Video => "Video",
                    Mode::Camera => "Camera",
                    Mode::Qr => "QR codes",
                },
                attached: s.attached.get(),
                status: s.status.text().unwrap_or_default(),
                budget: s.budget.clone(),
                playing: state == Some(ferroui_media_video::MediaPlaybackState::Playing),
                paused: state == Some(ferroui_media_video::MediaPlaybackState::Paused),
                frame: player.is_some_and(|p| p.frames().latest().is_some())
                    || s.camera
                        .session()
                        .is_some_and(|c| c.frames().latest().is_some()),
                camera_active: s.camera.session().is_some(),
            }
        })
    })
}
