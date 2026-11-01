//! System GStreamer sessions and immutable DMA-BUF ownership. No CPU preview route.
#![cfg(target_os = "linux")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_base::{
    platform::IOptionalFeatureProvider,
    reactive::IDisposable,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_media::{frames::*, *};
#[cfg(feature = "controls")]
pub use ferroui_media_opengl::NativeGlTextureImporter;
use ferroui_media_opengl::{NativeGlFrameProducer, ReadyGlFrame};
use ferroui_opengl::IGlContextExternalObjectsFeature;
pub use native::{NativeFrame, NativeSession, NativeState};
use std::{cell::RefCell, rc::Rc, task::Poll, time::Duration};

fn unsupported(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, message)
}
struct Timer(Rc<dyn IDisposable>);
impl Drop for Timer {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
pub fn poll_native<T: 'static>(
    mut poll: impl FnMut() -> Poll<Result<T, MediaError>> + 'static,
) -> MediaFuture<T> {
    let mut timer = None;
    Box::pin(std::future::poll_fn(move |cx| {
        timer.take();
        match poll() {
            Poll::Ready(v) => Poll::Ready(v),
            Poll::Pending => {
                let w = cx.waker().clone();
                timer = Some(Timer(DispatcherTimer::run_once(
                    move || w.wake_by_ref(),
                    Duration::from_millis(16),
                    DispatcherPriority::NORMAL,
                )));
                Poll::Pending
            }
        }
    }))
}
/// Enumerates the active EGL display's actual DRM format/modifier pairs. These
/// exact caps are used at source negotiation; implicit/unknown layouts are absent.
pub fn admitted_caps(
    producer: &NativeGlFrameProducer,
    size: Option<(u32, u32, u32, u32)>,
) -> MediaFuture<String> {
    producer.invoke(move |context| {
        context
            .native_egl_display()
            .ok_or_else(|| unsupported("Linux media needs an admitted EGL context"))?;
        let features: &dyn IOptionalFeatureProvider = &**context;
        let feature = features
            .try_get::<dyn IGlContextExternalObjectsFeature>()
            .ok_or_else(|| unsupported("EGL DMA-BUF import is unavailable"))?;
        let mut formats = Vec::new();
        for pair in feature.supported_dma_buf_formats().unwrap_or_default() {
            if pair.modifier == 0x00ff_ffff_ffff_ffff {
                continue;
            }
            let name = match pair.format {
                0x3231564e => "NV12",
                0x34325258 => "XR24",
                0x34324258 => "XB24",
                _ => continue,
            };
            if name == "NV12"
                && context.version().type_() != ferroui_opengl::GlProfileType::OpenGLES
            {
                continue;
            }
            let value = if pair.modifier == 0 {
                name.to_owned()
            } else {
                format!("{name}:0x{:016x}", pair.modifier)
            };
            if !formats.contains(&value) {
                formats.push(value);
            }
        }
        if formats.is_empty() {
            return Err(unsupported(
                "No explicitly negotiated native DMA-BUF formats",
            ));
        }
        let mut caps = format!(
            "video/x-raw(memory:DMABuf),format=DMA_DRM,drm-format=(string){{{}}}",
            formats.join(",")
        );
        if let Some((w, h, n, d)) = size {
            if w == 0 || h == 0 || w > 8192 || h > 8192 || n == 0 || d == 0 {
                return Err(unsupported("Invalid camera format"));
            }
            caps.push_str(&format!(
                ",width=(int){w},height=(int){h},framerate=(fraction){n}/{d}"
            ));
        }
        Ok(caps)
    })
}
/// At most one queued import/conversion. A native latest slot plus appsink's one
/// queued sample are separate bounded producer slots; driver pools are opaque.
pub struct LinuxFrameStream {
    pub native: NativeSession,
    producer: NativeGlFrameProducer,
    source: u64,
    generation: u64,
    epoch: Rc<std::cell::Cell<u64>>,
    pending: RefCell<Option<MediaFuture<Option<ReadyGlFrame>>>>,
    ready: RefCell<Option<ReadyGlFrame>>,
    retry: RefCell<Option<NativeFrame>>,
}
impl LinuxFrameStream {
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
        if self.producer.budget().quarantined_reads() != 0 {
            return Err(unsupported(
                "GPU completion is indeterminate; retained storage is quarantined",
            ));
        }
        if self.ready.borrow().is_some() {
            let frame = self.ready.borrow().as_ref().unwrap().take_if_ready()?;
            if frame.is_some() {
                self.ready.borrow_mut().take();
            }
            return Ok(frame);
        }
        // Polling is driven by the presentation tick; no blocking wait on UI.
        let mut pending = self.pending.borrow_mut();
        if let Some(job) = pending.as_mut() {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            match job.as_mut().poll(&mut cx) {
                Poll::Pending => return Ok(None),
                Poll::Ready(result) => {
                    pending.take();
                    match result {
                        Ok(Some(frame)) => {
                            self.retry.borrow_mut().take();
                            self.ready.replace(Some(frame));
                        }
                        Ok(None) => {} // Keep one native sample until budget admission.
                        Err(error) => {
                            self.retry.borrow_mut().take();
                            return Err(error);
                        }
                    }
                    return Ok(None);
                }
            }
        }
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
        let converted = info.format == 0x3231564e;
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
                x: 0,
                y: 0,
                width: info.width,
                height: info.height,
            },
            format: FrameFormat::Rgba8,
            color: if info.color == 1 {
                FrameColor::Srgb
            } else {
                FrameColor::Rec709Full
            },
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: !converted,
            timestamp: Duration::from_nanos(info.timestamp_ns),
            device: Some(self.producer.device()),
            device_generation: self.producer.device_generation(),
            planes: vec![FramePlane {
                offset: 0,
                stride: u64::from(info.width) * 4,
                length: output_bytes,
            }],
            retained_bytes: if converted {
                info.bytes
                    .checked_add(output_bytes)
                    .ok_or_else(|| unsupported("Retained bytes overflow"))?
            } else {
                info.bytes.max(output_bytes)
            },
        };
        let job = if converted {
            let input = frame.clone();
            let counters = self.producer.budget().counters();
            self.producer
                .produce_owned(metadata, frame, move |context, output| {
                    counters.gpu_conversion(GpuTransferReason::PixelFormatConversion);
                    input.convert(context, output)
                })
        } else {
            native::import(&self.producer, metadata, frame)
        };
        *pending = Some(job);
        Ok(None)
    }
}
