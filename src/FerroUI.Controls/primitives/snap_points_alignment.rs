/// Specify options for snap point alignment relative to an edge. Which edge
/// depends on the orientation of the object where the alignment is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SnapPointsAlignment {
    /// Use snap points grouped closer to the orientation edge.
    Near = 0,
    /// Use snap points that are centered in the orientation.
    Center = 1,
    /// Use snap points grouped farther from the orientation edge.
    Far = 2,
}
