use super::{IManagedPopupPositionerPopup, ManagedPopupPositionerScreenInfo};
use crate::platform::{IScreenImpl, ITopLevelImpl};
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::{PixelPoint, Point, Rect, Size};
use std::rc::Rc;

/// The callback that moves and resizes the popup: receives the position in
/// device pixels, the size in device-independent pixels and the scaling.
pub type MoveResizeDelegate = Rc<dyn Fn(PixelPoint, Size, f64)>;

/// This class is used to simplify integration of popup implementations with
/// the popup positioner.
pub struct ManagedPopupPositionerPopupImplHelper {
    parent: Rc<dyn ITopLevelImpl>,
    move_resize: MoveResizeDelegate,
}

impl ManagedPopupPositionerPopupImplHelper {
    /// Creates the helper for a popup that belongs to `parent`.
    pub fn new(parent: Rc<dyn ITopLevelImpl>, move_resize: MoveResizeDelegate) -> Self {
        Self { parent, move_resize }
    }
}

impl IManagedPopupPositionerPopup for ManagedPopupPositionerPopupImplHelper {
    fn screens(&self) -> Vec<ManagedPopupPositionerScreenInfo> {
        let features: &dyn IOptionalFeatureProvider = &*self.parent;
        let Some(screen_impl) = features.try_get::<dyn IScreenImpl>() else {
            return Vec::new();
        };

        screen_impl
            .all_screens()
            .iter()
            .map(|s| ManagedPopupPositionerScreenInfo::new(s.bounds().to_rect(1.0), s.working_area().to_rect(1.0)))
            .collect()
    }

    fn parent_client_area_screen_geometry(&self) -> Rect {
        // Popup positioner operates with abstract coordinates, but in our case they are pixel ones
        let point = self.parent.point_to_screen(Point::default());
        let size = self.parent.client_size() * self.scaling();
        Rect::new(point.x as f64, point.y as f64, size.width, size.height)
    }

    fn move_and_resize(&self, device_point: Point, virtual_size: Size) {
        (self.move_resize)(
            PixelPoint::new(device_point.x as i32, device_point.y as i32),
            virtual_size,
            self.parent.render_scaling(),
        );
    }

    fn scaling(&self) -> f64 {
        self.parent.desktop_scaling()
    }
}
