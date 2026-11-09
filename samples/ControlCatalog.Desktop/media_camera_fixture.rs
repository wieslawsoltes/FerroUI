//! Explicit development-only camera provider. Exercises the real CameraPreview,
//! mailbox, IOSurface importer, compositor and GPU completion without hardware.
use ferroui_base::rendering::composition::frames::*;
use ferroui_media_camera::*;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
pub struct Provider {
    pub device: DeviceIdentity,
    pub generation: u64,
}
fn camera() -> CameraDevice {
    CameraDevice {
        id: "quartz-gpu-fixture".into(),
        label: Some("GPU IOSurface test camera (synthetic)".into()),
        facing: CameraFacing::External,
        formats: vec![CameraFormat {
            width: 640,
            height: 480,
            frames_per_second: 30.,
        }],
    }
}
impl CameraProvider for Provider {
    fn devices(&self) -> CameraFuture<Vec<CameraDevice>> {
        Box::pin(async { Ok(vec![camera()]) })
    }
    fn permission(&self) -> CameraPermission {
        CameraPermission::Granted
    }
    fn start(
        &self,
        _: CameraStartOptions,
        source: u64,
        generation: u64,
        budget: FrameBudget,
    ) -> CameraFuture<Rc<dyn NativeCameraSession>> {
        let session = Session {
            device: self.device.clone(),
            device_generation: self.generation,
            source,
            generation,
            budget,
            stopped: Cell::new(false),
            sequence: Cell::new(0),
            start: Instant::now(),
        };
        Box::pin(async move { Ok(Rc::new(session) as Rc<dyn NativeCameraSession>) })
    }
}
struct Session {
    device: DeviceIdentity,
    device_generation: u64,
    source: u64,
    generation: u64,
    budget: FrameBudget,
    stopped: Cell<bool>,
    sequence: Cell<u64>,
    start: Instant,
}
fn unsupported() -> CameraError {
    CameraError::new(
        CameraErrorKind::Unsupported,
        "Synthetic GPU camera supports preview only",
    )
}
impl NativeCameraSession for Session {
    fn settings(&self) -> CameraSettings {
        let device = camera();
        CameraSettings {
            format: device.formats[0],
            device,
            native_still: false,
            torch: false,
            zoom: None,
        }
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, CameraError> {
        if self.stopped.get() || self.budget.usage().frames >= self.budget.limits().frames {
            return Ok(None);
        }
        let sequence = (self.start.elapsed().as_secs_f64() * 30.) as u64 + 1;
        if sequence == self.sequence.get() {
            return Ok(None);
        }
        let buffer = ferroui_quartz::create_media_test_buffer(sequence).ok_or_else(|| {
            CameraError::new(
                CameraErrorKind::NativeFailure,
                "GPU fixture allocation failed",
            )
        })?;
        self.sequence.set(sequence);
        let (width, height, stride, bytes, _) = buffer.info();
        let frame = self
            .budget
            .lease(
                FrameMetadata {
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
                    color: FrameColor::Srgb,
                    alpha: FrameAlpha::Opaque,
                    rotation: FrameRotation::None,
                    mirrored: false,
                    top_left_origin: true,
                    timestamp: Duration::from_secs_f64((sequence - 1) as f64 / 30.),
                    device: Some(self.device.clone()),
                    device_generation: self.device_generation,
                    planes: vec![FramePlane {
                        offset: 0,
                        stride,
                        length: stride * height as u64,
                    }],
                    retained_bytes: bytes,
                },
                buffer,
            )
            .map_err(|e| CameraError::new(CameraErrorKind::NativeFailure, e.to_string()))?;
        Ok(Some(frame))
    }
    fn photo(&self) -> CameraFuture<CameraPhoto> {
        Box::pin(async { Err(unsupported()) })
    }
    fn set_control(&self, _: CameraControl) -> Result<(), CameraError> {
        Err(unsupported())
    }
    fn stop(&self) {
        self.stopped.set(true);
    }
}
