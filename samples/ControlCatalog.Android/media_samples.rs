use control_catalog::media_samples::{install as install_providers, MediaSampleProviders};
use ferroui_android::{
    log::{self, LogPriority},
    FerroActivity,
};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::TextBlock;
use ferroui_media::frames::{FrameColor, PixelTransferCount};
use ferroui_media_android::{NativeGlFrameProducer, NativeGlTextureImporter};
use ferroui_media_camera::{CameraError, CameraErrorKind};
use ferroui_media_camera_android::AndroidCameraProvider;
use ferroui_media_video_android::AndroidVideoProvider;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

pub fn install() {
    install_providers(MediaSampleProviders {
        video: Rc::new(|interop, budget| {
            Ok((
                Rc::new(AndroidVideoProvider {
                    frames: NativeGlFrameProducer::from_interop(interop, budget)?,
                }),
                Rc::new(NativeGlTextureImporter),
            ))
        }),
        camera: Rc::new(|interop, budget| {
            let frames = NativeGlFrameProducer::from_interop(interop, budget).map_err(|error| {
                CameraError::new(CameraErrorKind::Unsupported, error.to_string())
            })?;
            Ok((
                Rc::new(AndroidCameraProvider {
                    frames: Some(frames),
                    untagged_sdr_color: FerroActivity::current_main_activity()
                        .and_then(|activity| {
                            activity.intent_string_extra("FERROUI_CAMERA_SDR_SRGB")
                        })
                        .filter(|value| value == "1")
                        .map(|_| FrameColor::Srgb),
                }),
                Rc::new(NativeGlTextureImporter),
            ))
        }),
        qr: None,
    });
}

pub fn smoke_camera() {
    use control_catalog::media_samples::smoke_capture;
    let started = Instant::now();
    let recording_started = Cell::new(None::<Instant>);
    let recording_preview_count = Cell::new(0);
    let stage = Cell::new(0);
    let write = |message: &str| log::write(LogPriority::Info, "ferroui-catalog", message);
    let fail = move |reason: &str| {
        write(&format!("CATALOG_CAMERA_FAILED {reason}"));
        if let Some(capture) = smoke_capture() {
            capture.control.cancel();
        }
        false
    };
    DispatcherTimer::run(move || {
        if started.elapsed() >= Duration::from_secs(60) {
            return fail(&format!("timeout at stage {}", stage.get()));
        }
        let Some(capture) = smoke_capture() else { return true };
        if let Some(error) = capture.error.get() {
            return fail(&format!("{error:?} at stage {}", stage.get()));
        }
        let counters = capture.budget.counters().snapshot();
        match stage.get() {
            0 => {
                capture.control.start_preview();
                stage.set(1);
            }
            1 if counters.gpu_conversions >= 5 => {
                capture.control.take_photo();
                stage.set(2);
            }
            2 if capture.photo.source().is_some() => {
                if !capture.control.preview().and_then(|view| view.session())
                    .is_some_and(|session| session.supports_video_recording()) {
                    return fail("camera did not advertise native recording");
                }
                capture.control.start_recording();
                stage.set(3);
            }
            3 if !capture.control.is_busy() => {
                let began = recording_started.get().unwrap_or_else(|| {
                    let began = Instant::now();
                    recording_started.set(Some(began));
                    recording_preview_count.set(counters.gpu_conversions);
                    began
                });
                if began.elapsed() >= Duration::from_secs(2) {
                    if counters.gpu_conversions < recording_preview_count.get() + 3 {
                        return fail("preview stopped while recording");
                    }
                    capture.control.finish_recording();
                    stage.set(4);
                }
            }
            4 if capture.movie_valid.get() => {
                write("CATALOG_CAMERA_UI_RECORDING_PASS silent H.264 MP4");
                capture.control.start_recording();
                stage.set(5);
            }
            5 if !capture.control.is_busy() => {
                capture.movie_valid.set(false);
                capture.control.cancel();
                stage.set(6);
            }
            6 if capture.budget.usage().frames == 0 && capture.budget.gpu_reads() == 0
                && capture.budget.quarantined_reads() == 0 => {
                if capture.photo.source().is_none() || counters.preview != PixelTransferCount::default()
                    || capture.movie_valid.get() {
                    return fail("photo ownership, recording cancellation or preview transfer check failed");
                }
                write("CATALOG_CAMERA_PASS photo retained, recording cancelled, no retained GPU frames or CPU preview transfers");
                return false;
            }
            _ => {}
        }
        true
    }, Duration::from_millis(100), DispatcherPriority::NORMAL).detach();
}

pub fn smoke_tracks(source: String) {
    use control_catalog::media_samples::{
        smoke_action, smoke_load_video_uri, smoke_transport_item, smoke_video_element,
    };
    use ferroui_media_video::MediaTrackKind;
    let started = Instant::now();
    let stage = Cell::new(0);
    let selected_audio = Cell::new(None);
    let selected_at = Cell::new(None::<(Instant, Duration)>);
    let report = |message: &str| log::write(LogPriority::Info, "ferroui-catalog", message);
    DispatcherTimer::run(move || {
        if started.elapsed() >= Duration::from_secs(45) {
            report(&format!("CATALOG_TRACKS_FAILED timeout at stage {}", stage.get()));
            return false;
        }
        let Some(view) = smoke_video_element() else { return true };
        if stage.get() == 0 {
            if smoke_load_video_uri(&source) { stage.set(1); }
            return true;
        }
        let Some(player) = view.media_player() else { return true };
        let audio = player.tracks(MediaTrackKind::Audio);
        let captions = player.tracks(MediaTrackKind::Captions);
        let position = player.playback_session().position;
        match stage.get() {
            1 if audio.len() >= 2 && !captions.is_empty() && player.frames().latest().is_some() => {
                let Some(other) = audio.iter().find(|track| !track.selected) else { return true };
                if !smoke_transport_item(&["Audio tracks", &other.label])
                    || !smoke_transport_item(&["Closed captions", &captions[0].label])
                    || !smoke_transport_item(&["Playback speed", "1.5×"])
                    || !smoke_action("play") {
                    report("CATALOG_TRACKS_FAILED menu action unavailable");
                    return false;
                }
                selected_audio.set(Some(other.id));
                stage.set(2);
            }
            2 if position >= Duration::from_secs(1) => {
                selected_at.set(Some((Instant::now(), position)));
                stage.set(3);
            }
            3 if audio.iter().any(|track| track.selected && Some(track.id) == selected_audio.get())
                && captions.iter().any(|track| track.selected)
                && view.get_visual_descendants().any(|visual| visual.cast::<TextBlock>()
                    .is_some_and(|text| text.text().as_deref() == Some("FerroUI caption fixture"))) => {
                let (then, original) = selected_at.get().unwrap();
                if then.elapsed() < Duration::from_secs(2) { return true; }
                let rate = position.saturating_sub(original).as_secs_f64() / then.elapsed().as_secs_f64();
                if !(1.25..=1.75).contains(&rate) || !smoke_transport_item(&["Closed captions", "Off"]) {
                    report(&format!("CATALOG_TRACKS_FAILED measured rate {rate:.2} or captions off unavailable"));
                    return false;
                }
                stage.set(4);
            }
            4 if player.caption_text().is_none() && captions.iter().all(|track| !track.selected) => {
                report("CATALOG_TRACKS_PASS alternate audio, caption rendering and Off, native 1.5× playback");
                return false;
            }
            _ => {}
        }
        true
    }, Duration::from_millis(100), DispatcherPriority::NORMAL).detach();
}
