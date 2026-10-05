use super::{IPopupPositioner, PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment, PopupPositionerParameters};
use ferroui_base::utilities::math_utilities;
use ferroui_base::{Point, Rect, Size};
use std::rc::Rc;

/// The popup that a [`ManagedPopupPositioner`] positions.
pub trait IManagedPopupPositionerPopup {
    /// The screens of the platform, in the positioner's coordinates.
    fn screens(&self) -> Vec<ManagedPopupPositionerScreenInfo>;

    /// The client area of the popup's parent, in screen coordinates.
    fn parent_client_area_screen_geometry(&self) -> Rect;

    /// The scaling between the positioning parameters and the positioner's
    /// coordinates.
    fn scaling(&self) -> f64;

    /// Moves the popup to `device_point` and resizes it to `virtual_size`.
    fn move_and_resize(&self, device_point: Point, virtual_size: Size);
}

/// A screen as seen by a [`ManagedPopupPositioner`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ManagedPopupPositionerScreenInfo {
    bounds: Rect,
    working_area: Rect,
}

impl ManagedPopupPositionerScreenInfo {
    /// Creates the screen info.
    pub fn new(bounds: Rect, working_area: Rect) -> Self {
        Self { bounds, working_area }
    }

    /// The bounds of the screen.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// The working area of the screen.
    pub fn working_area(&self) -> Rect {
        self.working_area
    }
}

/// An [`IPopupPositioner`] implementation for platforms on which a popup
/// can be arbitrarily positioned.
pub struct ManagedPopupPositioner {
    popup: Rc<dyn IManagedPopupPositionerPopup>,
}

impl ManagedPopupPositioner {
    /// Creates a positioner for `popup`.
    pub fn new(popup: Rc<dyn IManagedPopupPositionerPopup>) -> Self {
        Self { popup }
    }

    fn get_anchor_point(anchor_rect: Rect, edge: PopupAnchor) -> Point {
        let x = if edge.contains(PopupAnchor::LEFT) {
            anchor_rect.x
        } else if edge.contains(PopupAnchor::RIGHT) {
            anchor_rect.right()
        } else {
            anchor_rect.x + anchor_rect.width / 2.0
        };

        let y = if edge.contains(PopupAnchor::TOP) {
            anchor_rect.y
        } else if edge.contains(PopupAnchor::BOTTOM) {
            anchor_rect.bottom()
        } else {
            anchor_rect.y + anchor_rect.height / 2.0
        };
        Point::new(x, y)
    }

    fn gravitate(anchor_point: Point, size: Size, gravity: PopupGravity) -> Point {
        let x = if gravity.contains(PopupGravity::LEFT) {
            -size.width
        } else if gravity.contains(PopupGravity::RIGHT) {
            0.0
        } else {
            -size.width / 2.0
        };

        let y = if gravity.contains(PopupGravity::TOP) {
            -size.height
        } else if gravity.contains(PopupGravity::BOTTOM) {
            0.0
        } else {
            -size.height / 2.0
        };
        anchor_point + Point::new(x, y)
    }

    fn calculate(
        &self,
        translated_size: Size,
        anchor_rect: Rect,
        anchor: PopupAnchor,
        gravity: PopupGravity,
        constraint_adjustment: PopupPositionerConstraintAdjustment,
        offset: Point,
    ) -> Rect {
        let parent_geometry = self.popup.parent_client_area_screen_geometry();
        let anchor_rect = anchor_rect.translate(parent_geometry.top_left().into());

        let get_bounds = || -> Rect {
            let screens = self.popup.screens();
            let anchor_point = Self::get_anchor_point(anchor_rect, anchor);
            let parent_geometry_point = Self::get_anchor_point(parent_geometry, anchor);

            let target_screen = screens
                .iter()
                .find(|s| s.bounds().contains_exclusive(anchor_point))
                .or_else(|| screens.iter().find(|s| s.bounds().intersects(anchor_rect)))
                .or_else(|| screens.iter().find(|s| s.bounds().contains_exclusive(parent_geometry_point)))
                .or_else(|| screens.iter().find(|s| s.bounds().intersects(parent_geometry)))
                .or_else(|| screens.first());

            match target_screen {
                Some(screen) if screen.working_area().width == 0.0 && screen.working_area().height == 0.0 => {
                    screen.bounds()
                }
                Some(screen) => screen.working_area(),
                None => Rect::new(0.0, 0.0, f64::MAX, f64::MAX),
            }
        };

        let bounds = get_bounds();

        let fits_in_bounds = |rc: Rect, edge: PopupAnchor| -> bool {
            !(edge.contains(PopupAnchor::LEFT) && rc.x < bounds.x
                || edge.contains(PopupAnchor::TOP) && rc.y < bounds.y
                || edge.contains(PopupAnchor::RIGHT) && rc.right() > bounds.right()
                || edge.contains(PopupAnchor::BOTTOM) && rc.bottom() > bounds.bottom())
        };

        let is_valid = |rc: Rect| -> bool { rc.width > 0.0 && rc.height > 0.0 };

        let get_unconstrained = |a: PopupAnchor, g: PopupGravity| -> Rect {
            Rect::from_position_size(
                Self::gravitate(Self::get_anchor_point(anchor_rect, a), translated_size, g) + offset,
                translated_size,
            )
        };

        let mut geo = get_unconstrained(anchor, gravity);

        // If flipping geometry and anchor is allowed and helps, use the flipped one,
        // otherwise leave it as is
        if !fits_in_bounds(geo, PopupAnchor::HORIZONTAL_MASK)
            && constraint_adjustment.contains(PopupPositionerConstraintAdjustment::FLIP_X)
        {
            let flipped = get_unconstrained(anchor.flip_x(), gravity.flip_x());
            if fits_in_bounds(flipped, PopupAnchor::HORIZONTAL_MASK) {
                geo = geo.with_x(flipped.x);
            }
        }

        // If sliding is allowed, try moving the rect into the bounds
        if constraint_adjustment.contains(PopupPositionerConstraintAdjustment::SLIDE_X) {
            geo = geo.with_x(math_utilities::max(geo.x, bounds.x));
            if geo.right() > bounds.right() {
                geo = geo.with_x(bounds.right() - geo.width);
            }
        }

        // Resize the rect horizontally if allowed.
        if constraint_adjustment.contains(PopupPositionerConstraintAdjustment::RESIZE_X) {
            let mut unconstrained_rect = geo;

            if !fits_in_bounds(unconstrained_rect, PopupAnchor::LEFT) {
                unconstrained_rect = unconstrained_rect.with_x(bounds.x);
            }

            if !fits_in_bounds(unconstrained_rect, PopupAnchor::RIGHT) {
                unconstrained_rect = unconstrained_rect.with_width(bounds.width - unconstrained_rect.x);
            }

            if is_valid(unconstrained_rect) {
                geo = unconstrained_rect;
            }
        }

        // If flipping geometry and anchor is allowed and helps, use the flipped one,
        // otherwise leave it as is
        if !fits_in_bounds(geo, PopupAnchor::VERTICAL_MASK)
            && constraint_adjustment.contains(PopupPositionerConstraintAdjustment::FLIP_Y)
        {
            let flipped = get_unconstrained(anchor.flip_y(), gravity.flip_y());
            if fits_in_bounds(flipped, PopupAnchor::VERTICAL_MASK) {
                geo = geo.with_y(flipped.y);
            }
        }

        // If sliding is allowed, try moving the rect into the bounds
        if constraint_adjustment.contains(PopupPositionerConstraintAdjustment::SLIDE_Y) {
            geo = geo.with_y(math_utilities::max(geo.y, bounds.y));
            if geo.bottom() > bounds.bottom() {
                geo = geo.with_y(bounds.bottom() - geo.height);
            }
        }

        // Resize the rect vertically if allowed.
        if constraint_adjustment.contains(PopupPositionerConstraintAdjustment::RESIZE_Y) {
            let mut unconstrained_rect = geo;

            if !fits_in_bounds(unconstrained_rect, PopupAnchor::TOP) {
                unconstrained_rect = unconstrained_rect.with_y(bounds.y);
            }

            if !fits_in_bounds(unconstrained_rect, PopupAnchor::BOTTOM) {
                unconstrained_rect = unconstrained_rect.with_height(bounds.bottom() - unconstrained_rect.y);
            }

            if is_valid(unconstrained_rect) {
                geo = unconstrained_rect;
            }
        }

        geo
    }
}

impl IPopupPositioner for ManagedPopupPositioner {
    fn update(&self, parameters: PopupPositionerParameters) {
        let rect = self.calculate(
            parameters.size.deflate(parameters.deflate) * self.popup.scaling(),
            Rect::from_position_size(
                parameters.anchor_rectangle.top_left() * self.popup.scaling(),
                parameters.anchor_rectangle.size() * self.popup.scaling(),
            ),
            parameters.anchor(),
            parameters.gravity(),
            parameters.constraint_adjustment,
            parameters.offset * self.popup.scaling(),
        );

        let rect = rect.inflate_thickness(parameters.deflate * self.popup.scaling());

        self.popup.move_and_resize(rect.position(), rect.size() / self.popup.scaling());
    }
}
