//! Media Foundation camera acquisition; metadata-only discovery, no microphone.
#![cfg(windows)]
#![forbid(unsafe_code)]
use ferroui_media::{frames::*, MediaError, MediaErrorKind};
use ferroui_media_camera::*;
use ferroui_media_opengl::NativeGlFrameProducer;
#[cfg(feature = "controls")]
pub use ferroui_media_windows::NativeGlTextureImporter;
use ferroui_media_windows::{poll_native, NativeDevice, NativeSession, WindowsFrameStream};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
    time::Duration,
};
pub struct WindowsCameraProvider {
    pub frames: NativeGlFrameProducer,
    permission: Rc<Cell<CameraPermission>>,
    #[cfg(feature = "synthetic-camera")]
    synthetic: bool,
}
impl WindowsCameraProvider {
    pub fn new(frames: NativeGlFrameProducer) -> Self {
        Self {
            frames,
            permission: Rc::new(Cell::new(CameraPermission::NotDetermined)),
            #[cfg(feature = "synthetic-camera")]
            synthetic: false,
        }
    }
    /// Explicit in-process MF/D3D11 test source. It is not an OS device or a
    /// fallback, and its permission state does not describe Windows privacy.
    #[cfg(feature = "synthetic-camera")]
    pub fn synthetic(frames: NativeGlFrameProducer) -> Self {
        Self {
            synthetic: true,
            ..Self::new(frames)
        }
    }
    fn discover(&self) -> CameraFuture<Vec<CameraDevice>> {
        #[cfg(feature = "synthetic-camera")]
        if self.synthetic {
            return Box::pin(async {
                Ok([
                    "ferroui:synthetic-camera",
                    "ferroui:synthetic-camera/disconnect",
                    "ferroui:synthetic-camera/recovery",
                ]
                .into_iter()
                .map(|id| CameraDevice {
                    id: id.into(),
                    label: Some(format!("Synthetic MF GPU camera (in process): {id}")),
                    facing: CameraFacing::Unknown,
                    formats: vec![CameraFormat {
                        width: 320,
                        height: 240,
                        frames_per_second: 30000. / 1001.,
                    }],
                })
                .collect())
            });
        }
        Box::pin(async { discovery().await })
    }
}
fn error(kind: CameraErrorKind, s: impl Into<String>) -> CameraError {
    CameraError::new(kind, s)
}
fn native(e: MediaError) -> CameraError {
    error(
        match e.kind {
            MediaErrorKind::Permission => CameraErrorKind::Denied,
            MediaErrorKind::Unsupported => CameraErrorKind::Unsupported,
            MediaErrorKind::Cancelled => CameraErrorKind::Cancelled,
            MediaErrorKind::DeviceLost => CameraErrorKind::DeviceLost,
            _ => CameraErrorKind::NativeFailure,
        },
        e.message,
    )
}
fn discovery() -> CameraFuture<Vec<CameraDevice>> {
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = NativeSession::devices().and_then(|text| {
            let values: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| MediaError::new(MediaErrorKind::NativeFailure, e.to_string()))?;
            Ok(values
                .as_array()
                .ok_or_else(|| {
                    MediaError::new(
                        MediaErrorKind::NativeFailure,
                        "Invalid native discovery response",
                    )
                })?
                .iter()
                .map(|v| CameraDevice {
                    id: v["id"].as_str().unwrap_or_default().into(),
                    label: v["label"].as_str().map(str::to_owned),
                    facing: CameraFacing::Unknown,
                    formats: Vec::new(),
                })
                .collect())
        });
        let _ = send.send(result);
    });
    Box::pin(async move {
        poll_native(move || match receive.try_recv() {
            Ok(v) => Poll::Ready(v),
            Err(std::sync::mpsc::TryRecvError::Empty) => Poll::Pending,
            Err(_) => Poll::Ready(Err(MediaError::new(
                MediaErrorKind::NativeFailure,
                "Camera discovery worker failed",
            ))),
        })
        .await
        .map_err(native)
    })
}
impl CameraProvider for WindowsCameraProvider {
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        self.discover()
    }
    fn permission(&self) -> CameraPermission {
        self.permission.get()
    }
    fn start(
        &self,
        options: CameraStartOptions,
        source: u64,
        generation: u64,
        budget: FrameBudget,
    ) -> CameraFuture<Rc<dyn NativeCameraSession>> {
        let frames = self.frames.with_budget(budget.clone());
        let permission = self.permission.clone();
        let devices = self.discover();
        Box::pin(async move {
            if options.selection.exact
                && options
                    .selection
                    .preferred_facing
                    .is_some_and(|f| f != CameraFacing::Unknown)
            {
                return Err(error(
                    CameraErrorKind::Unsupported,
                    "This native camera profile has no facing metadata",
                ));
            }
            let devices = devices.await?;
            let device = devices
                .into_iter()
                .find(|d| {
                    options
                        .selection
                        .device_id
                        .as_ref()
                        .is_none_or(|id| id == &d.id)
                })
                .ok_or_else(|| {
                    error(CameraErrorKind::NoDevice, "Selected camera is unavailable")
                })?;
            let size = options
                .selection
                .format
                .map(|f| {
                    let n = (f.frames_per_second * 1000.).round();
                    if !n.is_finite() || n < 1. || n > u32::MAX as f64 {
                        return Err(error(CameraErrorKind::InvalidRequest, "Invalid frame rate"));
                    }
                    Ok((f.width, f.height, n as u32, 1000))
                })
                .transpose()?;
            let d3d = NativeDevice::from_renderer(&frames).await.map_err(native)?;
            let session =
                NativeSession::open(&d3d, &device.id, true, false, size).map_err(native)?;
            let mut prepared = Some(session);
            let result = poll_native(move || match prepared.as_ref().unwrap().state() {
                Err(e) => Poll::Ready(Err(e)),
                Ok(s) if s.state == 2 && s.width > 0 && s.height > 0 => {
                    Poll::Ready(Ok(prepared.take().unwrap()))
                }
                _ => Poll::Pending,
            })
            .await;
            let session = match result {
                Ok(v) => v,
                Err(e) => {
                    if e.kind == MediaErrorKind::Permission {
                        permission.set(CameraPermission::Denied);
                    }
                    return Err(native(e));
                }
            };
            let s = session.state().map_err(native)?;
            if s.fps_n == 0 || s.fps_d == 0 {
                return Err(error(
                    CameraErrorKind::Unsupported,
                    "Native camera did not report negotiated frame rate",
                ));
            }
            permission.set(CameraPermission::Granted);
            let settings = CameraSettings {
                device,
                format: CameraFormat {
                    width: s.width,
                    height: s.height,
                    frames_per_second: s.fps_n as f64 / s.fps_d as f64,
                },
                native_still: false,
                torch: false,
                zoom: None,
            };
            Ok(Rc::new(Camera {
                stream: RefCell::new(Some(WindowsFrameStream::new(
                    session, frames, source, generation,
                ))),
                settings,
                budget,
            }) as Rc<dyn NativeCameraSession>)
        })
    }
}
struct Camera {
    stream: RefCell<Option<WindowsFrameStream>>,
    settings: CameraSettings,
    budget: FrameBudget,
}
static PHOTO_ACTIVE: AtomicBool = AtomicBool::new(false);
#[cfg(feature = "synthetic-camera")]
static PHOTO_JOBS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Diagnostic for checking that a canceled fixture photo reached native work.
#[cfg(feature = "synthetic-camera")]
pub fn synthetic_photo_jobs_started() -> u64 {
    PHOTO_JOBS.load(Ordering::Acquire)
}
struct PhotoSlot;
impl Drop for PhotoSlot {
    fn drop(&mut self) {
        PHOTO_ACTIVE.store(false, Ordering::Release);
    }
}
impl NativeCameraSession for Camera {
    fn settings(&self) -> CameraSettings {
        let mut settings = self.settings.clone();
        if let Some(s) = self
            .stream
            .borrow()
            .as_ref()
            .and_then(|s| s.native.state().ok())
            .filter(|s| s.width > 0 && s.height > 0 && s.fps_d > 0 && s.fps_n > 0)
        {
            settings.format = CameraFormat {
                width: s.width,
                height: s.height,
                frames_per_second: s.fps_n as f64 / s.fps_d as f64,
            };
        }
        settings
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
        self.stream
            .borrow()
            .as_ref()
            .ok_or_else(|| error(CameraErrorKind::Cancelled, "Camera stopped"))?
            .take_frame()
            .map_err(native)
    }
    fn photo(&self) -> CameraFuture<CameraPhoto> {
        let frame = self
            .stream
            .borrow()
            .as_ref()
            .ok_or_else(|| error(CameraErrorKind::Cancelled, "Camera stopped"))
            .and_then(|s| {
                s.native.take_frame().map_err(native)?.ok_or_else(|| {
                    error(
                        CameraErrorKind::Busy,
                        "No new native preview sample for snapshot",
                    )
                })
            });
        let counters = self.budget.counters();
        let prepared = frame.and_then(|frame| {
            PHOTO_ACTIVE
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .map_err(|_| error(CameraErrorKind::Busy, "One native photo may be pending"))?;
            Ok((frame, PhotoSlot))
        });
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        match prepared {
            Err(e) => return Box::pin(async move { Err(e) }),
            Ok((frame, slot)) => {
                #[cfg(feature = "synthetic-camera")]
                PHOTO_JOBS.fetch_add(1, Ordering::Release);
                std::thread::spawn(move || {
                    let _slot = slot;
                    let info = frame.info();
                    let result = (|| {
                        let rgb = frame.photo_rgb(&counters).map_err(native)?;
                        let mut encoded = Vec::new();
                        jpeg_encoder::Encoder::new(&mut encoded, 90)
                            .encode(
                                &rgb,
                                info.width as u16,
                                info.height as u16,
                                jpeg_encoder::ColorType::Rgb,
                            )
                            .map_err(|e| error(CameraErrorKind::NativeFailure, e.to_string()))?;
                        if encoded.len() > 64 * 1024 * 1024 {
                            return Err(error(
                                CameraErrorKind::Unsupported,
                                "Encoded photo exceeds 64 MiB",
                            ));
                        }
                        counters
                            .cpu_transfer(
                                PixelTransferPurpose::Photo,
                                encoded.len() as u64,
                                PresentationRoute::SharedTexture,
                            )
                            .map_err(|e| error(CameraErrorKind::NativeFailure, e.to_string()))?;
                        Ok(CameraPhoto {
                            encoded: Arc::from(encoded),
                            content_type: "image/jpeg".into(),
                            width: info.width,
                            height: info.height,
                            timestamp: Duration::from_nanos(info.timestamp_ns),
                            rotation_degrees: info.rotation as u16,
                            mirrored: false,
                            origin: PhotoOrigin::PreviewSnapshot,
                        })
                    })();
                    let _ = send.send(result);
                });
            }
        }
        Box::pin(async move {
            poll_native(move || match receive.try_recv() {
                Ok(v) => Poll::Ready(Ok(v)),
                Err(std::sync::mpsc::TryRecvError::Empty) => Poll::Pending,
                Err(_) => Poll::Ready(Err(MediaError::new(
                    MediaErrorKind::NativeFailure,
                    "Native photo worker failed",
                ))),
            })
            .await
            .map_err(native)?
        })
    }
    fn set_control(&self, _: CameraControl) -> Result<(), CameraError> {
        Err(error(
            CameraErrorKind::Unsupported,
            "Camera controls are unavailable in this capture profile",
        ))
    }
    fn stop(&self) {
        self.stream.borrow_mut().take();
    }
}
impl Drop for Camera {
    fn drop(&mut self) {
        self.stream.get_mut().take();
    }
}
