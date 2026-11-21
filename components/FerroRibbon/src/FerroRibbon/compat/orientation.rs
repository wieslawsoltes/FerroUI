//! Pinned Uno/WinUI Orientation. Its integer order differs from FerroUI's enum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Orientation(i32);
ferroui_base::styled_value!(Orientation);
#[allow(non_upper_case_globals)]
impl Orientation {
    pub const Vertical: Self = Self(0);
    pub const Horizontal: Self = Self(1);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
impl From<ferroui_base::layout::Orientation> for Orientation {
    fn from(value: ferroui_base::layout::Orientation) -> Self {
        match value {
            ferroui_base::layout::Orientation::Vertical => Self::Vertical,
            ferroui_base::layout::Orientation::Horizontal => Self::Horizontal,
        }
    }
}
impl TryFrom<Orientation> for ferroui_base::layout::Orientation {
    type Error = i32;
    fn try_from(value: Orientation) -> Result<Self, Self::Error> {
        match value {
            Orientation::Vertical => Ok(Self::Vertical),
            Orientation::Horizontal => Ok(Self::Horizontal),
            _ => Err(value.raw()),
        }
    }
}
impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Vertical => f.write_str("Vertical"),
            Self::Horizontal => f.write_str("Horizontal"),
            _ => write!(f, "{}", self.0),
        }
    }
}
