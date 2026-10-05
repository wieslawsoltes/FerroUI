use super::renderer_factory::ITopLevelRenderer;
use super::PresentationSource;
use ferroui_base::layout::LayoutHelper;
use ferroui_base::rendering::{IHitTester, ManagedHitTester, SceneInvalidatedEventArgs};
use ferroui_base::{PixelPoint, Point, Size};
use std::rc::Rc;

impl PresentationSource {
    /// The renderer of the tree.
    pub fn typed_renderer(&self) -> &Rc<dyn ITopLevelRenderer> {
        self.renderer.get().expect("the renderer is created with the presentation source")
    }

    /// The hit tester of the tree: the override when one is set, otherwise
    /// the hit tester of the renderer, otherwise one that walks the visual
    /// tree.
    pub fn hit_tester(&self) -> Rc<dyn IHitTester> {
        if let Some(hit_tester) = self.hit_tester_override.borrow().clone() {
            return hit_tester;
        }
        self.typed_renderer().hit_tester().unwrap_or_else(|| Rc::new(ManagedHitTester::new()))
    }

    /// The hit tester used instead of the renderer's, for unit tests that
    /// do not set up a hit-testable visual tree.
    pub fn hit_tester_override(&self) -> Option<Rc<dyn IHitTester>> {
        self.hit_tester_override.borrow().clone()
    }

    pub fn set_hit_tester_override(&self, value: Option<Rc<dyn IHitTester>>) {
        *self.hit_tester_override.borrow_mut() = value;
    }

    /// The scaling factor to use in rendering.
    pub fn render_scaling(&self) -> f64 {
        self.render_scaling.get()
    }

    /// The size of the client area of the platform implementation.
    pub fn client_size(&self) -> Size {
        self.platform_impl().map(|platform_impl| platform_impl.client_size()).unwrap_or_default()
    }

    pub(super) fn scene_invalidated(&self, e: &SceneInvalidatedEventArgs) {
        let pre_processor = self.pointer_over_pre_processor.borrow().clone();
        if let Some(pre_processor) = pre_processor {
            pre_processor.scene_invalidated(e.dirty_rect());
        }
    }

    /// Converts a point from client to screen coordinates.
    pub fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
        self.platform_impl().map(|platform_impl| platform_impl.point_to_screen(point))
    }

    /// Converts a point from screen to client coordinates.
    pub fn point_to_client(&self, point: PixelPoint) -> Option<Point> {
        self.platform_impl().map(|platform_impl| platform_impl.point_to_client(point))
    }

    pub(super) fn handle_scaling_changed(&self, scaling: f64) {
        self.render_scaling.set(LayoutHelper::validate_scaling(scaling));
    }
}
