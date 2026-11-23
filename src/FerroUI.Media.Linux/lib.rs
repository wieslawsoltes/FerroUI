//! System GStreamer sessions and immutable DMA-BUF ownership. No CPU preview route.
#![cfg(target_os = "linux")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_base::{
    platform::IOptionalFeatureProvider,
    reactive::Subscription,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_media::{frames::*, *};
mod dma_buf;
#[cfg(feature = "controls")]
pub use dma_buf::NativeGlTextureImporter;
pub use dma_buf::{DmaBufFrameProducer, LinuxFrameSource};
use ferroui_media_opengl::{NativeGlFrameProducer, ReadyGlFrame};
use ferroui_opengl::IGlContextExternalObjectsFeature;
pub use native::{EncodedRecording, NativeFrame, NativeRecording, NativeSession, NativeState};
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
/// Enumerates the active EGL display's actual DRM format/modifier pairs. These
/// exact caps are used at source negotiation; implicit/unknown layouts are absent.
pub fn admitted_caps(
    source: &LinuxFrameSource,
    size: Option<(u32, u32, u32, u32)>,
) -> MediaFuture<String> {
    let producer = match source {
        LinuxFrameSource::Gl(producer) => producer,
        LinuxFrameSource::DmaBuf(producer) => {
            let caps = drm_caps(producer.formats(), size);
            return Box::pin(std::future::ready(caps));
        }
    };
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
        drm_caps(&formats, size)
    })
}
/// The source caps of exactly `formats`, at the size of a camera when it has one.
fn drm_caps(formats: &[String], size: Option<(u32, u32, u32, u32)>) -> Result<String, MediaError> {
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
}
/// At most one queued import/conversion. A native latest slot plus appsink's one
/// queued sample are separate bounded producer slots; driver pools are opaque.
pub struct LinuxFrameStream {
    pub native: NativeSession,
    /// The GL route; `None` on the DMA-BUF route.
    producer: Option<NativeGlFrameProducer>,
    /// The DMA-BUF route, for a renderer with no context of OpenGL.
    dma_buf: Option<DmaBufFrameProducer>,
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
        producer: impl Into<LinuxFrameSource>,
        source: u64,
        generation: u64,
    ) -> Self {
        let (producer, dma_buf) = match producer.into() {
            LinuxFrameSource::Gl(producer) => (Some(producer), None),
            LinuxFrameSource::DmaBuf(producer) => (None, Some(producer)),
        };
        Self {
            native,
            producer,
            dma_buf,
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
    /// The next frame of the DMA-BUF route: leased once its buffer's implicit fences have
    /// signalled; until then the sample stays the one retried.
    fn take_dma_buf_frame(&self, producer: &DmaBufFrameProducer) -> Result<Option<FrameLease>, MediaError> {
        self.native.state()?;
        if producer.budget.quarantined_reads() != 0 {
            return Err(unsupported(
                "GPU completion is indeterminate; retained storage is quarantined",
            ));
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
            self.retry.borrow_mut().take();
            return Ok(None);
        }
        let (format, plane_count) = match info.format {
            dma_buf::DRM_XR24 => (FrameFormat::Bgra8, 1),
            dma_buf::DRM_XB24 => (FrameFormat::Rgba8, 1),
            dma_buf::DRM_NV12 => (FrameFormat::Nv12, 2),
            _ => return Err(unsupported("The DMA-BUF route takes NV12, XRGB and XBGR frames")),
        };
        if info.plane_count as usize != plane_count {
            return Err(unsupported("The DMA-BUF frame has an unexpected number of planes"));
        }
        if !frame.is_ready() {
            return Ok(None);
        }
        // The last row of a plane need not reach its pitch.
        let (width, height) = (u64::from(info.width), u64::from(info.height));
        let planes = (0..plane_count)
            .map(|plane| {
                let (_, stride, offset) = info.plane(plane);
                let (rows, row_bytes) = match (format, plane) {
                    (FrameFormat::Nv12, 0) => (height, width),
                    (FrameFormat::Nv12, _) => (height.div_ceil(2), width.div_ceil(2) * 2),
                    _ => (height, width * 4),
                };
                FramePlane {
                    offset: u64::from(offset),
                    stride: u64::from(stride),
                    length: (rows - 1) * u64::from(stride) + row_bytes,
                }
            })
            .collect();
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
            format,
            color: match info.color {
                1 => FrameColor::Srgb,
                3 => FrameColor::Rec709Limited,
                _ => FrameColor::Rec709Full,
            },
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: true,
            timestamp: Duration::from_nanos(info.timestamp_ns),
            device: Some(producer.device.clone()),
            device_generation: producer.generation,
            planes,
            retained_bytes: info.bytes,
        };
        match producer.budget.lease(metadata, dma_buf::DmaBufFrame(frame)) {
            Ok(lease) => {
                self.retry.borrow_mut().take();
                Ok(Some(lease))
            }
            // The sample stays until a lease is given back.
            Err(FrameError::BudgetExceeded) => Ok(None),
            Err(error) => {
                self.retry.borrow_mut().take();
                Err(unsupported(error.to_string()))
            }
        }
    }
    pub fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        if let Some(producer) = &self.dma_buf {
            return self.take_dma_buf_frame(producer);
        }
        let Some(producer) = &self.producer else {
            return Ok(None);
        };
        // Report pipeline/capture failure even while an import is pending or
        // leases saturate the budget. A new native take may never become ready.
        self.native.state()?;
        if producer.budget().quarantined_reads() != 0 {
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
            device: Some(producer.device()),
            device_generation: producer.device_generation(),
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
            let counters = producer.budget().counters();
            producer
                .produce_owned(metadata, frame, move |context, output| {
                    counters.gpu_conversion(GpuTransferReason::PixelFormatConversion);
                    input.convert(context, output)
                })
        } else {
            native::import(producer, metadata, frame)
        };
        *pending = Some(job);
        Ok(None)
    }
}
