/// Describes how content is positioned vertically in a container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AlignmentY {
    #[default]
    Top = 0,
    Center = 1,
    Bottom = 2,
}
