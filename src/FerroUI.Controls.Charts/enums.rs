//! The public WinUI Charts enumeration contract, expressed as Rust enums.

use ferroui_base::ferro_markup_enum;

/// The display direction of a bar series. Data roles remain X/category, Y/value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BarOrientation {
    #[default]
    Horizontal = 0,
    Vertical = 1,
}

/// Whether categories follow source order or their values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CategorySortKey {
    #[default]
    Index = 0,
    Value = 1,
}

/// Calendar interval used for date-axis ticks; this does not aggregate samples.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DateTimeIntervalType {
    #[default]
    Auto = 0,
    Day = 1,
    Week = 2,
    Month = 3,
    Year = 4,
}

/// Which grid lines an axis draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GridLines {
    #[default]
    None = 0,
    Major = 1,
    Minor = 2,
}

/// The marker drawn at a sample. Discriminants match the WinUI public contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MarkerShape {
    None = 0,
    Square = 1,
    Diamond = 2,
    Triangle = 3,
    X = 4,
    Asterisk = 5,
    ShortDash = 6,
    LongDash = 7,
    #[default]
    Circle = 8,
    Plus = 9,
}

/// Category ordering direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SortOrder {
    #[default]
    Ascending = 0,
    Descending = 1,
}

/// Stroke pattern, in multiples of effective stroke thickness.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StrokeDashStyle {
    #[default]
    Solid = 0,
    Dash = 1,
    Dot = 2,
    DashDot = 3,
    DashDotDot = 4,
}

impl StrokeDashStyle {
    /// The backend-independent dash pattern. Empty means a continuous stroke.
    pub const fn dashes(self) -> &'static [f64] {
        match self {
            Self::Solid => &[],
            Self::Dash => &[3.0, 1.0],
            Self::Dot => &[1.0, 1.0],
            Self::DashDot => &[3.0, 1.0, 1.0, 1.0],
            Self::DashDotDot => &[3.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        }
    }
}

ferro_markup_enum!(BarOrientation { Horizontal, Vertical }, { namespace: "FerroUI.Controls.Charts" });
ferro_markup_enum!(CategorySortKey { Index, Value }, { namespace: "FerroUI.Controls.Charts" });
ferro_markup_enum!(DateTimeIntervalType { Auto, Day, Week, Month, Year }, { namespace: "FerroUI.Controls.Charts" });
ferro_markup_enum!(GridLines { None, Major, Minor }, { namespace: "FerroUI.Controls.Charts" });
ferro_markup_enum!(MarkerShape { None, Square, Diamond, Triangle, X, Asterisk, ShortDash, LongDash, Circle, Plus }, { namespace: "FerroUI.Controls.Charts" });
ferro_markup_enum!(SortOrder { Ascending, Descending }, { namespace: "FerroUI.Controls.Charts" });
ferro_markup_enum!(StrokeDashStyle { Solid, Dash, Dot, DashDot, DashDotDot }, { namespace: "FerroUI.Controls.Charts" });
