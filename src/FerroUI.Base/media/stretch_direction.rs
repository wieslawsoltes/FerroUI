/// Describes the type of scaling that can be used when scaling content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StretchDirection {
    #[default]
    UpOnly = 0,
    DownOnly = 1,
    Both = 2,
}
