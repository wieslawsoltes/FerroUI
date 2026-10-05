/// Specifies commonly used folders of the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WellKnownFolder {
    /// The Desktop folder of the current user.
    Desktop = 0,

    /// The Documents folder of the current user.
    Documents = 1,

    /// The Downloads folder of the current user.
    Downloads = 2,

    /// The Music folder of the current user.
    Music = 3,

    /// The Pictures folder of the current user.
    Pictures = 4,

    /// The Videos folder of the current user.
    Videos = 5,
}
