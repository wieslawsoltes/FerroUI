use ferroui_base::rendering::composition::ICompositionImportableSharedGpuContextImage;
use ferroui_base::utilities::ThreadBound;
use ferroui_base::PixelSize;
use ferroui_opengl::{ICompositionImportableOpenGlSharedTexture, IGlContext};
use std::any::Any;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

/// A texture of a context of the share group of the compositor, made for a
/// drawing surface of the compositor.
///
/// The texture is shared between the thread of the context it was made on
/// and the thread that renders. The identifier, the internal format and the
/// size are values both read; the identifier is atomic, since the disposal
/// clears it. The context is an object of the thread that made the texture
/// and stays bound to it.
pub struct GlSkiaSharedTextureForComposition {
    context: ThreadBound<Rc<dyn IGlContext>>,
    share_group: Arc<()>,
    texture_id: AtomicI32,
    internal_format: i32,
    size: PixelSize,
}

impl GlSkiaSharedTextureForComposition {
    /// Describes the texture `texture_id` of `context`.
    ///
    /// `share_group` stands for the context of the Skia GPU the texture was
    /// made for, which `context` was found to share with: see
    /// [`share_group`](Self::share_group).
    pub fn new(
        context: Rc<dyn IGlContext>,
        texture_id: i32,
        internal_format: i32,
        size: PixelSize,
        share_group: Arc<()>,
    ) -> Self {
        Self {
            context: ThreadBound::new(context),
            share_group,
            texture_id: AtomicI32::new(texture_id),
            internal_format,
            size,
        }
    }

    /// The context the texture was made on.
    ///
    /// # Panics
    /// Panics on a thread other than the one that made the texture.
    pub fn context(&self) -> &Rc<dyn IGlContext> {
        self.context.get()
    }

    /// Whether the current thread is the one of the context the texture was
    /// made on.
    pub fn is_context_on_thread(&self) -> bool {
        self.context.is_on_thread()
    }

    /// What stands for the context of the Skia GPU the texture was made for.
    ///
    // Deviation (DEVIATIONS.md, Skia backend): upstream asks
    // `Context.IsSharedWith` on the thread that imports the texture; the
    // context is bound to the thread that made the texture here, so that
    // thread asks when the texture is made and the answer is kept as this
    // token for a thread that cannot reach the context.
    pub fn share_group(&self) -> &Arc<()> {
        &self.share_group
    }

    /// Deletes the texture with `context`, a context of the share group that
    /// belongs to the calling thread (`Dispose(IGlContext)`).
    // The failure the original ignores (the context is lost while the
    // texture is deleted) is a panic of the calls into the context here, and
    // is not caught.
    pub fn dispose_with(&self, context: &dyn IGlContext) {
        // The identifier is taken by one caller only, as under the lock of
        // the original.
        let texture_id = self.texture_id.swap(0, Ordering::SeqCst);
        if texture_id == 0 {
            return;
        }

        let current = context.ensure_current();
        context.gl_interface().delete_texture(texture_id);
        current.dispose();
    }
}

impl ICompositionImportableSharedGpuContextImage for GlSkiaSharedTextureForComposition {
    /// Deletes the texture with the context it was made on.
    ///
    /// On a thread other than the one of that context nothing is deleted
    /// and the identifier is kept: that thread deletes the texture with a
    /// context of its own ([`dispose_with`](GlSkiaSharedTextureForComposition::dispose_with)),
    /// as the imported image does.
    fn dispose(&self) {
        if self.context.is_on_thread() {
            self.dispose_with(&**self.context.get());
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ICompositionImportableOpenGlSharedTexture for GlSkiaSharedTextureForComposition {
    fn texture_id(&self) -> i32 {
        self.texture_id.load(Ordering::SeqCst)
    }

    fn internal_format(&self) -> i32 {
        self.internal_format
    }

    fn size(&self) -> PixelSize {
        self.size
    }
}

const _: fn() = || {
    // The texture is what the caller of the sharing feature keeps and what
    // the render thread imports.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<GlSkiaSharedTextureForComposition>();
};

#[cfg(test)]
mod tests {
    // Not from upstream, which has no tests of the class.
    use super::*;
    use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext};
    use ferroui_base::reactive::{Disposable, IDisposable};
    use ferroui_opengl::gl_consts::GL_RGBA8;
    use ferroui_opengl::{GetProcAddress, GlInterface, GlProfileType, GlVersion};
    use std::any::TypeId;
    use std::cell::RefCell;
    use std::ffi::{c_char, c_void};

    thread_local! {
        static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn record(call: impl Into<String>) {
        CALLS.with(|calls| calls.borrow_mut().push(call.into()));
    }

    fn take_calls() -> Vec<String> {
        CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
    }

    extern "system" fn get_string(_name: i32) -> *const c_char {
        c"".as_ptr()
    }

    extern "system" fn delete_textures(count: i32, textures: *const i32) {
        // SAFETY: the interface passes `count` names.
        let names = (0..count.max(0) as usize).map(|i| unsafe { *textures.add(i) }.to_string()).collect::<Vec<_>>();
        record(format!("glDeleteTextures({})", names.join(",")));
    }

    /// Stands in for the entry points the tests never call. Only its address
    /// is used: an entry point has to resolve to something for the table to
    /// load.
    extern "system" fn never_called() {
        unreachable!("an entry point without a scripted implementation was called");
    }

    fn resolve(name: &str) -> *const c_void {
        match name {
            "glGetString" => {
                let entry_point: extern "system" fn(i32) -> *const c_char = get_string;
                entry_point as *const c_void
            }
            "glDeleteTextures" => {
                let entry_point: extern "system" fn(i32, *const i32) = delete_textures;
                entry_point as *const c_void
            }
            _ => {
                let entry_point: extern "system" fn() = never_called;
                entry_point as *const c_void
            }
        }
    }

    /// A context that records when it is made current and what is deleted
    /// with it.
    struct FakeGlContext {
        name: &'static str,
        gl: Rc<GlInterface>,
    }

    impl FakeGlContext {
        fn new(name: &'static str) -> Rc<FakeGlContext> {
            let version = GlVersion::new(GlProfileType::OpenGLES, 3, 0);
            let loader: GetProcAddress = Rc::new(resolve);
            // SAFETY: the loader resolves to functions of this module with
            // the signatures of the entry points they stand in for; the ones
            // without an implementation are never called by the tests.
            let gl = unsafe { GlInterface::new(version, loader) };
            Rc::new(FakeGlContext { name, gl: Rc::new(gl) })
        }
    }

    impl IOptionalFeatureProvider for FakeGlContext {
        fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
    }

    impl IPlatformGraphicsContext for FakeGlContext {
        fn is_lost(&self) -> bool {
            false
        }

        fn ensure_current(&self) -> Rc<dyn IDisposable> {
            let name = self.name;
            record(format!("{name}: current"));
            Disposable::create(move || record(format!("{name}: restored")))
        }

        fn dispose(&self) {}

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl IGlContext for FakeGlContext {
        fn version(&self) -> GlVersion {
            GlVersion::new(GlProfileType::OpenGLES, 3, 0)
        }

        fn gl_interface(&self) -> Rc<GlInterface> {
            self.gl.clone()
        }

        fn sample_count(&self) -> i32 {
            0
        }

        fn stencil_size(&self) -> i32 {
            0
        }

        fn make_current(&self) -> Rc<dyn IDisposable> {
            self.ensure_current()
        }

        fn is_shared_with(&self, _context: &dyn IGlContext) -> bool {
            true
        }

        fn can_create_shared_context(&self) -> bool {
            false
        }

        fn create_shared_context(&self, _preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
            None
        }
    }

    fn texture(context: &Rc<FakeGlContext>, texture_id: i32) -> GlSkiaSharedTextureForComposition {
        take_calls();
        GlSkiaSharedTextureForComposition::new(
            context.clone(),
            texture_id,
            GL_RGBA8,
            PixelSize::new(30, 20),
            Arc::new(()),
        )
    }

    #[test]
    fn the_texture_describes_what_it_was_made_with() {
        let context = FakeGlContext::new("owner");
        let share_group = Arc::new(());
        let texture = GlSkiaSharedTextureForComposition::new(
            context.clone(),
            7,
            GL_RGBA8,
            PixelSize::new(30, 20),
            share_group.clone(),
        );

        assert_eq!(7, texture.texture_id());
        assert_eq!(GL_RGBA8, texture.internal_format());
        assert_eq!(PixelSize::new(30, 20), texture.size());
        assert!(texture.is_context_on_thread());
        assert!(std::ptr::addr_eq(Rc::as_ptr(&context), Rc::as_ptr(texture.context())));
        assert!(Arc::ptr_eq(&share_group, texture.share_group()));
    }

    #[test]
    fn dispose_deletes_the_texture_with_its_own_context_once() {
        let context = FakeGlContext::new("owner");
        let texture = texture(&context, 7);

        ICompositionImportableSharedGpuContextImage::dispose(&texture);

        assert_eq!(vec!["owner: current", "glDeleteTextures(7)", "owner: restored"], take_calls());
        assert_eq!(0, texture.texture_id());

        ICompositionImportableSharedGpuContextImage::dispose(&texture);
        texture.dispose_with(&*context);

        assert!(take_calls().is_empty());
    }

    #[test]
    fn dispose_with_deletes_the_texture_with_the_context_of_the_caller() {
        let owner = FakeGlContext::new("owner");
        let other = FakeGlContext::new("other");
        let texture = texture(&owner, 9);

        texture.dispose_with(&*other);

        assert_eq!(vec!["other: current", "glDeleteTextures(9)", "other: restored"], take_calls());
        assert_eq!(0, texture.texture_id());

        ICompositionImportableSharedGpuContextImage::dispose(&texture);

        assert!(take_calls().is_empty());
    }

    #[test]
    fn another_thread_does_not_reach_the_context_and_keeps_the_texture() {
        let context = FakeGlContext::new("owner");
        let texture = Arc::new(texture(&context, 5));

        let moved = texture.clone();
        let (on_thread, texture_id) = std::thread::spawn(move || {
            let image: Arc<dyn ICompositionImportableSharedGpuContextImage> = moved;
            image.dispose();
            let texture = image.as_any().downcast_ref::<GlSkiaSharedTextureForComposition>().expect("the texture");
            (texture.is_context_on_thread(), texture.texture_id())
        })
        .join()
        .unwrap();

        assert!(!on_thread);
        assert_eq!(5, texture_id);
        assert!(take_calls().is_empty());

        // The thread of the context still deletes it.
        ICompositionImportableSharedGpuContextImage::dispose(&*texture);

        assert_eq!(vec!["owner: current", "glDeleteTextures(5)", "owner: restored"], take_calls());
    }
}
