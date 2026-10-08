//! Port of `Pages/OpenGl/OpenGlInteropPage.xaml.cs`: the class of the document
//! `Pages/OpenGl/OpenGlInteropPage.xaml`.
//!
//! The original creates the OpenGL context with an asynchronous method, because the
//! compositor answers from the render thread; the compositor of the port answers at once
//! (`OpenGlCompositionInterop::try_create_compatible_gl_context`), so `initialize` has
//! completed when it returns. The disposals the original awaits last (`DisposeAsync` of
//! the context) are started and their tasks dropped: nothing follows them.

use super::open_gl::{GlPageKnobs, OpenGlContent};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::{
    CompositionDrawingSurface, CompositionSurfaceVisual, ElementComposition, ServerJobTask,
};
use ferroui_base::{
    ferro_class_info, instantiate, BoxedValue, OwnedFerroPropertyChangedEventArgs, PixelSize, Ref, Vector,
    VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{ContentPage, Control, TopLevel};
use ferroui_opengl::composition::{ICompositionGlContext, ICompositionGlTexture, OpenGlCompositionInterop};
use ferroui_opengl::gl_consts::*;
use ferroui_opengl::{GlInterface, GlProfileType};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Demonstrates the `ICompositionGlContext` API: an OpenGL context compatible with the compositor
/// and a texture presented to a `CompositionDrawingSurface`, without using `OpenGlControlBase`.
#[repr(C)]
pub struct OpenGlInteropPage {
    base: ContentPage,
    context: RefCell<Option<Rc<dyn ICompositionGlContext>>>,
    texture: RefCell<Option<Rc<dyn ICompositionGlTexture>>>,
    last_present: RefCell<Option<ServerJobTask<()>>>,
    surface: RefCell<Option<CompositionDrawingSurface>>,
    visual: RefCell<Option<CompositionSurfaceVisual>>,
    content: RefCell<Option<OpenGlContent>>,
    fbo: Cell<i32>,
    depth_buffer: Cell<i32>,
    depth_buffer_size: Cell<PixelSize>,
    update_queued: Cell<bool>,
    attached: Cell<bool>,
    generation: Cell<i32>,
    /// The subscription of `on_viewport_size_changed`, which `initialize` adds and `cleanup` removes.
    viewport_size_changed: Cell<Option<RoutedEventHandlerToken>>,
}

content_page_class!(OpenGlInteropPage);
ferro_class_info!(OpenGlInteropPage {
    new: OpenGlInteropPage::new,
    markup: {
        methods: [
            fn ViewportAttachedToVisualTree(Option<BoxedValue>, VisualTreeAttachmentEventArgs) =>
                |this: &Ref<OpenGlInteropPage>, sender: Option<BoxedValue>, e: VisualTreeAttachmentEventArgs| {
                    this.viewport_attached_to_visual_tree(&sender, &e)
                },
            fn ViewportDetachedFromVisualTree(Option<BoxedValue>, VisualTreeAttachmentEventArgs) =>
                |this: &Ref<OpenGlInteropPage>, sender: Option<BoxedValue>, e: VisualTreeAttachmentEventArgs| {
                    this.viewport_detached_from_visual_tree(&sender, &e)
                },
            fn KnobsPropertyChanged(Option<BoxedValue>, OwnedFerroPropertyChangedEventArgs) =>
                |this: &Ref<OpenGlInteropPage>, sender: Option<BoxedValue>, e: OwnedFerroPropertyChangedEventArgs| {
                    this.knobs_property_changed(&sender, &e)
                },
        ],
    },
});
xaml_class!(OpenGlInteropPage, "/Pages/OpenGl/OpenGlInteropPage.xaml");

impl OpenGlInteropPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
            context: RefCell::new(None),
            texture: RefCell::new(None),
            last_present: RefCell::new(None),
            surface: RefCell::new(None),
            visual: RefCell::new(None),
            content: RefCell::new(None),
            fbo: Cell::new(0),
            depth_buffer: Cell::new(0),
            depth_buffer_size: Cell::new(PixelSize::default()),
            update_queued: Cell::new(false),
            attached: Cell::new(false),
            generation: Cell::new(0),
            viewport_size_changed: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// The element `Viewport` of the document.
    ///
    /// # Panics
    /// Panics if the document does not name the element (a null reference in the managed original).
    fn viewport(&self) -> Ref<Control> {
        self.find_control::<Control>("Viewport").expect("the element Viewport")
    }

    /// The element `Knobs` of the document.
    ///
    /// # Panics
    /// Panics if the document does not name the element (a null reference in the managed original).
    fn knobs(&self) -> Ref<GlPageKnobs> {
        self.find_control::<GlPageKnobs>("Knobs").expect("the element Knobs")
    }

    fn viewport_attached_to_visual_tree(&self, _sender: &Option<BoxedValue>, _e: &VisualTreeAttachmentEventArgs) {
        self.attached.set(true);
        self.initialize();
    }

    fn viewport_detached_from_visual_tree(&self, _sender: &Option<BoxedValue>, _e: &VisualTreeAttachmentEventArgs) {
        self.attached.set(false);
        self.cleanup();
    }

    /// # Panics
    /// Panics if the model of the content cannot be read or its shaders do not build (an
    /// exception of the asynchronous method in the managed original).
    fn initialize(&self) {
        self.generation.set(self.generation.get() + 1);
        let generation = self.generation.get();
        let viewport = self.viewport();
        let Some(element_visual) = ElementComposition::get_element_visual(&viewport) else {
            return;
        };
        let compositor = element_visual.compositor().clone();

        let Some(context) = OpenGlCompositionInterop::try_create_compatible_gl_context(&compositor, None) else {
            self.knobs().set_info("Compositor OpenGL interop is not available on this platform".to_string());
            return;
        };

        if !self.attached.get() || generation != self.generation.get() {
            // The viewport got detached or reinitialized while we were awaiting
            drop(context.dispose_async());
            return;
        }

        *self.context.borrow_mut() = Some(context.clone());
        let surface = compositor.create_drawing_surface();
        let visual = compositor.create_surface_visual();
        visual.set_surface(Some((*surface).clone()));
        *self.surface.borrow_mut() = Some(surface);
        *self.visual.borrow_mut() = Some(visual.clone());
        ElementComposition::set_element_child_visual(&viewport, Some((*visual).clone()));

        let gl_context = context.gl_context();
        let current = gl_context.make_current();
        let gl = gl_context.gl_interface();
        self.fbo.set(gl.gen_framebuffer());
        let mut content = OpenGlContent::new();
        content.init(&gl, gl_context.version());
        current.dispose();

        let info = content.info().to_string();
        *self.content.borrow_mut() = Some(content);
        self.knobs().set_info(info);
        // The handler belongs to the viewport, an element of the page: it holds the page weakly.
        let weak = self.to_ref().downgrade();
        self.viewport_size_changed.set(Some(viewport.size_changed(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.on_viewport_size_changed();
            }
        })));
        self.queue_update();
    }

    // The failures the original catches when it frees the objects of a context that is
    // likely lost are panics of the calls into the context here, and are not caught.
    fn cleanup(&self) {
        self.generation.set(self.generation.get() + 1);
        let viewport = self.viewport();
        if let Some(token) = self.viewport_size_changed.take() {
            viewport.remove_handler(Control::size_changed_event(), token);
        }
        ElementComposition::set_element_child_visual(&viewport, None);
        *self.visual.borrow_mut() = None;

        let context = self.context.borrow_mut().take();
        let Some(context) = context else {
            return;
        };

        let gl_context = context.gl_context();
        let current = gl_context.make_current();
        let gl = gl_context.gl_interface();
        if let Some(content) = self.content.borrow().as_ref() {
            content.deinit(&gl);
        }
        if self.fbo.get() != 0 {
            gl.delete_framebuffer(self.fbo.get());
        }
        if self.depth_buffer.get() != 0 {
            gl.delete_renderbuffer(self.depth_buffer.get());
        }
        current.dispose();

        *self.content.borrow_mut() = None;
        self.fbo.set(0);
        self.depth_buffer.set(0);
        self.depth_buffer_size.set(PixelSize::default());
        *self.texture.borrow_mut() = None;
        *self.last_present.borrow_mut() = None;
        let surface = self.surface.borrow_mut().take();
        if let Some(surface) = surface {
            surface.dispose();
        }

        // Also disposes the textures created from it
        drop(context.dispose_async());
    }

    fn on_viewport_size_changed(&self) {
        self.queue_update();
    }

    fn knobs_property_changed(&self, _sender: &Option<BoxedValue>, e: &OwnedFerroPropertyChangedEventArgs) {
        if e.property() == GlPageKnobs::yaw_property().as_property()
            || e.property() == GlPageKnobs::roll_property().as_property()
            || e.property() == GlPageKnobs::pitch_property().as_property()
            || e.property() == GlPageKnobs::disco_property().as_property()
        {
            self.queue_update();
        }
    }

    fn queue_update(&self) {
        if self.update_queued.get() {
            return;
        }
        let Some(context) = self.context.borrow().clone() else {
            return;
        };
        self.update_queued.set(true);
        // The update keeps the page alive, as the delegate of the original does.
        let this = self.to_ref();
        context.compositor().request_composition_update(move || this.update());
    }

    fn update(&self) {
        self.update_queued.set(false);
        let context = self.context.borrow().clone();
        let visual = self.visual.borrow().clone();
        let surface = self.surface.borrow().clone();
        let (Some(context), Some(visual), Some(surface)) = (context, visual, surface) else {
            return;
        };
        if self.content.borrow().is_none() {
            return;
        }

        if !context.is_valid_for_interop() {
            // The context is no longer usable for presentation, drop everything and start over
            self.cleanup();
            self.initialize();
            return;
        }

        let viewport = self.viewport();
        let knobs = self.knobs();
        let scaling = TopLevel::get_top_level(Some(&viewport)).map_or(1.0, |top_level| top_level.render_scaling());
        let bounds = viewport.bounds();
        let size = PixelSize::new(
            i32::max(1, (bounds.width * scaling) as i32),
            i32::max(1, (bounds.height * scaling) as i32),
        );
        visual.set_size(Vector::new(bounds.width, bounds.height));

        let texture = self.texture.borrow().clone();
        let texture = match texture {
            Some(texture)
                if texture.size() != size
                    || self.last_present.borrow().as_ref().is_some_and(|present| present.is_faulted()) =>
            {
                drop(texture.dispose_async());
                *self.texture.borrow_mut() = None;
                *self.last_present.borrow_mut() = None;
                None
            }
            texture => texture,
        };

        let texture = match texture {
            None => {
                let texture = context.create_texture(&surface, size);
                *self.texture.borrow_mut() = Some(texture.clone());
                texture
            }
            Some(texture) => {
                if !texture.is_ready_for_draw() {
                    // The previous frame hasn't reached the render thread yet, try again on the next composition update
                    self.queue_update();
                    return;
                }
                texture
            }
        };

        let gl_context = context.gl_context();
        let current = gl_context.make_current();
        let gl = gl_context.gl_interface();
        let lease = texture.begin_draw();
        let info = lease.texture_info();
        gl.bind_framebuffer(GL_FRAMEBUFFER, self.fbo.get());
        self.update_depth_buffer(&gl, size);
        gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, info.target, info.texture_id, 0);

        if let Some(content) = self.content.borrow().as_ref() {
            content.on_open_gl_render(
                &gl,
                self.fbo.get(),
                size,
                knobs.yaw(),
                knobs.pitch(),
                knobs.roll(),
                knobs.disco(),
            );
        }

        gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, info.target, 0, 0);
        gl.bind_framebuffer(GL_FRAMEBUFFER, 0);
        *self.last_present.borrow_mut() = Some(lease.present_async());
        lease.dispose();
        current.dispose();

        if f64::from(knobs.disco()) > 0.01 {
            self.queue_update();
        }
    }

    /// # Panics
    /// Panics if the page has no context (a null reference in the managed original).
    fn update_depth_buffer(&self, gl: &GlInterface, size: PixelSize) {
        if size == self.depth_buffer_size.get() && self.depth_buffer.get() != 0 {
            return;
        }

        let old_render_buffer = gl.get_integerv(GL_RENDERBUFFER_BINDING);
        if self.depth_buffer.get() != 0 {
            gl.delete_renderbuffer(self.depth_buffer.get());
        }

        self.depth_buffer.set(gl.gen_renderbuffer());
        gl.bind_renderbuffer(GL_RENDERBUFFER, self.depth_buffer.get());
        let version = self.context.borrow().as_ref().expect("the context of the page").gl_context().version();
        gl.renderbuffer_storage(
            GL_RENDERBUFFER,
            if version.type_() == GlProfileType::OpenGLES { GL_DEPTH_COMPONENT16 } else { GL_DEPTH_COMPONENT },
            size.width,
            size.height,
        );
        gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, self.depth_buffer.get());
        gl.bind_renderbuffer(GL_RENDERBUFFER, old_render_buffer);
        self.depth_buffer_size.set(size);
    }
}
