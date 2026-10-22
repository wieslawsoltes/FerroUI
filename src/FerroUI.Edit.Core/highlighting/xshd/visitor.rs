use super::model::*;

/// Visitors choose their result and error types. Child traversal is explicit,
/// permitting collecting, short-circuiting and rewriting consumers.
pub trait XshdVisitor {
    type Output;
    type Error;

    fn visit_rule_set(&mut self, element: &XshdRuleSet) -> Result<Self::Output, Self::Error>;
    fn visit_color(&mut self, element: &XshdColor) -> Result<Self::Output, Self::Error>;
    fn visit_property(&mut self, element: &XshdProperty) -> Result<Self::Output, Self::Error>;
    fn visit_keywords(&mut self, element: &XshdKeywords) -> Result<Self::Output, Self::Error>;
    fn visit_span(&mut self, element: &XshdSpan) -> Result<Self::Output, Self::Error>;
    fn visit_import(&mut self, element: &XshdImport) -> Result<Self::Output, Self::Error>;
    fn visit_rule(&mut self, element: &XshdRule) -> Result<Self::Output, Self::Error>;
}

/// A mutable visitor can edit syntax nodes without interior mutability.
pub trait XshdVisitorMut {
    type Output;
    type Error;

    fn visit_rule_set(&mut self, element: &mut XshdRuleSet) -> Result<Self::Output, Self::Error>;
    fn visit_color(&mut self, element: &mut XshdColor) -> Result<Self::Output, Self::Error>;
    fn visit_property(&mut self, element: &mut XshdProperty) -> Result<Self::Output, Self::Error>;
    fn visit_keywords(&mut self, element: &mut XshdKeywords) -> Result<Self::Output, Self::Error>;
    fn visit_span(&mut self, element: &mut XshdSpan) -> Result<Self::Output, Self::Error>;
    fn visit_import(&mut self, element: &mut XshdImport) -> Result<Self::Output, Self::Error>;
    fn visit_rule(&mut self, element: &mut XshdRule) -> Result<Self::Output, Self::Error>;
}
