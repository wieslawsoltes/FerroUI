/// The result of an intersection test between two geometries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IntersectionResult {
    #[default]
    NotCalculated = 0,
    Empty = 1,
    FullyInside = 2,
    FullyContains = 3,
    Intersects = 4,
}
