//! Native render-thread GL jobs and immutable, fence-retired frame storage.
//! The producer draws into RGBA8 storage in the active compositor context.
//! CPU preview transfer is not part of this API.
use ferroui_base::{
    platform::IOptionalFeatureProvider,
    reactive::IDisposable,
    rendering::composition::{Compositor, ICompositionGpuInterop},
};
use ferroui_media::{frames::*, *};
use ferroui_opengl::{IGlContext, SharedFrameTexture};
use std::{rc::Rc, sync::Arc};

struct CurrentContext(Rc<dyn IDisposable>);
impl Drop for CurrentContext {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

mod native_view;
mod renderer;
pub use native_view::NativeTextureView;
pub use renderer::{GlTextureRender, GlTextureRenderer};

fn unsupported(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, message)
}
#[derive(Clone)]
pub struct NativeGlFrameProducer {
    compositor: Rc<Compositor>,
    context: u64,
    generation: u64,
    budget: FrameBudget,
    wake: Arc<dyn Fn() + Send + Sync>,
}
impl NativeGlFrameProducer {
    pub fn from_interop(
        interop: &dyn ICompositionGpuInterop,
        budget: FrameBudget,
    ) -> Result<Self, MediaError> {
        let context = interop
            .shared_context_id()
            .ok_or_else(|| unsupported("No admitted native GL context"))?;
        if context < (1 << 32) {
            return Err(unsupported("Use the browser producer for WebGL"));
        }
        if !interop.supported_image_handle_types().iter().any(|v| {
            v ==
            ferroui_base::platform::KnownPlatformGraphicsExternalImageHandleTypes::SHARED_GL_TEXTURE
        }) {
            return Err(unsupported("No direct leased GL importer"));
        }
        let compositor = interop
            .render_compositor()
            .ok_or_else(|| unsupported("No render-thread producer jobs"))?;
        let render_loop = Arc::downgrade(compositor.render_loop());
        Ok(Self {
            compositor,
            context,
            generation: interop.shared_context_generation(),
            budget,
            wake: Arc::new(move || {
                if let Some(render_loop) = render_loop.upgrade() {
                    render_loop.wakeup();
                }
            }),
        })
    }
    pub fn budget(&self) -> FrameBudget {
        self.budget.clone()
    }
    pub fn with_budget(&self, budget: FrameBudget) -> Self {
        Self {
            budget,
            ..self.clone()
        }
    }
    pub fn device(&self) -> DeviceIdentity {
        DeviceIdentity::Context(self.context)
    }
    pub fn device_generation(&self) -> u64 {
        self.generation
    }
    pub fn retirement_wakeup(&self) -> Arc<dyn Fn() + Send + Sync> {
        self.wake.clone()
    }
    /// Runs a narrow native operation on the render thread with the admitted
    /// GL context current. The closure must restore any GL state it changes.
    /// Captures and results must be Send; the context never leaves its thread.
    pub fn invoke<T: Send + 'static>(
        &self,
        job: impl FnOnce(&Rc<dyn IGlContext>) -> Result<T, MediaError> + Send + 'static,
    ) -> MediaFuture<T> {
        let id = self.context;
        let generation = self.generation;
        let task = self.compositor.invoke_server_job_async(
            move |server| {
                let backend = server.render_interface().value();
                let features: &dyn IOptionalFeatureProvider = &*backend;
                let Some(context) = features.try_get::<dyn IGlContext>() else {
                    return Ok(Err(unsupported("The active renderer is not native GL")));
                };
                if context.is_lost()
                    || context.shared_frame_context_id() != Some(id)
                    || context.shared_frame_context_generation() != generation
                {
                    return Ok(Err(unsupported("The producer GL context was replaced")));
                }
                let _current = CurrentContext(context.ensure_current());
                let renderer = context
                    .gl_interface()
                    .renderer()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if renderer.is_empty()
                    || [
                        "swiftshader",
                        "llvmpipe",
                        "softpipe",
                        "lavapipe",
                        "software rasterizer",
                        "microsoft basic render",
                        "warp",
                    ]
                    .iter()
                    .any(|name| renderer.contains(name))
                {
                    return Ok(Err(unsupported(
                        "Software/unknown GL has no admitted GPU media route",
                    )));
                }
                Ok(job(&context))
            },
            false,
        );
        struct WakeRegistration(Arc<std::sync::Mutex<Option<std::task::Waker>>>);
        impl Drop for WakeRegistration {
            fn drop(&mut self) {
                self.0.lock().unwrap().take();
            }
        }
        let registration = WakeRegistration(Arc::new(std::sync::Mutex::new(None)));
        let wake = registration.0.clone();
        task.on_completed(move || {
            let waker = wake.lock().unwrap().take();
            if let Some(waker) = waker {
                waker.wake();
            }
        });
        Box::pin(std::future::poll_fn(move |cx| {
            *registration.0.lock().unwrap() = Some(cx.waker().clone());
            match task.take_result() {
                None => std::task::Poll::Pending,
                Some(result) => std::task::Poll::Ready(result.map_err(|error| {
                    MediaError::new(MediaErrorKind::NativeFailure, error.to_string())
                })?),
            }
        }))
    }
    /// Reserves before allocation. Saturation returns None without queuing a
    /// draw. `draw` must write the complete image and preserve GL state. A GPU
    /// fence retains storage even if this future is canceled after submission.
    pub fn produce(
        &self,
        metadata: FrameMetadata,
        draw: impl FnOnce(&Rc<dyn IGlContext>, u32) -> Result<(), MediaError> + Send + 'static,
    ) -> MediaFuture<Option<ReadyGlFrame>> {
        self.produce_owned(metadata, (), draw)
    }
    /// `producer_owner` keeps borrowed native input alive through conversion
    /// completion, even when the session is replaced while the job is queued.
    pub fn produce_owned<T: std::any::Any + Send + Sync>(
        &self,
        metadata: FrameMetadata,
        producer_owner: T,
        draw: impl FnOnce(&Rc<dyn IGlContext>, u32) -> Result<(), MediaError> + Send + 'static,
    ) -> MediaFuture<Option<ReadyGlFrame>> {
        if metadata.device != Some(self.device())
            || metadata.device_generation != self.generation
            || metadata.format != FrameFormat::Rgba8
            || metadata.alpha == FrameAlpha::Straight
        {
            return Box::pin(async { Err(unsupported("Incompatible native GL frame metadata")) });
        }
        if self.budget.limits().frames == 0 || metadata.retained_bytes > self.budget.limits().bytes
        {
            return Box::pin(async {
                Err(unsupported("Frame exceeds the configured storage budget"))
            });
        }
        let reservation = match self.budget.reserve(metadata.clone()) {
            Ok(value) => value,
            Err(FrameError::BudgetExceeded) => return Box::pin(async { Ok(None) }),
            Err(error) => return Box::pin(async move { Err(unsupported(error.to_string())) }),
        };
        let wake = self.wake.clone();
        self.invoke(move |context| {
            let queue = context
                .shared_frame_retirement_queue()
                .ok_or_else(|| unsupported("No owning-context texture retirement"))?;
            let wake = queue.register_wakeup(wake);
            let gl = context.gl_interface();
            let old_texture = gl.get_integerv(0x8069); // TEXTURE_BINDING_2D
            let old_unpack = gl.get_integerv(0x88EF); // PIXEL_UNPACK_BUFFER_BINDING
            let mut texture = 0;
            // SAFETY: current GL context, one output integer, and null initial
            // storage with the unpack buffer explicitly unbound (no CPU input).
            unsafe {
                gl.gen_textures(1, &mut texture);
            }
            if texture == 0 {
                return Err(unsupported("Native GL texture allocation failed"));
            }
            let owner = queue.owner(texture as u32, reservation.retirement(), wake);
            gl.bind_buffer(0x88EC, 0);
            gl.bind_texture(0x0DE1, texture);
            gl.tex_parameteri(0x0DE1, 0x2801, 0x2601);
            gl.tex_parameteri(0x0DE1, 0x2800, 0x2601);
            gl.tex_parameteri(0x0DE1, 0x2802, 0x812F);
            gl.tex_parameteri(0x0DE1, 0x2803, 0x812F);
            unsafe {
                gl.tex_image_2d(
                    0x0DE1,
                    0,
                    0x8058,
                    metadata.width as i32,
                    metadata.height as i32,
                    0,
                    0x1908,
                    0x1401,
                    std::ptr::null(),
                );
            }
            gl.bind_texture(0x0DE1, old_texture);
            gl.bind_buffer(0x88EC, old_unpack);
            if gl.get_error() != 0 {
                return Err(unsupported("Native RGBA storage allocation failed"));
            }
            let frame = reservation.lease(unsafe {
                SharedFrameTexture::new(
                    context.shared_frame_context_id().unwrap(),
                    metadata.device_generation,
                    texture as u32,
                    (owner, producer_owner),
                )
            });
            let completion = GpuCompletion::new();
            completion
                .retain(frame.clone())
                .map_err(|e| unsupported(e.to_string()))?;
            let result = draw(context, texture as u32);
            // Even an unsuccessful draw may have submitted writes.
            context.schedule_frame_completion(completion.clone());
            result?;
            Ok(Some(ReadyGlFrame { frame, completion }))
        })
    }
}
/// A submitted frame becomes publishable only after its producer fence.
pub struct ReadyGlFrame {
    frame: FrameLease,
    completion: GpuCompletion,
}
impl ReadyGlFrame {
    pub fn completion(&self) -> GpuCompletion {
        self.completion.clone()
    }
    pub fn take_if_ready(&self) -> Result<Option<FrameLease>, MediaError> {
        match self.completion.status() {
            GpuCompletionStatus::Complete => Ok(Some(self.frame.clone())),
            GpuCompletionStatus::Pending => Ok(None),
            _ => Err(unsupported("Native producer completion is indeterminate")),
        }
    }
}

#[cfg(feature = "controls")]
pub struct NativeGlTextureImporter;
#[cfg(feature = "controls")]
impl ferroui_media_controls::TextureFrameImporter for NativeGlTextureImporter {
    fn import(
        &self,
        interop: &dyn ICompositionGpuInterop,
        frame: &FrameLease,
    ) -> Result<ferroui_media_controls::ImportedTextureFrame, MediaError> {
        use ferroui_base::platform::{
            PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
        };
        let m = frame.metadata();
        if m.device != interop.shared_context_id().map(DeviceIdentity::Context)
            || m.device_generation != interop.shared_context_generation()
        {
            return Err(unsupported("Frame and renderer contexts differ"));
        }
        let image = interop.import_image(
            SharedFrameTexture::handle(frame).map_err(|e| unsupported(e.to_string()))?,
            PlatformGraphicsExternalImageProperties {
                width: m.width as i32,
                height: m.height as i32,
                format: PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm,
                top_left_origin: m.top_left_origin,
                memory_size: m.retained_bytes,
                ..Default::default()
            },
        );
        let producer_ready = GpuCompletion::new();
        producer_ready.complete();
        Ok(ferroui_media_controls::ImportedTextureFrame {
            image,
            producer_ready,
        })
    }
}
