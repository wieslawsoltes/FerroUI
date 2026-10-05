/// Specifies how the interior of a geometry is determined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FillRule {
    #[default]
    EvenOdd = 0,
    NonZero = 1,
}
