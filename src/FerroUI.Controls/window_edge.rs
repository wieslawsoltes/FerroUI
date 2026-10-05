/// The edge or corner of a window that an interactive resize is started
/// from.
// The order of the values matches the one used by GTK; backends rely on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowEdge {
    NorthWest = 0,
    North = 1,
    NorthEast = 2,
    West = 3,
    East = 4,
    SouthWest = 5,
    South = 6,
    SouthEast = 7,
}
