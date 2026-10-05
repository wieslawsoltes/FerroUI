/// Describes the shape that joins two lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PenLineJoin {
    #[default]
    Bevel = 0,
    Miter = 1,
    Round = 2,
}
