//! WinUI visibility values retained for source template-visible enum properties.
//! Native layout visibility uses IsVisible through an explicit converter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Visibility(i32);
ferroui_base::styled_value!(Visibility);
#[allow(non_upper_case_globals)]
impl Visibility {
    pub const Visible: Self = Self(0);
    pub const Collapsed: Self = Self(1);
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    pub const fn raw(self) -> i32 {
        self.0
    }
}
impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Visible => f.write_str("Visible"),
            Self::Collapsed => f.write_str("Collapsed"),
            _ => write!(f, "{}", self.0),
        }
    }
}
