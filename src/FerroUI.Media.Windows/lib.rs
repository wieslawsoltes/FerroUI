//! Media Foundation native surfaces on the compositor's compatible D3D11 device.
#![cfg(windows)]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
mod frame_producer;
pub use frame_producer::WindowsFrameProducer;
use frame_producer::ReadyWindowsFrame;
use ferroui_base::{
    reactive::Subscription,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_media::{frames::*, *};
#[cfg(feature = "controls")]
pub use ferroui_media_opengl::NativeGlTextureImporter;
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
    producer: WindowsFrameProducer,
    source: u64,
    generation: u64,
    epoch: Rc<std::cell::Cell<u64>>,
    pending: RefCell<Option<MediaFuture<Option<ReadyWindowsFrame>>>>,
    ready: RefCell<Option<ReadyWindowsFrame>>,
    retry: RefCell<Option<NativeFrame>>,
}

// Drain immediately completed stages in this tick. A real asynchronous import
// or producer fence can defer delivery; merely moving between Rust states must
// not consume another presentation interval.
fn poll_frame_import(
    pending: &mut Option<MediaFuture<Option<ReadyWindowsFrame>>>,
    ready: &mut Option<ReadyWindowsFrame>,
    admitted: impl FnOnce(),
) -> Result<Option<FrameLease>, MediaError> {
    if let Some(job) = pending.as_mut() {
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match job.as_mut().poll(&mut cx) {
            Poll::Pending => return Ok(None),
            Poll::Ready(result) => {
                pending.take();
                *ready = result?;
                if ready.is_some() { admitted(); }
            }
        }
    }
    let frame = match ready.as_ref() {
        Some(frame) => frame.take_if_ready()?,
        None => None,
    };
    if frame.is_some() { ready.take(); }
    Ok(frame)
}

impl WindowsFrameStream {
    pub fn new(
        native: NativeSession,
        producer: impl Into<WindowsFrameProducer>,
        source: u64,
        generation: u64,
    ) -> Self {
        Self {
            native,
            producer: producer.into(),
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
    fn poll_import(&self) -> Result<Option<FrameLease>, MediaError> {
        let result = poll_frame_import(
            &mut self.pending.borrow_mut(),
            &mut self.ready.borrow_mut(),
            || {
                self.producer.budget().counters()
                    .gpu_conversion(GpuTransferReason::PixelFormatConversion);
                self.retry.borrow_mut().take();
            },
        );
        if result.is_err() { self.retry.borrow_mut().take(); }
        result
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
        // Deliver a completed import on this tick and start its successor without waiting for
        // another tick. A pending import or producer fence still owns the sole import slot until
        // it completes.
        let frame = self.poll_import()?;
        if self.pending.borrow().is_some() || self.ready.borrow().is_some() {
            return Ok(frame);
        }
        let job = self.queue_import()?;
        *self.pending.borrow_mut() = job;
        if frame.is_none() {
            // An import that completes synchronously is delivered on the tick that queued it.
            return self.poll_import();
        }
        Ok(frame)
    }

    fn queue_import(&self) -> Result<Option<MediaFuture<Option<ReadyWindowsFrame>>>, MediaError> {
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
            format: self.producer.format(),
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
        Ok(Some(self.producer.import(metadata, frame)))
    }
}

#[cfg(all(test, feature = "direct2d"))]
mod import_pacing_tests {
    use super::*;
    use std::cell::Cell;

    fn frame(budget: &FrameBudget) -> ReadyWindowsFrame {
        let metadata = FrameMetadata {
            identity: FrameIdentity { source: 1, generation: 1, sequence: 1 },
            width: 2, height: 2,
            visible: FrameRegion { x: 0, y: 0, width: 2, height: 2 },
            format: FrameFormat::Bgra8, color: FrameColor::Srgb, alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None, mirrored: false, top_left_origin: true,
            timestamp: Duration::ZERO, device: Some(DeviceIdentity::Context(1)), device_generation: 1,
            planes: vec![FramePlane { offset: 0, stride: 8, length: 16 }], retained_bytes: 16,
        };
        ReadyWindowsFrame::Direct3D11(budget.lease(metadata, ()).unwrap())
    }

    #[test]
    fn synchronous_import_is_delivered_in_the_same_tick_and_keeps_its_lease() {
        let budget = FrameBudget::new(Default::default());
        let frame = frame(&budget);
        let mut pending: Option<MediaFuture<_>> = Some(Box::pin(async move { Ok(Some(frame)) }));
        let mut ready = None;
        let admissions = Cell::new(0);
        let delivered = poll_frame_import(&mut pending, &mut ready, || admissions.set(admissions.get() + 1)).unwrap().unwrap();
        assert_eq!(admissions.get(), 1);
        assert!(pending.is_none() && ready.is_none());
        assert_eq!(budget.usage().frames, 1);
        drop(delivered);
        assert_eq!(budget.usage().frames, 0);
        assert!(poll_frame_import(&mut pending, &mut ready, || panic!("duplicate admission")).unwrap().is_none());
    }

    #[test]
    fn pending_import_delivers_on_its_first_ready_poll_without_an_extra_tick() {
        let budget = FrameBudget::new(Default::default());
        let available = Rc::new(Cell::new(false));
        let signal = available.clone();
        let mut frame = Some(frame(&budget));
        let mut pending: Option<MediaFuture<_>> = Some(Box::pin(std::future::poll_fn(move |_| {
            if signal.get() { Poll::Ready(Ok(frame.take())) } else { Poll::Pending }
        })));
        let mut ready = None;
        assert!(poll_frame_import(&mut pending, &mut ready, || panic!("not admitted yet")).unwrap().is_none());
        assert!(pending.is_some());
        assert_eq!(budget.usage().frames, 1);
        available.set(true);
        let delivered = poll_frame_import(&mut pending, &mut ready, || {}).unwrap().unwrap();
        assert!(pending.is_none() && ready.is_none());
        drop(delivered);
        assert_eq!(budget.usage().frames, 0);
    }

    #[test]
    fn budget_backpressure_does_not_report_a_successful_import() {
        let mut pending: Option<MediaFuture<Option<ReadyWindowsFrame>>> = Some(Box::pin(async { Ok(None) }));
        let mut ready = None;
        assert!(poll_frame_import(&mut pending, &mut ready, || panic!("budget rejection is not admission")).unwrap().is_none());
        assert!(pending.is_none() && ready.is_none());
    }

    #[test]
    #[ignore = "Requires hardware D3D11, Media Foundation and an audio endpoint"]
    fn native_video_stream_delivery_cadence() {
        use std::time::Instant;
        use ferroui_base::platform::*;
        use ferroui_base::{PixelSize, Rect, Vector};
        let renderer = ferroui_direct2d::Direct2DRenderInterface;
        let context = renderer.create_backend_context(None);
        let feature = context.try_get_feature(std::any::TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>()).unwrap();
        let feature = feature.downcast_ref::<Rc<dyn IExternalObjectsRenderInterfaceContextFeature>>().unwrap().clone();
        let target = renderer.create_render_target_bitmap(PixelSize::new(32, 32), Vector::new(96., 96.));
        let device = NativeDevice::from_direct2d().unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../samples/ControlCatalog/Media/sintel-sample.mp4").canonicalize().unwrap();
        let path = path.to_string_lossy();
        let native = NativeSession::open(&device, path.trim_start_matches("\\\\?\\"), false, false, None).unwrap();
        let budget = FrameBudget::new(Default::default());
        let producer = WindowsFrameProducer::Direct3D11 {
            budget: budget.clone(), device: DeviceIdentity::Luid(ferroui_direct2d::device_luid()),
            generation: ferroui_direct2d::device_generation(),
        };
        let stream = WindowsFrameStream::new(native, producer, 1, 1);
        let preparing = Instant::now();
        while stream.native.state().unwrap().state != 1 {
            assert!(preparing.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(5));
        }
        stream.native.intent(1);
        let start = Instant::now();
        let mut count = 0;
        let mut first_pts = None;
        let mut last_pts = Duration::ZERO;
        while start.elapsed() < Duration::from_secs(5) {
            if let Some(frame) = stream.take_frame().unwrap() {
                count += 1;
                first_pts.get_or_insert(frame.metadata().timestamp);
                last_pts = frame.metadata().timestamp;
                let m = frame.metadata();
                let size = (m.width, m.height);
                let handle = LeasedPlatformHandle::new(0, Some("SharedFrameD3D11Texture2D"), frame.clone());
                let image = feature.import_image(Rc::new(handle), PlatformGraphicsExternalImageProperties {
                    width: m.width as i32, height: m.height as i32,
                    format: PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm,
                    top_left_origin: true, memory_size: m.retained_bytes, ..Default::default()
                });
                let bitmap = image.acquire_shared_bitmap(frame.clone()).unwrap();
                // Match preview ownership: the acquired bitmap must survive
                // disposal of the temporary import and the caller's lease.
                image.dispose();
                drop(image);
                drop(frame);
                let mut drawing = target.create_drawing_context();
                drawing.draw_bitmap(&*bitmap, 1., Rect::new(0., 0., size.0 as f64, size.1 as f64), Rect::new(0., 0., 32., 32.));
                drawing.dispose();
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let fps = count as f64 / start.elapsed().as_secs_f64();
        println!("NATIVE_STREAM_DELIVERY frames={count} fps={fps:.2} pts_span_ms={}",
            (last_pts - first_pts.unwrap()).as_millis());
        assert!(fps >= 21.6, "24 fps source was undersampled: {fps:.2}");
        stream.native.intent(0);
        drop(stream);
        let retiring = Instant::now();
        while budget.usage().frames != 0 && retiring.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(budget.usage().frames, 0);
    }
}
