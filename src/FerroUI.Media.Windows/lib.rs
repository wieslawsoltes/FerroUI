//! Media Foundation native surfaces on the compositor's compatible D3D11 device.
#![cfg(windows)]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_base::{
    reactive::Subscription,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_media::{frames::*, *};
#[cfg(feature = "controls")]
pub use ferroui_media_opengl::NativeGlTextureImporter;
use ferroui_media_opengl::{NativeGlFrameProducer, ReadyGlFrame};
pub use native::{
    EncodedRecording, NativeDevice, NativeFrame, NativeRecording, NativeSession, NativeState,
};

use std::{cell::RefCell, rc::Rc, task::Poll, time::Duration};

fn unsupported(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, message)
}
pub fn poll_native<T: 'static>(
    mut poll: impl FnMut() -> Poll<Result<T, MediaError>> + 'static,
) -> MediaFuture<T> {
    let mut timer: Option<Subscription> = None;
    Box::pin(std::future::poll_fn(move |cx| {
        timer.take();
        match poll() {
            Poll::Ready(v) => Poll::Ready(v),
            Poll::Pending => {
                let w = cx.waker().clone();
                timer = Some(DispatcherTimer::run_once(
                    move || w.wake_by_ref(),
                    Duration::from_millis(16),
                    DispatcherPriority::NORMAL,
                ));
                Poll::Pending
            }
        }
    }))
}
/// At most one queued import. The native worker separately bounds three pending
/// video samples, one latest sample and one read callback; driver pools are opaque.
pub struct WindowsFrameStream {
    pub native: NativeSession,
    producer: NativeGlFrameProducer,
    source: u64,
    generation: u64,
    epoch: Rc<std::cell::Cell<u64>>,
    pending: RefCell<Option<MediaFuture<Option<ReadyGlFrame>>>>,
    ready: RefCell<Option<ReadyGlFrame>>,
    retry: RefCell<Option<NativeFrame>>,
}
impl WindowsFrameStream {
    pub fn new(
        native: NativeSession,
        producer: NativeGlFrameProducer,
        source: u64,
        generation: u64,
    ) -> Self {
        Self {
            native,
            producer,
            source,
            generation,
            epoch: Rc::new(std::cell::Cell::new(1)),
            pending: RefCell::new(None),
            ready: RefCell::new(None),
            retry: RefCell::new(None),
        }
    }
    pub fn retire(&self, epoch: u64) {
        self.epoch.set(epoch);
        self.pending.borrow_mut().take();
        self.ready.borrow_mut().take();
        self.retry.borrow_mut().take();
    }
    pub fn has_pending_frame_work(&self) -> bool {
        self.pending.borrow().is_some()
            || self.ready.borrow().is_some()
            || self.retry.borrow().is_some()
    }
    pub fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        // Capture/device errors must interrupt even a saturated or pending
        // import; waiting for another native take can hide a disconnection.
        self.native.state()?;
        if self.producer.budget().quarantined_reads() != 0 {
            return Err(unsupported(
                "GPU completion is indeterminate; retained storage is quarantined",
            ));
        }
        let mut pending = self.pending.borrow_mut();
        if let Some(job) = pending.as_mut() {
            let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
            match job.as_mut().poll(&mut cx) {
                Poll::Pending => return Ok(None),
                Poll::Ready(result) => {
                    pending.take();
                    match result {
                        Ok(Some(frame)) => {
                            // Count the requested MF RGB-processing boundary
                            // once per admitted sample, including after retries.
                            self.producer
                                .budget()
                                .counters()
                                .gpu_conversion(GpuTransferReason::PixelFormatConversion);
                            self.retry.borrow_mut().take();
                            self.ready.replace(Some(frame));
                        }
                        Ok(None) => {} // Keep one native sample until budget admission.
                        Err(error) => {
                            self.retry.borrow_mut().take();
                            return Err(error);
                        }
                    }
                }
            }
        }
        // Deliver a completed import on this tick and start its successor
        // without waiting for another tick. A pending fence still owns the
        // sole import slot until it completes.
        let frame = {
            let mut ready = self.ready.borrow_mut();
            match ready.as_ref() {
                Some(imported) => {
                    let Some(frame) = imported.take_if_ready()? else {
                        return Ok(None);
                    };
                    ready.take();
                    Some(frame)
                }
                None => None,
            }
        };
        *pending = self.queue_import()?;
        Ok(frame)
    }

    fn queue_import(&self) -> Result<Option<MediaFuture<Option<ReadyGlFrame>>>, MediaError> {
        if let Some(frame) = self.native.take_frame()? {
            if frame.info().epoch == self.epoch.get() {
                self.retry.replace(Some(frame));
            }
        }
        let Some(frame) = self.retry.borrow().clone() else {
            return Ok(None);
        };
        let info = frame.info();
        if info.epoch != self.epoch.get() {
            return Ok(None);
        }
        let output_bytes = u64::from(info.width) * u64::from(info.height) * 4;
        let metadata = FrameMetadata {
            identity: FrameIdentity {
                source: self.source,
                generation: self.generation,
                sequence: info.sequence,
            },
            width: info.width,
            height: info.height,
            visible: FrameRegion {
                x: info.x,
                y: info.y,
                width: info.visible_width,
                height: info.visible_height,
            },
            format: FrameFormat::Rgba8,
            color: if info.color == 1 {
                FrameColor::Srgb
            } else {
                FrameColor::Rec709Full
            },
            alpha: FrameAlpha::Opaque,
            rotation: match info.rotation {
                0 => FrameRotation::None,
                90 => FrameRotation::Clockwise90,
                180 => FrameRotation::Clockwise180,
                270 => FrameRotation::Clockwise270,
                _ => return Err(unsupported("Unknown native frame rotation")),
            },
            mirrored: info.mirrored != 0,
            top_left_origin: true,
            timestamp: Duration::from_nanos(info.timestamp_ns),
            device: Some(self.producer.device()),
            device_generation: self.producer.device_generation(),
            planes: vec![FramePlane {
                offset: 0,
                stride: u64::from(info.width) * 4,
                length: output_bytes,
            }],
            retained_bytes: info.bytes,
        };
        // The requested native MF video processor supplies RGB DXGI storage.
        // No framework video copy/readback is used by this import.
        Ok(Some(native::import(&self.producer, metadata, frame)))
    }
}
