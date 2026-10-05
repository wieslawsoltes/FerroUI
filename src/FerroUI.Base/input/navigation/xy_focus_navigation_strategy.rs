/// Specifies the disambiguation strategy used for navigating between
/// multiple candidate targets with directional (XY) navigation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum XYFocusNavigationStrategy {
    /// Indicates that navigation strategy is inherited from the element's
    /// ancestors. If all ancestors have a value of `Auto`, the fallback
    /// strategy is `Projection`.
    #[default]
    Auto = 0,
    /// Indicates that focus moves to the first element encountered when
    /// projecting the edge of the currently focused element in the direction
    /// of navigation.
    Projection = 1,
    /// Indicates that focus moves to the element closest to the axis of the
    /// navigation direction.
    NavigationDirectionDistance = 2,
    /// Indicates that focus moves to the closest element based on the
    /// shortest 2D distance (Manhattan metric).
    RectilinearDistance = 3,
}
