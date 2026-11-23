//! Explicit video-only getUserMedia acquisition and bounded shared GPU preview.
#![forbid(unsafe_code)]
use ferroui_browser::interop::completion_helper::{await_promise, PromiseError};
use ferroui_media::{frames::*, MediaError};
#[cfg(feature = "controls")]
pub use ferroui_media_browser::BrowserTextureImporter;
use ferroui_media_browser::{BrowserFrameStream, BrowserImageFrame, WebGlFrameProducer};
use ferroui_media_camera::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/camera.js")]
extern "C" {
    #[wasm_bindgen(js_name=devices)]
    fn discover() -> JsValue;
    #[wasm_bindgen(js_name=permission)]
    fn consent() -> i32;
    #[wasm_bindgen(catch,js_name=start)]
    fn start_native(options: &str) -> Result<CameraRequest, JsValue>;
    type CameraRequest;
    #[wasm_bindgen(method, getter)]
    fn promise(this: &CameraRequest) -> JsValue;
    #[wasm_bindgen(method)]
    fn commit(this: &CameraRequest);
    #[wasm_bindgen(method)]
    fn close(this: &CameraRequest);
    type BrowserCamera;
    #[wasm_bindgen(method, getter)]
    fn active(this: &BrowserCamera) -> bool;
    #[wasm_bindgen(method, catch)]
    fn settings(this: &BrowserCamera) -> Result<String, JsValue>;
    #[wasm_bindgen(method, catch)]
    fn sample(this: &BrowserCamera) -> Result<Option<NativeImage>, JsValue>;
    #[wasm_bindgen(method)]
    fn photo(this: &BrowserCamera) -> JsValue;
    #[wasm_bindgen(method,js_name=close)]
    fn stop_camera(this: &BrowserCamera);
    type NativeImage;
    #[wasm_bindgen(method, getter)]
    fn image(this: &NativeImage) -> JsValue;
    #[wasm_bindgen(method, getter)]
    fn sequence(this: &NativeImage) -> f64;
    #[wasm_bindgen(method, getter)]
    fn timestamp(this: &NativeImage) -> f64;
    #[wasm_bindgen(method, getter)]
    fn width(this: &NativeImage) -> u32;
    #[wasm_bindgen(method, getter)]
    fn height(this: &NativeImage) -> u32;
    type NativePhoto;
    #[wasm_bindgen(method, getter)]
    fn bytes(this: &NativePhoto) -> Vec<u8>;
    #[wasm_bindgen(method,getter,js_name=length)]
    fn encoded_length(this: &NativePhoto) -> u32;
    #[wasm_bindgen(method,getter,js_name=width)]
    fn photo_width(this: &NativePhoto) -> u32;
    #[wasm_bindgen(method,getter,js_name=height)]
    fn photo_height(this: &NativePhoto) -> u32;
    #[wasm_bindgen(method,getter,js_name=timestamp)]
    fn photo_timestamp(this: &NativePhoto) -> f64;
    #[wasm_bindgen(method, getter)]
    fn origin(this: &NativePhoto) -> String;
}
struct Request(CameraRequest);
impl Drop for Request {
    fn drop(&mut self) {
        self.0.close();
    }
}
struct Camera(BrowserCamera);
impl Drop for Camera {
    fn drop(&mut self) {
        self.0.stop_camera();
    }
}
#[derive(Default)]
pub struct BrowserCameraProvider {
    frames: Option<WebGlFrameProducer>,
}
impl BrowserCameraProvider {
    /// Discovery does not require a compositor, hardware acquisition or consent.
    pub fn discovery() -> Self {
        Self::default()
    }
    pub fn new(frames: WebGlFrameProducer) -> Self {
        Self {
            frames: Some(frames),
        }
    }
}
impl CameraProvider for BrowserCameraProvider {
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        let pending = await_promise(&discover());
        Box::pin(async move {
            let value = pending
                .await
                .map_err(promise_error)?
                .as_string()
                .ok_or_else(|| error(CameraErrorKind::NativeFailure, "Invalid discovery result"))?;
            let values: serde_json::Value = serde_json::from_str(&value)
                .map_err(|e| error(CameraErrorKind::NativeFailure, e.to_string()))?;
            values
                .as_array()
                .ok_or_else(|| error(CameraErrorKind::NativeFailure, "Invalid discovery result"))?
                .iter()
                .map(|d| {
                    Ok(CameraDevice {
                        id: d["id"]
                            .as_str()
                            .ok_or_else(|| {
                                error(CameraErrorKind::NativeFailure, "Missing camera identity")
                            })?
                            .into(),
                        label: d["label"].as_str().map(str::to_owned),
                        facing: CameraFacing::Unknown,
                        formats: Vec::new(),
                    })
                })
                .collect()
        })
    }
    fn permission(&self) -> CameraPermission {
        match consent() {
            1 => CameraPermission::Granted,
            2 => CameraPermission::Denied,
            _ => CameraPermission::NotDetermined,
        }
    }
    fn start(
        &self,
        options: CameraStartOptions,
        source: u64,
        generation: u64,
        budget: FrameBudget,
    ) -> CameraFuture<Rc<dyn NativeCameraSession>> {
        let Some(producer) = self.frames.clone() else {
            return Box::pin(async {
                Err(error(
                    CameraErrorKind::Unsupported,
                    "Camera preview requires an admitted WebGL frame producer",
                ))
            });
        };
        let selection = options.selection;
        let facing = match selection.preferred_facing {
            Some(CameraFacing::Front) => Some("user"),
            Some(CameraFacing::Rear) => Some("environment"),
            _ => None,
        };
        let json=serde_json::json!({"device":selection.device_id,"facing":facing,"exact":selection.exact,"width":selection.format.map(|f|f.width),"height":selection.format.map(|f|f.height),"fps":selection.format.map(|f|f.frames_per_second)}).to_string();
        let request = start_native(&json).map(Request).map_err(js_error);
        let frames = producer
            .with_budget(budget.clone())
            .with_analysis(options.enable_analysis)
            .stream(source, generation);
        Box::pin(async move {
            let request = request?;
            let value = await_promise(&request.0.promise())
                .await
                .map_err(promise_error)?;
            let camera = Camera(value.unchecked_into());
            let settings = parse_settings(&camera.0.settings().map_err(js_error)?)?;
            request.0.commit();
            Ok(Rc::new(Session {
                camera,
                settings,
                frames,
                budget,
                stopped: Cell::new(false),
            }) as Rc<dyn NativeCameraSession>)
        })
    }
}
struct Session {
    camera: Camera,
    settings: CameraSettings,
    frames: BrowserFrameStream,
    budget: FrameBudget,
    stopped: Cell<bool>,
}
impl NativeCameraSession for Session {
    fn settings(&self) -> CameraSettings {
        self.settings.clone()
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
        if self.stopped.get() || !self.camera.0.active() {
            return Err(error(
                CameraErrorKind::Disconnected,
                "The camera track ended",
            ));
        }
        self.frames
            .poll(|| {
                self.camera
                    .0
                    .sample()
                    .map_err(|e| media_error(js_error(e)))?
                    .map(|sample| {
                        Ok(BrowserImageFrame {
                            image: sample.image(),
                            sequence: sample.sequence() as u64,
                            width: sample.width(),
                            height: sample.height(),
                            timestamp: Duration::try_from_secs_f64(sample.timestamp()).map_err(
                                |_| {
                                    media_error(error(
                                        CameraErrorKind::NativeFailure,
                                        "Invalid camera timestamp",
                                    ))
                                },
                            )?,
                        })
                    })
                    .transpose()
            })
            .map_err(|e| error(CameraErrorKind::NativeFailure, e.to_string()))
    }
    fn photo(&self) -> CameraFuture<CameraPhoto> {
        let pending = await_promise(&self.camera.0.photo());
        let counters = self.budget.counters();
        Box::pin(async move {
            let photo: NativePhoto = pending.await.map_err(promise_error)?.unchecked_into();
            let length = photo.encoded_length();
            if length > 64 * 1024 * 1024 {
                return Err(error(
                    CameraErrorKind::NativeFailure,
                    "Native photo exceeds the encoded-output bound",
                ));
            }
            counters
                .cpu_transfer(
                    PixelTransferPurpose::Photo,
                    u64::from(length),
                    PresentationRoute::SharedTexture,
                )
                .map_err(|e| error(CameraErrorKind::NativeFailure, e.to_string()))?;
            let encoded = photo.bytes();
            if encoded.len() != length as usize {
                return Err(error(
                    CameraErrorKind::NativeFailure,
                    "Native photo length changed",
                ));
            }
            Ok(CameraPhoto {
                encoded: encoded.into(),
                content_type: "image/jpeg".into(),
                width: photo.photo_width(),
                height: photo.photo_height(),
                timestamp: Duration::try_from_secs_f64(photo.photo_timestamp()).unwrap_or_default(),
                rotation_degrees: 0,
                mirrored: false,
                origin: if photo.origin() == "native" {
                    PhotoOrigin::NativeStill
                } else {
                    PhotoOrigin::PreviewSnapshot
                },
            })
        })
    }
    fn set_control(&self, _: CameraControl) -> Result<(), CameraError> {
        Err(error(
            CameraErrorKind::Unsupported,
            "This browser profile does not expose synchronous camera controls",
        ))
    }
    fn stop(&self) {
        if !self.stopped.replace(true) {
            self.frames.clear();
            self.camera.0.stop_camera();
        }
    }
}
fn parse_settings(text: &str) -> Result<CameraSettings, CameraError> {
    let s: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| error(CameraErrorKind::NativeFailure, e.to_string()))?;
    let width = s["width"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or_default();
    let height = s["height"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or_default();
    let fps = s["fps"].as_f64().unwrap_or_default();
    if width == 0 || height == 0 || !fps.is_finite() || fps <= 0. {
        return Err(error(
            CameraErrorKind::Unsupported,
            "The browser did not report negotiated dimensions and frame rate",
        ));
    }
    Ok(CameraSettings {
        device: CameraDevice {
            id: s["device"].as_str().unwrap_or_default().into(),
            label: s["label"].as_str().map(str::to_owned),
            facing: match s["facing"].as_str() {
                Some("user") => CameraFacing::Front,
                Some("environment") => CameraFacing::Rear,
                _ => CameraFacing::Unknown,
            },
            formats: Vec::new(),
        },
        format: CameraFormat {
            width,
            height,
            frames_per_second: fps,
        },
        native_still: s["nativeStill"].as_bool().unwrap_or(false),
        torch: false,
        zoom: None,
    })
}
fn error(kind: CameraErrorKind, message: impl Into<String>) -> CameraError {
    CameraError::new(kind, message)
}
fn media_error(error: CameraError) -> MediaError {
    MediaError::new(
        ferroui_media::MediaErrorKind::NativeFailure,
        error.to_string(),
    )
}
fn js_error(value: JsValue) -> CameraError {
    error(
        CameraErrorKind::NativeFailure,
        format!("Browser camera: {value:?}"),
    )
}
fn promise_error(value: PromiseError) -> CameraError {
    error(
        match value.name() {
            "NotAllowedError" => CameraErrorKind::Denied,
            "SecurityError" => CameraErrorKind::Restricted,
            "NotFoundError" => CameraErrorKind::NoDevice,
            "NotReadableError" => CameraErrorKind::Disconnected,
            "InvalidStateError" => CameraErrorKind::Busy,
            "OverconstrainedError" => CameraErrorKind::InvalidRequest,
            "NotSupportedError" => CameraErrorKind::Unsupported,
            "AbortError" => CameraErrorKind::Cancelled,
            _ => CameraErrorKind::NativeFailure,
        },
        value.to_string(),
    )
}
