use super::activators::{get_container, HeightActivator, WidthActivator};
use super::{
    Container, ContainerQuery, ContainerSizing, SelectorMatch, StyleBase, StyleQuery,
    StyleQueryComparisonOperator, StyleQueryNode, ValueStyleQuery, VisualQueryProvider,
};
use crate::{StyledElement, Visual};

/// The argument of a size query: the comparison and the size to compare
/// with.
pub(crate) type SizeQueryArgument = (StyleQueryComparisonOperator, f64);

fn is_true(actual: f64, comparison_operator: StyleQueryComparisonOperator, value: f64) -> bool {
    match comparison_operator {
        StyleQueryComparisonOperator::None => true,
        StyleQueryComparisonOperator::Equals => actual == value,
        StyleQueryComparisonOperator::LessThan => actual < value,
        StyleQueryComparisonOperator::GreaterThan => actual > value,
        StyleQueryComparisonOperator::LessThanOrEquals => actual <= value,
        StyleQueryComparisonOperator::GreaterThanOrEquals => actual >= value,
    }
}

fn evaluate_size(size: f64, argument: SizeQueryArgument) -> SelectorMatch {
    if size.is_nan() {
        return SelectorMatch::NEVER_THIS_INSTANCE;
    }

    if is_true(size, argument.0, argument.1) {
        SelectorMatch::ALWAYS_THIS_INSTANCE
    } else {
        SelectorMatch::NEVER_THIS_INSTANCE
    }
}

fn evaluate_unsubscribed(
    visual: &Visual,
    container_name: Option<&str>,
    evaluate: impl FnOnce(&VisualQueryProvider) -> SelectorMatch,
) -> SelectorMatch {
    if let Some(container) = get_container(visual, container_name) {
        if let Some(query_provider) = Container::get_query_provider(&container) {
            if Container::get_sizing(&container) == ContainerSizing::WidthAndHeight {
                return evaluate(&query_provider);
            }
        }
    }

    SelectorMatch::NEVER_THIS_INSTANCE
}

/// A query on the width of a container.
pub(crate) struct WidthQuery(ValueStyleQuery<SizeQueryArgument>);

impl WidthQuery {
    pub fn new(previous: Option<StyleQuery>, operator: StyleQueryComparisonOperator, value: f64) -> Self {
        WidthQuery(ValueStyleQuery::new(previous, (operator, value)))
    }

    pub(crate) fn evaluate_provider(query_provider: &VisualQueryProvider, argument: SizeQueryArgument) -> SelectorMatch {
        evaluate_size(query_provider.width(), argument)
    }
}

impl StyleQueryNode for WidthQuery {
    fn is_combinator(&self) -> bool {
        false
    }

    fn evaluate(
        &self,
        control: &StyledElement,
        _parent: Option<&StyleBase>,
        subscribe: bool,
        container_name: Option<&str>,
    ) -> SelectorMatch {
        let Some(visual) = control.downcast_ref::<Visual>() else {
            return SelectorMatch::NEVER_THIS_TYPE;
        };

        let argument = *self.0.argument();

        if subscribe {
            return SelectorMatch::sometimes(WidthActivator::new(visual, argument, container_name));
        }

        evaluate_unsubscribed(visual, container_name, |provider| Self::evaluate_provider(provider, argument))
    }

    fn to_string(&self, _owner: Option<&ContainerQuery>) -> String {
        let argument = self.0.argument();
        let prop = match argument.0 {
            StyleQueryComparisonOperator::None => "",
            StyleQueryComparisonOperator::Equals => "width",
            StyleQueryComparisonOperator::LessThan => "",
            StyleQueryComparisonOperator::GreaterThan => "",
            StyleQueryComparisonOperator::LessThanOrEquals => "max-width",
            StyleQueryComparisonOperator::GreaterThanOrEquals => "min-width",
        };

        format!("{prop}:{}", argument.1)
    }

    fn move_previous(&self) -> Option<&StyleQuery> {
        self.0.previous()
    }

    fn move_previous_or_parent(&self) -> Option<&StyleQuery> {
        self.0.previous()
    }
}

/// A query on the height of a container.
pub(crate) struct HeightQuery(ValueStyleQuery<SizeQueryArgument>);

impl HeightQuery {
    pub fn new(previous: Option<StyleQuery>, operator: StyleQueryComparisonOperator, value: f64) -> Self {
        HeightQuery(ValueStyleQuery::new(previous, (operator, value)))
    }

    pub(crate) fn evaluate_provider(query_provider: &VisualQueryProvider, argument: SizeQueryArgument) -> SelectorMatch {
        evaluate_size(query_provider.height(), argument)
    }
}

impl StyleQueryNode for HeightQuery {
    fn is_combinator(&self) -> bool {
        false
    }

    fn evaluate(
        &self,
        control: &StyledElement,
        _parent: Option<&StyleBase>,
        subscribe: bool,
        container_name: Option<&str>,
    ) -> SelectorMatch {
        let Some(visual) = control.downcast_ref::<Visual>() else {
            return SelectorMatch::NEVER_THIS_TYPE;
        };

        let argument = *self.0.argument();

        if subscribe {
            return SelectorMatch::sometimes(HeightActivator::new(visual, argument, container_name));
        }

        evaluate_unsubscribed(visual, container_name, |provider| Self::evaluate_provider(provider, argument))
    }

    fn to_string(&self, _owner: Option<&ContainerQuery>) -> String {
        let argument = self.0.argument();
        let prop = match argument.0 {
            StyleQueryComparisonOperator::None => "",
            StyleQueryComparisonOperator::Equals => "height",
            StyleQueryComparisonOperator::LessThan => "",
            StyleQueryComparisonOperator::GreaterThan => "",
            StyleQueryComparisonOperator::LessThanOrEquals => "max-height",
            StyleQueryComparisonOperator::GreaterThanOrEquals => "min-height",
        };

        format!("{prop}:{}", argument.1)
    }

    fn move_previous(&self) -> Option<&StyleQuery> {
        self.0.previous()
    }

    fn move_previous_or_parent(&self) -> Option<&StyleQuery> {
        self.0.previous()
    }
}
