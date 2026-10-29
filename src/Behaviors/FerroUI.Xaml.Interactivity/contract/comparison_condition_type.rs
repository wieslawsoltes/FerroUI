//! Port of `Contract/ComparisonConditionType.cs`.

/// Represents one ternary condition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ComparisonConditionType {
    /// Specifies an equal condition.
    #[default]
    Equal,
    /// Specifies a not equal condition.
    NotEqual,
    /// Specifies a less than condition.
    LessThan,
    /// Specifies a less than or equal condition.
    LessThanOrEqual,
    /// Specifies a greater than condition.
    GreaterThan,
    /// Specifies a greater than or equal condition.
    GreaterThanOrEqual,
}

impl std::fmt::Display for ComparisonConditionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Equal => "Equal",
            Self::NotEqual => "NotEqual",
            Self::LessThan => "LessThan",
            Self::LessThanOrEqual => "LessThanOrEqual",
            Self::GreaterThan => "GreaterThan",
            Self::GreaterThanOrEqual => "GreaterThanOrEqual",
        };
        f.write_str(name)
    }
}
