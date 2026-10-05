//! The day of the week (.NET `System.DayOfWeek`).

use std::fmt;

/// A day of the week; the numeric values are the ones of .NET (`Sunday` is 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DayOfWeek {
    #[default]
    Sunday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
}

impl DayOfWeek {
    /// The seven days, starting with Sunday.
    pub const ALL: [DayOfWeek; 7] = [
        DayOfWeek::Sunday,
        DayOfWeek::Monday,
        DayOfWeek::Tuesday,
        DayOfWeek::Wednesday,
        DayOfWeek::Thursday,
        DayOfWeek::Friday,
        DayOfWeek::Saturday,
    ];

    /// The day for a numeric value 0..=6 (the C# cast `(DayOfWeek)value`);
    /// `None` for values that name no day.
    pub const fn from_i32(value: i32) -> Option<DayOfWeek> {
        if value >= 0 && value < 7 {
            Some(Self::ALL[value as usize])
        } else {
            None
        }
    }

    /// The numeric value (the C# cast `(int)day`).
    #[inline]
    pub const fn to_i32(self) -> i32 {
        self as i32
    }

    /// The English name of the day, as `Enum.ToString()` gives it.
    pub const fn name(self) -> &'static str {
        match self {
            DayOfWeek::Sunday => "Sunday",
            DayOfWeek::Monday => "Monday",
            DayOfWeek::Tuesday => "Tuesday",
            DayOfWeek::Wednesday => "Wednesday",
            DayOfWeek::Thursday => "Thursday",
            DayOfWeek::Friday => "Friday",
            DayOfWeek::Saturday => "Saturday",
        }
    }

}

impl fmt::Display for DayOfWeek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl From<DayOfWeek> for i32 {
    #[inline]
    fn from(value: DayOfWeek) -> i32 {
        value as i32
    }
}
