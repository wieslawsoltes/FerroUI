//! The port of `AtSpiCoordType.cs`.

/// What the coordinates of a call are relative to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum AtSpiCoordType {
    Screen = 0,
    Window = 1,
    Parent = 2,
}

impl AtSpiCoordType {
    /// The member with the value `coord_type`; `None` for a value the
    /// enumeration does not have (the reference casts, and its
    /// `switch` statements take such a value as their default).
    pub(crate) fn from_u32(coord_type: u32) -> Option<Self> {
        match coord_type {
            0 => Some(Self::Screen),
            1 => Some(Self::Window),
            2 => Some(Self::Parent),
            _ => None,
        }
    }
}
