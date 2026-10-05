use super::activators::AndQueryActivatorBuilder;
use super::{ContainerQuery, SelectorMatch, SelectorMatchResult, StyleBase, StyleQuery, StyleQueryNode};
use crate::{StyledElement, Visual};
use std::cell::OnceCell;

/// The AND style query.
pub(crate) struct AndQuery {
    queries: Vec<StyleQuery>,
    query_string: OnceCell<String>,
}

impl AndQuery {
    /// Creates the query from the queries to AND.
    pub fn new(queries: Vec<StyleQuery>) -> Self {
        if queries.len() <= 1 {
            panic!("Need more than one query to AND.");
        }
        Self { queries, query_string: OnceCell::new() }
    }
}

impl StyleQueryNode for AndQuery {
    fn is_combinator(&self) -> bool {
        false
    }

    fn to_string(&self, owner: Option<&ContainerQuery>) -> String {
        self.query_string
            .get_or_init(|| {
                self.queries.iter().map(|x| x.to_string_with_owner(owner)).collect::<Vec<_>>().join(" and ")
            })
            .clone()
    }

    fn evaluate(
        &self,
        control: &StyledElement,
        parent: Option<&StyleBase>,
        subscribe: bool,
        container_name: Option<&str>,
    ) -> SelectorMatch {
        if !control.is::<Visual>() {
            return SelectorMatch::NEVER_THIS_TYPE;
        }

        let mut activators = AndQueryActivatorBuilder::new();
        let mut always_this_instance = false;

        for query in &self.queries {
            let match_ = query.match_(control, parent, subscribe, container_name);

            match match_.result() {
                SelectorMatchResult::AlwaysThisInstance => always_this_instance = true,
                SelectorMatchResult::NeverThisInstance | SelectorMatchResult::NeverThisType => return match_,
                SelectorMatchResult::Sometimes => activators.add(match_.into_activator()),
                SelectorMatchResult::AlwaysThisType => {}
            }
        }

        if activators.count() > 0 {
            SelectorMatch::sometimes(activators.get())
        } else if always_this_instance {
            SelectorMatch::ALWAYS_THIS_INSTANCE
        } else {
            SelectorMatch::ALWAYS_THIS_TYPE
        }
    }

    fn move_previous(&self) -> Option<&StyleQuery> {
        None
    }

    fn move_previous_or_parent(&self) -> Option<&StyleQuery> {
        None
    }
}
