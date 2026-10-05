/// Determines which of an animation's values stay applied outside of its
/// active period.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FillMode {
    /// Nothing is applied before the animation starts or after it ends.
    #[default]
    None = 0,
    /// The last value is retained after the animation ends.
    Forward = 1,
    /// The first value is applied during the initial delay.
    Backward = 2,
    /// Both of the above.
    Both = 3,
}
