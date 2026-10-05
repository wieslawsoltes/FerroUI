/// Specify options for snap point behaviour when scrolling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SnapPointsType {
    /// No snapping behaviour.
    None = 0,
    /// Content always stops at the snap point closest to where inertia would
    /// naturally stop along the direction of inertia.
    Mandatory = 1,
    /// Content always stops at the snap point closest to the release point
    /// along the direction of inertia.
    MandatorySingle = 2,
}
