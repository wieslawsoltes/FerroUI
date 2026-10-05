use crate::interop::canvas_helper::CanvasSurface;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{PixelSize, Size};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The state every surface of a view has: its canvas, its compositor and
/// its size in device pixels and logical units.
pub struct BrowserSurface {
    compositor: Rc<Compositor>,
    js_surface: RefCell<Option<CanvasSurface>>,
    scaling: Cell<f64>,
    client_size: Cell<Size>,
    render_size: Cell<PixelSize>,
    size_changed: HandlerList<dyn Fn()>,
    scaling_changed: HandlerList<dyn Fn()>,
}

impl BrowserSurface {
    /// Creates the surface state for a canvas of the page.
    pub fn new(js_surface: CanvasSurface, compositor: Rc<Compositor>) -> Self {
        Self {
            compositor,
            js_surface: RefCell::new(Some(js_surface)),
            scaling: Cell::new(1.0),
            client_size: Cell::new(Size::new(1.0, 1.0)),
            render_size: Cell::new(PixelSize::new(1, 1)),
            size_changed: HandlerList::new(),
            scaling_changed: HandlerList::new(),
        }
    }

    /// The compositor that renders to the surface.
    pub fn compositor(&self) -> Rc<Compositor> {
        self.compositor.clone()
    }

    /// The scaling from logical units to device pixels.
    pub fn scaling(&self) -> f64 {
        self.scaling.get()
    }

    /// The size in logical units.
    pub fn client_size(&self) -> Size {
        self.client_size.get()
    }

    /// The size in device pixels.
    pub fn render_size(&self) -> PixelSize {
        self.render_size.get()
    }

    /// Whether the surface has a size it can be rendered at.
    pub fn is_valid(&self) -> bool {
        let render_size = self.render_size.get();
        render_size.width > 0 && render_size.height > 0 && self.scaling.get() > 0.0
    }

    /// Subscribes to changes of the client size.
    pub fn size_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.size_changed.add(handler)
    }

    /// Subscribes to changes of the scaling.
    pub fn scaling_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.scaling_changed.add(handler)
    }

    /// The size and the scaling the canvas has right now, as
    /// `(pixel width, pixel height, scaling)`; `None` once disposed.
    pub fn initial_size(&self) -> Option<(f64, f64, f64)> {
        let js_surface = self.js_surface.borrow();
        let js_surface = js_surface.as_ref()?;
        Some((js_surface.width(), js_surface.height(), js_surface.scaling()))
    }

    /// Releases the canvas surface.
    pub fn dispose(&self) {
        let js_surface = self.js_surface.borrow_mut().take();
        if let Some(js_surface) = js_surface {
            CanvasSurface::destroy(&js_surface);
        }
        self.render_size.set(PixelSize::default());
        self.client_size.set(Size::default());
    }

    /// Records the size of the canvas and raises the change events.
    ///
    /// `pixel_width` and `pixel_height` are in device pixels; `dpr` is the
    /// device pixel ratio.
    pub fn on_size_changed(&self, pixel_width: f64, pixel_height: f64, dpr: f64) {
        let old_scaling = self.scaling.get();
        let old_client_size = self.client_size.get();
        self.render_size.set(PixelSize::new(pixel_width as i32, pixel_height as i32));
        self.client_size.set(self.render_size.get().to_size(dpr));
        self.scaling.set(dpr);
        if old_client_size != self.client_size.get() {
            for (_, handler) in self.size_changed.snapshot().iter() {
                handler();
            }
        }
        if (old_scaling - dpr).abs() > 0.0001 {
            for (_, handler) in self.scaling_changed.snapshot().iter() {
                handler();
            }
        }
    }
}
