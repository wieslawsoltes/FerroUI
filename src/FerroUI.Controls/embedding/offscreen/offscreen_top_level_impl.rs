use crate::platform::{IPlatformHandle, IPopupImpl, ITopLevelImpl, PlatformThemeVariant};
use crate::{AcrylicPlatformCompensationLevels, WindowResizeReason, WindowTransparencyLevel};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::{IInputRoot, IMouseDevice};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{PixelPoint, Point, Rect, Size};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The abstract and the overridable members of an offscreen top-level
/// implementation: what a class deriving from
/// [`OffscreenTopLevelImplBase`] provides.
///
/// This API is unstable.
pub trait OffscreenTopLevelImplOverrides {
    /// The surfaces the top-level is rendered to.
    fn surfaces(&self) -> Vec<std::sync::Arc<dyn IPlatformRenderSurface>>;

    /// The mouse device of the top-level.
    fn mouse_device(&self) -> Rc<dyn IMouseDevice>;

    /// Disposes the implementation. An override calls
    /// [`OffscreenTopLevelImplBase::base_dispose`], as this does.
    fn dispose(&self, base: &OffscreenTopLevelImplBase) {
        base.base_dispose();
    }

    /// The scaling of the desktop the top-level is on; the render scaling
    /// by default.
    fn desktop_scaling(&self, base: &OffscreenTopLevelImplBase) -> f64 {
        base.render_scaling()
    }

    /// Converts a point from screen to client coordinates.
    fn point_to_client(&self, point: PixelPoint) -> Point {
        point.to_point(1.0)
    }

    /// Converts a point from client to screen coordinates.
    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::from_point(point, 1.0)
    }

    /// Sets the cursor associated with the top-level; ignored by default.
    fn set_cursor(&self, _cursor: Option<Rc<dyn ICursorImpl>>) {}

    /// Queries for an optional feature; none by default.
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

/// The base of the top-level implementations that are not backed by a
/// window of the platform: the client size and the scaling are set by the
/// owner, which also feeds the input and provides the surfaces.
///
/// This API is unstable.
pub struct OffscreenTopLevelImplBase {
    overrides: Rc<dyn OffscreenTopLevelImplOverrides>,
    scaling: Cell<f64>,
    client_size: Cell<Size>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    is_disposed: Cell<bool>,
    compositor: Rc<Compositor>,
    input: RefCell<Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>>,
    paint: RefCell<Option<Rc<dyn Fn(Rect)>>>,
    resized: RefCell<Option<Rc<dyn Fn(Size, WindowResizeReason)>>>,
    scaling_changed: RefCell<Option<Rc<dyn Fn(f64)>>>,
    transparency_level_changed: RefCell<Option<Rc<dyn Fn(WindowTransparencyLevel)>>>,
    closed: RefCell<Option<Rc<dyn Fn()>>>,
    lost_focus: RefCell<Option<Rc<dyn Fn()>>>,
}

impl OffscreenTopLevelImplBase {
    /// Creates the implementation over the members of the deriving class,
    /// with a compositor of its own.
    pub fn new(overrides: Rc<dyn OffscreenTopLevelImplOverrides>) -> Rc<Self> {
        Rc::new(Self {
            overrides,
            scaling: Cell::new(1.0),
            client_size: Cell::new(Size::default()),
            input_root: RefCell::new(None),
            is_disposed: Cell::new(false),
            compositor: Compositor::new(None, false),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            transparency_level_changed: RefCell::new(None),
            closed: RefCell::new(None),
            lost_focus: RefCell::new(None),
        })
    }

    /// The members of the deriving class.
    pub fn overrides(&self) -> &Rc<dyn OffscreenTopLevelImplOverrides> {
        &self.overrides
    }

    /// The input root the top-level set.
    pub fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    /// Whether the implementation has been disposed.
    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    /// The `Dispose` of the base class.
    pub fn base_dispose(&self) {
        self.is_disposed.set(true);
    }

    /// The compositor of the top-level.
    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// The client size of the top-level.
    pub fn client_size(&self) -> Size {
        self.client_size.get()
    }

    /// The size of the frame of the top-level: an offscreen top-level has none.
    pub fn frame_size(&self) -> Option<Size> {
        None
    }

    /// Sets the client size and notifies the top-level.
    pub fn set_client_size(&self, value: Size) {
        self.client_size.set(value);
        let resized = self.resized.borrow().clone();
        if let Some(resized) = resized {
            resized(value, WindowResizeReason::Unspecified);
        }
    }

    /// The scaling factor of the top-level.
    pub fn render_scaling(&self) -> f64 {
        self.scaling.get()
    }

    /// Sets the scaling factor and notifies the top-level.
    pub fn set_render_scaling(&self, value: f64) {
        self.scaling.set(value);
        let scaling_changed = self.scaling_changed.borrow().clone();
        if let Some(scaling_changed) = scaling_changed {
            scaling_changed(value);
        }
    }

    /// The mouse device of the top-level.
    pub fn mouse_device(&self) -> Rc<dyn IMouseDevice> {
        self.overrides.mouse_device()
    }
}

impl IDisposable for OffscreenTopLevelImplBase {
    fn dispose(&self) {
        self.overrides.dispose(self);
    }
}

impl IOptionalFeatureProvider for OffscreenTopLevelImplBase {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        self.overrides.try_get_feature(feature_type)
    }
}

impl ITopLevelImpl for OffscreenTopLevelImplBase {
    fn desktop_scaling(&self) -> f64 {
        self.overrides.desktop_scaling(self)
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        None
    }

    fn client_size(&self) -> Size {
        self.client_size.get()
    }

    fn render_scaling(&self) -> f64 {
        self.scaling.get()
    }

    fn surfaces(&self) -> Vec<std::sync::Arc<dyn IPlatformRenderSurface>> {
        self.overrides.surfaces()
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.compositor.clone())
    }

    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>> {
        self.input.borrow().clone()
    }

    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>) {
        *self.input.borrow_mut() = value;
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        self.paint.borrow().clone()
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        *self.paint.borrow_mut() = value;
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        self.resized.borrow().clone()
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        *self.resized.borrow_mut() = value;
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        self.scaling_changed.borrow().clone()
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        *self.scaling_changed.borrow_mut() = value;
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.transparency_level_changed.borrow().clone()
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        *self.transparency_level_changed.borrow_mut() = value;
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        *self.input_root.borrow_mut() = Some(input_root);
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        self.overrides.point_to_client(point)
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        self.overrides.point_to_screen(point)
    }

    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        self.overrides.set_cursor(cursor)
    }

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        self.closed.borrow().clone()
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.closed.borrow_mut() = value;
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        self.lost_focus.borrow().clone()
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        *self.lost_focus.borrow_mut() = value;
    }

    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>> {
        None
    }

    fn set_transparency_level_hint(&self, _transparency_levels: &[WindowTransparencyLevel]) {}

    fn transparency_level(&self) -> WindowTransparencyLevel {
        WindowTransparencyLevel::none()
    }

    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels {
        AcrylicPlatformCompensationLevels::new(1.0, 1.0, 1.0)
    }

    fn set_frame_theme_variant(&self, _theme_variant: Option<PlatformThemeVariant>) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
