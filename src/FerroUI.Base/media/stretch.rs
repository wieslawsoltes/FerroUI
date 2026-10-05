/// Describes how content is resized to fill its allocated space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Stretch {
    #[default]
    None = 0,
    Fill = 1,
    Uniform = 2,
    UniformToFill = 3,
}
