use super::activators::OrActivatorBuilder;
use super::{Selector, SelectorMatch, SelectorMatchResult, SelectorNode, Style, StyleBase};
use crate::{StyledElement, TypeInfo};
use std::any::Any;
use std::cell::OnceCell;

/// The descendant combinator: matches a control with a logical ancestor that
/// matches the preceding selector.
pub(crate) struct DescendantSelector {
    parent: Selector,
    selector_string: OnceCell<String>,
}

impl DescendantSelector {
    pub fn new(parent: Option<Selector>) -> Self {
        match parent {
            Some(parent) => Self { parent, selector_string: OnceCell::new() },
            None => panic!("Descendant selector must be preceded by a selector."),
        }
    }
}

impl SelectorNode for DescendantSelector {
    fn in_template(&self) -> bool {
        self.parent.in_template()
    }

    fn is_combinator(&self) -> bool {
        true
    }

    fn target_type(&self) -> Option<&'static TypeInfo> {
        None
    }

    fn to_string(&self, owner: Option<&Style>) -> String {
        self.selector_string
            .get_or_init(|| format!("{} ", self.parent.to_string_with_next(owner, true)))
            .clone()
    }

    fn evaluate(&self, control: &StyledElement, parent: Option<&StyleBase>, subscribe: bool) -> SelectorMatch {
        let mut descendant_matches = OrActivatorBuilder::new();
        let mut c = control.parent();

        while let Some(s) = c {
            let match_ = self.parent.match_(&s, parent, subscribe);

            if match_.result() == SelectorMatchResult::Sometimes {
                descendant_matches.add(match_.into_activator());
            } else if match_.is_match() {
                return SelectorMatch::ALWAYS_THIS_INSTANCE;
            }

            c = s.parent();
        }

        if descendant_matches.count() > 0 {
            SelectorMatch::sometimes(descendant_matches.get())
        } else {
            SelectorMatch::NEVER_THIS_INSTANCE
        }
    }

    fn move_previous(&self) -> Option<&Selector> {
        None
    }

    fn move_previous_or_parent(&self) -> Option<&Selector> {
        Some(&self.parent)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
