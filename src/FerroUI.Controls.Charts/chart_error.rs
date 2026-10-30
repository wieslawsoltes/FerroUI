use std::fmt;

/// A rejected chart mutation. Failure preserves the previous state and does not
/// publish a collection or property notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartError {
    InvalidValue(&'static str),
    IndexOutOfBounds,
    DuplicateItem,
    AlreadyOwned,
    AxisNotInChart,
    AxisInUse,
}

impl fmt::Display for ChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidValue(message) => message,
            Self::IndexOutOfBounds => "The collection index is out of range.",
            Self::DuplicateItem => "The chart already contains this item.",
            Self::AlreadyOwned => "The item belongs to another chart.",
            Self::AxisNotInChart => "Add the explicit axis to the chart before adding its series.",
            Self::AxisInUse => "The axis is still referenced by a series in this chart.",
        })
    }
}

impl std::error::Error for ChartError {}
