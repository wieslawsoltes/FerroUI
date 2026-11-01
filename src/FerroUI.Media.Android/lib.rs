//! Shared Android native SurfaceTexture output. No video/camera peer dependency.
#![cfg(target_os = "android")]
#![forbid(unsafe_code)]
use ferroui_android::interop::java::*;
use ferroui_base::{
    reactive::IDisposable,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_media::{frames::*, *};
pub use ferroui_media_opengl::NativeGlFrameProducer;
#[cfg(feature = "controls")]
pub use ferroui_media_opengl::NativeGlTextureImporter;
use ferroui_media_opengl::ReadyGlFrame;
use ferroui_opengl::SharedGlTextureRetirementQueue;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::Duration,
};

/// Independent producer used to verify sharing before attaching video/camera.
pub fn gpu_quadrants(
    producer: &NativeGlFrameProducer,
    identity: FrameIdentity,
    width: u32,
    height: u32,
) -> MediaFuture<Option<ReadyGlFrame>> {
    let class = JavaClass::find("org/ferroui/media/SurfaceFrames");
    let bytes = width as u64 * height as u64 * 4;
    let metadata = FrameMetadata {
        identity,
        width,
        height,
        visible: FrameRegion {
            x: 0,
            y: 0,
            width,
            height,
        },
        format: FrameFormat::Rgba8,
        color: FrameColor::Srgb,
        alpha: FrameAlpha::Opaque,
        rotation: FrameRotation::None,
        mirrored: false,
        top_left_origin: false,
        timestamp: Duration::ZERO,
        device: Some(producer.device()),
        device_generation: producer.device_generation(),
        planes: vec![FramePlane {
            offset: 0,
            stride: width as u64 * 4,
            length: bytes,
        }],
        retained_bytes: bytes,
    };
    producer.produce(metadata, move |_, output| {
        let error = call_static_object(
            &class,
            "quadrants",
            "(III)Ljava/lang/String;",
            &[
                JavaValue::Int(output as i32),
                JavaValue::Int(width as i32),
                JavaValue::Int(height as i32),
            ],
        )
        .map(|v| string_of(&v))
        .unwrap_or_else(|| "No independent producer result".into());
        if error.is_empty() {
            Ok(())
        } else {
            Err(native_error(error))
        }
    })
}

pub fn native_error(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::NativeFailure, message)
}
/// Live native source objects, distinct from immutable output frame leases and
/// opaque codec/driver allocations. Destruction is counted after surface return.
pub fn live_surface_sources() -> u32 {
    call_static_int(
        &JavaClass::find("org/ferroui/media/SurfaceFrames"),
        "liveSources",
        "()I",
        &[],
    )
    .max(0) as u32
}
struct TimerRegistration(Rc<dyn IDisposable>);
impl Drop for TimerRegistration {
    fn drop(&mut self) {
        self.0.dispose();
    }
}
/// Poll native callbacks without busy-waiting or retaining canceled futures.
pub fn poll_native<T: 'static, E: 'static>(
    mut read: impl FnMut() -> Poll<Result<T, E>> + 'static,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>>>> {
    let mut timer: Option<TimerRegistration> = None;
    Box::pin(std::future::poll_fn(move |cx| {
        timer.take();
        match read() {
            Poll::Ready(value) => Poll::Ready(value),
            Poll::Pending => {
                let waker = cx.waker().clone();
                timer = Some(TimerRegistration(DispatcherTimer::run_once(
                    move || waker.wake_by_ref(),
                    Duration::from_millis(16),
                    DispatcherPriority::NORMAL,
                )));
                Poll::Pending
            }
        }
    }))
}
pub fn text(object: &JavaObject, method: &str) -> String {
    call_object(object, method, "()Ljava/lang/String;", &[])
        .map(|s| string_of(&s))
        .unwrap_or_default()
}
struct SourceOwner {
    java: JavaObject,
    retirement: SharedGlTextureRetirementQueue,
    _wake: Arc<dyn Fn() + Send + Sync>,
}
impl Drop for SourceOwner {
    fn drop(&mut self) {
        let java = self.java.clone();
        self.retirement.defer_native(move |context| {
            call_boolean(
                &java,
                "release",
                "(Z)Z",
                &[JavaValue::Boolean(context.is_some())],
            )
        });
    }
}
/// One source acquisition shared by every 2D/3D consumer of its mailbox.
#[derive(Clone)]
pub struct AndroidSurfaceSource(Arc<SourceOwner>);
impl AndroidSurfaceSource {
    pub fn create(producer: &NativeGlFrameProducer) -> MediaFuture<Self> {
        // Cache application classes on the Android UI thread before JNI calls
        // from a native render thread (FindClass there has a system loader).
        let class = JavaClass::find("org/ferroui/media/SurfaceFrames");
        let wake = producer.retirement_wakeup();
        producer.invoke(move |context| {
            let java = new_object(&class, "()V", &[]).to_global();
            let error = text(&java, "error");
            if !error.is_empty() {
                return Err(native_error(error));
            }
            let retirement = context
                .shared_frame_retirement_queue()
                .ok_or_else(|| native_error("No native GL retirement"))?;
            let wake = retirement.register_wakeup(wake);
            Ok(Self(Arc::new(SourceOwner {
                java,
                retirement,
                _wake: wake,
            })))
        })
    }
    pub fn java(&self) -> JavaObject {
        self.0.java.clone()
    }
    pub fn surface(&self) -> Result<JavaObject, MediaError> {
        call_object(&self.0.java, "surface", "()Landroid/view/Surface;", &[])
            .map(|v| v.to_global())
            .ok_or_else(|| native_error("Surface source was released"))
    }
}
pub struct AndroidFrameStream {
    producer: NativeGlFrameProducer,
    source: RefCell<Option<AndroidSurfaceSource>>,
    identity: FrameIdentity,
    last_sequence: Arc<std::sync::atomic::AtomicU64>,
    job: RefCell<Option<MediaFuture<Option<ReadyGlFrame>>>>,
    pending: RefCell<Option<ReadyGlFrame>>,
    stopped: Cell<bool>,
    epoch: Arc<std::sync::atomic::AtomicU64>,
    job_epoch: Rc<Cell<u64>>,
    frame_clock: Option<JavaObject>,
    capture_identity: bool,
    clock_nanoseconds: bool,
}
impl AndroidFrameStream {
    pub fn new(
        producer: NativeGlFrameProducer,
        source: AndroidSurfaceSource,
        id: u64,
        generation: u64,
    ) -> Self {
        Self {
            producer,
            source: RefCell::new(Some(source)),
            identity: FrameIdentity {
                source: id,
                generation,
                sequence: 0,
            },
            last_sequence: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            job: RefCell::new(None),
            pending: RefCell::new(None),
            stopped: Cell::new(false),
            epoch: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            job_epoch: Rc::new(Cell::new(0)),
            frame_clock: None,
            capture_identity: false,
            clock_nanoseconds: false,
        }
    }
    /// A bounded native frame-clock mapper for playback. Camera streams keep
    /// their native capture timestamps and do not depend on a video crate.
    pub fn with_frame_clock(mut self, clock: JavaObject) -> Self {
        self.frame_clock = Some(clock);
        self
    }
    /// Camera streams share sensor-timestamp identity with their optional
    /// analysis stream; playback keeps monotonic acquisition sequence instead.
    pub fn with_capture_identity(mut self) -> Self {
        self.capture_identity = true;
        self
    }
    /// Camera2 capture-result metadata belongs to the acquired sensor timestamp,
    /// including across CameraX use-case rebinding. No guessed current size.
    pub fn with_capture_clock(mut self, clock: JavaObject) -> Self {
        self.frame_clock = Some(clock);
        self.clock_nanoseconds = true;
        self.capture_identity = true;
        self
    }
    // Keep the current conversion until its producer fence retires. Canceling
    // publication must not allow updateTexImage to reuse its input early.
    pub fn clear(&self) {
        self.epoch.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    }
    pub fn stop(&self) {
        self.stopped.set(true);
        self.clear();
        self.job.borrow_mut().take();
        self.pending.borrow_mut().take();
        self.source.borrow_mut().take();
    }
    pub fn has_pending_frame_work(&self) -> bool {
        !self.stopped.get()
            && (self.job.borrow().is_some()
                || self.pending.borrow().is_some()
                || self
                    .source
                    .borrow()
                    .as_ref()
                    .is_some_and(|source| call_boolean(&source.0.java, "pending", "()Z", &[])))
    }
    pub fn take_frame(&self, width: u32, height: u32) -> Result<Option<FrameLease>, MediaError> {
        self.take_oriented_frame(width, height, FrameRotation::None, false)
    }
    pub fn take_oriented_frame(
        &self,
        width: u32,
        height: u32,
        rotation: FrameRotation,
        mirrored: bool,
    ) -> Result<Option<FrameLease>, MediaError> {
        if self.stopped.get() || width == 0 || height == 0 {
            return Ok(None);
        }
        let ready = self
            .pending
            .borrow()
            .as_ref()
            .map(ReadyGlFrame::take_if_ready);
        if let Some(ready) = ready {
            let frame = ready?;
            if frame.is_none() {
                return Ok(None);
            }
            self.pending.replace(None);
            return Ok(
                if self.epoch.load(std::sync::atomic::Ordering::Acquire) == self.job_epoch.get() {
                    frame
                } else {
                    None
                },
            );
        }
        let result = {
            let mut job = self.job.borrow_mut();
            job.as_mut()
                .map(|job| job.as_mut().poll(&mut Context::from_waker(Waker::noop())))
        };
        if let Some(result) = result {
            match result {
                Poll::Pending => return Ok(None),
                Poll::Ready(result) => {
                    self.job.borrow_mut().take();
                    self.pending.replace(result?);
                    return Ok(None);
                }
            }
        }
        let Some(source) = self.source.borrow().clone() else {
            return Ok(None);
        };
        if !call_boolean(&source.0.java, "pending", "()Z", &[]) {
            return Ok(None);
        }
        let producer = self.producer.clone();
        let last = self.last_sequence.clone();
        let mut identity = self.identity;
        let frame_clock = self.frame_clock.clone();
        let capture_identity = self.capture_identity;
        let clock_nanoseconds = self.clock_nanoseconds;
        let requested_epoch = self.epoch.load(std::sync::atomic::Ordering::Acquire);
        self.job_epoch.set(requested_epoch);
        let epoch = self.epoch.clone();
        let job_epoch = self.job_epoch.clone();
        self.job.replace(Some(Box::pin(async move {
            let native = source.clone();
            let acquisition_epoch = epoch.clone();
            let acquired = producer
                .invoke(move |_| {
                    // An obsolete queued job must not consume a newly sought
                    // paused frame: there may never be another frame callback.
                    if acquisition_epoch.load(std::sync::atomic::Ordering::Acquire)
                        != requested_epoch
                    {
                        return Ok(None);
                    }
                    if !call_boolean(&native.0.java, "pending", "()Z", &[]) {
                        return Ok(None);
                    }
                    let error = text(&native.0.java, "acquire");
                    if !error.is_empty() {
                        return Err(if call_boolean(&native.0.java, "unsupported", "()Z", &[]) {
                            MediaError::new(MediaErrorKind::Unsupported, error)
                        } else {
                            native_error(error)
                        });
                    }
                    Ok(Some((
                        call_long(&native.0.java, "sequence", "()J", &[]) as u64,
                        call_long(&native.0.java, "timestamp", "()J", &[]).max(0) as u64,
                        call_boolean(&native.0.java, "swapsAxes", "()Z", &[]),
                    )))
                })
                .await?;
            let Some((sequence, timestamp, swaps_axes)) = acquired else {
                return Ok(None);
            };
            if sequence <= last.load(std::sync::atomic::Ordering::Acquire) {
                return Ok(None);
            }
            identity.sequence = if capture_identity {
                timestamp
            } else {
                sequence
            };
            let (timestamp, width, height, color, rotation, mirrored, visible) =
                if let Some(clock) = frame_clock {
                    let value = call_object(
                        &clock,
                        "frameInfo",
                        "(J)Ljava/lang/String;",
                        &[JavaValue::Long(timestamp as i64)],
                    )
                    .map(|v| string_of(&v))
                    .unwrap_or_default();
                    if value.is_empty() {
                        return Ok(None);
                    }
                    let values = value
                        .split(',')
                        .map(str::parse::<u64>)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| native_error("Invalid native frame-clock metadata"))?;
                    if values.len() != if clock_nanoseconds { 6 } else { 10 }
                        || values[1] > u32::MAX as u64
                        || values[2] > u32::MAX as u64
                    {
                        return Err(native_error("Invalid native frame dimensions"));
                    }
                    (
                        values[0]
                            .checked_mul(if clock_nanoseconds { 1 } else { 1000 })
                            .ok_or_else(|| native_error("Native timestamp overflow"))?,
                        values[1] as u32,
                        values[2] as u32,
                        match values[3] {
                            1 => FrameColor::Srgb,
                            2 => FrameColor::Rec709Full,
                            _ => return Err(native_error("Unsupported native video color")),
                        },
                        {
                            match values[4] {
                                0 => FrameRotation::None,
                                90 => FrameRotation::Clockwise90,
                                180 => FrameRotation::Clockwise180,
                                270 => FrameRotation::Clockwise270,
                                _ => {
                                    return Err(native_error(
                                        "Unsupported native frame orientation",
                                    ))
                                }
                            }
                        },
                        {
                            match values[5] {
                                0 => false,
                                1 => true,
                                _ => return Err(native_error("Invalid native mirror flag")),
                            }
                        },
                        if clock_nanoseconds {
                            None
                        } else {
                            if values[6..].iter().any(|v| *v > u32::MAX as u64) {
                                return Err(native_error("Native crop overflow"));
                            }
                            Some(FrameRegion {
                                x: values[6] as u32,
                                y: values[7] as u32,
                                width: values[8] as u32,
                                height: values[9] as u32,
                            })
                        },
                    )
                } else {
                    if epoch.load(std::sync::atomic::Ordering::Acquire) != requested_epoch {
                        return Ok(None);
                    }
                    (
                        timestamp,
                        width,
                        height,
                        FrameColor::Srgb,
                        rotation,
                        mirrored,
                        None,
                    )
                };
            // A seek can race the render-thread acquisition itself. The
            // playback clock accepts only a frame belonging to its current
            // epoch; keep that newly sought frame instead of discarding the
            // sole paused callback just because the job was queued earlier.
            job_epoch.set(epoch.load(std::sync::atomic::Ordering::Acquire));
            // SurfaceTexture's native transform may already orient the image.
            // Preserve its aspect instead of rotating it again in composition.
            let (width, height) = if swaps_axes {
                (height, width)
            } else {
                (width, height)
            };
            let bytes = width as u64 * height as u64 * 4;
            let metadata = FrameMetadata {
                identity,
                width,
                height,
                visible: visible.unwrap_or(FrameRegion {
                    x: 0,
                    y: 0,
                    width,
                    height,
                }),
                format: FrameFormat::Rgba8,
                color,
                alpha: FrameAlpha::Opaque,
                rotation,
                mirrored,
                top_left_origin: false,
                timestamp: Duration::from_nanos(timestamp),
                device: Some(producer.device()),
                device_generation: producer.device_generation(),
                planes: vec![FramePlane {
                    offset: 0,
                    stride: width as u64 * 4,
                    length: bytes,
                }],
                retained_bytes: bytes,
            };
            let normalize_color = call_boolean(&source.0.java, "colorConversion", "()Z", &[]);
            let counters = producer.budget().counters();
            let owner = source.clone();
            let output = producer
                .produce_owned(metadata, owner, move |_, output| {
                    counters.gpu_conversion(GpuTransferReason::PixelFormatConversion);
                    if normalize_color {
                        counters.gpu_conversion(GpuTransferReason::ColorSpaceConversion);
                    }
                    let error = call_object(
                        &source.0.java,
                        "draw",
                        "(III)Ljava/lang/String;",
                        &[
                            JavaValue::Int(output as i32),
                            JavaValue::Int(width as i32),
                            JavaValue::Int(height as i32),
                        ],
                    )
                    .map(|v| string_of(&v))
                    .unwrap_or_default();
                    if error.is_empty() {
                        Ok(())
                    } else {
                        Err(native_error(error))
                    }
                })
                .await?;
            if output.is_some() {
                last.store(sequence, std::sync::atomic::Ordering::Release);
            }
            Ok(output)
        })));
        Ok(None)
    }
}
