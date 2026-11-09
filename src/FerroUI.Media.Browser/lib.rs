//! Immutable WebGL2 storage shared by independent producers, video and camera.
//! Two routes, chosen at run time by where the compositor's WebGL2 context lives:
//! on the page's thread, native media transfers use TexImageSource directly into
//! a shared texture; on the render worker of a module with threads, each frame is
//! a WebCodecs `VideoFrame` transferred to the worker and uploaded there when the
//! compositor imports it (#601). No framework preview pixel array or readback is
//! admitted on either route. Browser decoder/driver internals are opaque.
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_base::{
    platform::KnownPlatformGraphicsExternalImageHandleTypes,
    rendering::composition::ICompositionGpuInterop,
};
use ferroui_media::{frames::*, *};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};
use wasm_bindgen::JsValue;

/// Allocation policy for the active compositor context. Old generations remain
/// charged to the same budget until their native storage actually retires.
#[derive(Clone)]
pub struct WebGlFrameProducer {
    context: u64,
    generation: u64,
    budget: FrameBudget,
    graphics: Acceleration,
    /// Frames go to the render worker as transferred `VideoFrame`s (#601).
    transfer: bool,
}
impl WebGlFrameProducer {
    pub fn from_interop(
        interop: &dyn ICompositionGpuInterop,
        budget: FrameBudget,
    ) -> Result<Self, MediaError> {
        if !cfg!(target_os = "emscripten") {
            return Err(unsupported("Shared WebGL frames need the browser platform"));
        }
        let types = interop.supported_image_handle_types();
        let has = |kind: &str| types.iter().any(|v| v == kind);
        // The compositor renders on the render worker: its context cannot sample the page's
        // media elements, so the frames are transferred to it (#601).
        if interop.shared_context_id().is_none()
            && has(KnownPlatformGraphicsExternalImageHandleTypes::BROWSER_TRANSFERRED_FRAME)
        {
            if !native::can_transfer() {
                return Err(unsupported(
                    "Media frames on the render worker need WebCodecs VideoFrame and a started render worker",
                ));
            }
            return Ok(Self {
                context: 0,
                generation: interop.shared_context_generation(),
                budget,
                // The renderer of the worker's context is not observable from the page.
                graphics: Acceleration::Unknown,
                transfer: true,
            });
        }
        let context = interop
            .shared_context_id()
            .ok_or_else(|| unsupported("The compositor has no admitted shared WebGL context"))?;
        if context > i32::MAX as u64
            || !interop
                .supported_image_handle_types()
                .iter()
                .any(|v| v == KnownPlatformGraphicsExternalImageHandleTypes::SHARED_GL_TEXTURE)
        {
            return Err(unsupported(
                "The compositor has no lease-backed WebGL importer",
            ));
        }
        let graphics = match native::graphics(context as i32) {
            2 => Acceleration::Software,
            0 => Acceleration::Unknown,
            _ => return Err(unsupported("The WebGL context is unavailable")),
        };
        if graphics == Acceleration::Software {
            return Err(unsupported(
                "Software graphics has no admitted GPU media route",
            ));
        }
        Ok(Self {
            context,
            generation: interop.shared_context_generation(),
            budget,
            graphics,
            transfer: false,
        })
    }
    pub fn budget(&self) -> FrameBudget {
        self.budget.clone()
    }
    pub fn device(&self) -> DeviceIdentity {
        DeviceIdentity::Context(self.context)
    }
    pub fn device_generation(&self) -> u64 {
        self.generation
    }
    pub fn graphics(&self) -> Acceleration {
        self.graphics
    }
    pub fn with_budget(&self, budget: FrameBudget) -> Self {
        Self {
            budget,
            ..self.clone()
        }
    }
    pub fn stream(&self, source: u64, generation: u64) -> BrowserFrameStream {
        BrowserFrameStream {
            producer: self.clone(),
            source,
            generation,
            last_sequence: Cell::new(0),
            pending: RefCell::new(None),
            retrying: Cell::new(false),
        }
    }
    pub fn is_current(&self) -> bool {
        // A transferred frame is bound to the worker's context only when it is imported.
        self.transfer || native::current(self.context as i32, self.generation)
    }
    /// Whether frames are transferred to the render worker (#601) rather than written into a
    /// texture of a context of this thread.
    pub fn transfers_frames(&self) -> bool {
        self.transfer
    }
    /// Independent GPU producer. Its framebuffer is bottom-left oriented;
    /// presentation must preserve that origin rather than flip CPU pixels.
    pub fn quadrants(
        &self,
        identity: FrameIdentity,
        width: u32,
        height: u32,
    ) -> Result<Option<PendingWebGlFrame>, MediaError> {
        if self.transfer {
            return Err(unsupported(
                "A generated GPU test pattern needs the compositor's context on the page's thread",
            ));
        }
        self.produce(identity, width, height, Duration::ZERO, false, |texture| {
            native::quadrants(texture, width, height)
        })
    }
    /// Transfers a browser-owned HTMLVideoElement image directly to the active GL
    /// context. CORS/security failures remain errors; there is no CPU fallback.
    pub fn upload(
        &self,
        identity: FrameIdentity,
        width: u32,
        height: u32,
        timestamp: Duration,
        source: &JsValue,
    ) -> Result<Option<PendingWebGlFrame>, MediaError> {
        if !native::is_video_source(source) {
            let bytes = u64::from(width)
                .saturating_mul(u64::from(height))
                .saturating_mul(4);
            let _ = self.budget.counters().cpu_transfer(
                PixelTransferPurpose::Preview,
                bytes,
                PresentationRoute::SharedTexture,
            );
            return Err(unsupported(
                "CPU-backed image/canvas upload is forbidden in shared GPU preview",
            ));
        }
        if self.transfer {
            return self.produce_transferred(identity, width, height, timestamp, source);
        }
        self.produce(identity, width, height, timestamp, true, |texture| {
            self.budget
                .counters()
                .gpu_copy(GpuTransferReason::NativeImageTransfer);
            native::upload(texture, source, width, height)
        })
    }
    /// The route of the render worker (#601): the frame is reserved here and transferred to the
    /// worker as a WebCodecs `VideoFrame`, a GPU-backed image; the compositor uploads it into its
    /// context when it imports it. No pixel array is made on either side.
    fn produce_transferred(
        &self,
        identity: FrameIdentity,
        width: u32,
        height: u32,
        timestamp: Duration,
        source: &JsValue,
    ) -> Result<Option<PendingWebGlFrame>, MediaError> {
        let metadata = self.metadata(identity, width, height, timestamp, true, None)?;
        let reservation = match self.budget.reserve(metadata) {
            Ok(v) => v,
            Err(FrameError::BudgetExceeded) => return Ok(None),
            Err(e) => return Err(unsupported(e.to_string())),
        };
        let frame = native::transfer(source, width, height)?;
        self.budget
            .counters()
            .gpu_copy(GpuTransferReason::NativeImageTransfer);
        let frame = reservation.lease(frame);
        let ready = GpuCompletion::new();
        ready.complete();
        Ok(Some(PendingWebGlFrame { frame, ready }))
    }
    /// The metadata of a frame of this producer, within its budget. A transferred frame has no
    /// device until the compositor imports it (#601).
    fn metadata(
        &self,
        identity: FrameIdentity,
        width: u32,
        height: u32,
        timestamp: Duration,
        top_left_origin: bool,
        device: Option<DeviceIdentity>,
    ) -> Result<FrameMetadata, MediaError> {
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|v| v.checked_mul(4))
            .ok_or_else(|| unsupported("Texture dimensions overflow"))?;
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
            top_left_origin,
            timestamp,
            device,
            device_generation: self.generation,
            planes: vec![FramePlane {
                offset: 0,
                stride: u64::from(width) * 4,
                length: bytes,
            }],
            // RGBA8 storage payload is known. Browser allocation overhead and
            // decoder-owned storage are not observable through WebGL.
            retained_bytes: bytes,
        };
        if self.budget.limits().frames == 0 || bytes > self.budget.limits().bytes {
            return Err(unsupported("Frame exceeds the configured storage budget"));
        }
        Ok(metadata)
    }
    fn produce(
        &self,
        identity: FrameIdentity,
        width: u32,
        height: u32,
        timestamp: Duration,
        top_left_origin: bool,
        write: impl FnOnce(u32) -> Result<(), MediaError>,
    ) -> Result<Option<PendingWebGlFrame>, MediaError> {
        if !self.is_current() {
            return Err(unsupported(
                "The WebGL context generation was lost or replaced",
            ));
        }
        let metadata = self.metadata(
            identity,
            width,
            height,
            timestamp,
            top_left_origin,
            Some(self.device()),
        )?;
        let reservation = match self.budget.reserve(metadata) {
            Ok(v) => v,
            Err(FrameError::BudgetExceeded) => return Ok(None),
            Err(e) => return Err(unsupported(e.to_string())),
        };
        let resource = native::create(
            self.context as i32,
            self.generation,
            reservation.retirement(),
        )?;
        let texture = resource.texture();
        let frame = reservation.lease(resource);
        let ready = GpuCompletion::new();
        // Retain before the first producer GPU command, including error paths.
        if let Err(e) = ready.retain(frame.clone()) {
            ready.complete();
            if e == FrameError::BudgetExceeded {
                return Ok(None);
            }
            return Err(unsupported(e.to_string()));
        }
        let result = write(texture);
        ferroui_browser::rendering::schedule_webgl_completion(self.context as i32, ready.clone());
        result?;
        Ok(Some(PendingWebGlFrame { frame, ready }))
    }
}
/// Cancellation drops the caller's view; the fence retains submitted storage.
pub struct PendingWebGlFrame {
    frame: FrameLease,
    ready: GpuCompletion,
}
/// Native browser image and its source-frame timestamp, sampled synchronously
/// with the subsequent TexImageSource transfer. No framework pixel buffer.
pub struct BrowserImageFrame {
    pub image: JsValue,
    pub sequence: u64,
    pub timestamp: Duration,
    pub width: u32,
    pub height: u32,
}
/// One producer transfer in flight, with latest-native-frame dropping. A
/// second view reads the shared mailbox; it does not call this acquisition.
pub struct BrowserFrameStream {
    producer: WebGlFrameProducer,
    source: u64,
    generation: u64,
    last_sequence: Cell<u64>,
    pending: RefCell<Option<PendingWebGlFrame>>,
    retrying: Cell<bool>,
}
impl BrowserFrameStream {
    /// Retire queued producer output on seek/stop. Submitted GPU storage is
    /// still held by its completion; the last native sequence stays rejected.
    pub fn clear(&self) {
        self.pending.borrow_mut().take();
        self.retrying.set(false);
    }
    pub fn has_pending_frame_work(&self) -> bool {
        self.pending.borrow().is_some() || self.retrying.get()
    }
    pub fn poll(
        &self,
        sample: impl FnOnce() -> Result<Option<BrowserImageFrame>, MediaError>,
    ) -> Result<Option<FrameLease>, MediaError> {
        let ready = self
            .pending
            .borrow()
            .as_ref()
            .map(PendingWebGlFrame::poll)
            .transpose()?;
        let frame = match ready {
            Some(None) => return Ok(None),
            Some(Some(frame)) => {
                self.pending.borrow_mut().take();
                Some(frame)
            }
            None => None,
        };
        let Some(sample) = sample()? else {
            return Ok(frame);
        };
        if sample.sequence <= self.last_sequence.get() {
            return Ok(frame);
        }
        if let Some(pending) = self.producer.upload(
            FrameIdentity {
                source: self.source,
                generation: self.generation,
                sequence: sample.sequence,
            },
            sample.width,
            sample.height,
            sample.timestamp,
            &sample.image,
        )? {
            self.last_sequence.set(sample.sequence);
            *self.pending.borrow_mut() = Some(pending);
            self.retrying.set(false);
        } else {
            self.retrying.set(true);
        }
        Ok(frame)
    }
}
impl PendingWebGlFrame {
    pub fn poll(&self) -> Result<Option<FrameLease>, MediaError> {
        match self.ready.status() {
            GpuCompletionStatus::Pending => Ok(None),
            GpuCompletionStatus::Complete => Ok(Some(self.frame.clone())),
            GpuCompletionStatus::Indeterminate => Err(unsupported(
                "GPU completion failed; storage remains quarantined",
            )),
        }
    }
    pub async fn finish(self) -> Result<FrameLease, MediaError> {
        self.ready
            .wait()
            .await
            .map_err(|e| unsupported(e.to_string()))?;
        Ok(self.frame)
    }
}
pub fn unsupported(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, message)
}

#[cfg(feature = "controls")]
pub struct BrowserTextureImporter;
#[cfg(feature = "controls")]
impl ferroui_media_controls::TextureFrameImporter for BrowserTextureImporter {
    fn import(
        &self,
        interop: &dyn ICompositionGpuInterop,
        frame: &FrameLease,
    ) -> Result<ferroui_media_controls::ImportedTextureFrame, MediaError> {
        use ferroui_base::platform::{
            PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
        };
        let m = frame.metadata();
        // #601: a frame transferred to the render worker is uploaded into the compositor's
        // context when the compositor imports it.
        let handle: std::rc::Rc<dyn ferroui_base::platform::IPlatformHandle> =
            if let Some(transferred) = frame.resource::<native::TransferredFrame>() {
                std::rc::Rc::new(ferroui_base::platform::LeasedPlatformHandle::new(
                    transferred.id() as isize,
                    Some(KnownPlatformGraphicsExternalImageHandleTypes::BROWSER_TRANSFERRED_FRAME),
                    frame.clone(),
                ))
            } else {
                if m.device != interop.shared_context_id().map(DeviceIdentity::Context)
                    || m.device_generation != interop.shared_context_generation()
                {
                    return Err(unsupported("Frame and renderer contexts differ"));
                }
                ferroui_opengl::SharedFrameTexture::handle(frame)
                    .map_err(|e| unsupported(e.to_string()))?
            };
        let image = interop.import_image(
            handle,
            PlatformGraphicsExternalImageProperties {
                width: m.width as i32,
                height: m.height as i32,
                format: PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm,
                top_left_origin: m.top_left_origin,
                memory_size: m.retained_bytes,
                ..Default::default()
            },
        );
        // PendingWebGlFrame only exposes a frame after the producer fence.
        let producer_ready = GpuCompletion::new();
        producer_ready.complete();
        Ok(ferroui_media_controls::ImportedTextureFrame {
            image,
            producer_ready,
        })
    }
}
