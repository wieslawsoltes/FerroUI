use super::{IHitTester, IRenderer};
use crate::input::IInputRoot;
use crate::layout::ILayoutRoot;
use crate::platform::IPlatformSettings;
use crate::{PixelPoint, Point, Ref, Size, Visual};
use std::rc::Rc;

/// The host of a visual tree: connects the tree to its renderer, layout root
/// and input root.
pub trait IPresentationSource {
    /// The root visual of the tree.
    fn root_visual(&self) -> Option<Ref<Visual>>;

    /// The scaling factor to use in rendering.
    fn render_scaling(&self) -> f64;

    /// The renderer of the tree.
    fn renderer(&self) -> Rc<dyn IRenderer>;

    /// The layout root of the tree.
    fn layout_root(&self) -> Rc<dyn ILayoutRoot>;

    /// The size of the client area, in device-independent pixels.
    fn client_size(&self) -> Size;

    /// The platform settings of the host, if it has any.
    fn platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        None
    }

    /// The hit tester of the tree.
    fn hit_tester(&self) -> Rc<dyn IHitTester>;

    /// The input root of the tree.
    fn input_root(&self) -> Rc<dyn IInputRoot>;

    /// Converts a point from client to screen coordinates, if the host is
    /// on a screen.
    fn point_to_screen(&self, _point: Point) -> Option<PixelPoint> {
        None
    }

    /// Converts a point from screen to client coordinates, if the host is
    /// on a screen.
    fn point_to_client(&self, _point: PixelPoint) -> Option<Point> {
        None
    }
}
