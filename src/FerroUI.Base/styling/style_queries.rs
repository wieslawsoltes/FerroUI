use super::{AndQuery, HeightQuery, OrQuery, StyleQuery, StyleQueryComparisonOperator, WidthQuery};

/// Functions for building [`StyleQuery`]s.
pub struct StyleQueries;

impl StyleQueries {
    /// Returns a query which matches the width of the container.
    pub fn width(previous: Option<StyleQuery>, operator: StyleQueryComparisonOperator, value: f64) -> StyleQuery {
        StyleQuery::new(WidthQuery::new(previous, operator, value))
    }

    /// Returns a query which matches the height of the container.
    pub fn height(previous: Option<StyleQuery>, operator: StyleQueryComparisonOperator, value: f64) -> StyleQuery {
        StyleQuery::new(HeightQuery::new(previous, operator, value))
    }

    /// Returns a query which ORs queries.
    pub fn or(queries: impl IntoIterator<Item = StyleQuery>) -> StyleQuery {
        StyleQuery::new(OrQuery::new(queries.into_iter().collect()))
    }

    /// Returns a query which ANDs queries.
    pub fn and(queries: impl IntoIterator<Item = StyleQuery>) -> StyleQuery {
        StyleQuery::new(AndQuery::new(queries.into_iter().collect()))
    }
}

impl StyleQuery {
    /// Returns a query which additionally matches the width of the
    /// container.
    pub fn width(self, operator: StyleQueryComparisonOperator, value: f64) -> StyleQuery {
        StyleQueries::width(Some(self), operator, value)
    }

    /// Returns a query which additionally matches the height of the
    /// container.
    pub fn height(self, operator: StyleQueryComparisonOperator, value: f64) -> StyleQuery {
        StyleQueries::height(Some(self), operator, value)
    }
}
