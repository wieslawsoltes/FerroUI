//! Explicit portal-authorized PipeWire capture or an opt-in direct V4L2 profile.
//! Discovery does not open a device, request consent, or bypass portal denial.
#![cfg(target_os = "linux")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_media::{frames::*, MediaError, MediaErrorKind};
use ferroui_media_camera::*;
#[cfg(feature = "controls")]
pub use ferroui_media_linux::NativeGlTextureImporter;
use ferroui_media_linux::{admitted_caps, poll_native, LinuxFrameStream, NativeSession};
use ferroui_media_opengl::NativeGlFrameProducer;
use native::PortalAccess;
use std::{
    cell::{Cell, RefCell},
    os::fd::AsRawFd,
    path::PathBuf,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
    time::Duration,
};
#[derive(Clone, Debug, Default)]
pub enum LinuxCameraProfile {
    #[default]
    Portal,
    /// Explicit administrator/application-selected nodes. This never becomes a
    /// fallback for a denied or unavailable portal. No implicit /dev scanning.
    DirectV4l2 { devices: Vec<PathBuf> },
}
pub struct LinuxCameraProvider {
    pub frames: NativeGlFrameProducer,
    pub profile: LinuxCameraProfile,
    permission: Rc<Cell<CameraPermission>>,
}
impl LinuxCameraProvider {
    pub fn new(frames: NativeGlFrameProducer, profile: LinuxCameraProfile) -> Self {
        Self {
            frames,
            profile,
            permission: Rc::new(Cell::new(CameraPermission::NotDetermined)),
        }
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
            _ => CameraErrorKind::NativeFailure,
        },
        e.message,
    )
}
fn device(id: String) -> CameraDevice {
    CameraDevice {
        id,
        label: None,
        facing: CameraFacing::Unknown,
        formats: Vec::new(),
    }
}
impl CameraProvider for LinuxCameraProvider {
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        let profile = self.profile.clone();
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let devices = match profile {
                LinuxCameraProfile::Portal => {
                    if PortalAccess::present() {
                        vec![device("portal:default".into())]
                    } else {
                        Vec::new()
                    }
                }
                LinuxCameraProfile::DirectV4l2 { devices } => devices
                    .into_iter()
                    .filter(|path| path.exists())
                    .filter_map(|path| path.to_str().map(|id| device(id.to_owned())))
                    .collect(),
            };
            let _ = send.send(devices);
        });
        Box::pin(async move {
            poll_native(move || match receive.try_recv() {
                Ok(v) => Poll::Ready(Ok(v)),
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
        let profile = self.profile.clone();
        let frames = self.frames.with_budget(budget.clone());
        let permission = self.permission.clone();
        Box::pin(async move {
            if options
                .selection
                .preferred_facing
                .is_some_and(|f| f != CameraFacing::Unknown)
                && options.selection.exact
            {
                return Err(error(
                    CameraErrorKind::Unsupported,
                    "Native facing metadata is unavailable in this profile",
                ));
            }
            let size = options
                .selection
                .format
                .map(|f| {
                    // Preserve fractional rates to a bounded millisecond rational.
                    let n = (f.frames_per_second * 1000.).round();
                    if !n.is_finite() || n < 1. || n > u32::MAX as f64 {
                        return Err(error(CameraErrorKind::InvalidRequest, "Invalid frame rate"));
                    }
                    Ok((f.width, f.height, n as u32, 1000))
                })
                .transpose()?;
            let caps = admitted_caps(&frames, size).await.map_err(native)?;
            let (path, profile_id, remote, node, id) = match profile {
                LinuxCameraProfile::Portal => {
                    if options
                        .selection
                        .device_id
                        .as_deref()
                        .is_some_and(|id| id != "portal:default")
                    {
                        return Err(error(
                            CameraErrorKind::NoDevice,
                            "Only the portal-authorized default selection is admitted",
                        ));
                    }
                    let access = PortalAccess::begin().map_err(native)?;
                    let result = poll_native(move || access.poll()).await;
                    let (fd, node) = match result {
                        Ok(v) => {
                            permission.set(CameraPermission::Granted);
                            v
                        }
                        Err(e) => {
                            if e.kind == MediaErrorKind::Permission {
                                permission.set(CameraPermission::Denied);
                            }
                            return Err(native(e));
                        }
                    };
                    (String::new(), 1, Some(fd), node, format!("portal:{node}"))
                }
                LinuxCameraProfile::DirectV4l2 { devices } => {
                    let selected = options
                        .selection
                        .device_id
                        .as_ref()
                        .map(PathBuf::from)
                        .or_else(|| devices.first().cloned())
                        .ok_or_else(|| {
                            error(
                                CameraErrorKind::NoDevice,
                                "No explicitly configured V4L2 device",
                            )
                        })?;
                    if !devices.contains(&selected) {
                        return Err(error(
                            CameraErrorKind::NoDevice,
                            "Device is outside the explicit direct profile",
                        ));
                    }
                    let text = selected
                        .to_str()
                        .ok_or_else(|| {
                            error(
                                CameraErrorKind::InvalidRequest,
                                "Native device path must be UTF-8",
                            )
                        })?
                        .to_owned();
                    (text.clone(), 2, None, 0, text)
                }
            };
            let session = NativeSession::open(
                &path,
                &caps,
                profile_id,
                remote.as_ref().map_or(-1, |fd| fd.as_raw_fd()),
                node,
            )
            .map_err(native)?;
            let mut prepared = Some(session);
            let session = poll_native(move || match prepared.as_ref().unwrap().state() {
                Err(e) => Poll::Ready(Err(e)),
                Ok(s) if s.state == 2 && s.width > 0 && s.height > 0 => {
                    Poll::Ready(Ok(prepared.take().unwrap()))
                }
                _ => Poll::Pending,
            })
            .await
            .map_err(native)?;
            let state = session.state().map_err(native)?;
            if state.fps_d == 0 || state.fps_n == 0 {
                return Err(error(
                    CameraErrorKind::Unsupported,
                    "Camera did not report actual negotiated frame rate",
                ));
            }
            let settings = CameraSettings {
                device: device(id),
                format: CameraFormat {
                    width: state.width,
                    height: state.height,
                    frames_per_second: state.fps_n as f64 / state.fps_d as f64,
                },
                native_still: false,
                torch: false,
                zoom: None,
            };
            permission.set(CameraPermission::Granted);
            Ok(Rc::new(Camera {
                stream: RefCell::new(Some(LinuxFrameStream::new(
                    session, frames, source, generation,
                ))),
                settings,
                budget,
            }) as Rc<dyn NativeCameraSession>)
        })
    }
}
struct Camera {
    stream: RefCell<Option<LinuxFrameStream>>,
    settings: CameraSettings,
    budget: FrameBudget,
}
static PHOTO_ACTIVE: AtomicBool = AtomicBool::new(false);
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
                            rotation_degrees: 0,
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
