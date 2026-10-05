/// An inclusive range of code points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct CodepointRange {
    pub start: i32,
    pub end: i32,
}

impl CodepointRange {
    #[inline]
    pub const fn new(start: i32, end: i32) -> Self {
        Self { start, end }
    }
}
