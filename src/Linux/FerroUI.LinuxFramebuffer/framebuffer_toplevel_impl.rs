//! The one top-level of the platform (the port of
//! `FramebufferToplevelImpl.cs`): it covers the output and has no handle,
//! no frame, no cursor and no popups.

use crate::input::{IInputBackend, IScreenInfoProvider};
use crate::output::IOutputBackend;
use crate::raw_event_grouping::{IRawEventGrouperDispatchQueue, RawEvent, RawEventGrouper};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::{IInputRoot, IMouseDevice, MouseDevice};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider, ISurfaceOrientation, SurfaceOrientation};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{PixelPoint, Point, Rect, Size};
use ferroui_controls::platform::{IPlatformHandle, IPopupImpl, ITopLevelImpl, PlatformThemeVariant};
use ferroui_controls::{AcrylicPlatformCompensationLevels, WindowResizeReason, WindowTransparencyLevel};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

type InputCallback = Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>;

/// The callbacks of the top-level, which the input queue reaches too.
#[derive(Default)]
struct Callbacks {
    input: RefCell<Option<InputCallback>>,
    paint: RefCell<Option<Rc<dyn Fn(Rect)>>>,
    resized: RefCell<Option<Rc<dyn Fn(Size, WindowResizeReason)>>>,
    scaling_changed: RefCell<Option<Rc<dyn Fn(f64)>>>,
    transparency_level_changed: RefCell<Option<Rc<dyn Fn(WindowTransparencyLevel)>>>,
    closed: RefCell<Option<Rc<dyn Fn()>>>,
    lost_focus: RefCell<Option<Rc<dyn Fn()>>>,
}

pub struct FramebufferToplevelImpl {
    output_backend: Arc<dyn IOutputBackend>,
    input_backend: Rc<dyn IInputBackend>,
    input_queue: Rc<RawEventGrouper>,
    compositor: Option<Rc<Compositor>>,
    input_root: RefCell<Option<Rc<dyn IInputRoot>>>,
    mouse_device: Rc<MouseDevice>,
    callbacks: Rc<Callbacks>,
}

impl FramebufferToplevelImpl {
    /// Creates the top-level over an output and starts the input backend.
    /// `queue` is the raw event queue of the platform and `compositor`
    /// its compositor (statics of the platform in the reference).
    pub fn new(
        output_backend: Arc<dyn IOutputBackend>,
        input_backend: Rc<dyn IInputBackend>,
        queue: Option<Rc<dyn IRawEventGrouperDispatchQueue>>,
        compositor: Option<Rc<Compositor>>,
    ) -> Rc<Self> {
        let callbacks = Rc::new(Callbacks::default());
        let input_queue = {
            let callbacks = callbacks.clone();
            Rc::new(RawEventGrouper::new(
                move |grouped_input| {
                    let input = callbacks.input.borrow().clone();
                    if let Some(input) = input {
                        input(grouped_input);
                    }
                },
                queue,
            ))
        };

        let this = Rc::new(Self {
            output_backend,
            input_backend,
            input_queue,
            compositor,
            input_root: RefCell::new(None),
            mouse_device: MouseDevice::new(),
            callbacks,
        });

        // The backend calls on the UI thread (the reference posts each
        // event to it here, with the priority `Send`).
        let input_queue = this.input_queue.clone();
        let screen: Rc<dyn IScreenInfoProvider> = this.clone();
        this.input_backend.initialize(screen, Rc::new(move |e: RawEvent| input_queue.handle_event(e)));
        this
    }

    pub fn input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.input_root.borrow().clone()
    }

    pub fn mouse_device(&self) -> Rc<dyn IMouseDevice> {
        self.mouse_device.clone()
    }

    pub fn frame_size(&self) -> Option<Size> {
        None
    }
}

impl IScreenInfoProvider for FramebufferToplevelImpl {
    fn scaled_size(&self) -> Size {
        self.output_backend.pixel_size().to_size(self.render_scaling())
    }

    fn as_surface_orientation(&self) -> Option<&dyn ISurfaceOrientation> {
        Some(self)
    }
}

impl ISurfaceOrientation for FramebufferToplevelImpl {
    fn orientation(&self) -> SurfaceOrientation {
        match self.output_backend.as_surface_orientation() {
            Some(surface_orientation) => surface_orientation.orientation(),
            None => SurfaceOrientation::Rotation0,
        }
    }
}

impl IOptionalFeatureProvider for FramebufferToplevelImpl {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IDisposable for FramebufferToplevelImpl {
    /// # Panics
    /// Always: the top-level of the platform lives as long as the
    /// process, as in the reference.
    fn dispose(&self) {
        panic!("Specified method is not supported.");
    }
}

impl ITopLevelImpl for FramebufferToplevelImpl {
    fn desktop_scaling(&self) -> f64 {
        1.0
    }

    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>> {
        None
    }

    fn client_size(&self) -> Size {
        self.scaled_size()
    }

    fn render_scaling(&self) -> f64 {
        self.output_backend.scaling()
    }

    fn surfaces(&self) -> Vec<Arc<dyn IPlatformRenderSurface>> {
        vec![self.output_backend.clone().as_render_surface()]
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        self.compositor.clone()
    }

    fn input(&self) -> Option<InputCallback> {
        self.callbacks.input.borrow().clone()
    }

    fn set_input(&self, value: Option<InputCallback>) {
        *self.callbacks.input.borrow_mut() = value;
    }

    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>> {
        self.callbacks.paint.borrow().clone()
    }

    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>) {
        *self.callbacks.paint.borrow_mut() = value;
    }

    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>> {
        self.callbacks.resized.borrow().clone()
    }

    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>) {
        *self.callbacks.resized.borrow_mut() = value;
    }

    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>> {
        self.callbacks.scaling_changed.borrow().clone()
    }

    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>) {
        *self.callbacks.scaling_changed.borrow_mut() = value;
    }

    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>> {
        self.callbacks.transparency_level_changed.borrow().clone()
    }

    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>) {
        *self.callbacks.transparency_level_changed.borrow_mut() = value;
    }

    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>) {
        *self.input_root.borrow_mut() = Some(input_root.clone());
        self.input_backend.set_input_root(input_root);
    }

    fn point_to_client(&self, point: PixelPoint) -> Point {
        point.to_point(1.0)
    }

    fn point_to_screen(&self, point: Point) -> PixelPoint {
        PixelPoint::from_point(point, 1.0)
    }

    fn set_cursor(&self, _cursor: Option<Rc<dyn ICursorImpl>>) {}

    fn closed(&self) -> Option<Rc<dyn Fn()>> {
        self.callbacks.closed.borrow().clone()
    }

    fn set_closed(&self, value: Option<Rc<dyn Fn()>>) {
        *self.callbacks.closed.borrow_mut() = value;
    }

    fn lost_focus(&self) -> Option<Rc<dyn Fn()>> {
        self.callbacks.lost_focus.borrow().clone()
    }

    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>) {
        *self.callbacks.lost_focus.borrow_mut() = value;
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

}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use crate::input::null_input::NullInputBackend;
    use crate::native_unsafe_methods::fb_var_screeninfo;
    use crate::output::{FbDevOutputOptions, FbdevOutput, FileFbDevice};

    #[test]
    fn the_client_size_is_the_size_of_the_output_divided_by_its_scaling() {
        let path = std::env::temp_dir().join(format!("ferroui-linuxfb-toplevel-{}.fb", std::process::id()));
        let var_info = fb_var_screeninfo { xres: 320, yres: 240, bits_per_pixel: 32, ..Default::default() };
        let device = FileFbDevice::create(&path.to_string_lossy(), var_info).unwrap();
        let options = FbDevOutputOptions { scaling: 2.0, ..Default::default() };
        let output = FbdevOutput::with_device(Box::new(device), options).unwrap();
        let top_level = FramebufferToplevelImpl::new(output.clone(), Rc::new(NullInputBackend::new()), None, None);

        assert_eq!(top_level.render_scaling(), 2.0);
        assert_eq!(top_level.client_size(), Size::new(160.0, 120.0));
        assert_eq!(top_level.scaled_size(), Size::new(160.0, 120.0));
        assert_eq!(top_level.desktop_scaling(), 1.0);
        assert_eq!(top_level.orientation(), SurfaceOrientation::Rotation0);
        assert!(top_level.handle().is_none() && top_level.create_popup().is_none() && top_level.frame_size().is_none());
        assert_eq!(top_level.surfaces().len(), 1);
        assert!(top_level.surfaces()[0].as_framebuffer_surface().is_some());
        assert_eq!(top_level.point_to_client(PixelPoint::new(3, 4)), Point::new(3.0, 4.0));
        assert_eq!(top_level.point_to_screen(Point::new(3.0, 4.0)), PixelPoint::new(3, 4));

        output.set_scaling(1.0);
        assert_eq!(top_level.client_size(), Size::new(320.0, 240.0));
        let _ = std::fs::remove_file(&path);
    }
}
