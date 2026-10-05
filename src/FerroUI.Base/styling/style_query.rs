use super::activators::AndActivatorBuilder;
use super::{ContainerQuery, SelectorMatch, SelectorMatchResult, StyleBase};
use crate::StyledElement;
use std::fmt;
use std::rc::Rc;

/// The behaviour of one kind of style query.
pub(crate) trait StyleQueryNode: 'static {
    /// Whether this query is a combinator.
    ///
    /// A combinator is a query such as child or descendent which links simple
    /// queries.
    fn is_combinator(&self) -> bool;

    /// Gets a string representing the query, with the nesting separator
    /// (`^`) replaced with the parent query.
    fn to_string(&self, owner: Option<&ContainerQuery>) -> String;

    /// Evaluates the query for a match.
    fn evaluate(
        &self,
        control: &StyledElement,
        parent: Option<&StyleBase>,
        subscribe: bool,
        container_name: Option<&str>,
    ) -> SelectorMatch;

    /// Moves to the previous query.
    fn move_previous(&self) -> Option<&StyleQuery>;

    /// Moves to the previous query or the parent query.
    #[allow(dead_code)]
    fn move_previous_or_parent(&self) -> Option<&StyleQuery>;
}

/// A query in a [`ContainerQuery`].
///
/// Queries are built with the functions of [`StyleQueries`](super::StyleQueries)
/// and the chaining methods of this type.
#[derive(Clone)]
pub struct StyleQuery(pub(crate) Rc<dyn StyleQueryNode>);


/// Compares by identity (reference equality), as the reference type this
/// mirrors.
impl PartialEq for StyleQuery {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl StyleQuery {
    pub(crate) fn new(node: impl StyleQueryNode) -> Self {
        StyleQuery(Rc::new(node))
    }

    #[inline]
    pub(crate) fn is_combinator(&self) -> bool {
        self.0.is_combinator()
    }

    /// Tries to match the query with a control.
    ///
    /// `parent` is the parent container, if the container holding the query
    /// is a nested container. With `subscribe`, a match whose state can
    /// change over time carries an activator that tracks it; without, the
    /// match is only evaluated for the current state. `container_name` is the
    /// name of the container to query.
    pub fn match_(
        &self,
        control: &StyledElement,
        parent: Option<&StyleBase>,
        subscribe: bool,
        container_name: Option<&str>,
    ) -> SelectorMatch {
        // First match the query until a combinator is found. Queries are
        // stored from right-to-left, so the recursion reverses this order
        // because the type query will be on the left.
        let mut combinator = None;
        let mut activators = AndActivatorBuilder::new();
        let result =
            Self::match_query(control, self, parent, subscribe, &mut activators, &mut combinator, container_name);
        let mut match_ = if result == SelectorMatchResult::Sometimes {
            SelectorMatch::sometimes(activators.get())
        } else {
            SelectorMatch::new(result)
        };

        // If the pre-combinator query matches, we can now match the
        // combinator, if any.
        if match_.is_match() {
            if let Some(combinator) = combinator {
                match_ = match_.and(combinator.match_(control, parent, subscribe, container_name));

                // If we have a combinator then we can never say that we always
                // match a control of this type, because by definition the
                // combinator matches on things outside of the control.
                match_ = match match_.result() {
                    SelectorMatchResult::AlwaysThisType => SelectorMatch::ALWAYS_THIS_INSTANCE,
                    SelectorMatchResult::NeverThisType => SelectorMatch::NEVER_THIS_INSTANCE,
                    _ => match_,
                };
            }
        }

        match_
    }

    /// Evaluates the query for a match.
    pub(crate) fn evaluate(
        &self,
        control: &StyledElement,
        parent: Option<&StyleBase>,
        subscribe: bool,
        container_name: Option<&str>,
    ) -> SelectorMatch {
        self.0.evaluate(control, parent, subscribe, container_name)
    }

    /// Gets a string representing the query.
    pub fn to_string_with_owner(&self, owner: Option<&ContainerQuery>) -> String {
        self.0.to_string(owner)
    }

    fn match_query<'a>(
        control: &StyledElement,
        query: &'a StyleQuery,
        parent: Option<&StyleBase>,
        subscribe: bool,
        activators: &mut AndActivatorBuilder,
        combinator: &mut Option<&'a StyleQuery>,
        container_name: Option<&str>,
    ) -> SelectorMatchResult {
        let previous = query.0.move_previous();

        // Queries are stored from right-to-left, so we recurse into the query
        // in order to reverse this order, because the type query will be on
        // the left and is our best opportunity to exit early.
        if let Some(previous) = previous {
            if !previous.is_combinator() {
                let previous_match =
                    Self::match_query(control, previous, parent, subscribe, activators, combinator, container_name);

                if previous_match < SelectorMatchResult::Sometimes {
                    return previous_match;
                }
            }
        }

        // Match this query.
        let match_ = query.0.evaluate(control, parent, subscribe, container_name);

        if !match_.is_match() {
            *combinator = None;
            return match_.result();
        }

        let result = match_.result();
        activators.add(match_.into_activator());

        if let Some(previous) = previous {
            if previous.is_combinator() {
                *combinator = Some(previous);
            }
        }

        result
    }
}

impl fmt::Display for StyleQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string(None))
    }
}

impl fmt::Debug for StyleQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.to_string(None))
    }
}
