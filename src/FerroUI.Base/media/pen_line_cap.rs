/// Describes the shape at the end of a line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PenLineCap {
    #[default]
    Flat = 0,
    Round = 1,
    Square = 2,
}
