//! CameraX capture, consent and still output, independently of video and QR.
#![cfg(target_os = "android")]
#![forbid(unsafe_code)]
use ferroui_android::{interop::java::*, FerroActivity, FerroAndroidApplication};
use ferroui_media::{frames::*, MediaError};
#[cfg(feature = "controls")]
pub use ferroui_media_android::NativeGlTextureImporter;
use ferroui_media_android::{
    poll_native, text, AndroidFrameStream, AndroidSurfaceSource, NativeGlFrameProducer,
};
use ferroui_media_camera::*;
use std::{rc::Rc, task::Poll, time::Duration};
fn error(kind: CameraErrorKind, message: impl Into<String>) -> CameraError {
    CameraError::new(kind, message)
}
fn native(error: MediaError) -> CameraError {
    CameraError::new(
        match error.kind {
            ferroui_media::MediaErrorKind::Unsupported => CameraErrorKind::Unsupported,
            ferroui_media::MediaErrorKind::Cancelled => CameraErrorKind::Cancelled,
            ferroui_media::MediaErrorKind::DeviceLost => CameraErrorKind::DeviceLost,
            _ => CameraErrorKind::NativeFailure,
        },
        error.to_string(),
    )
}
fn class() -> JavaClass {
    JavaClass::find("org/ferroui/media/camera/CameraController")
}
fn json(text: &str) -> Result<serde_json::Value, CameraError> {
    serde_json::from_str(text).map_err(|e| error(CameraErrorKind::NativeFailure, e.to_string()))
}
fn facing(value: u64) -> CameraFacing {
    match value {
        1 => CameraFacing::Front,
        2 => CameraFacing::Rear,
        3 => CameraFacing::External,
        _ => CameraFacing::Unknown,
    }
}
fn dimensions(value: &serde_json::Value) -> CameraFormat {
    CameraFormat {
        width: value["width"].as_u64().unwrap_or(0) as u32,
        height: value["height"].as_u64().unwrap_or(0) as u32,
        frames_per_second: value["fps"].as_f64().unwrap_or(0.),
    }
}
/// Discovery can be used without graphics; starting requires an admitted active
/// compositor context. No CPU fallback is declared by this provider.
pub struct AndroidCameraProvider {
    pub frames: Option<NativeGlFrameProducer>,
    /// Explicit source contract for untagged SDR buffers only. Native color
    /// tags take priority. None rejects missing metadata rather than guessing.
    pub untagged_sdr_color: Option<FrameColor>,
}
/// Narrow native analysis extension. A clone keeps its bridge object alive,
/// while stopping/dropping the originating CameraSession still closes capture.
#[derive(Clone)]
pub struct AndroidCameraContext {
    java: JavaObject,
    source: u64,
    generation: u64,
    counters: FrameTransferCounters,
    untagged_sdr_color: Option<FrameColor>,
}
impl AndroidCameraContext {
    pub fn java(&self) -> JavaObject {
        self.java.clone()
    }
    pub fn source(&self) -> u64 {
        self.source
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn declared_untagged_color(&self) -> Option<FrameColor> {
        self.untagged_sdr_color
    }
    pub fn counters(&self) -> FrameTransferCounters {
        self.counters.clone()
    }
}
impl CameraProvider for AndroidCameraProvider {
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        let result = call_static_object(
            &class(),
            "devices",
            "(Landroid/content/Context;)Ljava/lang/String;",
            &[JavaValue::Object(Some(&FerroAndroidApplication::context()))],
        )
        .map(|v| string_of(&v))
        .ok_or_else(|| error(CameraErrorKind::NativeFailure, "No discovery result"));
        Box::pin(async move {
            let value = json(&result?)?;
            let values = value
                .as_array()
                .ok_or_else(|| error(CameraErrorKind::NativeFailure, "Camera discovery failed"))?;
            Ok(values
                .iter()
                .map(|v| CameraDevice {
                    id: v["id"].as_str().unwrap_or_default().into(),
                    label: None,
                    facing: facing(v["facing"].as_u64().unwrap_or(0)),
                    formats: v["formats"]
                        .as_array()
                        .map(|v| v.iter().map(dimensions).collect())
                        .unwrap_or_default(),
                })
                .collect())
        })
    }
    fn permission(&self) -> CameraPermission {
        match call_static_int(
            &class(),
            "permission",
            "(Landroid/content/Context;)I",
            &[JavaValue::Object(Some(&FerroAndroidApplication::context()))],
        ) {
            1 => CameraPermission::Granted,
            2 => CameraPermission::Denied,
            _ => CameraPermission::NotDetermined,
        }
    }
    fn start(
        &self,
        options: CameraStartOptions,
        id: u64,
        generation: u64,
        budget: FrameBudget,
    ) -> CameraFuture<Rc<dyn NativeCameraSession>> {
        let Some(frames) = self.frames.as_ref().map(|v| v.with_budget(budget.clone())) else {
            return Box::pin(async {
                Err(error(
                    CameraErrorKind::Unsupported,
                    "Camera preview needs an admitted GL context",
                ))
            });
        };
        let Some(activity) =
            FerroActivity::current_main_activity().map(|v| v.java_object().clone())
        else {
            return Box::pin(async {
                Err(error(
                    CameraErrorKind::Disconnected,
                    "No active Android activity",
                ))
            });
        };
        let untagged_sdr_color = self.untagged_sdr_color;
        let color_profile = match untagged_sdr_color {
            None => 0,
            Some(FrameColor::Srgb) => 1,
            Some(FrameColor::Rec709Full) => 2,
            _ => {
                return Box::pin(async {
                    Err(error(
                        CameraErrorKind::Unsupported,
                        "Unsupported declared camera color profile",
                    ))
                })
            }
        };
        let selection = options.selection;
        if selection.exact && selection.format.is_some() {
            return Box::pin(async {
                Err(error(
                    CameraErrorKind::Unsupported,
                    "Exact FPS/size negotiation is not admitted on this CameraX profile",
                ))
            });
        }
        let class = class();
        Box::pin(async move {
            let source = AndroidSurfaceSource::create(&frames)
                .await
                .map_err(native)?;
            let format = selection.format;
            let java = new_object(
                &class,
                "(Landroid/app/Activity;Lorg/ferroui/media/SurfaceFrames;Ljava/lang/String;IIIZI)V",
                &[
                    JavaValue::Object(Some(&activity)),
                    JavaValue::Object(Some(&source.java())),
                    JavaValue::String(selection.device_id.as_deref().unwrap_or("")),
                    JavaValue::Int(match selection.preferred_facing {
                        Some(CameraFacing::Front) => 1,
                        Some(CameraFacing::Rear) => 2,
                        _ => 0,
                    }),
                    JavaValue::Int(format.map(|v| v.width as i32).unwrap_or(0)),
                    JavaValue::Int(format.map(|v| v.height as i32).unwrap_or(0)),
                    JavaValue::Boolean(selection.exact),
                    JavaValue::Int(color_profile),
                ],
            )
            .to_global();
            let mut session = Some(Session {
                context: AndroidCameraContext {
                    java: java.clone(),
                    source: id,
                    generation,
                    counters: budget.counters(),
                    untagged_sdr_color,
                },
                java: java.clone(),
                stream: AndroidFrameStream::new(frames, source, id, generation)
                    .with_capture_clock(java),
                budget,
            });
            poll_native(move || {
                let value = session.as_ref().unwrap();
                if let Err(error) = value.check() {
                    return Poll::Ready(Err(error));
                }
                if !call_boolean(&value.java, "ready", "()Z", &[]) {
                    return Poll::Pending;
                }
                Poll::Ready(Ok(
                    Rc::new(session.take().unwrap()) as Rc<dyn NativeCameraSession>
                ))
            })
            .await
        })
    }
}
struct Session {
    context: AndroidCameraContext,
    java: JavaObject,
    stream: AndroidFrameStream,
    budget: FrameBudget,
}
impl Session {
    fn check(&self) -> Result<(), CameraError> {
        if call_boolean(&self.java, "closed", "()Z", &[]) {
            return Err(error(
                CameraErrorKind::Disconnected,
                "Camera activity/session closed",
            ));
        }
        let message = text(&self.java, "error");
        if message.is_empty() {
            return Ok(());
        }
        Err(error(
            match call_int(&self.java, "errorKind", "()I", &[]) {
                4 => CameraErrorKind::Denied,
                6 => CameraErrorKind::Disconnected,
                7 => CameraErrorKind::Unsupported,
                _ => CameraErrorKind::NativeFailure,
            },
            message,
        ))
    }
    fn info(&self) -> serde_json::Value {
        json(&text(&self.java, "settings")).unwrap_or_default()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}
impl NativeCameraSession for Session {
    fn platform_context(&self) -> Option<&dyn std::any::Any> {
        Some(&self.context)
    }
    fn settings(&self) -> CameraSettings {
        let info = self.info();
        let format = dimensions(&info);
        CameraSettings {
            device: CameraDevice {
                id: info["id"].as_str().unwrap_or_default().into(),
                label: None,
                facing: facing(info["facing"].as_u64().unwrap_or(0)),
                formats: vec![format],
            },
            format,
            native_still: true,
            torch: info["torch"].as_bool().unwrap_or(false),
            zoom: info["minZoom"].as_f64().zip(info["maxZoom"].as_f64()),
        }
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
        self.check()?;
        let info = self.info();
        let size = dimensions(&info);
        let rotation = match info["rotation"].as_u64() {
            Some(90) => FrameRotation::Clockwise90,
            Some(180) => FrameRotation::Clockwise180,
            Some(270) => FrameRotation::Clockwise270,
            _ => FrameRotation::None,
        };
        self.stream
            .take_oriented_frame(
                size.width,
                size.height,
                rotation,
                info["mirrored"].as_bool().unwrap_or(false),
            )
            .map_err(native)
    }
    fn photo(&self) -> CameraFuture<CameraPhoto> {
        let Some(java) = call_object(
            &self.java,
            "capture",
            "()Lorg/ferroui/media/camera/CapturedPhoto;",
            &[],
        )
        .map(|v| v.to_global()) else {
            return Box::pin(async {
                Err(error(
                    CameraErrorKind::NativeFailure,
                    "No native photo request",
                ))
            });
        };
        struct Request {
            java: JavaObject,
        }
        impl Drop for Request {
            fn drop(&mut self) {
                call_void(&self.java, "abandon", "()V", &[]);
            }
        }
        let request = Request { java };
        let counters = self.budget.counters();
        let mut copied: Option<
            std::sync::Arc<std::sync::Mutex<Option<Result<CameraPhoto, CameraError>>>>,
        > = None;
        poll_native(move || {
            if let Some(copied) = copied.as_ref() {
                return copied
                    .lock()
                    .unwrap()
                    .take()
                    .map(Poll::Ready)
                    .unwrap_or(Poll::Pending);
            }
            let java = &request.java;
            if call_boolean(java, "pending", "()Z", &[]) {
                return Poll::Pending;
            }
            let failure = text(java, "error");
            if !failure.is_empty() {
                return Poll::Ready(Err(error(CameraErrorKind::NativeFailure, failure)));
            }
            let length = call_int(java, "length", "()I", &[]);
            if length <= 0 || length > 64 * 1024 * 1024 {
                return Poll::Ready(Err(error(
                    CameraErrorKind::Cancelled,
                    "Photo unavailable or exceeds limit",
                )));
            }
            let info = text(java, "info")
                .split(',')
                .map(|v| v.parse::<u64>().unwrap_or(0))
                .collect::<Vec<_>>();
            let java = java.clone();
            let counters = counters.clone();
            let result = std::sync::Arc::new(std::sync::Mutex::new(None));
            let output = result.clone();
            let spawned = std::thread::Builder::new()
                .name("FerroUI.Media.Camera.Photo".into())
                .spawn(move || {
                    let value = (|| {
                        let Some(bytes) = call_object(&java, "takeBytes", "()[B", &[]) else {
                            return Err(error(CameraErrorKind::Cancelled, "Photo canceled"));
                        };
                        if let Err(e) = counters.cpu_transfer(
                            PixelTransferPurpose::Photo,
                            length as u64,
                            PresentationRoute::SharedTexture,
                        ) {
                            return Err(error(CameraErrorKind::NativeFailure, e.to_string()));
                        }
                        let Some(bytes) = bounded_byte_array_of(&bytes, 64 * 1024 * 1024) else {
                            return Err(error(
                                CameraErrorKind::NativeFailure,
                                "Photo exceeds limit",
                            ));
                        };
                        Ok(CameraPhoto {
                            encoded: bytes.into(),
                            content_type: "image/jpeg".into(),
                            width: info.first().copied().unwrap_or(0) as u32,
                            height: info.get(1).copied().unwrap_or(0) as u32,
                            rotation_degrees: info.get(2).copied().unwrap_or(0) as u16,
                            timestamp: Duration::from_nanos(info.get(3).copied().unwrap_or(0)),
                            mirrored: false,
                            origin: PhotoOrigin::NativeStill,
                        })
                    })();
                    *output.lock().unwrap() = Some(value);
                });
            if let Err(failure) = spawned {
                return Poll::Ready(Err(error(
                    CameraErrorKind::NativeFailure,
                    failure.to_string(),
                )));
            }
            copied = Some(result);
            Poll::Pending
        })
    }
    fn set_control(&self, control: CameraControl) -> Result<(), CameraError> {
        self.check()?;
        let (kind, value) = match control {
            CameraControl::Torch(value) => (0, if value { 1. } else { 0. }),
            CameraControl::Zoom(value) => (1, value as f32),
            _ => {
                return Err(error(
                    CameraErrorKind::Unsupported,
                    "Camera control not implemented",
                ))
            }
        };
        let message = call_object(
            &self.java,
            "control",
            "(IF)Ljava/lang/String;",
            &[JavaValue::Int(kind), JavaValue::Float(value)],
        )
        .map(|v| string_of(&v))
        .unwrap_or_default();
        if message.is_empty() {
            Ok(())
        } else {
            Err(error(CameraErrorKind::Unsupported, message))
        }
    }
    fn stop(&self) {
        self.stream.stop();
        call_void(&self.java, "close", "()V", &[]);
    }
}
