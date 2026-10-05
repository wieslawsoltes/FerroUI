/// Describes how content is positioned horizontally in a container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AlignmentX {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
}
