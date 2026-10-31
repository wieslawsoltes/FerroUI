use super::{phase, PROBE};
use ferroui_base::{
    reactive::IDisposable,
    rendering::composition::ElementComposition,
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::StackPanel;
use ferroui_media::{frames::*, MediaSubscription};
use ferroui_media_browser::{BrowserTextureImporter, WebGlFrameProducer};
use ferroui_media_camera::{CameraCapture, CameraPhoto, CameraProvider, CameraSession};
use ferroui_media_camera_browser::BrowserCameraProvider;
use ferroui_media_controls::TexturePresenter;
use ferroui_media_video::{MediaPlayer, MediaSource};
use ferroui_media_video_browser::BrowserVideoProvider;
use ferroui_media_video_controls::MediaPlayerElement;
use std::{cell::RefCell, rc::Rc, time::Duration};
use wasm_bindgen::prelude::*;
#[derive(Default)]
struct Demo {
    generation: u64,
    video: Option<Ref<MediaPlayerElement>>,
    presenter: Option<Ref<TexturePresenter>>,
    capture: Option<CameraCapture>,
    camera: Option<CameraSession>,
    photo: Option<CameraPhoto>,
    poll: Option<Rc<dyn IDisposable>>,
    subscriptions: Vec<MediaSubscription>,
}
thread_local! {static DEMO:RefCell<Demo>=RefCell::new(Demo::default());}
pub fn clear() {
    let mut old = DEMO.with(|d| {
        let mut current = d.borrow_mut();
        let generation = current.generation + 1;
        std::mem::replace(
            &mut *current,
            Demo {
                generation,
                ..Default::default()
            },
        )
    });
    if let Some(capture) = old.capture.take() {
        capture.cancel_pending_start();
    }
    if let Some(camera) = old.camera.take() {
        camera.stop();
    }
    if let Some(timer) = old.poll.take() {
        timer.dispose();
    }
    if let Some(video) = old.video.take() {
        if let Some(player) = video.media_player() {
            player.clear_source();
        }
        video.set_media_player(None, None);
    }
    if let Some(view) = old.presenter.take() {
        view.set_frame_source(None, None);
    }
}
fn current(generation: u64) -> bool {
    DEMO.with(|d| d.borrow().generation == generation)
}
#[wasm_bindgen(js_name=rejectCpuUpload)]
pub fn reject_cpu_upload(source: JsValue) {
    let view = PROBE.with(|p| p.borrow().as_ref().and_then(|p| p.views.first().cloned()));
    let Some(view) = view else {
        return;
    };
    let Some(visual) = ElementComposition::get_element_visual(&view) else {
        return;
    };
    let compositor = visual.compositor().clone();
    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
        let Some(interop) = compositor.try_get_composition_gpu_interop().await else {
            return;
        };
        let budget = FrameBudget::new(FrameBudgetLimits::default());
        let Ok(producer) = WebGlFrameProducer::from_interop(&*interop, budget.clone()) else {
            return;
        };
        let result = producer.upload(
            FrameIdentity {
                source: 701,
                generation: 1,
                sequence: 1,
            },
            4,
            4,
            Duration::ZERO,
            &source,
        );
        let counts = budget.counters().snapshot();
        phase(format!(
            "cpu-rejected:{}:{}:{}:{}",
            result.is_err(),
            counts.preview.attempts,
            counts.preview.bytes,
            budget.usage().frames
        ));
    });
}
fn root() -> Option<(Ref<StackPanel>, FrameBudget, u64)> {
    super::clear();
    let generation = DEMO.with(|d| d.borrow().generation);
    let result = PROBE.with(|p| {
        p.borrow()
            .as_ref()
            .map(|p| (p.root.clone(), p.budget.clone(), generation))
    });
    if let Some((root, _, _)) = &result {
        root.children().clear();
    }
    result
}
#[wasm_bindgen(js_name=openVideo)]
pub fn open_video(url: &str) {
    open(MediaSource::Url(url.into()));
}
#[wasm_bindgen(js_name=openResource)]
pub fn open_resource(bytes: &[u8], content_type: &str) {
    open(MediaSource::Resource {
        name: "browser-selected-file".into(),
        content_type: content_type.into(),
        encoded: bytes.into(),
    });
}
fn open(source: MediaSource) {
    let Some((root, budget, generation)) = root() else {
        return;
    };
    let view = MediaPlayerElement::new();
    view.set_height(360.);
    view.set_are_transport_controls_enabled(true);
    let subscription = view.errors().subscribe(|e| phase(format!("error:{e}")));
    root.children().add(&view);
    let weak = view.downgrade();
    DEMO.with(|d| {
        let mut d = d.borrow_mut();
        d.video = Some(view);
        d.subscriptions.push(subscription);
    });
    phase("video-opening");
    let source = RefCell::new(Some(source));
    DispatcherTimer::run(
        move || {
            if !current(generation) {
                return false;
            }
            let Some(view) = weak.upgrade() else {
                return false;
            };
            let Some(visual) = ElementComposition::get_element_visual(&view) else {
                return true;
            };
            let compositor = visual.compositor().clone();
            let budget = budget.clone();
            let source = source.borrow_mut().take().unwrap();
            let weak = weak.clone();
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let result: Result<(), String> = async {
                    let interop = compositor
                        .try_get_composition_gpu_interop()
                        .await
                        .ok_or("No GPU interop".to_owned())?;
                    if !current(generation) {
                        return Ok(());
                    }
                    let producer = WebGlFrameProducer::from_interop(&*interop, budget)
                        .map_err(|e| e.to_string())?;
                    let Some(view) = weak.upgrade() else {
                        return Ok(());
                    };
                    view.set_source(
                        source,
                        Rc::new(BrowserVideoProvider { frames: producer }),
                        Rc::new(BrowserTextureImporter),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    Ok(())
                }
                .await;
                if current(generation) {
                    match result {
                        Ok(()) => phase("video-ready"),
                        Err(e) => phase(format!("error:{e}")),
                    }
                }
            });
            false
        },
        Duration::from_millis(25),
        DispatcherPriority::NORMAL,
    );
}
fn player() -> Option<MediaPlayer> {
    DEMO.with(|d| d.borrow().video.as_ref().and_then(|v| v.media_player()))
}
#[wasm_bindgen(js_name=playVideo)]
pub fn play_video() {
    let (view, generation) = DEMO.with(|d| {
        let d = d.borrow();
        (d.video.clone(), d.generation)
    });
    if let Some(view) = view {
        let pending = view.play();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let result = pending.await;
            if !current(generation) {
                return;
            }
            match result {
                Ok(()) => phase("video-playing"),
                Err(e) => phase(format!("play-error:{:?}", e.kind)),
            }
        });
    }
}
#[wasm_bindgen(js_name=pauseVideo)]
pub fn pause_video() {
    if let Some(p) = player() {
        if let Err(e) = p.pause() {
            phase(format!("error:{e}"));
        }
    }
}
#[wasm_bindgen(js_name=seekVideo)]
pub fn seek_video(milliseconds: u32) {
    let generation = DEMO.with(|d| d.borrow().generation);
    if let Some(p) = player() {
        let pending = p.seek(Duration::from_millis(milliseconds.into()));
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let result = pending.await;
            if !current(generation) {
                return;
            }
            match result {
                Ok(position) => phase(format!("seek:{}", position.as_millis())),
                Err(e) if e.kind == ferroui_media::MediaErrorKind::Cancelled => {}
                Err(e) => phase(format!("seek-error:{:?}", e.kind)),
            }
        });
    }
}
#[wasm_bindgen(js_name=stopVideo)]
pub fn stop_video() {
    let generation = DEMO.with(|d| d.borrow().generation);
    if let Some(p) = player() {
        let pending = p.stop();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let result = pending.await;
            if !current(generation) {
                return;
            }
            match result {
                Ok(position) => phase(format!("stop:{}", position.as_millis())),
                Err(e) => phase(format!("error:{e}")),
            }
        });
    }
}
#[wasm_bindgen(js_name=discoverCameras)]
pub fn discover_cameras() {
    let provider = BrowserCameraProvider::discovery();
    let before = provider.permission();
    let pending = provider.devices();
    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
        match pending.await {
            Ok(devices) => phase(format!(
                "discovery:{}:{before:?}:{:?}",
                devices.len(),
                provider.permission()
            )),
            Err(e) => phase(format!("discovery-error:{e}")),
        }
    });
}
#[wasm_bindgen(js_name=videoState)]
pub fn video_state() -> String {
    player()
        .map(|p| {
            let s = p.playback_session();
            format!(
                "state={:?};position={};duration={};seekable={};frame={}",
                s.state,
                s.position.as_millis(),
                s.duration.map(|d| d.as_millis()).unwrap_or_default(),
                s.seekable,
                p.frames()
                    .latest()
                    .map(|f| f.metadata().timestamp.as_millis().to_string())
                    .unwrap_or_default()
            )
        })
        .unwrap_or_default()
}
#[wasm_bindgen(js_name=startCamera)]
pub fn start_camera() {
    let Some((root, budget, generation)) = root() else {
        return;
    };
    let view = TexturePresenter::new();
    view.set_height(360.);
    root.children().add(&view);
    let weak = view.downgrade();
    DEMO.with(|d| d.borrow_mut().presenter = Some(view));
    phase("camera-opening");
    DispatcherTimer::run(
        move || {
            if !current(generation) {
                return false;
            }
            let Some(view) = weak.upgrade() else {
                return false;
            };
            let Some(visual) = ElementComposition::get_element_visual(&view) else {
                return true;
            };
            let compositor = visual.compositor().clone();
            let weak = weak.clone();
            let budget = budget.clone();
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let result: Result<(), String> = async {
                    let interop = compositor
                        .try_get_composition_gpu_interop()
                        .await
                        .ok_or("No GPU interop".to_owned())?;
                    if !current(generation) {
                        return Ok(());
                    }
                    let frames = WebGlFrameProducer::from_interop(&*interop, budget.clone())
                        .map_err(|e| e.to_string())?;
                    let capture =
                        CameraCapture::new(Rc::new(BrowserCameraProvider::new(frames)), budget);
                    let pending = capture.start(Default::default());
                    DEMO.with(|d| d.borrow_mut().capture = Some(capture));
                    let session = pending.await.map_err(|e| e.to_string())?;
                    if !current(generation) {
                        session.stop();
                        return Ok(());
                    }
                    let Some(view) = weak.upgrade() else {
                        session.stop();
                        return Ok(());
                    };
                    let subscription = view.errors().subscribe(|e| phase(format!("error:{e}")));
                    view.set_frame_source(
                        Some(session.frames()),
                        Some(Rc::new(BrowserTextureImporter)),
                    );
                    DEMO.with(|d| {
                        let mut d = d.borrow_mut();
                        d.camera = Some(session);
                        d.subscriptions.push(subscription);
                    });
                    let poll = DispatcherTimer::run(
                        move || {
                            if !current(generation) {
                                return false;
                            }
                            let session = DEMO.with(|d| d.borrow().camera.clone());
                            let Some(session) = session else {
                                return false;
                            };
                            if let Err(e) = session.poll() {
                                phase(format!("error:{e}"));
                                return false;
                            }
                            if let Some(view) = weak.upgrade() {
                                view.present_latest();
                                true
                            } else {
                                session.stop();
                                false
                            }
                        },
                        Duration::from_millis(33),
                        DispatcherPriority::RENDER,
                    );
                    DEMO.with(|d| d.borrow_mut().poll = Some(poll));
                    Ok(())
                }
                .await;
                if current(generation) {
                    match result {
                        Ok(()) => phase("camera-ready"),
                        Err(e) => phase(format!("camera-error:{e}")),
                    }
                }
            });
            false
        },
        Duration::from_millis(25),
        DispatcherPriority::NORMAL,
    );
}
#[wasm_bindgen(js_name=capturePhoto)]
pub fn capture_photo() {
    let (session, generation) = DEMO.with(|d| {
        let d = d.borrow();
        (d.camera.clone(), d.generation)
    });
    if let Some(session) = session {
        let pending = session.photo();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            let result = pending.await;
            if !current(generation) {
                return;
            }
            match result {
                Ok(photo) => {
                    phase(format!(
                        "photo:{}x{}:{:?}",
                        photo.width, photo.height, photo.origin
                    ));
                    DEMO.with(|d| d.borrow_mut().photo = Some(photo));
                }
                Err(e) => phase(format!("photo-error:{e}")),
            }
        });
    }
}
#[wasm_bindgen(js_name=photoBytes)]
pub fn photo_bytes() -> Vec<u8> {
    DEMO.with(|d| {
        d.borrow()
            .photo
            .as_ref()
            .map(|p| p.encoded.to_vec())
            .unwrap_or_default()
    })
}
#[wasm_bindgen(js_name=cameraSettings)]
pub fn camera_settings() -> String {
    DEMO.with(|d| {
        d.borrow()
            .camera
            .as_ref()
            .map(|s| format!("{:?}", s.settings()))
            .unwrap_or_default()
    })
}
