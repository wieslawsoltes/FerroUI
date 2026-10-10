//! A top-level over a surface a host owns.

use ferroui_base::input::platform::IClipboard;
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::text_input::ITextInputMethodImpl;
use ferroui_base::input::IInputRoot;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider, IPlatformGraphics, PlatformThemeVariant};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Point, Rect, Size};
use ferroui_controls::platform::{IPlatformHandle, IPopupImpl, ITopLevelImpl};
use ferroui_controls::{AcrylicPlatformCompensationLevels, WindowResizeReason, WindowTransparencyLevel};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// A minimal top-level implementation that hosts a content tree on top of
/// an externally managed swap-chain surface (e.g. a swap chain panel of
/// another toolkit or any other host that supplies an OpenGL surface).
/// Sizing, scaling and input pumping are driven by the host.
#[allow(dead_code)] // Created by a host of another toolkit, which the port does not have yet; the tests construct it.
pub(crate) struct SwapChainTopLevelImpl {
    gl_surface: Arc<dyn IPlatformRenderSurface>,
    client_size: Cell<Size>,
    scaling: Cell<f64>,
    compositor: Rc<Compositor>,
    input: RefCell<Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>>,
    paint: RefCell<Option<Rc<dyn Fn(Rect)>>>,
    resized: RefCell<Option<Rc<dyn Fn(Size, WindowResizeReason)>>>,
    scaling_changed: RefCell<Option<Rc<dyn Fn(f64)>>>,
    transparency_level_changed: RefCell<Option<Rc<dyn Fn(WindowTransparencyLevel)>>>,
    closed: RefCell<Option<Rc<dyn Fn()>>>,
    lost_focus: RefCell<Option<Rc<dyn Fn()>>>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    cursor_changed: RefCell<Option<Rc<dyn Fn(Option<Rc<dyn ICursorImpl>>)>>>,
    text_input_method: RefCell<Option<Rc<dyn ITextInputMethodImpl>>>,
}

#[allow(dead_code)]
impl SwapChainTopLevelImpl {
    /// `gl_surface` is the surface of the host, which answers as an OpenGL
    /// surface (the reference takes it by that type).
    pub fn new(gl_surface: Arc<dyn IPlatformRenderSurface>) -> Rc<SwapChainTopLevelImpl> {
        let platform_graphics = FerroLocator::current().get_service::<Arc<dyn IPlatformGraphics>>().map(|graphics| (*graphics).clone());
        Rc::new(SwapChainTopLevelImpl {
            gl_surface,
            client_size: Cell::new(Size::default()),
            scaling: Cell::new(1.0),
            compositor: Compositor::new(platform_graphics, false),
            input: RefCell::new(None),
            paint: RefCell::new(None),
            resized: RefCell::new(None),
            scaling_changed: RefCell::new(None),
            transparency_level_changed: RefCell::new(None),
            closed: RefCell::new(None),
            lost_focus: RefCell::new(None),
            input_root: RefCell::new(None),
            cursor_changed: RefCell::new(None),
            text_input_method: RefCell::new(None),
        })
    }

    /// The host sets the size, which is reported as a resize.
    pub fn set_client_size(&self, value: Size) {
        self.client_size.set(value);
        let resized = self.resized.borrow().clone();
        if let Some(resized) = resized {
            resized(value, WindowResizeReason::Unspecified);
        }
    }

    /// The host sets the scaling, which is reported as a change.
    pub fn set_render_scaling(&self, value: f64) {
        self.scaling.set(value);
        let scaling_changed = self.scaling_changed.borrow().clone();
        if let Some(scaling_changed) = scaling_changed {
            scaling_changed(value);
        }
    }

    pub fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    /// Raised when the framework requests a cursor change. The host is
    /// responsible for translating the (host-supplied) cursor into a
    /// native cursor and applying it to its surface.
    pub fn cursor_changed(&self) -> Option<Rc<dyn Fn(Option<Rc<dyn ICursorImpl>>)>> {
        self.cursor_changed.borrow().clone()
    }

    pub fn set_cursor_changed(&self, value: Option<Rc<dyn Fn(Option<Rc<dyn ICursorImpl>>)>>) {
        *self.cursor_changed.borrow_mut() = value;
    }

    /// Optional IME implementation provided by the host.
    pub fn text_input_method(&self) -> Option<Rc<dyn ITextInputMethodImpl>> {
        self.text_input_method.borrow().clone()
    }

    pub fn set_text_input_method(&self, value: Option<Rc<dyn ITextInputMethodImpl>>) {
        *self.text_input_method.borrow_mut() = value;
    }
}

impl IOptionalFeatureProvider for SwapChainTopLevelImpl {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IClipboard>() {
            return FerroLocator::current().get_service::<dyn IClipboard>().map(|clipboard| Rc::new(clipboard) as Rc<dyn Any>);
        }
        if feature_type == TypeId::of::<dyn ITextInputMethodImpl>() {
            return self.text_input_method.borrow().clone().map(|method| Rc::new(method) as Rc<dyn Any>);
        }
        None
    }
}

impl IDisposable for SwapChainTopLevelImpl {
    fn dispose(&self) {
        let closed = self.closed.borrow().clone();
        if let Some(closed) = closed {
            closed();
        }
    }
}

impl ITopLevelImpl for SwapChainTopLevelImpl {
    fn desktop_scaling(&self) -> f64 {
        self.scaling.get()
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

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        vec![self.gl_surface.clone()]
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
        point.to_point(self.scaling.get())
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::from_point(point, self.scaling.get())
    }

    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>) {
        let cursor_changed = self.cursor_changed.borrow().clone();
        if let Some(cursor_changed) = cursor_changed {
            cursor_changed(cursor);
        }
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

    // Uses overlays instead of popups.
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

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the top-level.
    use super::*;

    struct Surface;

    impl IPlatformRenderSurface for Surface {
        fn try_get_surface_kind(&self, _kind: TypeId) -> Option<Rc<dyn Any>> {
            None
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    /// A render timer that never ticks.
    struct Timer;

    impl ferroui_base::rendering::IRenderTimer for Timer {
        fn tick(&self) -> Option<ferroui_base::rendering::RenderTimerTick> {
            None
        }

        fn set_tick(&self, _value: Option<ferroui_base::rendering::RenderTimerTick>) {}

        fn runs_in_background(&self) -> bool {
            true
        }
    }

    #[test]
    fn the_host_drives_size_scaling_cursor_and_closing() {
        // The compositor of the top-level asks the services of the thread
        // for the render loop, which a platform has bound.
        let render_loop: Arc<dyn ferroui_base::rendering::IRenderLoop> =
            ferroui_base::rendering::RenderLoop::from_timer(Arc::new(Timer));
        FerroLocator::current_mutable().bind::<Arc<dyn ferroui_base::rendering::IRenderLoop>>().to_constant(Rc::new(render_loop));

        let surface: Arc<dyn IPlatformRenderSurface> = Arc::new(Surface);
        let top_level = SwapChainTopLevelImpl::new(surface.clone());

        // What a top-level without a window answers.
        assert!(top_level.handle().is_none());
        assert!(top_level.create_popup().is_none());
        assert_eq!(1.0, top_level.render_scaling());
        assert_eq!(1.0, top_level.desktop_scaling());
        assert_eq!(Size::default(), top_level.client_size());
        assert!(top_level.transparency_level() == WindowTransparencyLevel::none());
        assert!(top_level.compositor().is_some());
        let surfaces = top_level.surfaces();
        assert_eq!(1, surfaces.len());
        assert!(Arc::ptr_eq(&surfaces[0], &surface));

        // The size and the scaling of the host are reported.
        let resized = Rc::new(RefCell::new(Vec::new()));
        let scaled = Rc::new(RefCell::new(Vec::new()));
        {
            let resized = resized.clone();
            top_level.set_resized(Some(Rc::new(move |size, reason| resized.borrow_mut().push((size, reason)))));
            let scaled = scaled.clone();
            top_level.set_scaling_changed(Some(Rc::new(move |scaling| scaled.borrow_mut().push(scaling))));
        }
        top_level.set_client_size(Size::new(320.0, 200.0));
        top_level.set_render_scaling(2.0);
        assert_eq!(vec![(Size::new(320.0, 200.0), WindowResizeReason::Unspecified)], *resized.borrow());
        assert_eq!(vec![2.0], *scaled.borrow());
        assert_eq!(Size::new(320.0, 200.0), top_level.client_size());
        assert_eq!(2.0, top_level.desktop_scaling());

        // Points are scaled; there is no screen position.
        assert_eq!(Point::new(5.0, 10.0), top_level.point_to_client(PixelPoint::new(10, 20)));
        assert_eq!(PixelPoint::new(10, 20), top_level.point_to_screen(Point::new(5.0, 10.0)));

        // The cursor goes to the host; disposing closes.
        let cursors = Rc::new(Cell::new(0));
        let closed = Rc::new(Cell::new(0));
        {
            let cursors = cursors.clone();
            top_level.set_cursor_changed(Some(Rc::new(move |cursor| {
                assert!(cursor.is_none());
                cursors.set(cursors.get() + 1);
            })));
            let closed = closed.clone();
            top_level.set_closed(Some(Rc::new(move || closed.set(closed.get() + 1))));
        }
        top_level.set_cursor(None);
        assert_eq!(1, cursors.get());
        top_level.dispose();
        assert_eq!(1, closed.get());

        // The text input method is the one of the host, when it gave one.
        assert!(top_level.try_get_feature(TypeId::of::<dyn ITextInputMethodImpl>()).is_none());
        assert!(top_level.text_input_method().is_none());
        assert!(top_level.input_root().is_none());
    }
}
