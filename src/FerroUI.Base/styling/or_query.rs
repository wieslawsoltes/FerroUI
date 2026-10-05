use super::activators::OrQueryActivatorBuilder;
use super::{ContainerQuery, SelectorMatch, SelectorMatchResult, StyleBase, StyleQuery, StyleQueryNode};
use crate::{StyledElement, Visual};
use std::cell::OnceCell;

/// The OR style query.
pub(crate) struct OrQuery {
    queries: Vec<StyleQuery>,
    query_string: OnceCell<String>,
}

impl OrQuery {
    /// Creates the query from the queries to OR.
    pub fn new(queries: Vec<StyleQuery>) -> Self {
        if queries.len() <= 1 {
            panic!("Need more than one query to OR.");
        }
        Self { queries, query_string: OnceCell::new() }
    }
}

impl StyleQueryNode for OrQuery {
    fn is_combinator(&self) -> bool {
        false
    }

    fn to_string(&self, owner: Option<&ContainerQuery>) -> String {
        self.query_string
            .get_or_init(|| self.queries.iter().map(|x| x.to_string_with_owner(owner)).collect::<Vec<_>>().join(", "))
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

        let mut activators = OrQueryActivatorBuilder::new();
        let mut never_this_instance = false;

        for query in &self.queries {
            let match_ = query.match_(control, parent, subscribe, container_name);

            match match_.result() {
                SelectorMatchResult::AlwaysThisType | SelectorMatchResult::AlwaysThisInstance => return match_,
                SelectorMatchResult::NeverThisInstance => never_this_instance = true,
                SelectorMatchResult::Sometimes => activators.add(match_.into_activator()),
                SelectorMatchResult::NeverThisType => {}
            }
        }

        if activators.count() > 0 {
            SelectorMatch::sometimes(activators.get())
        } else if never_this_instance {
            SelectorMatch::NEVER_THIS_INSTANCE
        } else {
            SelectorMatch::NEVER_THIS_TYPE
        }
    }

    fn move_previous(&self) -> Option<&StyleQuery> {
        None
    }

    fn move_previous_or_parent(&self) -> Option<&StyleQuery> {
        None
    }
}
