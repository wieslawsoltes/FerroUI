//! Presentation declarations from RibbonGroup.Panels.cs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RibbonPanelPresentation(i32);
#[allow(non_upper_case_globals)]
impl RibbonPanelPresentation {
    pub const Full: Self = Self(0);
    pub const Buttons: Self = Self(1);
    pub const Titles: Self = Self(2);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
impl std::fmt::Display for RibbonPanelPresentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Full => f.write_str("Full"),
            Self::Buttons => f.write_str("Buttons"),
            Self::Titles => f.write_str("Titles"),
            _ => self.0.fmt(f),
        }
    }
}
