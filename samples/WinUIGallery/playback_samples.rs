//! Gallery compositions over the existing portable media controls. Host factories
//! and the bundled video are shared with ControlCatalog; no second media stack.
use ferroui_base::{
    platform::storage::{FilePickerOpenOptions, IStorageFile},
    rendering::composition::{frames::FrameBudget, ElementComposition, ICompositionGpuInterop},
    threading::Dispatcher,
    Ref, WeakRef,
};
use ferroui_controls::{Button, CheckBox, Control, Image, Slider, TextBlock, TopLevel};
use ferroui_media::{MediaError, MediaErrorKind, MediaSubscription};
use ferroui_media_camera::{CameraError, CameraProvider};
use ferroui_media_camera_controls::{CameraCaptureResult, CameraCaptureUI};
use ferroui_media_controls::TextureFrameImporter;
use ferroui_media_video::{MediaPlayerProvider, MediaSource};
use ferroui_media_video_controls::MediaPlayerElement;
use std::{
    cell::{Cell, RefCell},
    io::Cursor,
    rc::Rc,
    sync::Arc,
};

type VideoFactory =
    dyn Fn(
        &dyn ICompositionGpuInterop,
        FrameBudget,
    ) -> Result<(Rc<dyn MediaPlayerProvider>, Rc<dyn TextureFrameImporter>), MediaError>;
type CameraFactory =
    dyn Fn(
        &dyn ICompositionGpuInterop,
        FrameBudget,
    ) -> Result<(Rc<dyn CameraProvider>, Rc<dyn TextureFrameImporter>), CameraError>;
#[derive(Clone)]
pub struct MediaProviders {
    pub video: Rc<VideoFactory>,
    pub camera: Rc<CameraFactory>,
}
thread_local! { static PROVIDERS: RefCell<Option<MediaProviders>> = const { RefCell::new(None) }; }
pub fn install(providers: MediaProviders) {
    PROVIDERS.with(|slot| *slot.borrow_mut() = Some(providers));
}
#[cfg(feature = "desktop")]
pub fn install_desktop() {
    install(MediaProviders {
        video: Rc::new(crate::desktop_media::video),
        camera: Rc::new(crate::desktop_media::camera),
    });
}

struct VideoBinding {
    view: WeakRef<MediaPlayerElement>,
    status: WeakRef<TextBlock>,
    factory: Option<Rc<VideoFactory>>,
    budget: FrameBudget,
    request: Cell<u64>,
    volume: Cell<f64>,
    muted: Cell<bool>,
    storage: RefCell<Option<Rc<dyn IStorageFile>>>,
    errors: RefCell<Option<MediaSubscription>>,
}
impl VideoBinding {
    fn new(view: Ref<MediaPlayerElement>, status: Ref<TextBlock>) -> Rc<Self> {
        let factory = PROVIDERS.with(|slot| slot.borrow().as_ref().map(|p| p.video.clone()));
        let budget = FrameBudget::new(Default::default());
        if let Some(factory) = factory.clone() {
            let budget = budget.clone();
            view.set_provider_factory(Some(Rc::new(move |interop| {
                factory(interop, budget.clone())
            })));
        }
        let binding = Rc::new(Self {
            view: view.downgrade(),
            status: status.downgrade(),
            factory,
            budget,
            request: Cell::new(0),
            volume: Cell::new(1.),
            muted: Cell::new(false),
            storage: RefCell::new(None),
            errors: RefCell::new(None),
        });
        let weak = Rc::downgrade(&binding);
        *binding.errors.borrow_mut() = Some(view.errors().subscribe(move |error| {
            if let Some(binding) = weak.upgrade() {
                binding.message(&error.to_string());
            }
        }));
        binding
    }
    fn message(&self, text: &str) {
        if let Some(status) = self.status.upgrade() {
            status.set_text(Some(text));
        }
    }
    fn cancel(&self) {
        self.request.set(self.request.get().wrapping_add(1));
        if let Some(view) = self.view.upgrade() {
            view.set_media_player(None, None);
        }
        self.storage.borrow_mut().take();
    }
    fn audio_options(&self) {
        if let Some(player) = self.view.upgrade().and_then(|v| v.media_player()) {
            let mut options = player.options();
            options.volume = self.volume.get();
            options.muted = self.muted.get();
            if let Err(error) = player.set_options(options) {
                self.message(&error.to_string());
            }
        }
    }
    fn load(self: &Rc<Self>, source: MediaSource, storage: Option<Rc<dyn IStorageFile>>) {
        self.cancel();
        let Some(view) = self
            .view
            .upgrade()
            .filter(|v| v.is_attached_to_visual_tree())
        else {
            return;
        };
        let Some(factory) = self.factory.clone() else {
            self.message("Native media providers are unavailable in this host");
            return;
        };
        let Some(visual) = ElementComposition::get_element_visual(&view) else {
            self.message("No composition renderer");
            return;
        };
        *self.storage.borrow_mut() = storage;
        let compositor = visual.compositor().clone();
        let request = self.request.get();
        let weak = Rc::downgrade(self);
        self.message("Loading media");
        Dispatcher::ui_thread().spawn_local(async move {
            let interop = compositor.try_get_composition_gpu_interop().await;
            let Some(binding) = weak.upgrade().filter(|b| b.request.get() == request) else {
                return;
            };
            let Some(view) = binding
                .view
                .upgrade()
                .filter(|v| v.is_attached_to_visual_tree())
            else {
                return;
            };
            let result = match interop {
                Some(interop) => match factory(interop.as_ref(), binding.budget.clone()) {
                    Ok((provider, importer)) => {
                        let pending = view.set_source(source, provider, importer);
                        binding.audio_options();
                        pending.await
                    }
                    Err(error) => Err(error),
                },
                None => Err(MediaError::new(
                    MediaErrorKind::Unsupported,
                    "No compatible GPU media profile",
                )),
            };
            if binding.request.get() == request {
                match result {
                    Ok(()) => binding.message("Media ready"),
                    Err(error) => binding.message(&error.to_string()),
                }
            }
        });
    }
}
fn bundled_video() -> MediaSource {
    MediaSource::Resource {
        name: "sintel-sample.mp4".into(),
        content_type: "video/mp4".into(),
        encoded: Arc::from(include_bytes!("../ControlCatalog/Media/sintel-sample.mp4").as_slice()),
    }
}
pub(crate) fn wire(root: &Ref<Control>) {
    if let Some(player) = root.find_control::<MediaPlayerElement>("GalleryPlayer") {
        if let Ok(poster) = crate::assets::load_bitmap(
            "ferres://WinUIGallery/Assets/ControlImages/MediaPlayerElement.png",
        ) {
            player.set_poster_source(Some(Rc::new(poster)));
        }
        let binding = VideoBinding::new(player.clone(), root.get_control("GalleryMediaStatus"));
        let auto = VideoBinding::new(
            root.get_control("GalleryAutoPlayer"),
            root.get_control("GalleryMediaStatus"),
        );
        let load = binding.clone();
        root.get_control::<Button>("LoadGalleryVideo")
            .click(move |_, _| load.load(bundled_video(), None));
        let load = auto.clone();
        root.get_control::<Button>("LoadGalleryAutoVideo")
            .click(move |_, _| load.load(bundled_video(), None));
        let weak_player = player.downgrade();
        root.get_control::<CheckBox>("GalleryCompact")
            .is_checked_changed(move |sender, _| {
                if let Some(player) = weak_player.upgrade() {
                    player.transport_controls().set_is_compact(
                        sender
                            .to_ref()
                            .cast::<CheckBox>()
                            .is_some_and(|c| c.is_checked() == Some(true)),
                    );
                }
            });
        let weak_player = player.downgrade();
        root.get_control::<CheckBox>("GalleryFullWindow")
            .is_checked_changed(move |sender, _| {
                if let Some(player) = weak_player.upgrade() {
                    player.set_is_full_window(
                        sender
                            .to_ref()
                            .cast::<CheckBox>()
                            .is_some_and(|c| c.is_checked() == Some(true)),
                    );
                }
            });
        let weak_check = root
            .get_control::<CheckBox>("GalleryFullWindow")
            .downgrade();
        player.add_property_changed_handler(Rc::new(move |change| {
            if change.property() == MediaPlayerElement::is_full_window_property().as_property() {
                if let Some(check) = weak_check.upgrade() {
                    let (_, value) = change.get_old_and_new_value::<bool>();
                    check.set_is_checked(Some(value));
                }
            }
        }));
        let load = binding.clone();
        let weak_root = root.downgrade();
        root.get_control::<Button>("OpenGalleryVideo")
            .click(move |_, _| {
                let Some(root) = weak_root.upgrade() else {
                    return;
                };
                let Some(top) = TopLevel::get_top_level(Some(&root)) else {
                    return;
                };
                let storage = top.storage_provider();
                if !storage.can_open() {
                    load.message("File picker unavailable");
                    return;
                }
                let load = load.clone();
                let request = load.request.get();
                Dispatcher::ui_thread().spawn_local(async move {
                    match storage
                        .open_file_picker_async(FilePickerOpenOptions::new())
                        .await
                    {
                        Ok(files) if load.request.get() == request => {
                            if let Some(file) = files.into_iter().next() {
                                load.load(MediaSource::Url(file.path().to_string()), Some(file));
                            }
                        }
                        Ok(_) => {}
                        Err(error) => load.message(&error.to_string()),
                    }
                });
            });
        root.detached_from_visual_tree(move |_| {
            binding.cancel();
            auto.cancel();
        })
        .detach();
    }
    if let Some(capture) = root.find_control::<CameraCaptureUI>("GalleryCapture") {
        let budget = FrameBudget::new(Default::default());
        if let Some(providers) = PROVIDERS.with(|slot| slot.borrow().clone()) {
            let factory_budget = budget.clone();
            capture.set_provider_factory(
                Some(Rc::new(move |interop| {
                    (providers.camera)(interop, factory_budget.clone())
                })),
                budget,
            );
        }
        let weak_photo = root
            .get_control::<Image>("GalleryCapturedPhoto")
            .downgrade();
        let weak_status = root
            .get_control::<TextBlock>("GalleryCaptureResult")
            .downgrade();
        let subscription = capture.completed().subscribe(move |result| {
            let Some(status) = weak_status.upgrade() else {
                return;
            };
            match result {
                CameraCaptureResult::Photo(photo) => {
                    match ferroui_base::media::imaging::Bitmap::from_stream(&mut Cursor::new(
                        photo.encoded.as_ref(),
                    )) {
                        Ok(bitmap) => {
                            if let Some(image) = weak_photo.upgrade() {
                                image.set_source(Some(Rc::new(bitmap)));
                            }
                            status.set_text(Some(&format!(
                                "{} × {} photo ({:?})",
                                photo.width, photo.height, photo.origin
                            )));
                        }
                        Err(error) => {
                            status.set_text(Some(&format!("Photo decode failed: {error}")))
                        }
                    }
                }
                CameraCaptureResult::Video(video) => status.set_text(Some(&format!(
                    "{} × {} silent video, {:.2} s, {} encoded bytes",
                    video.width,
                    video.height,
                    video.duration.as_secs_f64(),
                    video.encoded.len()
                ))),
            }
        });
        // The root keeps the result subscription alive; the callback holds only
        // weak child references, so abandoning a page cannot retain captures.
        root.detached_from_visual_tree(move |_| {
            let _ = &subscription;
        })
        .detach();
    }
    if let Some(player) = root.find_control::<MediaPlayerElement>("GallerySoundPlayer") {
        let binding = VideoBinding::new(player, root.get_control("GallerySoundStatus"));
        binding.volume.set(0.5);
        for (index, name) in [
            "Focus",
            "Invoke",
            "Show",
            "Hide",
            "MovePrevious",
            "MoveNext",
            "GoBack",
        ]
        .iter()
        .enumerate()
        {
            let load = binding.clone();
            let name = *name;
            root.get_control::<Button>(&format!("Sound{name}"))
                .click(move |_, _| {
                    if !load.muted.get() {
                        load.load(sound_source(index, name), None);
                    }
                });
        }
        let sound = binding.clone();
        root.get_control::<CheckBox>("GallerySoundEnabled")
            .is_checked_changed(move |sender, _| {
                sound.muted.set(
                    !sender
                        .to_ref()
                        .cast::<CheckBox>()
                        .is_some_and(|c| c.is_checked() == Some(true)),
                );
                sound.audio_options();
            });
        let sound = binding.clone();
        root.get_control::<Slider>("GallerySoundVolume")
            .value_changed(move |_, args| {
                sound.volume.set(args.new_value());
                sound.audio_options();
            });
        root.detached_from_visual_tree(move |_| binding.cancel())
            .detach();
    }
}
/// Small, application-owned PCM cues. No Windows system-sound assets or claims
/// of system sound/spatial parity. Native playback still uses the media stack.
fn sound_source(index: usize, name: &str) -> MediaSource {
    // Match the Media Foundation/WASAPI provider’s native PCM format.
    let rate = 48_000u32;
    let samples = rate / 5;
    let length = samples * 4;
    let mut bytes = Vec::with_capacity(44 + length as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + length).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&length.to_le_bytes());
    let frequency = 330. * 2f64.powf(index as f64 / 12.);
    for sample in 0..samples {
        let phase = sample as f64 / rate as f64;
        let envelope = (std::f64::consts::PI * sample as f64 / samples as f64)
            .sin()
            .powi(2);
        let value = (std::f64::consts::TAU * frequency * phase).sin() * envelope * 6000.;
        let pcm = (value as i16).to_le_bytes();
        bytes.extend_from_slice(&pcm);
        bytes.extend_from_slice(&pcm);
    }
    MediaSource::Resource {
        name: format!("{name}.wav"),
        content_type: "audio/wav".into(),
        encoded: bytes.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn media_pages_are_native_and_idle_without_a_provider() {
        let _app = crate::tests::start();
        for page in ["MediaPlayerElement", "CaptureElementPreview", "Sound"] {
            let root = crate::examples::create(crate::examples::find(page).unwrap()).unwrap();
            if page == "MediaPlayerElement" {
                let player = root.get_control::<MediaPlayerElement>("GalleryPlayer");
                assert!(player.media_player().is_none());
                assert!(!player.autoplay());
                root.get_control::<CheckBox>("GalleryCompact")
                    .set_is_checked(Some(true));
                assert!(player.transport_controls().is_compact());
                assert!(root
                    .get_control::<MediaPlayerElement>("GalleryAutoPlayer")
                    .autoplay());
            } else if page == "CaptureElementPreview" {
                let capture = root.get_control::<CameraCaptureUI>("GalleryCapture");
                assert!(!capture.is_busy());
                assert!(
                    capture.preview().is_none(),
                    "creating an untemplated view cannot acquire hardware"
                );
            } else {
                let player = root.get_control::<MediaPlayerElement>("GallerySoundPlayer");
                assert!(player.media_player().is_none());
                assert!(player.autoplay());
                for name in [
                    "Focus",
                    "Invoke",
                    "Show",
                    "Hide",
                    "MovePrevious",
                    "MoveNext",
                    "GoBack",
                ] {
                    assert!(root
                        .find_control::<Button>(&format!("Sound{name}"))
                        .is_some());
                }
            }
        }
    }
    #[test]
    fn sound_cues_are_bounded_valid_pcm_with_distinct_content() {
        let mut previous = None;
        for index in 0..7 {
            let MediaSource::Resource {
                encoded,
                content_type,
                ..
            } = sound_source(index, "fixture")
            else {
                unreachable!()
            };
            assert_eq!(content_type, "audio/wav");
            assert_eq!(&encoded[..4], b"RIFF");
            assert_eq!(&encoded[8..16], b"WAVEfmt ");
            assert_eq!(encoded.len(), 38444);
            assert_eq!(u16::from_le_bytes(encoded[22..24].try_into().unwrap()), 2);
            assert_eq!(
                u32::from_le_bytes(encoded[24..28].try_into().unwrap()),
                48000
            );
            assert_eq!(
                u32::from_le_bytes(encoded[40..44].try_into().unwrap()),
                38400
            );
            if let Some(previous) = previous {
                assert_ne!(encoded, previous);
            }
            previous = Some(encoded);
        }
    }
}
