use super::{
    IPlatformHandle, IPopupImpl, IWin32OptionsTopLevelImpl, IWindowBaseImpl, IWindowImpl, PlatformThemeVariant,
};
use crate::{AcrylicPlatformCompensationLevels, WindowResizeReason, WindowTransparencyLevel};
use ferroui_base::input::raw::IRawInputEventArgs;
use ferroui_base::input::IInputRoot;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{ICursorImpl, IOptionalFeatureProvider};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::{PixelPoint, Point, Rect, Size};
use std::any::Any;
use std::rc::Rc;

/// Defines a platform-specific top-level window implementation.
///
/// This is the common contract of [`IWindowImpl`] and [`IPopupImpl`].
///
/// The callback members (`input`, `paint`, `resized`, ...) are set by the
/// top-level that owns the implementation and called by the platform; each
/// one is a getter/setter pair holding an optional callback.
///
/// Optional features are queried through the
/// [`IOptionalFeatureProvider`] supertrait:
/// `(top_level as &dyn IOptionalFeatureProvider).try_get::<dyn IInputPane>()`.
pub trait ITopLevelImpl: IOptionalFeatureProvider + IDisposable {
    /// Gets the scaling factor for window positioning and sizing.
    fn desktop_scaling(&self) -> f64;

    /// Get the platform handle.
    fn handle(&self) -> Option<Rc<dyn IPlatformHandle>>;

    /// Gets the client size of the toplevel.
    fn client_size(&self) -> Size;

    /// Gets the scaling factor for the toplevel. This is used for rendering.
    fn render_scaling(&self) -> f64;

    /// The list of native platform's surfaces that can be consumed by
    /// rendering subsystems.
    ///
    /// The rendering platform checks that list and sees if it can utilize
    /// one of them to output. It should be enough to expose a native window
    /// handle surface and add support for a framebuffer (even if it's an
    /// emulated one) via the framebuffer platform surface.
    fn surfaces(&self) -> Vec<Rc<dyn IPlatformRenderSurface>>;

    /// Gets the compositor that is used for the toplevel, if the platform
    /// renders through one.
    fn compositor(&self) -> Option<Rc<Compositor>> {
        None
    }

    /// Gets the method called when the toplevel receives input.
    fn input(&self) -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>;

    /// Sets a method called when the toplevel receives input.
    fn set_input(&self, value: Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>);

    /// Gets the method called when the toplevel requires painting.
    fn paint(&self) -> Option<Rc<dyn Fn(Rect)>>;

    /// Sets a method called when the toplevel requires painting.
    fn set_paint(&self, value: Option<Rc<dyn Fn(Rect)>>);

    /// Gets the method called when the toplevel is resized.
    fn resized(&self) -> Option<Rc<dyn Fn(Size, WindowResizeReason)>>;

    /// Sets a method called when the toplevel is resized.
    fn set_resized(&self, value: Option<Rc<dyn Fn(Size, WindowResizeReason)>>);

    /// Gets the method called when the toplevel's scaling changes.
    fn scaling_changed(&self) -> Option<Rc<dyn Fn(f64)>>;

    /// Sets a method called when the toplevel's scaling changes.
    fn set_scaling_changed(&self, value: Option<Rc<dyn Fn(f64)>>);

    /// Gets the method called when the toplevel's transparency level
    /// changes.
    fn transparency_level_changed(&self) -> Option<Rc<dyn Fn(WindowTransparencyLevel)>>;

    /// Sets a method called when the toplevel's transparency level changes.
    fn set_transparency_level_changed(&self, value: Option<Rc<dyn Fn(WindowTransparencyLevel)>>);

    /// Gets the platform-specific scene info, which is passed to the render
    /// target of the toplevel.
    fn platform_specific_scene_info(&self) -> Option<std::sync::Arc<dyn Any + Send + Sync>> {
        None
    }

    /// Gets the method called when the platform-specific scene info
    /// changes.
    fn platform_specific_scene_info_changed(&self) -> Option<Rc<dyn Fn(Option<std::sync::Arc<dyn Any + Send + Sync>>)>> {
        None
    }

    /// Sets a method called when the platform-specific scene info changes.
    fn set_platform_specific_scene_info_changed(&self, _value: Option<Rc<dyn Fn(Option<std::sync::Arc<dyn Any + Send + Sync>>)>>) {}

    /// Sets the input root for the toplevel.
    fn set_input_root(&self, input_root: Rc<dyn IInputRoot>);

    /// Converts a point from screen to client coordinates.
    fn point_to_client(&self, point: PixelPoint) -> Point;

    /// Converts a point from client to screen coordinates.
    fn point_to_screen(&self, point: Point) -> PixelPoint;

    /// Sets the cursor associated with the toplevel; `None` stands for the
    /// default cursor.
    fn set_cursor(&self, cursor: Option<Rc<dyn ICursorImpl>>);

    /// Gets the method called when the underlying implementation is
    /// destroyed.
    fn closed(&self) -> Option<Rc<dyn Fn()>>;

    /// Sets a method called when the underlying implementation is
    /// destroyed.
    fn set_closed(&self, value: Option<Rc<dyn Fn()>>);

    /// Gets the method called when the input focus is lost.
    fn lost_focus(&self) -> Option<Rc<dyn Fn()>>;

    /// Sets a method called when the input focus is lost.
    fn set_lost_focus(&self, value: Option<Rc<dyn Fn()>>);

    /// Creates a popup that belongs to the toplevel, if the platform
    /// supports native popups.
    fn create_popup(&self) -> Option<Rc<dyn IPopupImpl>>;

    /// Sets the transparency level hint of the toplevel, in order of
    /// preference.
    fn set_transparency_level_hint(&self, transparency_levels: &[WindowTransparencyLevel]);

    /// Gets the current transparency level of the toplevel.
    fn transparency_level(&self) -> WindowTransparencyLevel;

    /// Gets the acrylic compensation levels for the platform.
    fn acrylic_compensation_levels(&self) -> AcrylicPlatformCompensationLevels;

    /// Sets the theme variant on the frame if it should be dark or light.
    /// Also applies for the mica and acrylic backdrops, if they are
    /// applied; `None` follows the system.
    fn set_frame_theme_variant(&self, theme_variant: Option<PlatformThemeVariant>);

    /// Lets callers recover the concrete implementation type.
    fn as_any(&self) -> &dyn Any;

    /// The toplevel as a window base implementation, if it is one.
    fn as_window_base_impl(&self) -> Option<&dyn IWindowBaseImpl> {
        None
    }

    /// The toplevel as a window implementation, if it is one.
    fn as_window_impl(&self) -> Option<&dyn IWindowImpl> {
        None
    }

    /// The toplevel as a popup implementation, if it is one.
    fn as_popup_impl(&self) -> Option<&dyn IPopupImpl> {
        None
    }

    /// The toplevel as the Win32-specific options of a toplevel, if it has them.
    fn as_win32_options_top_level_impl(&self) -> Option<&dyn IWin32OptionsTopLevelImpl> {
        None
    }
}
