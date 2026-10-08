use super::open_gl_control_resources::OpenGlControlBaseResources;
use crate::composition::OpenGlCompositionInterop;
use crate::{GlInterface, GlVersion};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{CompositionSurfaceVisual, Compositor, ElementComposition};
use ferroui_base::rendering::IPresentationSource;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, PixelSize, Ref, StyledElementImpl, Vector, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Base class for controls that render using OpenGL.
/// Provides infrastructure for OpenGL context management, surface creation, and rendering lifecycle.
///
/// The control automatically manages OpenGL context creation, surface setup, and cleanup.
///
/// **Important:** Any interaction with [`GlInterface`] should only happen within the
/// `on_open_gl_init`, `on_open_gl_deinit`, or `on_open_gl_render` overrides.
///
/// The framework ensures proper OpenGL context synchronization and makes the context current
/// only during these calls. Accessing OpenGL functions outside of them may result in
/// undefined behavior, crashes, or rendering corruption.
///
/// The class is abstract: a deriving class implements `on_open_gl_render`.
// The original creates the OpenGL context with an asynchronous method, because the
// compositor answers from the render thread; the compositor of this crate answers at once
// (`OpenGlCompositionInterop::try_create_compatible_gl_context`), so the initialization has
// completed when `initialize` returns and its task is the result it returned.
//
// The failures the original catches and logs (the deinitialization of the user and the
// creation of the resources throwing) are panics of those calls here, and are not caught.
#[repr(C)]
pub struct OpenGlControlBase {
    base: Control,
    visual: RefCell<Option<CompositionSurfaceVisual>>,
    update_queued: Cell<bool>,
    /// The result of the initialization, once it has run (`_initialization`).
    initialization: Cell<Option<bool>>,
    resources: RefCell<Option<Rc<OpenGlControlBaseResources>>>,
    compositor: RefCell<Option<Rc<Compositor>>>,
    generation: Cell<i32>,
}

ferro_class! {
    OpenGlControlBase: Control, virtuals OpenGlControlBaseImpl: ControlImpl {
        /// Called when the OpenGL context is first created.
        ///
        /// `gl` is the interface for making OpenGL calls. Use [`GlInterface::get_proc_address`]
        /// to access additional APIs not covered by [`GlInterface`].
        fn on_open_gl_init(this, gl: &GlInterface);

        /// Called when the OpenGL context is being destroyed.
        ///
        /// `gl` is the interface for making OpenGL calls. Use [`GlInterface::get_proc_address`]
        /// to access additional APIs not covered by [`GlInterface`].
        fn on_open_gl_deinit(this, gl: &GlInterface);

        /// Called when the OpenGL context is lost and cannot be recovered.
        fn on_open_gl_lost(this);

        /// Called to render the OpenGL content for the current frame.
        ///
        /// `gl` is the interface for making OpenGL calls. Use [`GlInterface::get_proc_address`]
        /// to access additional APIs not covered by [`GlInterface`]. `fb` is the framebuffer
        /// ID to render into.
        fn on_open_gl_render(this, gl: &GlInterface, fb: i32);
    }
}
ferro_class_info!(OpenGlControlBase {});

ferro_impl_classes!(OpenGlControlBase: StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl OpenGlControlBaseImpl for OpenGlControlBase {
    fn on_open_gl_init(_this: &Self, _gl: &GlInterface) {}

    fn on_open_gl_deinit(_this: &Self, _gl: &GlInterface) {}

    fn on_open_gl_lost(_this: &Self) {}

    fn on_open_gl_render(_this: &Self, _gl: &GlInterface, _fb: i32) {
        panic!("OpenGlControlBase is abstract: 'on_open_gl_render' must be implemented by the deriving class")
    }
}

impl VisualImpl for OpenGlControlBase {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.do_cleanup();
        Self::parent_on_detached_from_visual_tree(this, e);
    }

    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let compositor = this.get_presentation_source().and_then(|source| source.renderer().compositor());
        *this.compositor.borrow_mut() = compositor;
        this.request_next_frame_rendering();
    }
}

impl FerroObjectImpl for OpenGlControlBase {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let visual = this.visual.borrow().clone();
        if let Some(visual) = visual {
            if change.property() == Visual::bounds_property().as_property() {
                let bounds = this.bounds();
                visual.set_size(Vector::new(bounds.width, bounds.height));
                this.request_next_frame_rendering();
            }
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl OpenGlControlBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            visual: RefCell::new(None),
            update_queued: Cell::new(false),
            initialization: Cell::new(None),
            resources: RefCell::new(None),
            compositor: RefCell::new(None),
            generation: Cell::new(0),
        }
    }

    fn is_initialized_successfully(&self) -> bool {
        self.initialization.get() == Some(true)
    }

    fn resources(&self) -> Option<Rc<OpenGlControlBaseResources>> {
        self.resources.borrow().clone()
    }

    /// Gets the OpenGL version information for the current context: `None` while the
    /// control has no context (the default value of the version in the original).
    pub fn gl_version(&self) -> Option<GlVersion> {
        self.resources().map(|resources| resources.context().version())
    }

    fn do_cleanup(&self) {
        self.generation.set(self.generation.get() + 1);
        if self.is_initialized_successfully() {
            if let Some(resources) = self.resources() {
                let context = resources.context();
                let current = context.ensure_current();
                self.on_open_gl_deinit(&context.gl_interface());
                current.dispose();
            }
        }

        let this: Ref<Visual> = self.to_ref().upcast();
        ElementComposition::set_element_child_visual(&this, None);

        self.update_queued.set(false);
        *self.visual.borrow_mut() = None;
        let resources = self.resources.borrow_mut().take();
        if let Some(resources) = resources {
            drop(resources.dispose_async());
        }
        self.initialization.set(None);
    }

    fn context_lost(&self) {
        self.initialization.set(None);
        let resources = self.resources.borrow_mut().take();
        if let Some(resources) = resources {
            drop(resources.dispose_async());
        }
        self.on_open_gl_lost();
    }

    /// The resources to draw with, once the control is initialized and its context is not
    /// lost; otherwise the initialization is started (again) and nothing is returned.
    fn ensure_initialized(&self) -> Option<Rc<OpenGlControlBaseResources>> {
        if self.initialization.get().is_some() {
            // Check if we've previously failed to initialize OpenGL on this platform
            // or if we are still waiting for init to complete
            if !self.is_initialized_successfully() {
                return None;
            }

            match self.resources() {
                Some(resources) if !resources.is_lost() => return Some(resources),
                _ => self.context_lost(),
            }
        }

        let initialized = self.initialize();
        self.initialization.set(Some(initialized));

        // `ContinueOnInitialization` of the original.
        if initialized {
            self.request_next_frame_rendering();
        }
        None
    }

    fn update(&self) {
        self.update_queued.set(false);
        let Some(source) = self.get_presentation_source() else {
            return;
        };
        let Some(resources) = self.ensure_initialized() else {
            return;
        };
        let frame = match resources.begin_draw(self.get_pixel_size(&*source)) {
            Ok(frame) => frame,
            Err(error) => panic!("{error}"),
        };
        self.on_open_gl_render(&resources.context().gl_interface(), resources.fbo());
        frame.dispose();
    }

    /// `InitializeAsync` of the original; see the note of the class.
    fn initialize(&self) -> bool {
        let Some(compositor) = self.compositor.borrow().clone() else {
            log_error("Unable to obtain Compositor instance");
            return false;
        };

        let generation = self.generation.get();
        let Some(context) = OpenGlCompositionInterop::try_create_compatible_gl_context(&compositor, None) else {
            log_error("Unable to initialize OpenGL: current platform doesn't support OpenGL interop with the compositor");
            return false;
        };

        if generation != self.generation.get() {
            // The control was detached while we were awaiting
            drop(context.dispose_async());
            return false;
        }

        let surface = compositor.create_drawing_surface();
        let resources = Rc::new(OpenGlControlBaseResources::new(context, surface));
        *self.resources.borrow_mut() = Some(resources.clone());

        let visual = compositor.create_surface_visual();
        let bounds = self.bounds();
        visual.set_size(Vector::new(bounds.width, bounds.height));
        visual.set_surface(Some((**resources.surface()).clone()));
        *self.visual.borrow_mut() = Some(visual.clone());
        let this: Ref<Visual> = self.to_ref().upcast();
        ElementComposition::set_element_child_visual(&this, Some((*visual).clone()));

        let gl_context = resources.context();
        let current = gl_context.make_current();
        self.on_open_gl_init(&gl_context.gl_interface());
        current.dispose();

        true
    }

    /// Requests that the control be rendered on the next frame; hides the member of the
    /// visual, as the original does.
    #[deprecated(note = "Use request_next_frame_rendering()")]
    pub fn invalidate_visual(&self) {
        self.request_next_frame_rendering()
    }

    /// Requests that the control be rendered on the next frame.
    pub fn request_next_frame_rendering(&self) {
        if (self.initialization.get().is_none() || self.is_initialized_successfully()) && !self.update_queued.get() {
            let compositor = self.compositor.borrow().clone();
            if let Some(compositor) = compositor {
                self.update_queued.set(true);
                let this = self.to_ref();
                compositor.request_composition_update(move || this.update());
            }
        }
    }

    fn get_pixel_size(&self, source: &dyn IPresentationSource) -> PixelSize {
        let scaling = source.render_scaling();
        let bounds = self.bounds();
        PixelSize::new(i32::max(1, (bounds.width * scaling) as i32), i32::max(1, (bounds.height * scaling) as i32))
    }
}

fn log_error(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
        let source: &dyn Any = &"OpenGlControlBase";
        logger.log(Some(source), message);
    }
}
