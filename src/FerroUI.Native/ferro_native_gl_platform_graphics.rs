//! OpenGL over CGL: the platform graphics, the display, the contexts and
//! their external objects feature, the render surface of a top-level and its
//! render target and rendering session.
//!
//! The native side (`native/FerroUI.Native/src/OSX/cgl.mm` and
//! `rendertarget.mm`) creates the contexts (OpenGL 3.2 core), makes them
//! current under the lock of the context (`CGLLockContext`) and renders the
//! frames of a top-level to an IOSurface that the view shows.

use crate::gpu_handle_wrap_feature::GpuHandleWrapFeature;
use crate::helpers::ComResultExt;
use crate::interop::*;
use crate::top_level_impl::SurfaceTopLevel;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IOptionalFeatureProvider, IPlatformGraphics,
    IPlatformGraphicsContext, IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes,
    KnownPlatformGraphicsExternalSemaphoreHandleTypes, PlatformGraphicsContextLostError,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties, PlatformRenderTargetState,
    RenderTargetSceneInfo,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, RenderTargetNotReadyError};
use ferroui_microcom::{ComPtr, HResult, IUnknown};
use ferroui_opengl::gl_consts::*;
use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use ferroui_opengl::{
    GetProcAddress, GlBasicInfoInterface, GlInterface, GlProfileType, GlVersion, IGlContext,
    IGlContextExternalObjectsFeature, IGlExportableExternalImageTexture, IGlExternalImageTexture,
    IGlExternalSemaphore, OpenGlError,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::ffi::{c_void, CString};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The OpenGL platform graphics of the macOS backend: one context shared by
/// every top-level and both threads that render.
pub struct FerroNativeGlPlatformGraphics {
    display: ComPtr<IFrnGlDisplay>,
    gl_display: Rc<GlDisplay>,
    version: GlVersion,
    shared_context: Rc<GlContext>,
}

// SAFETY: the platform graphics are shared by the UI thread and the thread
// that renders (`IPlatformGraphics: Send + Sync`), as the Metal graphics are
// (`metal.rs`; docs/porting/render-thread.md, R5.5). They are created on the
// UI thread, and afterwards the only member that reaches what they hold is
// the context manager of the server compositor: `uses_shared_context` and
// `get_shared_context`, under the compositor lock, which serialises the two
// threads that render; the shared context it hands out lives in the graph of
// the server compositor from then on and is used under that lock only, as the
// Metal device is. `create_context` asks the native display for a context; the
// display is a process-lifetime singleton of the native side
// (`ComStaticPtr<FrnGlDisplay>` in `native/FerroUI.Native/src/OSX/cgl.mm`)
// whose `CreateContext` reads nothing of the object, and the reference counts
// of native objects are atomic (`ComObject` in
// `native/FerroUI.Native/inc/comimpl.h`). The `Rc` handles the graphics keep
// (the display object of this file and the shared context) are cloned and
// released only by those callers, one at a time; the graphics themselves are
// held by the platform and the service locator of the UI thread, so their
// last handle is given back there.
unsafe impl Send for FerroNativeGlPlatformGraphics {}
// SAFETY: see `Send`.
unsafe impl Sync for FerroNativeGlPlatformGraphics {}

impl FerroNativeGlPlatformGraphics {
    /// Creates a context to find out the version of OpenGL the system
    /// gives, the entry points and the format of the default framebuffer,
    /// then the shared context.
    ///
    /// Fails, as the original throws, when the native display cannot create
    /// a context or make it current, and when the factory has no memory
    /// management helper.
    pub fn new(
        display: ComPtr<IFrnGlDisplay>,
        factory: &ComPtr<IFerroNativeFactory>,
    ) -> Result<FerroNativeGlPlatformGraphics, OpenGlError> {
        let context = display
            .create_context(None)
            .map_err(native_error("CreateContext"))?
            .ok_or_else(|| OpenGlError::new("CreateContext returned no context"))?;

        let gl_interface;
        let version;
        {
            let current = context.make_current().map_err(native_error("MakeCurrent"))?;
            let get_proc_address = proc_address_of(&display);
            // SAFETY: `GetProcAddress` of the native display resolves the
            // names with `dlsym` in the OpenGL library of the system, and the
            // context of that library created above is current on this
            // thread until `current` is released.
            let basic = unsafe { GlBasicInfoInterface::new(&get_proc_address) };
            let major = basic.get_integerv(GL_MAJOR_VERSION);
            let minor = basic.get_integerv(GL_MINOR_VERSION);
            let profile_mask = basic.get_integerv(GL_CONTEXT_PROFILE_MASK);
            let is_compatibility_profile =
                (profile_mask & GL_CONTEXT_COMPATIBILITY_PROFILE_BIT) == GL_CONTEXT_COMPATIBILITY_PROFILE_BIT;

            version = GlVersion::with_compatibility_profile(GlProfileType::OpenGL, major, minor, is_compatibility_profile);
            // SAFETY: as above.
            gl_interface = Rc::new(unsafe { GlInterface::new(version, proc_address_of(&display)) });
            drop(current);
        }

        let gl_display = Rc::new(GlDisplay::new(
            display.clone(),
            gl_interface,
            context.get_sample_count(),
            context.get_stencil_size(),
            factory.clone(),
        )?);
        let shared_context = GlContext::new(
            &gl_display,
            None,
            display
                .create_context(None)
                .map_err(native_error("CreateContext"))?
                .ok_or_else(|| OpenGlError::new("CreateContext returned no context"))?,
            version,
        );

        Ok(FerroNativeGlPlatformGraphics { display, gl_display, version, shared_context })
    }

    /// Whether contexts may share their objects: always.
    pub fn can_share_contexts(&self) -> bool {
        true
    }

    /// Whether the graphics create contexts: always.
    pub fn can_create_contexts(&self) -> bool {
        true
    }

    /// The display the contexts belong to.
    pub fn gl_display(&self) -> &Rc<GlDisplay> {
        &self.gl_display
    }

    /// The context every top-level renders with.
    pub fn shared_context(&self) -> &Rc<GlContext> {
        &self.shared_context
    }
}

impl IPlatformGraphics for FerroNativeGlPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        true
    }

    /// A new context of the display, which shares nothing with the shared
    /// context, as in the original.
    ///
    /// # Panics
    /// Panics when the native display cannot create a context.
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        let native = self.display.create_context(None).check();
        let Some(native) = native else {
            panic!("CreateContext returned no context");
        };
        GlContext::new(&self.gl_display, None, native, self.version)
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.shared_context.clone()
    }
}

/// The function that resolves the entry points of OpenGL through the native
/// display (`dlsym` in the OpenGL library of the system).
fn proc_address_of(display: &ComPtr<IFrnGlDisplay>) -> GetProcAddress {
    let display = display.clone();
    Rc::new(move |name: &str| -> *const c_void {
        let Ok(name) = CString::new(name) else {
            return std::ptr::null();
        };
        display.get_proc_address(Some(&name)) as *const c_void
    })
}

/// Turns the failure of a native call into the exception of the original.
fn native_error(call: &'static str) -> impl Fn(HResult) -> OpenGlError {
    move |error| OpenGlError::new(format!("{call} failed: {error:?}"))
}

/// The native OpenGL display and what every context of it shares: the
/// entry points, the format of the default framebuffer and the objects of
/// the native factory the contexts use.
pub struct GlDisplay {
    display: ComPtr<IFrnGlDisplay>,
    gl_interface: Rc<GlInterface>,
    sample_count: i32,
    stencil_size: i32,
    factory: ComPtr<IFerroNativeFactory>,
    memory_helper: ComPtr<IFrnNativeObjectsMemoryManagement>,
}

impl GlDisplay {
    /// Fails when the factory has no memory management helper.
    // The factory is kept, as in the original, for the import of shared
    // events (`ImportMTLSharedEvent`), which a frame of either thread may ask
    // for. That call has no state on the native side (it wraps the event in
    // an object of its own), the reference count of the factory is atomic,
    // and the reference held here is never its last one: the platform binds
    // the factory in the service locator of the UI thread for the life of
    // the process (`FerroNativePlatform::initialize_with_factory`). So the
    // destructor of the native factory, which must run on the UI thread
    // (DEVIATIONS.md, Native backend), never runs from here.
    fn new(
        display: ComPtr<IFrnGlDisplay>,
        gl_interface: Rc<GlInterface>,
        sample_count: i32,
        stencil_size: i32,
        factory: ComPtr<IFerroNativeFactory>,
    ) -> Result<GlDisplay, OpenGlError> {
        let memory_helper = factory
            .create_memory_management_helper()
            .map_err(native_error("CreateMemoryManagementHelper"))?
            .ok_or_else(|| OpenGlError::new("CreateMemoryManagementHelper returned no helper"))?;
        Ok(GlDisplay { display, sample_count, stencil_size, factory, gl_interface, memory_helper })
    }

    pub fn gl_interface(&self) -> &Rc<GlInterface> {
        &self.gl_interface
    }

    pub fn sample_count(&self) -> i32 {
        self.sample_count
    }

    pub fn stencil_size(&self) -> i32 {
        self.stencil_size
    }

    pub fn factory(&self) -> &ComPtr<IFerroNativeFactory> {
        &self.factory
    }

    pub fn memory_helper(&self) -> &ComPtr<IFrnNativeObjectsMemoryManagement> {
        &self.memory_helper
    }

    /// Makes no context current on this thread.
    pub fn clear_context(&self) {
        self.display.legacy_clear_current_context();
    }

    /// Creates a context that shares its objects with `share`.
    ///
    /// # Panics
    /// Panics when `share` is lost.
    pub fn create_shared_context(self: &Rc<Self>, share: &Rc<GlContext>) -> Result<Rc<GlContext>, OpenGlError> {
        let share_native = share.native();
        let native = self
            .display
            .create_context(Some(&share_native))
            .map_err(native_error("CreateContext"))?
            .ok_or_else(|| OpenGlError::new("CreateContext returned no context"))?;
        Ok(GlContext::new(self, Some(share.clone()), native, share.version()))
    }
}

/// A CGL context.
pub struct GlContext {
    this: Weak<GlContext>,
    display: Rc<GlDisplay>,
    shared_with: Option<Rc<GlContext>>,
    handle_wrap_feature: Rc<GpuHandleWrapFeature>,
    external_objects: Rc<GlExternalObjectsFeature>,
    context: RefCell<Option<ComPtr<IFrnGlContext>>>,
    version: GlVersion,
}

impl GlContext {
    pub(crate) fn new(
        display: &Rc<GlDisplay>,
        shared_with: Option<Rc<GlContext>>,
        context: ComPtr<IFrnGlContext>,
        version: GlVersion,
    ) -> Rc<GlContext> {
        Rc::new_cyclic(|this| GlContext {
            this: this.clone(),
            display: display.clone(),
            shared_with,
            handle_wrap_feature: Rc::new(GpuHandleWrapFeature::new(display.memory_helper().clone())),
            external_objects: Rc::new(GlExternalObjectsFeature::new(this.clone(), &context, display.clone())),
            context: RefCell::new(Some(context)),
            version,
        })
    }

    /// The native context; `None` once the context is disposed (lost).
    pub fn context(&self) -> Option<ComPtr<IFrnGlContext>> {
        self.context.borrow().clone()
    }

    /// The native context.
    ///
    /// # Panics
    /// Panics when the context is lost.
    #[track_caller]
    fn native(&self) -> ComPtr<IFrnGlContext> {
        self.throw_if_lost();
        self.context().expect("the context is not lost")
    }

    /// # Panics
    /// Panics when the context is lost (the
    /// `PlatformGraphicsContextLostException` of the original).
    #[track_caller]
    pub fn throw_if_lost(&self) {
        if self.is_lost() {
            panic!("{}", PlatformGraphicsContextLostError);
        }
    }

    /// Makes the context current, or says why it cannot be: for the callers
    /// that ignore a broken context.
    fn try_make_current(&self) -> Result<Rc<dyn IDisposable>, HResult> {
        let Some(native) = self.context() else {
            return Err(HResult::FAIL);
        };
        let saved = native.make_current()?;
        Ok(Rc::new(RestoreContext { saved: RefCell::new(saved) }))
    }
}

/// What making a context current returned: the native object that unlocks
/// the context and makes the context that was current before current again
/// when it is released.
struct RestoreContext {
    saved: RefCell<Option<ComPtr<IUnknown>>>,
}

impl IDisposable for RestoreContext {
    fn dispose(&self) {
        let saved = self.saved.borrow_mut().take();
        drop(saved);
    }
}

impl Drop for RestoreContext {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl IOptionalFeatureProvider for GlContext {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>() {
            let feature: Rc<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature> =
                self.handle_wrap_feature.clone();
            return Some(Rc::new(feature));
        }
        if feature_type == TypeId::of::<dyn IGlContextExternalObjectsFeature>() {
            let feature: Rc<dyn IGlContextExternalObjectsFeature> = self.external_objects.clone();
            return Some(Rc::new(feature));
        }
        // The context as an OpenGL context: what the original tests the
        // object for.
        if feature_type == TypeId::of::<dyn IGlContext>() {
            let this: Rc<dyn IGlContext> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsContext for GlContext {
    fn is_lost(&self) -> bool {
        self.context.borrow().is_none()
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        IGlContext::make_current(self)
    }

    fn dispose(&self) {
        let context = self.context.borrow_mut().take();
        drop(context);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlContext for GlContext {
    fn version(&self) -> GlVersion {
        self.version
    }

    fn gl_interface(&self) -> Rc<GlInterface> {
        self.display.gl_interface().clone()
    }

    fn sample_count(&self) -> i32 {
        self.display.sample_count()
    }

    fn stencil_size(&self) -> i32 {
        self.display.stencil_size()
    }

    /// # Panics
    /// Panics when the context is lost and when the native context cannot
    /// be made current.
    fn make_current(&self) -> Rc<dyn IDisposable> {
        self.throw_if_lost();
        self.try_make_current().check()
    }

    /// # Panics
    /// Panics when `context` is not a context of this backend (the failing
    /// cast of the original).
    fn is_shared_with(&self, context: &dyn IGlContext) -> bool {
        let Some(c) = context.as_any().downcast_ref::<GlContext>() else {
            panic!("Unable to cast the context to type 'GlContext'.");
        };
        let same = |a: &GlContext, b: &GlContext| std::ptr::eq(a, b);
        same(c, self)
            || c.shared_with.as_deref().is_some_and(|shared| same(shared, self))
            || self.shared_with.as_deref().is_some_and(|shared| same(shared, c))
            || match (&self.shared_with, &c.shared_with) {
                (Some(a), Some(b)) => same(a, b),
                _ => false,
            }
    }

    fn can_create_shared_context(&self) -> bool {
        true
    }

    /// Creates a context that shares its objects with this one (with the
    /// context this one shares with, when it does). The preferred versions
    /// are not used, as in the original. A failure is logged and gives
    /// `None` (the original throws), as for the contexts of GLX and EGL.
    fn create_shared_context(&self, _preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        let share = match &self.shared_with {
            Some(shared_with) => shared_with.clone(),
            None => self.this.upgrade()?,
        };
        match self.display.create_shared_context(&share) {
            Ok(context) => Some(context),
            Err(error) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    let source: &dyn Any = &"GlContext";
                    logger.log_with_values(Some(source), "Unable to create a shared context: {0}", &[&error]);
                }
                None
            }
        }
    }
}

/// The OpenGL render target of a top-level.
pub struct GlPlatformSurfaceRenderTarget {
    target: RefCell<Option<ComPtr<IFrnGlSurfaceRenderTarget>>>,
    context: Rc<dyn IGlContext>,
}

impl GlPlatformSurfaceRenderTarget {
    pub(crate) fn new(target: ComPtr<IFrnGlSurfaceRenderTarget>, context: Rc<dyn IGlContext>) -> Self {
        Self { target: RefCell::new(Some(target)), context }
    }
}

impl IPlatformRenderSurfaceRenderTarget for GlPlatformSurfaceRenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }
}

impl IGlPlatformSurfaceRenderTarget for GlPlatformSurfaceRenderTarget {
    /// # Panics
    /// Panics when the render target is disposed and when the native target
    /// cannot begin a frame.
    fn begin_draw(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        // The original: "TODO: use expectedPixelSize".
        let Some(target) = self.target.borrow().clone() else {
            panic!("Cannot access a disposed object: GlPlatformSurfaceRenderTarget");
        };
        let session = target.begin_drawing().check();
        let Some(session) = session else {
            panic!("BeginDrawing returned no session");
        };
        Rc::new(GlPlatformSurfaceRenderingSession::new(self.context.clone(), session))
    }

    fn dispose(&self) {
        let target = self.target.borrow_mut().take();
        drop(target);
    }
}

/// One frame drawn to the OpenGL render target of a top-level: the native
/// session made the context current and bound the framebuffer of the
/// IOSurface; releasing it presents the frame and restores the context.
pub struct GlPlatformSurfaceRenderingSession {
    context: Rc<dyn IGlContext>,
    session: RefCell<Option<ComPtr<IFrnGlSurfaceRenderingSession>>>,
}

impl GlPlatformSurfaceRenderingSession {
    pub(crate) fn new(context: Rc<dyn IGlContext>, session: ComPtr<IFrnGlSurfaceRenderingSession>) -> Self {
        Self { context, session: RefCell::new(Some(session)) }
    }

    /// Calls the native session where it is held.
    ///
    /// # Panics
    /// Panics when the session is disposed.
    #[track_caller]
    fn with_session<R>(&self, f: impl FnOnce(&IFrnGlSurfaceRenderingSession) -> R) -> R {
        match self.session.borrow().as_ref() {
            Some(session) => f(session),
            None => panic!("Cannot access a disposed object: GlPlatformSurfaceRenderingSession"),
        }
    }
}

impl IGlPlatformSurfaceRenderingSession for GlPlatformSurfaceRenderingSession {
    fn context(&self) -> Rc<dyn IGlContext> {
        self.context.clone()
    }

    fn size(&self) -> PixelSize {
        let s = self.with_session(|session| session.get_pixel_size()).check();
        PixelSize::new(s.width, s.height)
    }

    fn scaling(&self) -> f64 {
        self.with_session(|session| session.get_scaling()).check()
    }

    fn is_y_flipped(&self) -> bool {
        true
    }

    fn dispose(&self) {
        let session = self.session.borrow_mut().take();
        drop(session);
    }
}

/// The OpenGL render surface of a top-level.
///
/// The surface is shared with the thread that renders, like the Metal
/// surface (`metal.rs`): what it holds of the native side is the native
/// top-level, which only the UI thread may use, so the render target is
/// created on the UI thread, as in the original, and the surface is not
/// ready on any other thread.
pub struct GlPlatformSurface {
    weak_self: std::sync::Weak<GlPlatformSurface>,
    top_level: SurfaceTopLevel,
}

impl GlPlatformSurface {
    pub(crate) fn new(top_level: ComPtr<IFrnTopLevel>) -> Arc<GlPlatformSurface> {
        Arc::new_cyclic(|weak_self| GlPlatformSurface {
            weak_self: weak_self.clone(),
            top_level: SurfaceTopLevel::new(top_level),
        })
    }

    /// Releases the native top-level; called by the top-level when it is
    /// disposed, on the UI thread.
    pub(crate) fn close(&self) {
        self.top_level.release();
    }
}

impl IPlatformRenderSurface for GlPlatformSurface {
    /// Deviation (DEVIATIONS.md, Native backend), as for the Metal surface:
    /// the original throws `RenderTargetNotReadyException` from
    /// `CreateGlRenderTarget` off the UI thread and the composition target
    /// catches it; here the surface answers that it is not ready there.
    fn is_ready(&self) -> bool {
        self.top_level.get().is_some()
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() {
            // The contract hands out an `Rc`, and the surface lives in an
            // `Arc`: the view forwards to the surface.
            let this: Rc<dyn IGlPlatformSurface> = Rc::new(GlPlatformSurfaceView(self.weak_self.upgrade()?));
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlPlatformSurface for GlPlatformSurface {
    /// # Panics
    /// Panics when called off the UI thread or when the top-level is
    /// disposed (the render target is not ready), when `context` is not a
    /// context of this backend and when the native top-level cannot create
    /// the render target.
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        if !Dispatcher::ui_thread().check_access() {
            panic!("{}", RenderTargetNotReadyError::new());
        }
        let Some(top_level) = self.top_level.get() else {
            panic!("{}", RenderTargetNotReadyError::new());
        };
        let Some(native_context) = context.as_any().downcast_ref::<GlContext>() else {
            panic!("Unable to cast the context to type 'GlContext'.");
        };
        let native = native_context.context();
        let target = top_level.create_gl_render_target(native.as_deref()).check();
        let Some(target) = target else {
            panic!("CreateGlRenderTarget returned no render target");
        };
        Rc::new(GlPlatformSurfaceRenderTarget::new(target, context.clone()))
    }
}

/// The OpenGL surface as the contract hands it out
/// (`Rc<dyn IGlPlatformSurface>`); it forwards to the shared surface.
struct GlPlatformSurfaceView(Arc<GlPlatformSurface>);

impl IPlatformRenderSurface for GlPlatformSurfaceView {
    fn is_ready(&self) -> bool {
        self.0.is_ready()
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        self.0.try_get_surface_kind(kind)
    }

    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }
}

impl IGlPlatformSurface for GlPlatformSurfaceView {
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        self.0.create_gl_render_target(context)
    }
}

/// The external objects of a CGL context: the import of an IOSurface as a
/// rectangle texture and of a Metal shared event as a timeline semaphore.
// The original holds its context, which holds the feature: here the feature
// holds the context weakly, so that the two do not keep each other alive.
pub(crate) struct GlExternalObjectsFeature {
    context: Weak<GlContext>,
    display: Rc<GlDisplay>,
    device_luid: Option<Vec<u8>>,
}

impl GlExternalObjectsFeature {
    fn new(context: Weak<GlContext>, native: &IFrnGlContext, display: Rc<GlDisplay>) -> GlExternalObjectsFeature {
        let mut registry_id = 0u64;
        // SAFETY: the pointer is valid for the one value the native method
        // writes.
        let has_registry_id = unsafe { native.get_io_kit_registry_id(&mut registry_id) };
        // The bytes are reversed to match MoltenVK (LUID is a Vulkan term
        // after all): the bytes of the identifier, the most significant
        // first, as for the Metal device.
        let device_luid = has_registry_id.then(|| registry_id.to_be_bytes().to_vec());
        GlExternalObjectsFeature { context, display, device_luid }
    }

    /// # Panics
    /// Panics when the context is gone.
    #[track_caller]
    fn context(&self) -> Rc<GlContext> {
        match self.context.upgrade() {
            Some(context) => context,
            None => panic!("{}", PlatformGraphicsContextLostError),
        }
    }
}

impl IGlContextExternalObjectsFeature for GlExternalObjectsFeature {
    fn supported_importable_external_image_types(&self) -> Vec<String> {
        vec![KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF.to_string()]
    }

    fn supported_exportable_external_image_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn supported_importable_external_semaphore_types(&self) -> Vec<String> {
        vec![KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT.to_string()]
    }

    fn supported_exportable_external_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_supported_formats_for_external_memory_type(&self, _type_: &str) -> Vec<PlatformGraphicsExternalImageFormat> {
        vec![PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm]
    }

    fn create_image(
        &self,
        _type_: &str,
        _size: PixelSize,
        _format: PlatformGraphicsExternalImageFormat,
    ) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlError> {
        Err(OpenGlError::new("Specified method is not supported."))
    }

    fn create_semaphore(&self, _type_: &str) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlError> {
        Err(OpenGlError::new("Specified method is not supported."))
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Rc<dyn IGlExternalImageTexture>, OpenGlError> {
        if handle.handle_descriptor() == Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF) {
            if properties.format != PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm {
                return Err(OpenGlError::new("Only B8G8R8A8UNorm format is supported for IOSurfaceRef"));
            }
            let context = self.context();
            let current = context.ensure_current();
            let gl = context.gl_interface();
            let old_texture = gl.get_integerv(GL_TEXTURE_BINDING_RECTANGLE);
            let texture_id = gl.gen_texture();
            gl.bind_texture(GL_TEXTURE_RECTANGLE, texture_id);
            // SAFETY: by its descriptor the handle is an `IOSurfaceRef`,
            // which the caller keeps alive for the call (the compositor
            // hands over the handle the wrapping feature of the context
            // made of it); the texture CGL binds to it retains the surface.
            let error = unsafe {
                context.native().tex_image_io_surface2d(
                    GL_TEXTURE_RECTANGLE,
                    GL_RGBA8,
                    properties.width,
                    properties.height,
                    GL_BGRA,
                    GL_UNSIGNED_INT_8_8_8_8_REV,
                    handle.handle() as *mut c_void,
                    0,
                )
            };
            gl.bind_texture(GL_TEXTURE_RECTANGLE, old_texture);

            if error != 0 {
                gl.delete_texture(texture_id);
                current.dispose();
                return Err(OpenGlError::new(format!("CGLTexImageIOSurface2D returned {error}")));
            }
            current.dispose();
            return Ok(Rc::new(ImportedTexture::new(
                context,
                GL_TEXTURE_RECTANGLE,
                texture_id,
                GL_RGBA8,
                properties,
            )));
        }
        Err(OpenGlError::new("This handle type is not supported"))
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Result<Rc<dyn IGlExternalSemaphore>, OpenGlError> {
        if handle.handle_descriptor() == Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT) {
            // SAFETY: by its descriptor the handle is an
            // `id<MTLSharedEvent>`, which the caller keeps alive for the
            // call; the native side makes an object of its own that retains
            // the event.
            let imported = unsafe { self.display.factory().import_mtl_shared_event(handle.handle() as *mut c_void) }
                .map_err(native_error("ImportMTLSharedEvent"))?
                .ok_or_else(|| OpenGlError::new("ImportMTLSharedEvent returned no event"))?;
            return Ok(Rc::new(MtlEventSemaphore::new(imported)));
        }

        Err(OpenGlError::new("This handle type is not supported"))
    }

    fn get_synchronization_capabilities(
        &self,
        _image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC
            | CompositionGpuImportedImageSynchronizationCapabilities::TIMELINE_SEMAPHORES
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.device_luid.clone()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }
}

/// A Metal shared event imported as a timeline semaphore.
pub(crate) struct MtlEventSemaphore {
    inner: RefCell<Option<ComPtr<IFrnMTLSharedEvent>>>,
}

impl MtlEventSemaphore {
    fn new(inner: ComPtr<IFrnMTLSharedEvent>) -> MtlEventSemaphore {
        MtlEventSemaphore { inner: RefCell::new(Some(inner)) }
    }

    /// # Panics
    /// Panics when the semaphore is disposed.
    #[track_caller]
    fn inner(&self) -> ComPtr<IFrnMTLSharedEvent> {
        match self.inner.borrow().clone() {
            Some(inner) => inner,
            None => panic!("Cannot access a disposed object: MtlEventSemaphore"),
        }
    }
}

impl IGlExternalSemaphore for MtlEventSemaphore {
    /// # Panics
    /// Always: a shared event is a timeline semaphore.
    fn wait_semaphore(&self, _texture: &dyn IGlExternalImageTexture) {
        panic!("This is a timeline semaphore");
    }

    /// # Panics
    /// Always: a shared event is a timeline semaphore.
    fn signal_semaphore(&self, _texture: &dyn IGlExternalImageTexture) {
        panic!("This is a timeline semaphore");
    }

    fn wait_timeline_semaphore(&self, _texture: &dyn IGlExternalImageTexture, value: u64) {
        self.inner().wait(value, 1000);
    }

    fn signal_timeline_semaphore(&self, _texture: &dyn IGlExternalImageTexture, value: u64) {
        self.inner().set_signaled_value(value);
    }

    fn dispose(&self) {
        let inner = self.inner.borrow_mut().take();
        drop(inner);
    }
}

/// An IOSurface imported as a texture of a context.
pub(crate) struct ImportedTexture {
    context: Rc<GlContext>,
    type_: i32,
    id: i32,
    internal_format: i32,
    properties: PlatformGraphicsExternalImageProperties,
    disposed: Cell<bool>,
}

impl ImportedTexture {
    fn new(
        context: Rc<GlContext>,
        type_: i32,
        id: i32,
        internal_format: i32,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> ImportedTexture {
        ImportedTexture { context, type_, id, internal_format, properties, disposed: Cell::new(false) }
    }
}

impl IGlExternalImageTexture for ImportedTexture {
    /// # Panics
    /// Always: an IOSurface has no keyed mutex.
    fn acquire_keyed_mutex(&self, _key: u32) {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: an IOSurface has no keyed mutex.
    fn release_keyed_mutex(&self, _key: u32) {
        panic!("Specified method is not supported.");
    }

    fn texture_id(&self) -> i32 {
        self.id
    }

    fn internal_format(&self) -> i32 {
        self.internal_format
    }

    fn texture_type(&self) -> i32 {
        self.type_
    }

    fn properties(&self) -> PlatformGraphicsExternalImageProperties {
        self.properties.clone()
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        // A context that is lost or cannot be made current is likely
        // broken: the texture is left to it, as in the original.
        if let Ok(current) = self.context.try_make_current() {
            self.context.gl_interface().delete_texture(self.id);
            current.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream, which has no tests of these classes: the graphics
    // over the real native display, the OpenGL of the system and real
    // IOSurfaces and Metal shared events. None of this needs a window.
    use super::*;
    use crate::interop::create_ferro_native;
    use ferroui_base::platform::PlatformHandle;
    use std::ffi::c_char;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn graphics() -> Option<FerroNativeGlPlatformGraphics> {
        let factory = create_ferro_native().expect("factory");
        let display = factory.obtain_gl_display().ok().flatten()?;
        match FerroNativeGlPlatformGraphics::new(display, &factory) {
            Ok(graphics) => Some(graphics),
            Err(error) => {
                eprintln!("skipped: OpenGL is not available: {error}");
                None
            }
        }
    }

    fn gl_context(context: Rc<dyn IPlatformGraphicsContext>) -> Rc<dyn IGlContext> {
        let features: &dyn IOptionalFeatureProvider = &*context;
        features.try_get::<dyn IGlContext>().expect("an OpenGL context")
    }

    fn external_objects(context: &Rc<dyn IGlContext>) -> Rc<dyn IGlContextExternalObjectsFeature> {
        let features: &dyn IOptionalFeatureProvider = &**context;
        features.try_get::<dyn IGlContextExternalObjectsFeature>().expect("the feature")
    }

    /// The message of the panic `f` ends with.
    fn panic_message(f: impl FnOnce()) -> String {
        let payload = catch_unwind(AssertUnwindSafe(f)).expect_err("a panic");
        match payload.downcast::<String>() {
            Ok(message) => *message,
            Err(payload) => payload.downcast::<&str>().map(|message| message.to_string()).unwrap_or_default(),
        }
    }

    #[test]
    fn the_graphics_hand_out_one_shared_context_of_the_core_profile() {
        let Some(graphics) = graphics() else { return };

        assert!(graphics.uses_shared_context());
        assert!(graphics.can_share_contexts());
        assert!(graphics.can_create_contexts());
        let shared = gl_context(graphics.get_shared_context());
        assert!(Rc::ptr_eq(&shared, &gl_context(graphics.get_shared_context())));

        let version = shared.version();
        assert_eq!(GlProfileType::OpenGL, version.type_());
        assert!(version.major() > 3 || version.major() == 3 && version.minor() >= 2, "{version:?}");
        assert!(!version.is_compatibility_profile());

        // The entry points work with the context current.
        let current = shared.make_current();
        assert_eq!(version.major(), shared.gl_interface().get_integerv(GL_MAJOR_VERSION));
        current.dispose();
        assert!(shared.stencil_size() >= 8);
        assert!(shared.sample_count() >= 0);
    }

    #[test]
    fn contexts_share_with_the_context_they_were_created_from() {
        let Some(graphics) = graphics() else { return };
        let shared = gl_context(graphics.get_shared_context());
        let other = gl_context(graphics.create_context());

        assert!(shared.is_shared_with(&*shared));
        assert!(!shared.is_shared_with(&*other));
        assert!(!other.is_shared_with(&*shared));

        assert!(shared.can_create_shared_context());
        let child = shared.create_shared_context(None).expect("a shared context");
        assert!(child.is_shared_with(&*shared));
        assert!(shared.is_shared_with(&*child));
        // A context created from a shared one shares with the same context.
        let grandchild = child.create_shared_context(None).expect("a shared context");
        assert!(grandchild.is_shared_with(&*child));
        assert!(grandchild.is_shared_with(&*shared));
        assert!(!grandchild.is_shared_with(&*other));

        child.dispose();
        grandchild.dispose();
        other.dispose();
    }

    #[test]
    fn a_disposed_context_is_lost() {
        let Some(graphics) = graphics() else { return };
        let context = gl_context(graphics.create_context());
        assert!(!context.is_lost());
        context.dispose();
        assert!(context.is_lost());
        assert_eq!("The platform graphics context was lost.", panic_message(|| drop(context.make_current())));
    }

    #[test]
    fn the_external_objects_feature_names_what_it_imports() {
        let Some(graphics) = graphics() else { return };
        let context = gl_context(graphics.get_shared_context());
        let feature = external_objects(&context);

        assert_eq!(vec!["IOSurfaceRef".to_string()], feature.supported_importable_external_image_types());
        assert!(feature.supported_exportable_external_image_types().is_empty());
        assert_eq!(vec!["MetalSharedEvent".to_string()], feature.supported_importable_external_semaphore_types());
        assert!(feature.supported_exportable_external_semaphore_types().is_empty());
        assert_eq!(
            vec![PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm],
            feature.get_supported_formats_for_external_memory_type("IOSurfaceRef")
        );
        assert_eq!(
            CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC
                | CompositionGpuImportedImageSynchronizationCapabilities::TIMELINE_SEMAPHORES,
            feature.get_synchronization_capabilities("IOSurfaceRef")
        );
        assert_eq!(None, feature.device_uuid());
        if let Some(luid) = feature.device_luid() {
            assert_eq!(8, luid.len());
        }

        let size = PixelSize::new(4, 4);
        let format = PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm;
        assert!(feature.create_image("IOSurfaceRef", size, format).is_err());
        assert!(feature.create_semaphore("MetalSharedEvent").is_err());

        let event =
            Rc::new(PlatformHandle::new(1, Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT)));
        let error = feature.import_image(event, properties(format)).err().expect("refused");
        assert_eq!("This handle type is not supported", error.to_string());
        let surface =
            Rc::new(PlatformHandle::new(1, Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF)));
        let error = feature
            .import_image(surface.clone(), properties(PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm))
            .err()
            .expect("refused");
        assert_eq!("Only B8G8R8A8UNorm format is supported for IOSurfaceRef", error.to_string());
        let error = feature.import_semaphore(surface).err().expect("refused");
        assert_eq!("This handle type is not supported", error.to_string());
    }

    fn properties(format: PlatformGraphicsExternalImageFormat) -> PlatformGraphicsExternalImageProperties {
        PlatformGraphicsExternalImageProperties { width: 16, height: 8, format, ..Default::default() }
    }

    // IOSurface and Core Foundation, for a surface to import.
    type CFTypeRef = *const c_void;
    #[link(name = "IOSurface", kind = "framework")]
    extern "C" {
        static kIOSurfaceWidth: CFTypeRef;
        static kIOSurfaceHeight: CFTypeRef;
        static kIOSurfaceBytesPerElement: CFTypeRef;
        static kIOSurfacePixelFormat: CFTypeRef;
        fn IOSurfaceCreate(properties: CFTypeRef) -> CFTypeRef;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;
        fn CFDictionaryCreateMutable(
            allocator: CFTypeRef,
            capacity: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> *mut c_void;
        fn CFDictionarySetValue(dictionary: *mut c_void, key: CFTypeRef, value: CFTypeRef);
        fn CFNumberCreate(allocator: CFTypeRef, type_: isize, value: *const c_void) -> CFTypeRef;
        fn CFRelease(object: CFTypeRef);
    }

    /// A new IOSurface of 16 by 8 pixels, four bytes each, blue first.
    fn io_surface() -> CFTypeRef {
        const K_CF_NUMBER_SINT32_TYPE: isize = 3;
        // SAFETY: plain Core Foundation calls with valid arguments; every
        // object created here is released here, apart from the surface,
        // which the caller releases.
        unsafe {
            let dictionary = CFDictionaryCreateMutable(
                std::ptr::null(),
                0,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            );
            let pixel_format = u32::from_be_bytes(*b"BGRA") as i32;
            for (key, value) in [
                (kIOSurfaceWidth, 16i32),
                (kIOSurfaceHeight, 8),
                (kIOSurfaceBytesPerElement, 4),
                (kIOSurfacePixelFormat, pixel_format),
            ] {
                let number = CFNumberCreate(std::ptr::null(), K_CF_NUMBER_SINT32_TYPE, (&value as *const i32).cast());
                CFDictionarySetValue(dictionary, key, number);
                CFRelease(number);
            }
            let surface = IOSurfaceCreate(dictionary);
            CFRelease(dictionary);
            surface
        }
    }

    fn import(feature: &Rc<dyn IGlContextExternalObjectsFeature>, surface: CFTypeRef) -> Rc<dyn IGlExternalImageTexture> {
        let handle = Rc::new(PlatformHandle::new(
            surface as isize,
            Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF),
        ));
        feature
            .import_image(handle, properties(PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm))
            .expect("the surface is imported")
    }

    #[test]
    fn an_io_surface_is_imported_as_a_rectangle_texture() {
        let Some(graphics) = graphics() else { return };
        let context = gl_context(graphics.get_shared_context());
        let feature = external_objects(&context);
        let surface = io_surface();
        assert!(!surface.is_null());

        let texture = import(&feature, surface);
        assert_eq!(GL_TEXTURE_RECTANGLE, texture.texture_type());
        assert_eq!(GL_RGBA8, texture.internal_format());
        assert!(texture.texture_id() > 0);
        assert_eq!(16, texture.properties().width);
        assert_eq!(8, texture.properties().height);
        assert_eq!("Specified method is not supported.", panic_message(|| texture.acquire_keyed_mutex(0)));
        assert_eq!("Specified method is not supported.", panic_message(|| texture.release_keyed_mutex(0)));

        // The import put the binding of the context back.
        let current = context.make_current();
        assert_eq!(0, context.gl_interface().get_integerv(GL_TEXTURE_BINDING_RECTANGLE));
        current.dispose();

        texture.dispose();
        texture.dispose();
        // SAFETY: the surface was created above; the texture let go of it.
        unsafe { CFRelease(surface) };
    }

    // Metal and the Objective-C runtime, for a shared event to import.
    #[link(name = "Metal", kind = "framework")]
    extern "C" {
        fn MTLCreateSystemDefaultDevice() -> *mut c_void;
    }
    #[link(name = "objc")]
    extern "C" {
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
        fn objc_release(object: *mut c_void);
    }

    /// A new shared event of the default Metal device, or `None` without one.
    fn shared_event() -> Option<*mut c_void> {
        // SAFETY: `objc_msgSend` is called through a pointer with the C
        // signature of `newSharedEvent`; the device is a live object,
        // released here, and the event comes with the reference the caller
        // owns.
        unsafe {
            let device = MTLCreateSystemDefaultDevice();
            if device.is_null() {
                return None;
            }
            let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            let event = send(device, sel_registerName(c"newSharedEvent".as_ptr()));
            objc_release(device);
            (!event.is_null()).then_some(event)
        }
    }

    /// The `signaledValue` of a shared event.
    fn signaled_value(event: *mut c_void) -> u64 {
        // SAFETY: as in `shared_event`, with the signature of `signaledValue`.
        unsafe {
            let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> u64 =
                std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
            send(event, sel_registerName(c"signaledValue".as_ptr()))
        }
    }

    #[test]
    fn a_shared_event_is_imported_as_a_timeline_semaphore() {
        let Some(graphics) = graphics() else { return };
        let Some(event) = shared_event() else {
            eprintln!("skipped: no Metal device");
            return;
        };
        let context = gl_context(graphics.get_shared_context());
        let feature = external_objects(&context);
        let surface = io_surface();
        let texture = import(&feature, surface);

        let semaphore = feature
            .import_semaphore(Rc::new(PlatformHandle::new(
                event as isize,
                Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::METAL_SHARED_EVENT),
            )))
            .expect("the event is imported");
        semaphore.signal_timeline_semaphore(&*texture, 5);
        assert_eq!(5, signaled_value(event));
        // Signalled: the wait returns at once.
        semaphore.wait_timeline_semaphore(&*texture, 5);

        assert_eq!("This is a timeline semaphore", panic_message(|| semaphore.wait_semaphore(&*texture)));
        assert_eq!("This is a timeline semaphore", panic_message(|| semaphore.signal_semaphore(&*texture)));

        semaphore.dispose();
        assert_eq!(
            "Cannot access a disposed object: MtlEventSemaphore",
            panic_message(|| semaphore.signal_timeline_semaphore(&*texture, 6))
        );
        texture.dispose();
        // SAFETY: both were created above and are released once.
        unsafe {
            CFRelease(surface);
            objc_release(event);
        }
    }
}
