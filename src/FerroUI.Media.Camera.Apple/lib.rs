//! AVFoundation camera capture, independent of video playback and QR.
#![cfg(target_vendor = "apple")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_media::frames::*;
use ferroui_media_apple::poll_native as wait;
#[cfg(feature = "controls")]
pub use ferroui_media_apple::AppleTextureImporter;
use ferroui_media_camera::*;
use std::{cell::Cell, rc::Rc, task::Poll, time::Duration};
fn duration(seconds: f64) -> Option<Duration> {
    if seconds.is_finite() && seconds >= 0. {
        Duration::try_from_secs_f64(seconds).ok()
    } else {
        None
    }
}
fn invalid() -> CameraError {
    CameraError::new(
        CameraErrorKind::NativeFailure,
        "Invalid native camera metadata",
    )
}
fn number(value: &serde_json::Value, key: &str) -> Result<u32, CameraError> {
    value[key]
        .as_u64()
        .and_then(|n| n.try_into().ok())
        .ok_or_else(invalid)
}
fn frame_rate(value: &serde_json::Value) -> Result<f64, CameraError> {
    value["fps"]
        .as_f64()
        .filter(|rate| rate.is_finite() && *rate > 0.)
        .ok_or_else(invalid)
}
fn parse_device(value: &serde_json::Value) -> Result<CameraDevice, CameraError> {
    let mut formats = Vec::new();
    for f in value["formats"].as_array().ok_or_else(invalid)? {
        formats.push(CameraFormat {
            width: number(f, "width")?,
            height: number(f, "height")?,
            frames_per_second: frame_rate(f)?,
        });
    }
    formats.sort_by(|a, b| {
        (a.width, a.height)
            .cmp(&(b.width, b.height))
            .then_with(|| a.frames_per_second.total_cmp(&b.frames_per_second))
    });
    formats.dedup();
    Ok(CameraDevice {
        id: value["id"].as_str().ok_or_else(invalid)?.into(),
        label: value["label"].as_str().map(String::from),
        facing: match value["facing"].as_u64() {
            Some(1) => CameraFacing::Front,
            Some(2) => CameraFacing::Rear,
            Some(3) => CameraFacing::External,
            _ => CameraFacing::Unknown,
        },
        formats,
    })
}
/// Native camera capture on an already admitted Metal compositor device.
/// The application must declare NSCameraUsageDescription in its bundle. This
/// provider never asks for microphone or photo-library access.
pub struct AppleCameraProvider {
    pub device: DeviceIdentity,
    pub device_generation: u64,
}
impl CameraProvider for AppleCameraProvider {
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        Box::pin(async {
            native::devices()?
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .map(parse_device)
                .collect()
        })
    }
    fn permission(&self) -> CameraPermission {
        native::permission()
    }
    fn start(
        &self,
        options: CameraStartOptions,
        source: u64,
        generation: u64,
        budget: FrameBudget,
    ) -> CameraFuture<Rc<dyn NativeCameraSession>> {
        let device = self.device.clone();
        let device_generation = self.device_generation;
        let mut provisional = Some(native::Camera::new(&options.selection));
        let mut camera = None;
        wait(move || {
            if let Some(initial) = provisional.take() {
                match initial {
                    Ok(c) => camera = Some(c),
                    Err(e) => return Poll::Ready(Err(e)),
                }
            }
            let c = camera.as_ref().unwrap();
            match c.status() {
                Ok(false) => Poll::Pending,
                Err(error) => {
                    camera.take();
                    Poll::Ready(Err(error))
                }
                Ok(true) => {
                    let settings = (|| {
                        let s = c.settings()?;
                        let format = CameraFormat {
                            width: number(&s, "width")?,
                            height: number(&s, "height")?,
                            frames_per_second: frame_rate(&s)?,
                        };
                        if format.width > budget.limits().max_dimension
                            || format.height > budget.limits().max_dimension
                            || u64::from(format.width) * u64::from(format.height) * 4
                                > budget.limits().bytes
                        {
                            return Err(CameraError::new(
                                CameraErrorKind::Unsupported,
                                "Negotiated camera format exceeds the frame budget",
                            ));
                        }
                        Ok(CameraSettings {
                            device: parse_device(&s["device"])?,
                            format,
                            native_still: s["native_still"].as_bool().unwrap_or(false),
                            torch: false,
                            zoom: None,
                        })
                    })();
                    match settings {
                        Err(e) => {
                            camera.take();
                            Poll::Ready(Err(e))
                        }
                        Ok(settings) => Poll::Ready(Ok(Rc::new(AppleCameraSession {
                            camera: camera.take().unwrap(),
                            settings,
                            source,
                            generation,
                            device: device.clone(),
                            device_generation,
                            budget: budget.clone(),
                            sequence: Cell::new(0),
                        })
                            as Rc<dyn NativeCameraSession>)),
                    }
                }
            }
        })
    }
}
struct AppleCameraSession {
    camera: native::Camera,
    settings: CameraSettings,
    source: u64,
    generation: u64,
    device: DeviceIdentity,
    device_generation: u64,
    budget: FrameBudget,
    sequence: Cell<u64>,
}
impl NativeCameraSession for AppleCameraSession {
    fn settings(&self) -> CameraSettings {
        self.settings.clone()
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
        self.camera.status()?;
        if self.budget.quarantined_reads() > 0 {
            return Err(CameraError::new(
                CameraErrorKind::NativeFailure,
                "GPU completion is indeterminate; camera storage remains quarantined",
            ));
        }
        let Some((buffer, timestamp)) = self.camera.frame() else {
            return Ok(None);
        };
        let (width, height, stride, bytes, color) = buffer.info();
        if !matches!(color, 1 | 2) || buffer.surface().is_none() {
            return Err(CameraError::new(
                CameraErrorKind::Unsupported,
                "Camera frame requires known SDR RGB color and IOSurface storage",
            ));
        }
        let sequence = self
            .sequence
            .get()
            .checked_add(1)
            .expect("camera frame identity exhausted");
        self.sequence.set(sequence);
        let metadata = FrameMetadata {
            identity: FrameIdentity {
                source: self.source,
                generation: self.generation,
                sequence,
            },
            width,
            height,
            visible: FrameRegion {
                x: 0,
                y: 0,
                width,
                height,
            },
            format: FrameFormat::Bgra8,
            color: if color == 1 {
                FrameColor::Srgb
            } else {
                FrameColor::Rec709Full
            },
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: true,
            timestamp: duration(timestamp).unwrap_or_default(),
            device: Some(self.device.clone()),
            device_generation: self.device_generation,
            planes: vec![FramePlane {
                offset: 0,
                stride,
                length: stride.checked_mul(height as u64).ok_or_else(invalid)?,
            }],
            retained_bytes: bytes,
        };
        match self.budget.lease(metadata, buffer) {
            Ok(frame) => Ok(Some(frame)),
            Err(FrameError::BudgetExceeded) => Ok(None),
            Err(e) => Err(CameraError::new(
                CameraErrorKind::Unsupported,
                e.to_string(),
            )),
        }
    }
    fn supports_video_recording(&self) -> bool {
        true
    }
    fn start_recording(&self) -> CameraFuture<Rc<dyn NativeCameraRecording>> {
        let recording = self.camera.record().map(|native| {
            Rc::new(AppleCameraRecording(Rc::new(native))) as Rc<dyn NativeCameraRecording>
        });
        Box::pin(async move { recording })
    }
    fn photo(&self) -> CameraFuture<CameraPhoto> {
        if !self.settings.native_still {
            return Box::pin(async {
                Err(CameraError::new(
                    CameraErrorKind::Unsupported,
                    "Native JPEG capture is unavailable",
                ))
            });
        }
        let initial = self.camera.photo();
        let counters = self.budget.counters();
        let mut initial = Some(initial);
        let mut request = None;
        wait(move || {
            if let Some(result) = initial.take() {
                match result {
                    Ok(photo) => request = Some(photo),
                    Err(e) => return Poll::Ready(Err(e)),
                }
            }
            let photo = request.as_ref().unwrap();
            match photo.poll() {
                None => Poll::Pending,
                Some(Err(e)) => Poll::Ready(Err(e)),
                Some(Ok(info)) => {
                    // Only encoded still output is copied. Preview storage is
                    // never mapped/read back by this capture provider.
                    let result = (|| {
                        counters
                            .cpu_transfer(
                                PixelTransferPurpose::Photo,
                                info.length,
                                PresentationRoute::SharedTexture,
                            )
                            .map_err(|_| invalid())?;
                        let encoded = photo.copy(&info)?;
                        let (rotation_degrees, mirrored) = match info.orientation {
                            1 => (0, false),
                            2 => (0, true),
                            3 => (180, false),
                            4 => (180, true),
                            5 => (90, true),
                            6 => (90, false),
                            7 => (270, true),
                            8 => (270, false),
                            _ => return Err(invalid()),
                        };
                        Ok(CameraPhoto {
                            encoded: encoded.into(),
                            content_type: "image/jpeg".into(),
                            width: info.width,
                            height: info.height,
                            timestamp: duration(info.timestamp).unwrap_or_default(),
                            rotation_degrees,
                            mirrored,
                            origin: PhotoOrigin::NativeStill,
                        })
                    })();
                    Poll::Ready(result)
                }
            }
        })
    }
    fn set_control(&self, _: CameraControl) -> Result<(), CameraError> {
        Err(CameraError::new(
            CameraErrorKind::Unsupported,
            "Native camera controls are not exposed by this profile",
        ))
    }
    fn stop(&self) {
        self.camera.stop();
    }
}

struct AppleCameraRecording(Rc<native::Recording>);
impl NativeCameraRecording for AppleCameraRecording {
    fn finish(&self) -> CameraFuture<CameraVideo> {
        self.0.finish();
        let native = self.0.clone();
        wait(move || native.poll())
    }
    fn cancel(&self) {
        self.0.cancel();
    }
}
