// The documentation and flag names in this file are initially taken from the
// xdg_shell wayland protocol this API is designed after; see the `NOTICE.md` of
// the crate for the license of the wayland-protocols repository.

use bitflags::bitflags;
use ferroui_base::{Point, Rect, Size, Thickness};
use {
    super::{CustomPopupPlacement, PopupPositionRequest},
    crate::{PlacementMode, TopLevel},
    ferroui_base::media::FlowDirection,
    ferroui_base::Visual,
};

/// Provides positioning parameters to [`IPopupPositioner`].
///
/// The popup positioner provides a collection of rules for the placement of
/// a popup relative to its parent. Rules can be defined to ensure the popup
/// remains within the visible area's borders, and to specify how the popup
/// changes its position, such as sliding along an axis, or flipping around
/// a rectangle. These positioner-created rules are constrained by the
/// requirement that a popup must intersect with or be at least partially
/// adjacent to its parent surface.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PopupPositionerParameters {
    gravity: PopupGravity,
    anchor: PopupAnchor,

    /// The size of the popup that is to be positioned with the positioner
    /// object, in device-independent pixels.
    pub size: Size,

    /// How far to deflate [`size`](Self::size) when positioning the popup.
    ///
    /// This is normally set to the margin of the popup's child. This allows
    /// out-of-bounds effects like drop shadows to be drawn without affecting
    /// the final position of the child.
    pub deflate: Thickness,

    /// Specifies the anchor rectangle within the parent that the popup will
    /// be placed relative to, in device-independent pixels.
    ///
    /// The rectangle is relative to the parent geometry and may not extend
    /// outside the window geometry of the popup's parent.
    pub anchor_rectangle: Rect,

    /// Specify how the popup should be positioned if the originally
    /// intended position caused the popup to be constrained.
    ///
    /// Adjusts the popup position if the intended position caused the popup
    /// to be constrained; meaning at least partially outside positioning
    /// boundaries set by the positioner. The adjustment is set by
    /// constructing a bitmask describing the adjustment to be made when the
    /// popup is constrained on that axis.
    ///
    /// If no bit for one axis is set, the positioner will assume that the
    /// child surface should not change its position on that axis when
    /// constrained.
    ///
    /// If more than one bit for one axis is set, the order of how
    /// adjustments are applied is specified in the corresponding adjustment
    /// descriptions.
    ///
    /// The default adjustment is none.
    pub constraint_adjustment: PopupPositionerConstraintAdjustment,

    /// Specify the popup position offset relative to the position of the
    /// anchor on the anchor rectangle and the anchor on the popup.
    ///
    /// For example if the anchor of the anchor rectangle is at (x, y), the
    /// popup has the gravity bottom|right, and the offset is (ox, oy), the
    /// calculated surface position will be (x + ox, y + oy). The offset
    /// position of the surface is the one used for constraint testing. See
    /// [`constraint_adjustment`](Self::constraint_adjustment).
    ///
    /// An example use case is placing a popup menu on top of a user
    /// interface element, while aligning the user interface element of the
    /// parent surface with some user interface element placed somewhere in
    /// the popup.
    pub offset: Point,
}

impl PopupPositionerParameters {
    /// Gets the anchor point for the anchor rectangle.
    ///
    /// The specified anchor is used derive an anchor point that the popup
    /// will be positioned relative to. If a corner anchor is set (e.g.
    /// 'TopLeft' or 'BottomRight'), the anchor point will be at the
    /// specified corner; otherwise, the derived anchor point will be
    /// centered on the specified edge, or in the center of the anchor
    /// rectangle if no edge is specified.
    pub fn anchor(&self) -> PopupAnchor {
        self.anchor
    }

    /// Defines the anchor point for the anchor rectangle.
    ///
    /// # Panics
    ///
    /// Panics when opposite edges are specified.
    pub fn set_anchor(&mut self, value: PopupAnchor) {
        value.validate_edge();
        self.anchor = value;
    }

    /// Gets in what direction a popup should be positioned, relative to the
    /// anchor point of the parent.
    ///
    /// If a corner gravity is specified (e.g. 'BottomRight' or 'TopLeft'),
    /// then the popup will be placed towards the specified gravity;
    /// otherwise, the popup will be centered over the anchor point on any
    /// axis that had no gravity specified.
    pub fn gravity(&self) -> PopupGravity {
        self.gravity
    }

    /// Defines in what direction a popup should be positioned, relative to
    /// the anchor point of the parent.
    ///
    /// # Panics
    ///
    /// Panics when opposite edges are specified.
    pub fn set_gravity(&mut self, value: PopupGravity) {
        value.validate_gravity();
        self.gravity = value;
    }
}

bitflags! {
    /// Defines how a popup position will be adjusted if the unadjusted
    /// position would result in the popup being partly constrained.
    ///
    /// Whether a popup is considered 'constrained' is left to the
    /// positioner to determine. For example, the popup may be partly
    /// outside the target platform defined 'work area', thus necessitating
    /// the popup's position be adjusted until it is entirely inside the
    /// work area.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct PopupPositionerConstraintAdjustment: i32 {
        /// Don't alter the surface position even if it is constrained on
        /// some axis, for example partially outside the edge of an output.
        const NONE = 0;

        /// Slide the surface along the x axis until it is no longer
        /// constrained.
        ///
        /// First try to slide towards the direction of the gravity on the x
        /// axis until either the edge in the opposite direction of the
        /// gravity is unconstrained or the edge in the direction of the
        /// gravity is constrained.
        ///
        /// Then try to slide towards the opposite direction of the gravity
        /// on the x axis until either the edge in the direction of the
        /// gravity is unconstrained or the edge in the opposite direction
        /// of the gravity is constrained.
        const SLIDE_X = 1;

        /// Slide the surface along the y axis until it is no longer
        /// constrained.
        ///
        /// First try to slide towards the direction of the gravity on the y
        /// axis until either the edge in the opposite direction of the
        /// gravity is unconstrained or the edge in the direction of the
        /// gravity is constrained.
        ///
        /// Then try to slide towards the opposite direction of the gravity
        /// on the y axis until either the edge in the direction of the
        /// gravity is unconstrained or the edge in the opposite direction
        /// of the gravity is constrained.
        const SLIDE_Y = 2;

        /// Invert the anchor and gravity on the x axis if the surface is
        /// constrained on the x axis.
        ///
        /// For example, if the left edge of the surface is constrained, the
        /// gravity is 'left' and the anchor is 'left', change the gravity
        /// to 'right' and the anchor to 'right'.
        ///
        /// If the adjusted position also ends up being constrained, the
        /// resulting position of the FlipX adjustment will be the one
        /// before the adjustment.
        const FLIP_X = 4;

        /// Invert the anchor and gravity on the y axis if the surface is
        /// constrained on the y axis.
        ///
        /// For example, if the bottom edge of the surface is constrained,
        /// the gravity is 'bottom' and the anchor is 'bottom', change the
        /// gravity to 'top' and the anchor to 'top'.
        ///
        /// The adjusted position is calculated given the original anchor
        /// rectangle and offset, but with the new flipped anchor and
        /// gravity values.
        ///
        /// If the adjusted position also ends up being constrained, the
        /// resulting position of the FlipY adjustment will be the one
        /// before the adjustment.
        const FLIP_Y = 8;

        /// Horizontally resize the surface.
        ///
        /// Resize the surface horizontally so that it is completely
        /// unconstrained.
        const RESIZE_X = 16;

        /// Vertically resize the surface.
        ///
        /// Resize the surface vertically so that it is completely
        /// unconstrained.
        const RESIZE_Y = 32;

        /// All the adjustments.
        const ALL = Self::SLIDE_X.bits()
            | Self::SLIDE_Y.bits()
            | Self::FLIP_X.bits()
            | Self::FLIP_Y.bits()
            | Self::RESIZE_X.bits()
            | Self::RESIZE_Y.bits();
    }
}

// The edge helper: validation and flipping of anchors and gravities.
impl PopupAnchor {
    /// Checks that the edge does not specify opposite edges.
    ///
    /// # Panics
    ///
    /// Panics when opposite edges are specified.
    pub(crate) fn validate_edge(self) {
        if self.contains(PopupAnchor::LEFT | PopupAnchor::RIGHT) || self.contains(PopupAnchor::TOP | PopupAnchor::BOTTOM)
        {
            panic!("Opposite edges specified");
        }
    }

    /// Inverts the edge on both axes.
    #[allow(dead_code)] // Part of the edge helper; unused in the reference implementation too.
    pub(crate) fn flip(self) -> PopupAnchor {
        let mut edge = self;
        if edge.intersects(PopupAnchor::HORIZONTAL_MASK) {
            edge ^= PopupAnchor::HORIZONTAL_MASK;
        }

        if edge.intersects(PopupAnchor::VERTICAL_MASK) {
            edge ^= PopupAnchor::VERTICAL_MASK;
        }

        edge
    }

    /// Inverts the edge on the x axis.
    pub(crate) fn flip_x(self) -> PopupAnchor {
        let mut edge = self;
        if edge.intersects(PopupAnchor::HORIZONTAL_MASK) {
            edge ^= PopupAnchor::HORIZONTAL_MASK;
        }
        edge
    }

    /// Inverts the edge on the y axis.
    pub(crate) fn flip_y(self) -> PopupAnchor {
        let mut edge = self;
        if edge.intersects(PopupAnchor::VERTICAL_MASK) {
            edge ^= PopupAnchor::VERTICAL_MASK;
        }
        edge
    }
}

impl PopupGravity {
    /// Checks that the gravity does not specify opposite directions.
    ///
    /// # Panics
    ///
    /// Panics when opposite directions are specified.
    pub(crate) fn validate_gravity(self) {
        PopupAnchor::from_bits_retain(self.bits()).validate_edge();
    }

    /// Inverts the gravity on the x axis.
    pub(crate) fn flip_x(self) -> PopupGravity {
        PopupGravity::from_bits_retain(PopupAnchor::from_bits_retain(self.bits()).flip_x().bits())
    }

    /// Inverts the gravity on the y axis.
    pub(crate) fn flip_y(self) -> PopupGravity {
        PopupGravity::from_bits_retain(PopupAnchor::from_bits_retain(self.bits()).flip_y().bits())
    }
}

bitflags! {
    /// Defines the edges around an anchor rectangle on which a popup will
    /// open.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct PopupAnchor: i32 {
        /// The center of the anchor rectangle.
        const NONE = 0;

        /// The top edge of the anchor rectangle.
        const TOP = 1;

        /// The bottom edge of the anchor rectangle.
        const BOTTOM = 2;

        /// The left edge of the anchor rectangle.
        const LEFT = 4;

        /// The right edge of the anchor rectangle.
        const RIGHT = 8;

        /// The top-left corner of the anchor rectangle.
        const TOP_LEFT = Self::TOP.bits() | Self::LEFT.bits();

        /// The top-right corner of the anchor rectangle.
        const TOP_RIGHT = Self::TOP.bits() | Self::RIGHT.bits();

        /// The bottom-left corner of the anchor rectangle.
        const BOTTOM_LEFT = Self::BOTTOM.bits() | Self::LEFT.bits();

        /// The bottom-right corner of the anchor rectangle.
        const BOTTOM_RIGHT = Self::BOTTOM.bits() | Self::RIGHT.bits();

        /// A mask for the vertical component flags.
        const VERTICAL_MASK = Self::TOP.bits() | Self::BOTTOM.bits();

        /// A mask for the horizontal component flags.
        const HORIZONTAL_MASK = Self::LEFT.bits() | Self::RIGHT.bits();

        /// A mask for all flags.
        const ALL_MASK = Self::VERTICAL_MASK.bits() | Self::HORIZONTAL_MASK.bits();
    }
}

bitflags! {
    /// Defines the direction in which a popup will open.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct PopupGravity: i32 {
        /// The popup will be centered over the anchor edge.
        const NONE = 0;

        /// The popup will be positioned above the anchor edge.
        const TOP = 1;

        /// The popup will be positioned below the anchor edge.
        const BOTTOM = 2;

        /// The popup will be positioned to the left of the anchor edge.
        const LEFT = 4;

        /// The popup will be positioned to the right of the anchor edge.
        const RIGHT = 8;

        /// The popup will be positioned to the top-left of the anchor edge.
        const TOP_LEFT = Self::TOP.bits() | Self::LEFT.bits();

        /// The popup will be positioned to the top-right of the anchor edge.
        const TOP_RIGHT = Self::TOP.bits() | Self::RIGHT.bits();

        /// The popup will be positioned to the bottom-left of the anchor
        /// edge.
        const BOTTOM_LEFT = Self::BOTTOM.bits() | Self::LEFT.bits();

        /// The popup will be positioned to the bottom-right of the anchor
        /// edge.
        const BOTTOM_RIGHT = Self::BOTTOM.bits() | Self::RIGHT.bits();
    }
}

/// Positions a popup host.
///
/// [`IPopupPositioner`] is an abstraction of the wayland xdg_positioner
/// spec.
///
/// The popup positioner implementation is determined by the platform
/// implementation. A default managed implementation is provided in
/// [`ManagedPopupPositioner`](super::ManagedPopupPositioner) for platforms
/// on which popups can be arbitrarily positioned.
pub trait IPopupPositioner {
    /// Updates the position of the associated popup host according to the
    /// specified parameters.
    fn update(&self, parameters: PopupPositionerParameters);
}

/// The positioner extension members: positioning from a popup position
/// request.
impl dyn IPopupPositioner {
    /// Updates the position of the popup of size `popup_size` that belongs
    /// to `top_level` according to `position_request`; `deflate` is the
    /// margin of the popup's child and `flow_direction` the flow direction
    /// of the popup (internal upstream).
    ///
    /// # Panics
    ///
    /// Panics when the placement is custom and the request has no placement
    /// callback, or when the placement is not relative to the pointer and
    /// the target is not attached to the visual tree of the popup parent.
    pub fn update_request(
        &self,
        top_level: &TopLevel,
        position_request: &PopupPositionRequest,
        popup_size: Size,
        deflate: Thickness,
        flow_direction: FlowDirection,
    ) {
        if popup_size == Size::default() {
            return;
        }

        let mut positioner_parameters = PopupPositionerParameters {
            offset: position_request.offset(),
            size: popup_size,
            deflate,
            constraint_adjustment: position_request.constraint_adjustment(),
            ..PopupPositionerParameters::default()
        };

        if position_request.placement() == PlacementMode::Pointer {
            // We need a better way for tracking the last pointer position
            let position = top_level.point_to_client(top_level.last_pointer_position().unwrap_or_default());

            positioner_parameters.anchor_rectangle = Rect::from_position_size(position, Size::new(1.0, 1.0));
            positioner_parameters.set_anchor(PopupAnchor::TOP_LEFT);
            positioner_parameters.set_gravity(PopupGravity::BOTTOM_RIGHT);
        } else if position_request.placement() == PlacementMode::Custom {
            let Some(placement_callback) = position_request.placement_callback() else {
                panic!("CustomPopupPlacementCallback property must be set, when Placement=PlacementMode.Custom");
            };

            positioner_parameters.anchor_rectangle = calculate_anchor_rect(top_level, position_request);

            let mut custom_placement_parameters =
                CustomPopupPlacement::new(popup_size, deflate, position_request.target());
            custom_placement_parameters.anchor_rectangle = positioner_parameters.anchor_rectangle;
            custom_placement_parameters.set_anchor(positioner_parameters.anchor());
            custom_placement_parameters.set_gravity(positioner_parameters.gravity());
            custom_placement_parameters.constraint_adjustment = positioner_parameters.constraint_adjustment;
            custom_placement_parameters.offset = positioner_parameters.offset;

            placement_callback(&mut custom_placement_parameters);

            positioner_parameters.anchor_rectangle = custom_placement_parameters.anchor_rectangle;
            positioner_parameters.set_anchor(custom_placement_parameters.anchor());
            positioner_parameters.set_gravity(custom_placement_parameters.gravity());
            positioner_parameters.constraint_adjustment = custom_placement_parameters.constraint_adjustment;
            positioner_parameters.offset = custom_placement_parameters.offset;
        } else {
            positioner_parameters.anchor_rectangle = calculate_anchor_rect(top_level, position_request);

            let (anchor, gravity) = match position_request.placement() {
                PlacementMode::Bottom => (PopupAnchor::BOTTOM, PopupGravity::BOTTOM),
                PlacementMode::Right => (PopupAnchor::RIGHT, PopupGravity::RIGHT),
                PlacementMode::Left => (PopupAnchor::LEFT, PopupGravity::LEFT),
                PlacementMode::Top => (PopupAnchor::TOP, PopupGravity::TOP),
                PlacementMode::Center => (PopupAnchor::NONE, PopupGravity::NONE),
                PlacementMode::AnchorAndGravity => (position_request.anchor(), position_request.gravity()),
                PlacementMode::TopEdgeAlignedRight => (PopupAnchor::TOP_RIGHT, PopupGravity::TOP_LEFT),
                PlacementMode::TopEdgeAlignedLeft => (PopupAnchor::TOP_LEFT, PopupGravity::TOP_RIGHT),
                PlacementMode::BottomEdgeAlignedLeft => (PopupAnchor::BOTTOM_LEFT, PopupGravity::BOTTOM_RIGHT),
                PlacementMode::BottomEdgeAlignedRight => (PopupAnchor::BOTTOM_RIGHT, PopupGravity::BOTTOM_LEFT),
                PlacementMode::LeftEdgeAlignedTop => (PopupAnchor::TOP_LEFT, PopupGravity::BOTTOM_LEFT),
                PlacementMode::LeftEdgeAlignedBottom => (PopupAnchor::BOTTOM_LEFT, PopupGravity::TOP_LEFT),
                PlacementMode::RightEdgeAlignedTop => (PopupAnchor::TOP_RIGHT, PopupGravity::BOTTOM_RIGHT),
                PlacementMode::RightEdgeAlignedBottom => (PopupAnchor::BOTTOM_RIGHT, PopupGravity::TOP_RIGHT),
                // The pointer and custom placements are handled above.
                placement @ (PlacementMode::Pointer | PlacementMode::Custom) => {
                    panic!("Invalid value for Popup.PlacementMode: {placement:?}")
                }
            };
            positioner_parameters.set_anchor(anchor);
            positioner_parameters.set_gravity(gravity);
        }

        // Invert coordinate system if FlowDirection is RTL
        if flow_direction == FlowDirection::RightToLeft {
            let mut anchor = positioner_parameters.anchor();
            if anchor.contains(PopupAnchor::RIGHT) {
                anchor ^= PopupAnchor::RIGHT;
                anchor |= PopupAnchor::LEFT;
            } else if anchor.contains(PopupAnchor::LEFT) {
                anchor ^= PopupAnchor::LEFT;
                anchor |= PopupAnchor::RIGHT;
            }
            positioner_parameters.set_anchor(anchor);

            let mut gravity = positioner_parameters.gravity();
            if gravity.contains(PopupGravity::RIGHT) {
                gravity ^= PopupGravity::RIGHT;
                gravity |= PopupGravity::LEFT;
            } else if gravity.contains(PopupGravity::LEFT) {
                gravity ^= PopupGravity::LEFT;
                gravity |= PopupGravity::RIGHT;
            }
            positioner_parameters.set_gravity(gravity);
        }

        self.update(positioner_parameters);
    }
}

/// The anchor rectangle of a position request, in the coordinates of the
/// root visual of `top_level`.
fn calculate_anchor_rect(top_level: &TopLevel, position_request: &PopupPositionRequest) -> Rect {
    let target = position_request.target();
    let root_visual = top_level.presentation_source().root_visual();
    let matrix = root_visual.and_then(|root_visual| {
        let root_visual: &Visual = &root_visual;
        target.transform_to_visual(root_visual)
    });

    let Some(matrix) = matrix else {
        if !target.is_attached_to_visual_tree() {
            panic!("Target control is not attached to the visual tree");
        }
        panic!("Target control is not in the same tree as the popup parent");
    };

    let bounds = Rect::from_size(target.bounds().size());
    let anchor_rect = position_request.anchor_rect().unwrap_or(bounds);
    anchor_rect.intersect(bounds).transform_to_aabb(matrix)
}
