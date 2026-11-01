//! Renderer-neutral Metal encoding on FerroUI's active device and command queue.
#![cfg(target_vendor = "apple")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_base::{
    platform::{
        IExternalObjectsRenderInterfaceContextFeature, IOptionalFeatureProvider,
        KnownPlatformGraphicsExternalImageHandleTypes,
    },
    rendering::composition::{Compositor, ICompositionGpuInterop},
};
use ferroui_media::{
    frames::*, MediaError, MediaErrorKind, MediaFuture, RendererViewport, TextureRenderer,
};
use ferroui_media_apple::{poll_native, PixelBuffer};
use ferroui_metal::IMetalDevice;
use std::{
    ffi::c_void,
    rc::Rc,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::Poll,
    time::Duration,
};
fn unsupported(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, message)
}
/// Borrowed handles valid only during encode. The client must end its encoders
/// and must not commit or retain this command buffer. The framework submits it
/// once, retains all input/output leases and signals actual native completion.
pub struct MetalRenderContext<'a> {
    device: *mut c_void,
    output: &'a native::Output,
    completion: GpuCompletion,
    device_identity: DeviceIdentity,
    generation: u64,
    _thread: std::marker::PhantomData<Rc<()>>,
}
impl MetalRenderContext<'_> {
    pub fn device(&self) -> *mut c_void {
        self.device
    }
    pub fn command_buffer(&self) -> *mut c_void {
        self.output.commands()
    }
    pub fn output_texture(&self) -> *mut c_void {
        self.output.texture()
    }
    #[allow(unsafe_code)]
    pub fn input_texture(&self, frame: &FrameLease) -> Result<MetalTextureView, MediaError> {
        if !frame.is_current() {
            return Err(MediaError::cancelled());
        }
        if frame.metadata().device.as_ref() != Some(&self.device_identity)
            || frame.metadata().device_generation != self.generation
        {
            return Err(unsupported("Incompatible or retired Metal input"));
        }
        let buffer = frame
            .resource::<PixelBuffer>()
            .ok_or_else(|| unsupported("Input is not retained CoreVideo storage"))?;
        let (width, height, _, _, _) = buffer.info();
        if (width, height) != (frame.metadata().width, frame.metadata().height) {
            return Err(unsupported(
                "Metal input dimensions differ from frame metadata",
            ));
        }
        let texture = unsafe { native::Texture::new(self.device, buffer) }
            .ok_or_else(|| unsupported("No compatible Metal input texture view"))?;
        self.completion
            .retain(frame.clone())
            .map_err(|e| unsupported(e.to_string()))?;
        Ok(MetalTextureView {
            texture,
            _frame: frame.clone(),
            _thread: std::marker::PhantomData,
        })
    }
}
pub struct MetalTextureView {
    texture: native::Texture,
    _frame: FrameLease,
    _thread: std::marker::PhantomData<Rc<()>>,
}
impl MetalTextureView {
    pub fn handle(&self) -> *mut c_void {
        self.texture.handle()
    }
}
/// Client-owned scene and library adapter. Encode premultiplied sRGB BGRA8
/// output, including transparent pixels. Input metadata supplies crop,
/// orientation and color; the client owns its shading and hit testing.
pub trait MetalTextureRender: Send + Sync {
    fn encode(
        &self,
        context: &MetalRenderContext<'_>,
        viewport: RendererViewport,
        inputs: &[FrameLease],
    ) -> Result<(), MediaError>;
}
pub struct MetalTextureRenderer {
    compositor: Rc<Compositor>,
    device: Vec<u8>,
    generation: u64,
    budget: FrameBudget,
    source: u64,
    sequence: AtomicU64,
    client: Arc<dyn MetalTextureRender>,
}
impl MetalTextureRenderer {
    pub fn from_interop(
        interop: &dyn ICompositionGpuInterop,
        budget: FrameBudget,
        source: u64,
        client: Arc<dyn MetalTextureRender>,
    ) -> Result<Self, MediaError> {
        if !interop
            .supported_image_handle_types()
            .iter()
            .any(|v| v == KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF)
        {
            return Err(unsupported("No admitted IOSurface presentation route"));
        }
        Ok(Self {
            compositor: interop
                .render_compositor()
                .ok_or_else(|| unsupported("No native renderer jobs"))?,
            device: interop
                .device_luid()
                .filter(|v| v.len() == 8)
                .ok_or_else(|| unsupported("No active Metal device identity"))?,
            generation: interop.shared_context_generation(),
            budget,
            source,
            sequence: AtomicU64::new(1),
            client,
        })
    }
}
struct Unsubmitted(Option<GpuCompletion>);
impl Drop for Unsubmitted {
    fn drop(&mut self) {
        if let Some(completion) = self.0.take() {
            completion.complete();
        }
    }
}
impl TextureRenderer for MetalTextureRenderer {
    #[allow(unsafe_code)]
    fn render(
        &self,
        viewport: RendererViewport,
        inputs: Vec<FrameLease>,
        completion: GpuCompletion,
    ) -> MediaFuture<FrameLease> {
        if self.budget.quarantined_reads() > 0 {
            return Box::pin(async {
                Err(unsupported("Renderer GPU completion is indeterminate"))
            });
        }
        let source = self.source;
        let sequence = self
            .sequence
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1))
            .expect("renderer sequence exhausted");
        let device = self.device.clone();
        let generation = self.generation;
        let budget = self.budget.clone();
        let client = self.client.clone();
        // The queued native job owns rollback; abandoning a UI future cannot
        // complete a token while that job is still going to execute.
        let mut rollback = Unsubmitted(Some(completion.clone()));
        let task = self.compositor.invoke_server_job_async(
            move |server| {
                let job = (|| {
                    let backend = server.render_interface().value();
                    let features: &dyn IOptionalFeatureProvider = &*backend;
                    let metal = features
                        .try_get::<dyn IMetalDevice>()
                        .ok_or_else(|| unsupported("Active renderer is not Metal"))?;
                    let interop = features
                        .try_get::<dyn IExternalObjectsRenderInterfaceContextFeature>()
                        .ok_or_else(|| unsupported("No Metal import feature"))?;
                    if metal.is_lost()
                        || interop.shared_context_generation() != generation
                        || interop.device_luid().as_ref() != Some(&device)
                    {
                        return Err(unsupported("Metal renderer device/generation changed"));
                    }
                    if inputs.iter().any(|v| !v.is_current()) {
                        return Err(MediaError::cancelled());
                    }
                    if inputs.iter().any(|v| {
                        v.metadata().device != Some(DeviceIdentity::Luid(device.clone()))
                            || v.metadata().device_generation != generation
                    }) {
                        return Err(unsupported("Incompatible or retired renderer input"));
                    }
                    let (stride, bytes) = native::layout(viewport.width, viewport.height);
                    let metadata = FrameMetadata {
                        identity: FrameIdentity {
                            source,
                            generation: viewport.generation,
                            sequence,
                        },
                        width: viewport.width,
                        height: viewport.height,
                        visible: FrameRegion {
                            x: 0,
                            y: 0,
                            width: viewport.width,
                            height: viewport.height,
                        },
                        format: FrameFormat::Bgra8,
                        color: FrameColor::Srgb,
                        alpha: FrameAlpha::Premultiplied,
                        rotation: FrameRotation::None,
                        mirrored: false,
                        top_left_origin: true,
                        timestamp: Duration::ZERO,
                        device: Some(DeviceIdentity::Luid(device)),
                        device_generation: generation,
                        planes: vec![FramePlane {
                            offset: 0,
                            stride,
                            length: stride * viewport.height as u64,
                        }],
                        retained_bytes: bytes,
                    };
                    let reservation = budget.reserve(metadata).map_err(|e| {
                        if e == FrameError::BudgetExceeded {
                            MediaError::cancelled()
                        } else {
                            unsupported(e.to_string())
                        }
                    })?;
                    let output = unsafe {
                        native::Output::create(
                            metal.device(),
                            metal.command_queue(),
                            viewport.width,
                            viewport.height,
                        )
                    }?;
                    let buffer = output.buffer();
                    let (_, _, actual_stride, actual_bytes, _) = buffer.info();
                    if actual_bytes > bytes || actual_stride != stride {
                        return Err(unsupported(
                            "Native Metal allocation differs from admitted layout",
                        ));
                    }
                    let frame = reservation.lease(buffer);
                    completion
                        .retain(frame.clone())
                        .map_err(|e| unsupported(e.to_string()))?;
                    let context = MetalRenderContext {
                        device: metal.device(),
                        output: &output,
                        completion: completion.clone(),
                        device_identity: frame.metadata().device.clone().unwrap(),
                        generation,
                        _thread: std::marker::PhantomData,
                    };
                    client.encode(&context, viewport, &inputs)?;
                    rollback.0.take();
                    output.commit(completion.clone());
                    Ok((frame, completion))
                })();
                Ok(job)
            },
            false,
        );
        Box::pin(async move {
            let (frame, completion) =
                poll_native(move || match task.take_result() {
                    None => Poll::Pending,
                    Some(result) => Poll::Ready(result.map_err(|e| {
                        MediaError::new(MediaErrorKind::NativeFailure, e.to_string())
                    })?),
                })
                .await?;
            completion
                .wait()
                .await
                .map_err(|e| unsupported(e.to_string()))?;
            Ok(frame)
        })
    }
}
