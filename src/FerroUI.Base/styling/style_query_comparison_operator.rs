/// The comparison applied by a size query.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StyleQueryComparisonOperator {
    #[default]
    None,
    Equals,
    LessThan,
    GreaterThan,
    LessThanOrEquals,
    GreaterThanOrEquals,
}
