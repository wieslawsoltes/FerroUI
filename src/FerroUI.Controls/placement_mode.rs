/// Defines the placement for a popup control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PlacementMode {
    /// The popup is placed at the pointer position.
    #[default]
    Pointer = 0,

    /// Preferred location is below the target element.
    Bottom = 1,

    /// Preferred location is to the right of the target element.
    Right = 2,

    /// Preferred location is to the left of the target element.
    Left = 3,

    /// Preferred location is above the target element.
    Top = 4,

    /// Preferred location is centered over the target element.
    Center = 5,

    /// The popup is placed according to the popup's placement anchor and
    /// placement gravity rules.
    AnchorAndGravity = 6,

    /// Preferred location is above the target element, with the left edge
    /// of the popup aligned with the left edge of the target element.
    TopEdgeAlignedLeft = 7,

    /// Preferred location is above the target element, with the right edge
    /// of popup aligned with right edge of the target element.
    TopEdgeAlignedRight = 8,

    /// Preferred location is below the target element, with the left edge
    /// of popup aligned with left edge of the target element.
    BottomEdgeAlignedLeft = 9,

    /// Preferred location is below the target element, with the right edge
    /// of popup aligned with right edge of the target element.
    BottomEdgeAlignedRight = 10,

    /// Preferred location is to the left of the target element, with the
    /// top edge of popup aligned with top edge of the target element.
    LeftEdgeAlignedTop = 11,

    /// Preferred location is to the left of the target element, with the
    /// bottom edge of popup aligned with bottom edge of the target element.
    LeftEdgeAlignedBottom = 12,

    /// Preferred location is to the right of the target element, with the
    /// top edge of popup aligned with top edge of the target element.
    RightEdgeAlignedTop = 13,

    /// Preferred location is to the right of the target element, with the
    /// bottom edge of popup aligned with bottom edge of the target element.
    RightEdgeAlignedBottom = 14,

    /// A position and repositioning behavior that is defined by the popup's
    /// custom popup placement callback.
    Custom = 15,
}
